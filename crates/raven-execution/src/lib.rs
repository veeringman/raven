//! Explicit transitions for a goal.
//!
//! Illegal jumps fail. The runtime cannot mark a goal completed without
//! passing through understanding, planning, authorization, execution, and
//! verification.

use raven_core::GoalState;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IllegalTransition {
    pub from: GoalState,
    pub to: GoalState,
}

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
}
