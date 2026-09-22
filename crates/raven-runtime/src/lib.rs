//! The foundation loop.
//!
//! A model never calls a tool directly. The runtime plans, asks policy, then
//! invokes a registered tool and verifies the evidence.

use raven_core::{Capability, Goal, GoalState, Phase, PolicyDecision};
use raven_events::EventLog;
use raven_execution::Execution;
use raven_policy::PolicyEngine;
use raven_tools::Registry;
use raven_verification::{verify, ToolOutcome, Verification};
use std::collections::HashMap;

pub struct PlannedStep {
    pub capability_id: String,
    pub summary: String,
    pub expected_claim: String,
    pub input: String,
}

pub struct Plan {
    pub steps: Vec<PlannedStep>,
}

pub trait Planner {
    fn plan(&self, goal: &Goal, available: &[Capability]) -> Plan;
}

pub trait Tool {
    fn id(&self) -> &str;
    fn invoke(&mut self, input: &str) -> ToolOutcome;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TraceEntry {
    pub phase: Phase,
    pub summary: String,
    pub decision: Option<PolicyDecision>,
    pub verification: Option<Verification>,
}

pub struct RunReport {
    pub goal: Goal,
    pub state: GoalState,
    pub trace: Vec<TraceEntry>,
    pub executed: Vec<String>,
    pub events: EventLog,
}

pub struct Runtime {
    registry: Registry,
    policy: PolicyEngine,
    tools: HashMap<String, Box<dyn Tool>>,
}

impl Runtime {
    pub fn new(registry: Registry, policy: PolicyEngine) -> Self {
        Self {
            registry,
            policy,
            tools: HashMap::new(),
        }
    }

    pub fn insert_tool(&mut self, tool: Box<dyn Tool>) {
        self.tools.insert(tool.id().to_string(), tool);
    }

    pub fn run(&mut self, goal: Goal, planner: &dyn Planner) -> RunReport {
        let mut execution = Execution::new();
        let mut trace = Vec::new();
        let mut executed = Vec::new();
        let mut events = EventLog::default();

        let goal_id = goal.id.clone();
        events.publish("goal.created", goal_id, Some(Phase::Perceive));
        if !advance(&mut execution, GoalState::Understanding, &mut trace) {
            return finish(goal, execution, trace, executed, events);
        }
        trace.push(entry(
            Phase::Perceive,
            format!("intent: {}", goal.intent),
            None,
            None,
        ));

        if !advance(&mut execution, GoalState::Planning, &mut trace) {
            return finish(goal, execution, trace, executed, events);
        }
        let available = self.registry.available();
        let plan = planner.plan(&goal, &available);
        trace.push(entry(
            Phase::Plan,
            format!("{} step(s)", plan.steps.len()),
            None,
            None,
        ));

        if !advance(&mut execution, GoalState::Authorized, &mut trace) {
            return finish(goal, execution, trace, executed, events);
        }

        for step in &plan.steps {
            let Some(capability) = self.registry.get(&step.capability_id).cloned() else {
                trace.push(entry(
                    Phase::Act,
                    format!("unknown capability {}", step.capability_id),
                    Some(PolicyDecision::Deny),
                    None,
                ));
                let _ = advance(&mut execution, GoalState::Failed, &mut trace);
                break;
            };

            let decision = self.policy.evaluate(&capability);
            match decision {
                PolicyDecision::Deny => {
                    trace.push(entry(
                        Phase::Act,
                        format!("{} denied", step.capability_id),
                        Some(decision),
                        None,
                    ));
                    let _ = advance(&mut execution, GoalState::Failed, &mut trace);
                    break;
                }
                PolicyDecision::AskUser => {
                    trace.push(entry(
                        Phase::Act,
                        format!("{} needs approval — {}", step.capability_id, step.summary),
                        Some(decision),
                        None,
                    ));
                    events.publish("policy.ask", step.capability_id.clone(), Some(Phase::Act));
                    let _ = advance(&mut execution, GoalState::WaitingForUser, &mut trace);
                    break;
                }
                PolicyDecision::Allow | PolicyDecision::AllowWithPolicy => {
                    if execution.state() != GoalState::Executing
                        && !advance(&mut execution, GoalState::Executing, &mut trace)
                    {
                        break;
                    }
                    let Some(tool) = self.tools.get_mut(&step.capability_id) else {
                        trace.push(entry(
                            Phase::Act,
                            format!("no tool bound for {}", step.capability_id),
                            Some(decision),
                            None,
                        ));
                        let _ = advance(&mut execution, GoalState::Failed, &mut trace);
                        break;
                    };
                    let outcome = tool.invoke(&step.input);
                    executed.push(step.capability_id.clone());
                    events.publish("tool.invoked", step.capability_id.clone(), Some(Phase::Act));
                    if !advance(&mut execution, GoalState::Verifying, &mut trace) {
                        break;
                    }
                    let verdict = verify(&step.expected_claim, &outcome);
                    let confirmed = verdict.is_confirmed();
                    trace.push(entry(
                        Phase::Verify,
                        format!("{} — {}", step.capability_id, outcome.summary),
                        Some(decision),
                        Some(verdict),
                    ));
                    if !confirmed {
                        events.publish(
                            "verify.rejected",
                            step.capability_id.clone(),
                            Some(Phase::Adapt),
                        );
                        let _ = advance(&mut execution, GoalState::Recovering, &mut trace);
                        trace.push(entry(
                            Phase::Adapt,
                            format!("{} did not verify; stopped", step.capability_id),
                            None,
                            None,
                        ));
                        break;
                    }
                }
            }
        }

        if execution.state() == GoalState::Verifying {
            let _ = advance(&mut execution, GoalState::Completed, &mut trace);
            events.publish("goal.completed", goal.id.clone(), Some(Phase::Verify));
        }

        finish(goal, execution, trace, executed, events)
    }
}

fn advance(execution: &mut Execution, to: GoalState, trace: &mut Vec<TraceEntry>) -> bool {
    if execution.transition(to).is_ok() {
        true
    } else {
        trace.push(entry(
            Phase::Adapt,
            format!("illegal transition to {to:?}"),
            None,
            None,
        ));
        false
    }
}

fn entry(
    phase: Phase,
    summary: impl Into<String>,
    decision: Option<PolicyDecision>,
    verification: Option<Verification>,
) -> TraceEntry {
    TraceEntry {
        phase,
        summary: summary.into(),
        decision,
        verification,
    }
}

fn finish(
    goal: Goal,
    execution: Execution,
    trace: Vec<TraceEntry>,
    executed: Vec<String>,
    events: EventLog,
) -> RunReport {
    RunReport {
        goal,
        state: execution.state(),
        trace,
        executed,
        events,
    }
}

struct StaticPlanner {
    steps: Vec<PlannedStep>,
}

impl Planner for StaticPlanner {
    fn plan(&self, _goal: &Goal, available: &[Capability]) -> Plan {
        let steps = self
            .steps
            .iter()
            .filter(|step| available.iter().any(|cap| cap.id == step.capability_id))
            .map(|step| PlannedStep {
                capability_id: step.capability_id.clone(),
                summary: step.summary.clone(),
                expected_claim: step.expected_claim.clone(),
                input: step.input.clone(),
            })
            .collect();
        Plan { steps }
    }
}

struct FixtureTool {
    id: String,
    claim: String,
    summary: String,
}

impl Tool for FixtureTool {
    fn id(&self) -> &str {
        &self.id
    }

    fn invoke(&mut self, _input: &str) -> ToolOutcome {
        ToolOutcome::verified(self.summary.clone(), self.claim.clone(), self.id.clone())
    }
}

/// Canonical demonstration: "Prepare my day."
///
/// Reads stay local and verify. Sending a message stops and asks.
/// Nothing leaves the process.
pub fn prepare_my_day() -> RunReport {
    use raven_core::{Autonomy, Capability, RiskLevel};

    let mut registry = Registry::default();
    for (id, description, risk) in [
        ("read_calendar", "Read today's calendar", RiskLevel::L0),
        ("read_tasks", "Read open tasks", RiskLevel::L0),
        ("read_weather", "Read local weather", RiskLevel::L0),
        ("read_location", "Read coarse location", RiskLevel::L0),
        ("send_message", "Send a message", RiskLevel::L2),
    ] {
        registry.register(Capability::new(id, description, risk));
    }

    let mut runtime = Runtime::new(registry, PolicyEngine::new(Autonomy::Suggest));
    for (id, claim, summary) in [
        ("read_calendar", "calendar.loaded", "calendar inspected"),
        ("read_tasks", "tasks.loaded", "tasks inspected"),
        ("read_weather", "weather.loaded", "weather inspected"),
        ("read_location", "location.loaded", "location inspected"),
        (
            "send_message",
            "message.sent",
            "message handed to a transport",
        ),
    ] {
        runtime.insert_tool(Box::new(FixtureTool {
            id: id.to_string(),
            claim: claim.to_string(),
            summary: summary.to_string(),
        }));
    }

    let planner = StaticPlanner {
        steps: vec![
            step("read_calendar", "Inspect the calendar", "calendar.loaded"),
            step("read_tasks", "Inspect open tasks", "tasks.loaded"),
            step("read_weather", "Check the weather", "weather.loaded"),
            step("read_location", "Check where you are", "location.loaded"),
            step("send_message", "Send the day summary", "message.sent"),
        ],
    };

    runtime.run(
        Goal::from_intent("prepare-my-day", "Prepare my day."),
        &planner,
    )
}

fn step(id: &str, summary: &str, claim: &str) -> PlannedStep {
    PlannedStep {
        capability_id: id.to_string(),
        summary: summary.to_string(),
        expected_claim: claim.to_string(),
        input: String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use raven_core::{GoalState, Phase, PolicyDecision};

    #[test]
    fn prepare_my_day_verifies_reads_and_asks_before_sending() {
        let report = prepare_my_day();
        assert_eq!(report.state, GoalState::WaitingForUser);
        assert_eq!(
            report.executed,
            vec![
                "read_calendar".to_string(),
                "read_tasks".to_string(),
                "read_weather".to_string(),
                "read_location".to_string(),
            ]
        );
        assert!(!report.executed.iter().any(|id| id == "send_message"));
        assert!(report.trace.iter().any(|entry| {
            entry.summary.contains("send_message")
                && entry.decision == Some(PolicyDecision::AskUser)
        }));
        assert!(report.trace.iter().any(|entry| {
            entry.phase == Phase::Verify
                && entry
                    .verification
                    .as_ref()
                    .is_some_and(|v| v.is_confirmed())
        }));
    }
}
