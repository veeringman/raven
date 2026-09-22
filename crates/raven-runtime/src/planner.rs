//! Replaceable planning. A model proposes steps; the runtime still authorizes
//! and verifies. The model never invokes a tool.

use raven_core::{Capability, Goal, RiskLevel};

use crate::{Plan, PlannedStep, Planner};

/// Compact capability view shown to a model. Intentionally less than the full
/// registry record so providers cannot rely on host-only fields.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CapabilityBrief {
    pub id: String,
    pub description: String,
    pub risk: RiskLevel,
}

impl From<&Capability> for CapabilityBrief {
    fn from(capability: &Capability) -> Self {
        Self {
            id: capability.id.clone(),
            description: capability.description.clone(),
            risk: capability.risk,
        }
    }
}

/// What a model may see when asked to plan. Built by the runtime from the goal
/// and the discovery registry — not from raw tool bindings.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanRequest {
    pub intent: String,
    pub desired_outcome: String,
    pub constraints: Vec<String>,
    pub available: Vec<CapabilityBrief>,
}

impl PlanRequest {
    pub fn from_goal(goal: &Goal, available: &[Capability]) -> Self {
        Self {
            intent: goal.intent.clone(),
            desired_outcome: goal.desired_outcome.clone(),
            constraints: goal.constraints.clone(),
            available: available.iter().map(CapabilityBrief::from).collect(),
        }
    }
}

/// One step a model proposes. The runtime filters unknown capabilities before
/// any policy check or tool call.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProposedStep {
    pub capability_id: String,
    pub summary: String,
    pub expected_claim: String,
    pub input: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanProposal {
    pub steps: Vec<ProposedStep>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PlanError {
    /// Provider is missing, offline, or refused to answer.
    Unavailable { message: String },
    /// Provider returned something the runtime cannot accept as a plan.
    Invalid { message: String },
}

impl PlanError {
    pub fn unavailable(message: impl Into<String>) -> Self {
        Self::Unavailable {
            message: message.into(),
        }
    }

    pub fn invalid(message: impl Into<String>) -> Self {
        Self::Invalid {
            message: message.into(),
        }
    }
}

/// Replaceable intelligence behind planning.
///
/// Implementations may call on-device system models, local open models, or
/// cloud providers. They return a proposal only. They must not execute tools.
pub trait ModelAdapter {
    fn id(&self) -> &str;
    fn propose(&self, request: &PlanRequest) -> Result<PlanProposal, PlanError>;
}

/// Upper bound on steps accepted from any model proposal.
pub const MAX_PLAN_STEPS: usize = 32;

/// Planner that asks a [`ModelAdapter`] and filters the proposal to discovered
/// capabilities. Unknown or empty capability ids are dropped.
pub struct ModelPlanner<M> {
    model: M,
}

impl<M> ModelPlanner<M> {
    pub fn new(model: M) -> Self {
        Self { model }
    }

    pub fn model(&self) -> &M {
        &self.model
    }
}

impl<M: ModelAdapter> Planner for ModelPlanner<M> {
    fn provider(&self) -> Option<&str> {
        Some(self.model.id())
    }

    fn plan(&self, goal: &Goal, available: &[Capability]) -> Result<Plan, PlanError> {
        let request = PlanRequest::from_goal(goal, available);
        let proposal = self.model.propose(&request)?;
        let steps = accept_proposal(proposal, available);
        if steps.is_empty() {
            return Err(PlanError::invalid(
                "model proposal contained no usable steps",
            ));
        }
        Ok(Plan { steps })
    }
}

/// Fixed plan with no model call. Useful for tests and deterministic demos.
#[derive(Clone, Debug)]
pub struct StaticPlanner {
    steps: Vec<PlannedStep>,
}

impl StaticPlanner {
    pub fn new(steps: Vec<PlannedStep>) -> Self {
        Self { steps }
    }
}

impl Planner for StaticPlanner {
    fn plan(&self, _goal: &Goal, available: &[Capability]) -> Result<Plan, PlanError> {
        let steps = filter_known(&self.steps, available);
        if steps.is_empty() {
            return Err(PlanError::invalid("static plan has no available steps"));
        }
        Ok(Plan { steps })
    }
}

/// In-process model that returns a scripted proposal. Stands in for a real
/// provider until a host supplies Foundation Models or another adapter.
#[derive(Clone, Debug)]
pub struct ScriptedModel {
    id: String,
    steps: Vec<ProposedStep>,
}

impl ScriptedModel {
    pub fn new(id: impl Into<String>, steps: Vec<ProposedStep>) -> Self {
        Self {
            id: id.into(),
            steps,
        }
    }
}

impl ModelAdapter for ScriptedModel {
    fn id(&self) -> &str {
        &self.id
    }

    fn propose(&self, _request: &PlanRequest) -> Result<PlanProposal, PlanError> {
        Ok(PlanProposal {
            steps: self.steps.clone(),
        })
    }
}

fn accept_proposal(proposal: PlanProposal, available: &[Capability]) -> Vec<PlannedStep> {
    proposal
        .steps
        .into_iter()
        .filter(|step| !step.capability_id.is_empty())
        .filter(|step| available.iter().any(|cap| cap.id == step.capability_id))
        .take(MAX_PLAN_STEPS)
        .map(|step| PlannedStep {
            capability_id: step.capability_id,
            summary: step.summary,
            expected_claim: step.expected_claim,
            input: step.input,
        })
        .collect()
}

fn filter_known(steps: &[PlannedStep], available: &[Capability]) -> Vec<PlannedStep> {
    steps
        .iter()
        .filter(|step| available.iter().any(|cap| cap.id == step.capability_id))
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use raven_core::RiskLevel;

    fn caps() -> Vec<Capability> {
        vec![
            Capability::new("read_calendar", "calendar", RiskLevel::L0),
            Capability::new("send_message", "message", RiskLevel::L2),
        ]
    }

    #[test]
    fn model_planner_drops_unknown_capabilities() {
        let model = ScriptedModel::new(
            "scripted",
            vec![
                ProposedStep {
                    capability_id: "read_calendar".into(),
                    summary: "calendar".into(),
                    expected_claim: "calendar.loaded".into(),
                    input: String::new(),
                },
                ProposedStep {
                    capability_id: "launch_missiles".into(),
                    summary: "no".into(),
                    expected_claim: "boom".into(),
                    input: String::new(),
                },
            ],
        );
        let planner = ModelPlanner::new(model);
        let goal = Goal::from_intent("day", "Prepare my day.");
        let plan = planner.plan(&goal, &caps()).expect("plan");
        assert_eq!(plan.steps.len(), 1);
        assert_eq!(plan.steps[0].capability_id, "read_calendar");
        assert_eq!(planner.provider(), Some("scripted"));
    }

    #[test]
    fn unavailable_model_surfaces_as_plan_error() {
        struct Down;
        impl ModelAdapter for Down {
            fn id(&self) -> &str {
                "down"
            }
            fn propose(&self, _: &PlanRequest) -> Result<PlanProposal, PlanError> {
                Err(PlanError::unavailable("offline"))
            }
        }
        let planner = ModelPlanner::new(Down);
        let err = planner
            .plan(&Goal::from_intent("x", "x"), &caps())
            .unwrap_err();
        assert_eq!(err, PlanError::unavailable("offline"));
    }
}
