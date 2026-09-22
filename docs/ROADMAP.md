# Roadmap

Phases follow the concept baseline. Checked items exist in this repository.

## Phase 1 — Runtime foundation

- [x] Rust workspace and domain types
- [x] Capability registry
- [x] Policy engine with the ask-before-consequential invariant
- [x] Goal state machine
- [x] Verification against evidence
- [x] Event log
- [x] In-process loop: intent → plan → tool → verify
- [x] Canonical demo: prepare my day
- [ ] Durable run state and cancellation tokens
- [ ] Planner interface backed by a replaceable model

## Phase 2 — Mobile runtime

- [ ] Swift bridge, App Intents, Foundation Models
- [ ] Kotlin bridge, AppFunctions exploration, on-device model adapter

## Phase 3 — Memory and context

- [ ] Working, session, and preference memory
- [ ] Encryption, retention, deletion, consent

## Phase 4 — Cross-device continuity

- [ ] Move goal, plan, evidence, and pending approval between devices

## Phase 5 — Agent to agent

- [ ] Identity, contract, delegation, trust

## Phase 6 — Edge and IoT

- [ ] Local agents, sensor context, physical verification

## Phase 7 — Developer platform

- [ ] Simulator, inspector, evaluation suite, package, deploy

The CLI sketched in the concept (`raven init`, `raven simulate`, `raven inspect`, `raven evaluate`) waits until the runtime has something those commands can observe beyond the foundation demo.
