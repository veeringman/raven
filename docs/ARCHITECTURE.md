# Architecture of this tree

The concept baseline in [CONCEPT.md](CONCEPT.md) is the invariant. This document describes what the local repository actually contains.

## What runs today

```text
Intent
  → Perceive
  → Reason (replaceable model proposes a plan)
  → Plan (runtime accepts only discovered capabilities)
  → Authorize
  → Act
  → Verify
  → Completed, WaitingForUser, Recovering, Failed, or Cancelled
```

A tool is reached only after the policy engine returns `Allow` or `AllowWithPolicy`. `AskUser` and `Deny` do not invoke the tool.

A run can stop with a durable `RunCheckpoint` when policy asks the user. The host keeps that checkpoint, collects `Approve`, `Deny`, or `Cancel`, and calls `resume`. A shared `CancellationToken` is checked between steps; it does not preempt a tool already in flight.

Planning goes through a [`ModelAdapter`](../crates/raven-runtime/src/planner.rs). The adapter returns a proposal. The runtime filters unknown capability ids, bounds step count, then authorizes. No model calls a tool. The demo uses `ScriptedModel`; a host can swap in on-device or cloud providers without changing the loop.

## Crates

| Crate | Responsibility |
|---|---|
| `raven-core` | Goal, capability, risk, autonomy, evidence, principles |
| `raven-policy` | Contextual authorization |
| `raven-tools` | Discovery registry. Registration is not permission |
| `raven-execution` | Legal goal-state transitions, cancellation tokens |
| `raven-verification` | Evidence must support the expected claim |
| `raven-events` | Ordered log of the run |
| `raven-runtime` | Loop, checkpoints, replaceable planner, resume |
| `raven-ffi` | UniFFI bridge for Swift hosts |
| `raven-cli` | `raven demo`, `raven principles` |

iOS lives under `apps/ios/`: `RavenKit` (Swift package + XCFramework) and `PrepareMyDay` (sample app).

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

Live cloud model providers, credential brokers, network calls, shell execution, and physical actuators are not in this tree. Fixture tools still stay in-process. The iOS sample plans with Apple Foundation Models when the system model is available, otherwise falls back to `ScriptedDayPlanner`. Verification and policy remain in Rust.

Rust edition 2024 is the architectural target (MSRV 1.85+). The workspace compiles as edition 2021 on the current toolchain. The source uses no edition-2024-only syntax.

## Next boundary

Add App Intents for the sample goal, then Android. The Rust core stays platform-independent.
