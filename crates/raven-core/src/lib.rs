//! Domain primitives for the RAVEN agentic runtime.
//!
//! These types are the vocabulary shared by every later crate. They describe
//! intent, risk, policy, and evidence. They do not execute tools.

/// Where a goal sits in the RAVEN loop.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Perceive,
    Reason,
    Plan,
    Act,
    Verify,
    Adapt,
}

/// Risk of a capability. Higher levels never run silently in this foundation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum RiskLevel {
    /// Observation only.
    L0,
    /// Local, low-risk action.
    L1,
    /// External communication.
    L2,
    /// Sensitive action.
    L3,
    /// Financial or otherwise consequential action.
    L4,
    /// Physical-world action.
    L5,
}

/// How much autonomy the user has granted for one capability.
///
/// Autonomy is capability-specific. It is not an application-wide switch.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Autonomy {
    Manual,
    Assist,
    Suggest,
    Guided,
    Conditional,
    PolicyBounded,
}

/// What the policy engine may answer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PolicyDecision {
    Allow,
    AllowWithPolicy,
    AskUser,
    Deny,
}

/// Lifecycle of one goal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GoalState {
    Created,
    Understanding,
    Planning,
    Authorized,
    Executing,
    Verifying,
    Completed,
    WaitingForUser,
    WaitingForResource,
    Failed,
    Recovering,
    Cancelled,
    Paused,
    Delegated,
}

/// How strongly a context fact should be trusted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Confidence {
    Low,
    Medium,
    High,
}

/// Memory classes from the concept baseline. Persistence arrives in a later phase.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemoryClass {
    Working,
    Session,
    Episodic,
    Preference,
    Knowledge,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Goal {
    pub id: String,
    pub intent: String,
    pub desired_outcome: String,
    pub constraints: Vec<String>,
    pub risk_tolerance: RiskLevel,
}

impl Goal {
    pub fn from_intent(id: impl Into<String>, intent: impl Into<String>) -> Self {
        let intent = intent.into();
        Self {
            id: id.into(),
            intent: intent.clone(),
            desired_outcome: intent,
            constraints: Vec::new(),
            risk_tolerance: RiskLevel::L2,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Capability {
    pub id: String,
    pub description: String,
    pub risk: RiskLevel,
    pub available: bool,
    pub forbidden: bool,
}

impl Capability {
    pub fn new(id: impl Into<String>, description: impl Into<String>, risk: RiskLevel) -> Self {
        Self {
            id: id.into(),
            description: description.into(),
            risk,
            available: true,
            forbidden: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextFact {
    pub key: String,
    pub value: String,
    pub confidence: Confidence,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ContextSnapshot {
    pub facts: Vec<ContextFact>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Evidence {
    pub claim: String,
    pub source: String,
}

/// Stable principles. Wording can grow; the commitments should not drift.
pub fn principles() -> &'static [(&'static str, &'static str)] {
    &[
        (
            "Intent over workflow",
            "Ask what the user is trying to accomplish.",
        ),
        (
            "Goal over command",
            "A goal carries outcome, constraints, and completion.",
        ),
        (
            "Capability over API",
            "Tools are discovered, not hard-wired.",
        ),
        (
            "Context before action",
            "Act from a scoped, time-aware picture.",
        ),
        (
            "Policy before execution",
            "Permission is decided at runtime.",
        ),
        (
            "Verification before completion",
            "An invocation is not a result.",
        ),
        (
            "Human authority over autonomy",
            "Autonomy stays inside a boundary.",
        ),
        (
            "Local before cloud",
            "Cloud is a capability, not a dependency.",
        ),
        ("Evidence over assertion", "Claims cite what was observed."),
        (
            "Environment is first-class",
            "The same agent behaves differently by place.",
        ),
        ("Agents are portable", "Goal, memory, and policy can move."),
        ("Failure is normal", "Retry, compensate, ask, or stop."),
        ("Data is not instruction", "External content is untrusted."),
        (
            "Capability is not permission",
            "Discovery does not authorize use.",
        ),
        (
            "Model intelligence is replaceable",
            "The runtime does not belong to one provider.",
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn risk_order_matches_the_scale() {
        assert!(RiskLevel::L0 < RiskLevel::L4);
        assert!(RiskLevel::L4 < RiskLevel::L5);
        assert!(Autonomy::Manual < Autonomy::PolicyBounded);
    }

    #[test]
    fn goal_keeps_the_intent_as_the_desired_outcome() {
        let goal = Goal::from_intent("day", "Prepare my day.");
        assert_eq!(goal.intent, goal.desired_outcome);
        assert_eq!(goal.risk_tolerance, RiskLevel::L2);
    }
}
