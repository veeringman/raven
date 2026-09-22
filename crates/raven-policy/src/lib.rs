//! Decides whether a capability may run.
//!
//! L3, L4, and L5 always return [`PolicyDecision::AskUser`]. Higher autonomy
//! does not bypass sensitive, consequential, or physical actions in this
//! foundation. That is the human-authority invariant.

use raven_core::{Autonomy, Capability, PolicyDecision, RiskLevel};
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct PolicyEngine {
    default_autonomy: Autonomy,
    by_capability: HashMap<String, Autonomy>,
}

impl Default for PolicyEngine {
    fn default() -> Self {
        Self {
            default_autonomy: Autonomy::Suggest,
            by_capability: HashMap::new(),
        }
    }
}

impl PolicyEngine {
    pub fn new(default_autonomy: Autonomy) -> Self {
        Self {
            default_autonomy,
            by_capability: HashMap::new(),
        }
    }

    pub fn grant(&mut self, capability_id: impl Into<String>, autonomy: Autonomy) {
        self.by_capability.insert(capability_id.into(), autonomy);
    }

    pub fn evaluate(&self, capability: &Capability) -> PolicyDecision {
        let autonomy = self
            .by_capability
            .get(&capability.id)
            .copied()
            .unwrap_or(self.default_autonomy);
        evaluate(capability, autonomy)
    }
}

pub fn evaluate(capability: &Capability, autonomy: Autonomy) -> PolicyDecision {
    if !capability.available || capability.forbidden {
        return PolicyDecision::Deny;
    }
    match capability.risk {
        RiskLevel::L0 => PolicyDecision::Allow,
        RiskLevel::L1 => {
            if autonomy == Autonomy::Manual {
                PolicyDecision::AskUser
            } else {
                PolicyDecision::Allow
            }
        }
        RiskLevel::L2 => {
            if autonomy >= Autonomy::Guided {
                PolicyDecision::AllowWithPolicy
            } else {
                PolicyDecision::AskUser
            }
        }
        RiskLevel::L3 | RiskLevel::L4 | RiskLevel::L5 => PolicyDecision::AskUser,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use raven_core::Capability;

    fn cap(risk: RiskLevel) -> Capability {
        Capability::new("tool", "a tool", risk)
    }

    #[test]
    fn consequential_and_physical_actions_always_ask() {
        for risk in [RiskLevel::L3, RiskLevel::L4, RiskLevel::L5] {
            for autonomy in [
                Autonomy::Manual,
                Autonomy::Assist,
                Autonomy::Suggest,
                Autonomy::Guided,
                Autonomy::Conditional,
                Autonomy::PolicyBounded,
            ] {
                assert_eq!(
                    evaluate(&cap(risk), autonomy),
                    PolicyDecision::AskUser,
                    "{risk:?} at {autonomy:?} must ask"
                );
            }
        }
    }

    #[test]
    fn forbidden_or_unavailable_capabilities_are_denied() {
        let mut hidden = cap(RiskLevel::L0);
        hidden.forbidden = true;
        assert_eq!(
            evaluate(&hidden, Autonomy::PolicyBounded),
            PolicyDecision::Deny
        );
        hidden.forbidden = false;
        hidden.available = false;
        assert_eq!(
            evaluate(&hidden, Autonomy::PolicyBounded),
            PolicyDecision::Deny
        );
    }

    #[test]
    fn external_communication_asks_until_guided() {
        let message = cap(RiskLevel::L2);
        assert_eq!(
            evaluate(&message, Autonomy::Suggest),
            PolicyDecision::AskUser
        );
        assert_eq!(
            evaluate(&message, Autonomy::Guided),
            PolicyDecision::AllowWithPolicy
        );
    }
}
