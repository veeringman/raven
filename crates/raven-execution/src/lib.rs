//! Explicit transitions for a goal.
//!
//! Illegal jumps fail. The runtime cannot mark a goal completed without
//! passing through understanding, planning, authorization, execution, and
//! verification. Cancellation is checked between steps; it never skips the
//! state machine.

use raven_core::GoalState;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IllegalTransition {
    pub from: GoalState,
    pub to: GoalState,
}

/// Shared signal a host can flip to stop a run between steps.
///
/// Cancellation is cooperative. The runtime checks the token at step
/// boundaries; it does not preempt a tool already in flight.
#[derive(Clone, Debug, Default)]
pub struct CancellationToken {
    cancelled: Arc<AtomicBool>,
}

impl CancellationToken {
    pub fn new() -> Self {
        Self {
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }

    pub fn check(&self) -> Result<(), Cancelled> {
        if self.is_cancelled() {
            Err(Cancelled)
        } else {
            Ok(())
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cancelled;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Execution {
    state: GoalState,
}

impl Default for Execution {
    fn default() -> Self {
        Self::new()
    }
}

impl Execution {
    pub fn new() -> Self {
        Self {
            state: GoalState::Created,
        }
    }

    /// Rebuild an execution cursor from a durable checkpoint.
    pub fn from_state(state: GoalState) -> Self {
        Self { state }
    }

    pub fn state(&self) -> GoalState {
        self.state
    }

    pub fn transition(&mut self, to: GoalState) -> Result<(), IllegalTransition> {
        if allowed(self.state, to) {
            self.state = to;
            Ok(())
        } else {
            Err(IllegalTransition {
                from: self.state,
                to,
            })
        }
    }
}

fn allowed(from: GoalState, to: GoalState) -> bool {
    use GoalState::*;
    matches!(
        (from, to),
        (Created, Understanding)
            | (Understanding, Planning)
            | (Understanding, WaitingForUser)
            | (Planning, Authorized)
            | (Planning, Failed)
            | (Authorized, Executing)
            | (Authorized, WaitingForUser)
            | (Authorized, Failed)
            | (Executing, Verifying)
            | (Executing, Failed)
            | (Verifying, Executing)
            | (Verifying, Completed)
            | (Verifying, Recovering)
            | (Verifying, WaitingForUser)
            | (Verifying, Failed)
            | (Recovering, Planning)
            | (Recovering, WaitingForUser)
            | (Recovering, Failed)
            | (WaitingForUser, Authorized)
            | (WaitingForUser, Failed)
            | (WaitingForUser, Cancelled)
            | (Paused, Planning)
            | (Created, Cancelled)
            | (Understanding, Cancelled)
            | (Planning, Cancelled)
            | (Authorized, Cancelled)
            | (Executing, Cancelled)
            | (Verifying, Cancelled)
            | (WaitingForUser, Paused)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn happy_path_reaches_completed() {
        let mut run = Execution::new();
        for next in [
            GoalState::Understanding,
            GoalState::Planning,
            GoalState::Authorized,
            GoalState::Executing,
            GoalState::Verifying,
            GoalState::Completed,
        ] {
            run.transition(next).expect("legal transition");
        }
        assert_eq!(run.state(), GoalState::Completed);
    }

    #[test]
    fn completion_cannot_skip_the_loop() {
        let mut run = Execution::new();
        let err = run.transition(GoalState::Completed).unwrap_err();
        assert_eq!(err.from, GoalState::Created);
        assert_eq!(err.to, GoalState::Completed);
        assert_eq!(run.state(), GoalState::Created);
    }

    #[test]
    fn waiting_for_user_may_fail_or_cancel() {
        let mut run = Execution::from_state(GoalState::WaitingForUser);
        run.transition(GoalState::Failed).expect("deny");
        let mut run = Execution::from_state(GoalState::WaitingForUser);
        run.transition(GoalState::Cancelled).expect("cancel");
    }

    #[test]
    fn cancellation_token_is_shared() {
        let token = CancellationToken::new();
        let clone = token.clone();
        assert!(token.check().is_ok());
        clone.cancel();
        assert_eq!(token.check(), Err(Cancelled));
        assert!(token.is_cancelled());
    }
}
