//! Swift-facing bridge. Types here are the mobile contract; the Rust core
//! stays free of UniFFI attributes.

uniffi::setup_scaffolding!();

use once_cell::sync::Lazy;
use raven_core::GoalState;
use raven_runtime::{
    PrepareMyDayDemo, ProposedStep, RunCheckpoint, TraceEntry, UserReply,
};
use std::sync::Mutex;

#[derive(uniffi::Enum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum FfiGoalState {
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

#[derive(uniffi::Enum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum FfiUserReply {
    Approve,
    Deny,
    Cancel,
}

#[derive(uniffi::Record, Debug, Clone, PartialEq, Eq)]
pub struct FfiTraceEntry {
    pub phase: String,
    pub summary: String,
    pub policy: Option<String>,
    pub verified: Option<bool>,
}

#[derive(uniffi::Record, Debug, Clone, PartialEq, Eq)]
pub struct FfiProposedStep {
    pub capability_id: String,
    pub summary: String,
    pub expected_claim: String,
    pub input: String,
}

#[derive(uniffi::Record, Debug, Clone, PartialEq, Eq)]
pub struct FfiRunSnapshot {
    pub intent: String,
    pub state: FfiGoalState,
    pub trace: Vec<FfiTraceEntry>,
    pub executed: Vec<String>,
    pub pending_capability_id: Option<String>,
    pub pending_summary: Option<String>,
}

struct Bridge {
    demo: PrepareMyDayDemo,
    checkpoint: Option<RunCheckpoint>,
}

impl Bridge {
    fn fresh() -> Self {
        Self {
            demo: PrepareMyDayDemo::new(),
            checkpoint: None,
        }
    }
}

static BRIDGE: Lazy<Mutex<Bridge>> = Lazy::new(|| Mutex::new(Bridge::fresh()));

/// Start the canonical prepare-my-day demo with the built-in scripted planner.
#[uniffi::export]
pub fn start_prepare_my_day() -> FfiRunSnapshot {
    let mut bridge = BRIDGE.lock().expect("raven bridge lock");
    *bridge = Bridge::fresh();
    let checkpoint = bridge.demo.start();
    let snapshot = snapshot_of(&checkpoint);
    bridge.checkpoint = Some(checkpoint);
    snapshot
}

/// Start prepare-my-day using steps proposed by the host (for example an
/// on-device Foundation Models adapter). The runtime still filters, authorizes,
/// and verifies; the host never invokes tools directly.
///
/// `provider` is recorded on the Reason phase (for example
/// `foundation-models` or `scripted-fallback`).
#[uniffi::export]
pub fn start_prepare_my_day_with_steps(
    provider: String,
    steps: Vec<FfiProposedStep>,
) -> FfiRunSnapshot {
    let mut bridge = BRIDGE.lock().expect("raven bridge lock");
    let proposed = steps
        .into_iter()
        .map(|step| ProposedStep {
            capability_id: step.capability_id,
            summary: step.summary,
            expected_claim: step.expected_claim,
            input: step.input,
        })
        .collect();
    let provider = if provider.trim().is_empty() {
        "host".to_string()
    } else {
        provider
    };
    bridge.demo = PrepareMyDayDemo::with_steps(provider, proposed);
    bridge.checkpoint = None;
    let checkpoint = bridge.demo.start();
    let snapshot = snapshot_of(&checkpoint);
    bridge.checkpoint = Some(checkpoint);
    snapshot
}

/// Resume after `WaitingForUser` with the person's decision.
#[uniffi::export]
pub fn resume_prepare_my_day(reply: FfiUserReply) -> FfiRunSnapshot {
    let mut bridge = BRIDGE.lock().expect("raven bridge lock");
    let Some(checkpoint) = bridge.checkpoint.take() else {
        return FfiRunSnapshot {
            intent: String::new(),
            state: FfiGoalState::Failed,
            trace: vec![FfiTraceEntry {
                phase: "Adapt".into(),
                summary: "no checkpoint to resume".into(),
                policy: None,
                verified: None,
            }],
            executed: Vec::new(),
            pending_capability_id: None,
            pending_summary: None,
        };
    };
    let checkpoint = bridge.demo.resume(checkpoint, map_reply(reply));
    let snapshot = snapshot_of(&checkpoint);
    if !checkpoint.is_terminal() {
        bridge.checkpoint = Some(checkpoint);
    }
    snapshot
}

/// Architectural principles for the sample surface.
#[uniffi::export]
pub fn raven_principles() -> Vec<String> {
    raven_core::principles()
        .iter()
        .map(|(title, body)| format!("{title}: {body}"))
        .collect()
}

fn map_reply(reply: FfiUserReply) -> UserReply {
    match reply {
        FfiUserReply::Approve => UserReply::Approve,
        FfiUserReply::Deny => UserReply::Deny,
        FfiUserReply::Cancel => UserReply::Cancel,
    }
}

fn map_state(state: GoalState) -> FfiGoalState {
    match state {
        GoalState::Created => FfiGoalState::Created,
        GoalState::Understanding => FfiGoalState::Understanding,
        GoalState::Planning => FfiGoalState::Planning,
        GoalState::Authorized => FfiGoalState::Authorized,
        GoalState::Executing => FfiGoalState::Executing,
        GoalState::Verifying => FfiGoalState::Verifying,
        GoalState::Completed => FfiGoalState::Completed,
        GoalState::WaitingForUser => FfiGoalState::WaitingForUser,
        GoalState::WaitingForResource => FfiGoalState::WaitingForResource,
        GoalState::Failed => FfiGoalState::Failed,
        GoalState::Recovering => FfiGoalState::Recovering,
        GoalState::Cancelled => FfiGoalState::Cancelled,
        GoalState::Paused => FfiGoalState::Paused,
        GoalState::Delegated => FfiGoalState::Delegated,
    }
}

fn snapshot_of(checkpoint: &RunCheckpoint) -> FfiRunSnapshot {
    FfiRunSnapshot {
        intent: checkpoint.goal.intent.clone(),
        state: map_state(checkpoint.state),
        trace: checkpoint.trace.iter().map(map_trace).collect(),
        executed: checkpoint.executed.clone(),
        pending_capability_id: checkpoint
            .pending
            .as_ref()
            .map(|pending| pending.capability_id.clone()),
        pending_summary: checkpoint
            .pending
            .as_ref()
            .map(|pending| pending.summary.clone()),
    }
}

fn map_trace(entry: &TraceEntry) -> FfiTraceEntry {
    FfiTraceEntry {
        phase: format!("{:?}", entry.phase),
        summary: entry.summary.clone(),
        policy: entry.decision.map(|decision| format!("{decision:?}")),
        verified: entry
            .verification
            .as_ref()
            .map(|verdict| verdict.is_confirmed()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bridge_asks_then_completes_on_approve() {
        let snap = start_prepare_my_day();
        assert_eq!(snap.state, FfiGoalState::WaitingForUser);
        assert_eq!(snap.pending_capability_id.as_deref(), Some("send_message"));
        let done = resume_prepare_my_day(FfiUserReply::Approve);
        assert_eq!(done.state, FfiGoalState::Completed);
        assert!(done.executed.iter().any(|id| id == "send_message"));
    }
}
