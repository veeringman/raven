# Architecture of this tree

The concept baseline in [CONCEPT.md](CONCEPT.md) is the invariant. This document describes what the local repository actually contains.

## What runs today

```text
Intent
  → Perceive
  → Plan
  → Authorize
  → Act
  → Verify
  → Completed, WaitingForUser, Recovering, or Failed
```

A tool is reached only after the policy engine returns `Allow` or `AllowWithPolicy`. `AskUser` and `Deny` do not invoke the tool.

## Crates

| Crate | Responsibility |
|---|---|
| `raven-core` | Goal, capability, risk, autonomy, evidence, principles |
| `raven-policy` | Contextual authorization |
| `raven-tools` | Discovery registry. Registration is not permission |
| `raven-execution` | Legal goal-state transitions |
| `raven-verification` | Evidence must support the expected claim |
| `raven-events` | Ordered log of the run |
| `raven-runtime` | One loop over a planner and bound tools |
| `raven-cli` | `raven demo`, `raven principles` |

## Policy invariant

| Risk | Default result |
|---|---|
| L0 Observation | Allow |
| L1 Local low-risk | Allow, unless autonomy is Manual |
| L2 External communication | Ask, unless autonomy is Guided or higher (`AllowWithPolicy`) |
| L3 Sensitive | Ask |
| L4 Consequential | Ask |
| L5 Physical | Ask |

L3–L5 ask even when autonomy is `PolicyBounded`. A later phase may define a narrower, explicit grant. This foundation does not.

## What is deliberately absent

Models, device bridges, credential brokers, network calls, shell execution, and physical actuators are not in this tree. The demo tools are in-process fixtures. Their evidence is a claim the verifier can accept or reject.

Rust edition 2024 is the architectural target (MSRV 1.85+). The workspace compiles as edition 2021 on the current toolchain. The source uses no edition-2024-only syntax.

## Next boundary

Phase 2 is a native bridge, not more framework surface. iOS through Swift and App Intents. Android through Kotlin. The Rust core stays platform-independent.
