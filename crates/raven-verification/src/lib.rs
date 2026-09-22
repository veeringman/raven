//! Verification treats a successful call as incomplete until evidence matches
//! the expected claim.

use raven_core::Evidence;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolOutcome {
    pub ok: bool,
    pub summary: String,
    pub evidence: Vec<Evidence>,
}

impl ToolOutcome {
    pub fn verified(
        summary: impl Into<String>,
        claim: impl Into<String>,
        source: impl Into<String>,
    ) -> Self {
        Self {
            ok: true,
            summary: summary.into(),
            evidence: vec![Evidence {
                claim: claim.into(),
                source: source.into(),
            }],
        }
    }

    pub fn failed(summary: impl Into<String>) -> Self {
        Self {
            ok: false,
            summary: summary.into(),
            evidence: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Verification {
    Confirmed { evidence: Vec<Evidence> },
    Rejected { reason: String },
}

impl Verification {
    pub fn is_confirmed(&self) -> bool {
        matches!(self, Self::Confirmed { .. })
    }
}

pub fn verify(expected_claim: &str, outcome: &ToolOutcome) -> Verification {
    if !outcome.ok {
        return Verification::Rejected {
            reason: outcome.summary.clone(),
        };
    }
    let supporting: Vec<_> = outcome
        .evidence
        .iter()
        .filter(|item| item.claim == expected_claim)
        .cloned()
        .collect();
    if supporting.is_empty() {
        Verification::Rejected {
            reason: format!("evidence does not support '{expected_claim}'"),
        }
    } else {
        Verification::Confirmed {
            evidence: supporting,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn success_without_the_expected_claim_is_rejected() {
        let outcome = ToolOutcome::verified("sent", "recipient=ada", "mailer");
        let verdict = verify("recipient=john", &outcome);
        assert!(!verdict.is_confirmed());
    }

    #[test]
    fn matching_evidence_confirms_the_step() {
        let outcome = ToolOutcome::verified("read", "calendar.loaded", "calendar");
        assert!(verify("calendar.loaded", &outcome).is_confirmed());
    }
}
