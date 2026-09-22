//! The foundation loop.
//!
//! A model never calls a tool directly. The runtime plans, asks policy, then
//! invokes a registered tool and verifies the evidence. Runs can pause with a
//! durable checkpoint, resume after a human reply, or stop on cancellation.

mod planner;

pub use planner::{
    CapabilityBrief, ModelAdapter, ModelPlanner, PlanError, PlanProposal, PlanRequest,
    ProposedStep, ScriptedModel, StaticPlanner, MAX_PLAN_STEPS,
};

use raven_core::{Capability, Goal, GoalState, Phase, PolicyDecision};
use raven_events::EventLog;
use raven_execution::{CancellationToken, Execution};
use raven_policy::PolicyEngine;
use raven_tools::Registry;
use raven_verification::{verify, ToolOutcome, Verification};
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlannedStep {
    pub capability_id: String,
    pub summary: String,
    pub expected_claim: String,
    pub input: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Plan {
    pub steps: Vec<PlannedStep>,
}

/// Produces an ordered plan for a goal. Prefer [`ModelPlanner`] with a host
/// [`ModelAdapter`] so the intelligence layer stays replaceable.
pub trait Planner {
    fn plan(&self, goal: &Goal, available: &[Capability]) -> Result<Plan, PlanError>;

    /// Identifier of the model or strategy behind this planner, when any.
    fn provider(&self) -> Option<&str> {
        None
    }
}

pub trait Tool: Send {
    fn id(&self) -> &str;
    fn invoke(&mut self, input: &str) -> ToolOutcome;
}

/// What the human answered when the run was waiting.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UserReply {
    /// Authorize the pending capability and continue.
    Approve,
    /// Refuse the pending capability; the goal fails.
    Deny,
    /// Abandon the goal.
    Cancel,
}

/// Capability that stopped the run for approval.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PendingApproval {
    pub capability_id: String,
    pub summary: String,
    pub expected_claim: String,
    pub input: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TraceEntry {
    pub phase: Phase,
    pub summary: String,
    pub decision: Option<PolicyDecision>,
    pub verification: Option<Verification>,
}

/// Durable snapshot of a run. A host may hold this across process or UI
/// boundaries and later call [`Runtime::resume`].
#[derive(Clone, Debug)]
pub struct RunCheckpoint {
    pub goal: Goal,
    pub state: GoalState,
    pub steps: Vec<PlannedStep>,
    pub next_index: usize,
    pub executed: Vec<String>,
    pub pending: Option<PendingApproval>,
    pub trace: Vec<TraceEntry>,
    pub events: EventLog,
}

impl RunCheckpoint {
    pub fn is_waiting(&self) -> bool {
        self.state == GoalState::WaitingForUser && self.pending.is_some()
    }

    pub fn is_terminal(&self) -> bool {
        matches!(
            self.state,
            GoalState::Completed | GoalState::Failed | GoalState::Cancelled
        )
    }

    pub fn into_report(self) -> RunReport {
        RunReport {
            goal: self.goal,
            state: self.state,
            trace: self.trace,
            executed: self.executed,
            events: self.events,
            pending: self.pending,
        }
    }
}

pub struct RunReport {
    pub goal: Goal,
    pub state: GoalState,
    pub trace: Vec<TraceEntry>,
    pub executed: Vec<String>,
    pub events: EventLog,
    pub pending: Option<PendingApproval>,
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

    /// Run until completed, failed, cancelled, or waiting for the user.
    pub fn run(&mut self, goal: Goal, planner: &dyn Planner) -> RunReport {
        self.run_with_cancel(goal, planner, &CancellationToken::new())
            .into_report()
    }

    pub fn run_with_cancel(
        &mut self,
        goal: Goal,
        planner: &dyn Planner,
        cancel: &CancellationToken,
    ) -> RunCheckpoint {
        let mut execution = Execution::new();
        let mut trace = Vec::new();
        let executed = Vec::new();
        let mut events = EventLog::default();

        events.publish("goal.created", goal.id.clone(), Some(Phase::Perceive));
        if cancel.check().is_err() {
            let _ = advance(&mut execution, GoalState::Cancelled, &mut trace);
            events.publish("goal.cancelled", goal.id.clone(), Some(Phase::Adapt));
            return checkpoint(goal, execution, Vec::new(), 0, executed, None, trace, events);
        }

        if !advance(&mut execution, GoalState::Understanding, &mut trace) {
            return checkpoint(goal, execution, Vec::new(), 0, executed, None, trace, events);
        }
        trace.push(entry(
            Phase::Perceive,
            format!("intent: {}", goal.intent),
            None,
            None,
        ));

        if !advance(&mut execution, GoalState::Planning, &mut trace) {
            return checkpoint(goal, execution, Vec::new(), 0, executed, None, trace, events);
        }
        if let Some(provider) = planner.provider() {
            trace.push(entry(
                Phase::Reason,
                format!("planner via {provider}"),
                None,
                None,
            ));
        }
        let available = self.registry.available();
        let steps = match planner.plan(&goal, &available) {
            Ok(plan) => {
                trace.push(entry(
                    Phase::Plan,
                    format!("{} step(s)", plan.steps.len()),
                    None,
                    None,
                ));
                plan.steps
            }
            Err(error) => {
                let summary = match &error {
                    PlanError::Unavailable { message } => {
                        format!("planner unavailable: {message}")
                    }
                    PlanError::Invalid { message } => format!("planner invalid: {message}"),
                };
                trace.push(entry(Phase::Plan, summary, None, None));
                events.publish("plan.failed", format!("{error:?}"), Some(Phase::Plan));
                let _ = advance(&mut execution, GoalState::Failed, &mut trace);
                return checkpoint(goal, execution, Vec::new(), 0, executed, None, trace, events);
            }
        };

        if !advance(&mut execution, GoalState::Authorized, &mut trace) {
            return checkpoint(goal, execution, steps, 0, executed, None, trace, events);
        }

        self.drive(
            goal,
            execution,
            steps,
            0,
            executed,
            None,
            trace,
            events,
            cancel,
        )
    }

    /// Continue from a checkpoint after a human reply or a cancel signal.
    pub fn resume(
        &mut self,
        checkpoint: RunCheckpoint,
        reply: UserReply,
        cancel: &CancellationToken,
    ) -> RunCheckpoint {
        if checkpoint.is_terminal() {
            return checkpoint;
        }

        let RunCheckpoint {
            goal,
            state,
            steps,
            next_index,
            mut executed,
            pending,
            mut trace,
            mut events,
        } = checkpoint;

        let mut execution = Execution::from_state(state);

        if cancel.is_cancelled() || reply == UserReply::Cancel {
            cancel.cancel();
            if execution.state() != GoalState::Cancelled {
                let _ = advance(&mut execution, GoalState::Cancelled, &mut trace);
            }
            events.publish("goal.cancelled", goal.id.clone(), Some(Phase::Adapt));
            trace.push(entry(
                Phase::Adapt,
                "run cancelled",
                None,
                None,
            ));
            return checkpoint_from(goal, execution, steps, next_index, executed, None, trace, events);
        }

        if execution.state() != GoalState::WaitingForUser {
            return checkpoint_from(goal, execution, steps, next_index, executed, pending, trace, events);
        }

        let Some(pending) = pending else {
            let _ = advance(&mut execution, GoalState::Failed, &mut trace);
            return checkpoint_from(goal, execution, steps, next_index, executed, None, trace, events);
        };

        match reply {
            UserReply::Deny => {
                trace.push(entry(
                    Phase::Act,
                    format!("{} denied by user", pending.capability_id),
                    Some(PolicyDecision::Deny),
                    None,
                ));
                events.publish(
                    "policy.denied",
                    pending.capability_id.clone(),
                    Some(Phase::Act),
                );
                let _ = advance(&mut execution, GoalState::Failed, &mut trace);
                checkpoint_from(goal, execution, steps, next_index, executed, None, trace, events)
            }
            UserReply::Approve => {
                trace.push(entry(
                    Phase::Act,
                    format!("{} approved by user", pending.capability_id),
                    Some(PolicyDecision::Allow),
                    None,
                ));
                events.publish(
                    "policy.approved",
                    pending.capability_id.clone(),
                    Some(Phase::Act),
                );
                if !advance(&mut execution, GoalState::Authorized, &mut trace) {
                    return checkpoint_from(
                        goal, execution, steps, next_index, executed, None, trace, events,
                    );
                }
                // Execute the approved step, then continue the remainder.
                let approved_index = next_index;
                let remainder_index = next_index + 1;
                match self.invoke_step(
                    &pending.into_step(),
                    &mut execution,
                    &mut executed,
                    &mut trace,
                    &mut events,
                    PolicyDecision::Allow,
                ) {
                    StepResult::Continue => self.drive(
                        goal,
                        execution,
                        steps,
                        remainder_index,
                        executed,
                        None,
                        trace,
                        events,
                        cancel,
                    ),
                    StepResult::Stop => checkpoint_from(
                        goal,
                        execution,
                        steps,
                        approved_index,
                        executed,
                        None,
                        trace,
                        events,
                    ),
                }
            }
            UserReply::Cancel => unreachable!("handled above"),
        }
    }

    fn drive(
        &mut self,
        goal: Goal,
        mut execution: Execution,
        steps: Vec<PlannedStep>,
        start_index: usize,
        mut executed: Vec<String>,
        mut pending: Option<PendingApproval>,
        mut trace: Vec<TraceEntry>,
        mut events: EventLog,
        cancel: &CancellationToken,
    ) -> RunCheckpoint {
        let mut index = start_index;
        while index < steps.len() {
            if cancel.check().is_err() {
                let _ = advance(&mut execution, GoalState::Cancelled, &mut trace);
                events.publish("goal.cancelled", goal.id.clone(), Some(Phase::Adapt));
                trace.push(entry(Phase::Adapt, "run cancelled", None, None));
                return checkpoint_from(
                    goal, execution, steps, index, executed, None, trace, events,
                );
            }

            let step = &steps[index];
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
                    pending = Some(PendingApproval {
                        capability_id: step.capability_id.clone(),
                        summary: step.summary.clone(),
                        expected_claim: step.expected_claim.clone(),
                        input: step.input.clone(),
                    });
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
                    match self.invoke_step(
                        step,
                        &mut execution,
                        &mut executed,
                        &mut trace,
                        &mut events,
                        decision,
                    ) {
                        StepResult::Continue => index += 1,
                        StepResult::Stop => {
                            return checkpoint_from(
                                goal, execution, steps, index, executed, None, trace, events,
                            );
                        }
                    }
                }
            }
        }

        if execution.state() == GoalState::Verifying {
            let _ = advance(&mut execution, GoalState::Completed, &mut trace);
            events.publish("goal.completed", goal.id.clone(), Some(Phase::Verify));
        }

        checkpoint_from(goal, execution, steps, index, executed, pending, trace, events)
    }

    fn invoke_step(
        &mut self,
        step: &PlannedStep,
        execution: &mut Execution,
        executed: &mut Vec<String>,
        trace: &mut Vec<TraceEntry>,
        events: &mut EventLog,
        decision: PolicyDecision,
    ) -> StepResult {
        if execution.state() != GoalState::Executing
            && !advance(execution, GoalState::Executing, trace)
        {
            return StepResult::Stop;
        }
        let Some(tool) = self.tools.get_mut(&step.capability_id) else {
            trace.push(entry(
                Phase::Act,
                format!("no tool bound for {}", step.capability_id),
                Some(decision),
                None,
            ));
            let _ = advance(execution, GoalState::Failed, trace);
            return StepResult::Stop;
        };
        let outcome = tool.invoke(&step.input);
        executed.push(step.capability_id.clone());
        events.publish("tool.invoked", step.capability_id.clone(), Some(Phase::Act));
        if !advance(execution, GoalState::Verifying, trace) {
            return StepResult::Stop;
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
            let _ = advance(execution, GoalState::Recovering, trace);
            trace.push(entry(
                Phase::Adapt,
                format!("{} did not verify; stopped", step.capability_id),
                None,
                None,
            ));
            return StepResult::Stop;
        }
        StepResult::Continue
    }
}

enum StepResult {
    Continue,
    Stop,
}

impl PendingApproval {
    fn into_step(self) -> PlannedStep {
        PlannedStep {
            capability_id: self.capability_id,
            summary: self.summary,
            expected_claim: self.expected_claim,
            input: self.input,
        }
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

#[allow(clippy::too_many_arguments)]
fn checkpoint(
    goal: Goal,
    execution: Execution,
    steps: Vec<PlannedStep>,
    next_index: usize,
    executed: Vec<String>,
    pending: Option<PendingApproval>,
    trace: Vec<TraceEntry>,
    events: EventLog,
) -> RunCheckpoint {
    checkpoint_from(
        goal, execution, steps, next_index, executed, pending, trace, events,
    )
}

#[allow(clippy::too_many_arguments)]
fn checkpoint_from(
    goal: Goal,
    execution: Execution,
    steps: Vec<PlannedStep>,
    next_index: usize,
    executed: Vec<String>,
    pending: Option<PendingApproval>,
    trace: Vec<TraceEntry>,
    events: EventLog,
) -> RunCheckpoint {
    RunCheckpoint {
        goal,
        state: execution.state(),
        steps,
        next_index,
        executed,
        pending,
        trace,
        events,
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

fn day_runtime() -> (Runtime, ModelPlanner<ScriptedModel>) {
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

    let model = ScriptedModel::new("scripted", default_day_steps());
    (runtime, ModelPlanner::new(model))
}

/// Host-owned demo session: start, pause for approval, resume.
///
/// Mobile bridges keep one of these behind the FFI boundary so tools stay
/// bound across a `WaitingForUser` checkpoint.
pub struct PrepareMyDayDemo {
    runtime: Runtime,
    planner: ModelPlanner<ScriptedModel>,
}

impl Default for PrepareMyDayDemo {
    fn default() -> Self {
        Self::new()
    }
}

impl PrepareMyDayDemo {
    pub fn new() -> Self {
        let (runtime, planner) = day_runtime();
        Self { runtime, planner }
    }

    /// Build a demo whose planner uses host-supplied steps (for example from
    /// an on-device model). Unknown capability ids are still filtered out.
    pub fn with_steps(provider: impl Into<String>, steps: Vec<ProposedStep>) -> Self {
        let (runtime, _) = day_runtime();
        let planner = ModelPlanner::new(ScriptedModel::new(provider, steps));
        Self { runtime, planner }
    }

    pub fn start(&mut self) -> RunCheckpoint {
        self.runtime.run_with_cancel(
            Goal::from_intent("prepare-my-day", "Prepare my day."),
            &self.planner,
            &CancellationToken::new(),
        )
    }

    pub fn resume(&mut self, checkpoint: RunCheckpoint, reply: UserReply) -> RunCheckpoint {
        self.runtime
            .resume(checkpoint, reply, &CancellationToken::new())
    }
}

/// Canonical demonstration: "Prepare my day."
///
/// Reads stay local and verify. Sending a message stops and asks.
/// Nothing leaves the process. Planning goes through a replaceable
/// [`ModelAdapter`] (`ScriptedModel` here).
pub fn prepare_my_day() -> RunReport {
    PrepareMyDayDemo::new().start().into_report()
}

/// Same demonstration, then approve the pending message and finish.
pub fn prepare_my_day_with_approval() -> RunReport {
    let mut demo = PrepareMyDayDemo::new();
    let paused = demo.start();
    demo.resume(paused, UserReply::Approve).into_report()
}

fn default_day_steps() -> Vec<ProposedStep> {
    vec![
        proposed("read_calendar", "Inspect the calendar", "calendar.loaded"),
        proposed("read_tasks", "Inspect open tasks", "tasks.loaded"),
        proposed("read_weather", "Check the weather", "weather.loaded"),
        proposed("read_location", "Check where you are", "location.loaded"),
        proposed("send_message", "Send the day summary", "message.sent"),
    ]
}

fn proposed(id: &str, summary: &str, claim: &str) -> ProposedStep {
    ProposedStep {
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
    use raven_execution::CancellationToken;

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
        assert_eq!(
            report.pending.as_ref().map(|p| p.capability_id.as_str()),
            Some("send_message")
        );
        assert!(report.trace.iter().any(|entry| {
            entry.phase == Phase::Reason && entry.summary.contains("scripted")
        }));
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

    #[test]
    fn approval_resumes_and_completes_the_run() {
        let report = prepare_my_day_with_approval();
        assert_eq!(report.state, GoalState::Completed);
        assert!(report.executed.iter().any(|id| id == "send_message"));
        assert!(report.pending.is_none());
        assert!(report
            .trace
            .iter()
            .any(|entry| entry.summary.contains("approved by user")));
    }

    #[test]
    fn deny_fails_without_invoking_the_pending_tool() {
        let (mut runtime, planner) = day_runtime();
        let cancel = CancellationToken::new();
        let paused = runtime.run_with_cancel(
            Goal::from_intent("prepare-my-day", "Prepare my day."),
            &planner,
            &cancel,
        );
        let report = runtime
            .resume(paused, UserReply::Deny, &cancel)
            .into_report();
        assert_eq!(report.state, GoalState::Failed);
        assert!(!report.executed.iter().any(|id| id == "send_message"));
    }

    #[test]
    fn cancellation_token_stops_between_steps() {
        let (mut runtime, planner) = day_runtime();
        let cancel = CancellationToken::new();
        cancel.cancel();
        let checkpoint = runtime.run_with_cancel(
            Goal::from_intent("prepare-my-day", "Prepare my day."),
            &planner,
            &cancel,
        );
        assert_eq!(checkpoint.state, GoalState::Cancelled);
        assert!(checkpoint.executed.is_empty());
    }

    #[test]
    fn cancel_reply_abandons_a_waiting_run() {
        let (mut runtime, planner) = day_runtime();
        let cancel = CancellationToken::new();
        let paused = runtime.run_with_cancel(
            Goal::from_intent("prepare-my-day", "Prepare my day."),
            &planner,
            &cancel,
        );
        assert!(paused.is_waiting());
        let report = runtime
            .resume(paused, UserReply::Cancel, &cancel)
            .into_report();
        assert_eq!(report.state, GoalState::Cancelled);
        assert!(!report.executed.iter().any(|id| id == "send_message"));
    }

    #[test]
    fn unavailable_model_fails_the_goal_before_tools() {
        struct Down;
        impl ModelAdapter for Down {
            fn id(&self) -> &str {
                "down"
            }
            fn propose(&self, _: &PlanRequest) -> Result<PlanProposal, PlanError> {
                Err(PlanError::unavailable("no model"))
            }
        }

        let (mut runtime, _) = day_runtime();
        let planner = ModelPlanner::new(Down);
        let report = runtime.run(Goal::from_intent("x", "Prepare my day."), &planner);
        assert_eq!(report.state, GoalState::Failed);
        assert!(report.executed.is_empty());
        assert!(report
            .trace
            .iter()
            .any(|entry| entry.summary.contains("planner unavailable")));
    }
}
