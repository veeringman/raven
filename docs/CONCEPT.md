# RAVEN

## The Runtime for Agentic Applications

**Positioning:** From Intent to Action

> Today's applications wait for commands. RAVEN understands intent.

**Status:** Foundational concept and reference architecture
**Role:** Canonical baseline for design, engineering, research, and implementation
**Platforms:** Mobile, web, desktop, edge, cloud, IoT
**Core language:** Rust. Architectural target is edition 2024. This repository currently builds as edition 2021.
**First mobile platforms:** iOS and Android
**Architecture:** Local-first, event-driven, model-agnostic, policy-governed, verification-driven

Future work extends this document. It does not silently replace the invariants at the end.

## 1. What RAVEN is

RAVEN is a cross-platform agentic application runtime. It turns command-driven software into systems that understand a goal, reason over context, discover capabilities, plan, execute under policy, verify the outcome, and adapt.

RAVEN is not a chatbot, a conversational UI kit, an LLM wrapper, a bag of model APIs, a workflow engine, a virtual assistant, or a macro runner.

It is an execution substrate for agentic applications.

```text
Traditional
  Human → UI → Command → Function → Result

RAVEN
  Intent → Context → Goal → Reason → Plan
        → Discover capabilities → Policy
        → Execute → Observe → Verify → Adapt → Outcome
```

The question the application answers:

> What is the user trying to accomplish, and what is the safest effective way to accomplish it?

## 2. Vision

RAVEN is an agentic computing layer between humans, applications, devices, services, and the physical world.

```text
Human
  │ intent
  ▼
RAVEN agent runtime
  ├── Mobile   Web   Desktop
  └── Edge     Cloud  IoT
        │
        ▼
Physical world
```

The application becomes an active participant in the goal. Mobile is the first body. It is not the limit of the architecture.

## 3. One abstraction

```text
RAVEN = Agent + Goal + Environment + Capabilities
      + Context + Memory + Policy + Execution + Evidence
```

An agent without capabilities cannot act. Capabilities without policy are unsafe. Policy without context is blunt. Context without memory is ephemeral. Memory without governance is a liability. Execution without verification cannot claim success.

## 4. The loop

```text
Perceive → Reason → Plan → Act → Verify → Adapt → Perceive
```

**Perceive.** Intent, application state, device, sensors, documents, network, location, time, permissions, capabilities, previous execution.

**Reason.** Meaning, implied goal, constraints, dependencies, options, risk, missing information.

**Plan.** Decomposition, sequence, fallbacks, verification strategy, points where a person must intervene.

**Act.** Application capabilities, system APIs, remote services, other agents, devices, actuators. The model does not call these directly.

**Verify.** Did the action happen, did the intended state arrive, is the evidence enough?

**Adapt.** Retry, change strategy, ask, compensate, roll back, delegate, abandon safely, or continue from the new state.

## 5. Intent over workflow

A banking flow of screens becomes one intent: "Move ₹50,000 from savings to the investment account." RAVEN identifies accounts, checks ownership, balance, and limits, judges risk, prepares the transaction, requests authorization, executes, verifies ledger state, and reports.

The UI is a way to interact. It is not the operating model.

## 6. Application model

A RAVEN application declares what it can accomplish:

Identity, goals, agents, capabilities, context, events, memory, policies, permissions, UI surfaces, execution, verification, evidence.

## 7. The agent

An agent has identity, purpose, goals, capabilities, context, memory, policies, permissions, trust, delegation rules, constraints, and verification requirements.

An agent may be personal, application-specific, enterprise, device-local, edge, cloud, specialized, delegated, or one of many.

## 8. Goal

A goal is not a script. It carries intent, desired outcome, constraints, priority, deadline, context, allowed capabilities, forbidden actions, risk tolerance, verification requirements, and completion criteria.

"Prepare my day" is deliberately underspecified. The plan is generated at runtime: calendar, tasks, location, travel, weather, documents, conflicts, recommendations, and approval for consequential steps.

## 9. Context

Personal, temporal, spatial, device, application, environmental, and security context.

Context is typed, scoped, time-aware, confidence-aware, permission-aware, and revocable.

## 10. Memory

Working, session, episodic, preference, and knowledge memory.

Memory supports consent, classification, retention, expiration, encryption, deletion, provenance, and access policy. This repository names the classes. It does not store them yet.

## 11. Capability

A capability is the universal action primitive. It exposes identity, description, input and output schema, preconditions, postconditions, required permissions, risk, cost, latency, availability, verification method, and compensation or undo.

Examples range from `read_calendar` to `transfer_money` and `open_door`. Describing an action here does not implement it and does not permit it.

## 12. Discovery

The agent asks what it can do here. The runtime answers with the registry, filtered by policy, context, and trust.

A laptop, a phone, and an edge node expose different capabilities. The agent does not need to know which adapter produced them.

## 13. Adapters

A capability may be implemented by a native API, REST, GraphQL, MCP, App Intents, Android AppFunctions, a local process, a CLI, a browser API, a database, a driver, an IoT protocol, or another agent.

RAVEN sits above those technologies. It does not replace them.

## 14. Tool registry and tool runtime

The registry tracks discovery, registration, version, schema, permission, risk, availability, trust, verification, and lifecycle. Entries can be revoked.

The tool runtime validates arguments, authorizes, executes, times out, retries, cancels, normalizes results, verifies, and records telemetry.

```text
Model → Intent → Planner → Policy → Tool runtime → Capability
```

The model never directly executes arbitrary system actions.

## 15. Risk

| Level | Meaning | Example |
|---|---|---|
| L0 | Observation | Read weather |
| L1 | Local low-risk action | Create a note |
| L2 | External communication | Send email |
| L3 | Sensitive action | Share a private document |
| L4 | Financial or consequential | Transfer money |
| L5 | Physical-world action | Open a gate |

## 16. Policy

Outcomes are `ALLOW`, `ALLOW_WITH_POLICY`, `ASK_USER`, and `DENY`.

Policy may depend on user, agent, capability, context, device, location, time, risk, identity, trust, history, and organization.

In this foundation, L3, L4, and L5 always ask. See `docs/ARCHITECTURE.md`.

## 17. Permission is part of intelligence

Traditional permissions ask whether an application may access X. RAVEN asks whether this agent may do X, for this goal, under these circumstances.

The same camera is allowed for an explicit photo, questioned for background observation, and denied when enterprise policy forbids capture.

## 18. Human control and autonomy

The person can observe, suggest, prepare, ask, execute, verify, undo, and stop.

Autonomy is per capability:

```text
L0 Manual
L1 Assist
L2 Suggest
L3 Guided
L4 Conditional
L5 Policy-bounded
```

A person may allow calendar reads at L5 and keep money movement at L1.

## 19. Execution and verification

The executor schedules work, honors dependencies, bounds concurrency, retries, times out, cancels, compensates, and checkpoints.

```text
Action → Observe → Verify → success or adapt
```

"Send this document to John" is not done because a send API returned. The recipient, the document, the submission, and the resulting state have to check out.

## 20. Evidence

RAVEN keeps the decision, the evidence, the action, the result, and the verification. Explanations cite observations. They do not dump private model traces.

An evidence graph links goal, evidence, decision, action, outcome, and verification for audit, debugging, compliance, and evaluation.

## 21. Events and environment

RAVEN does not wait only for prompts. Location, calendar, battery, network, sensors, tool completion, and deadlines are triggers.

```text
Event → Context update → Goal evaluation → Policy → Decision
```

The environment is a first-class object: platform, device, sensors, applications, network, capabilities, identity, policies, physical context, and other agents.

## 22. Platforms

The agent model stays stable. Capabilities change.

Mobile is the richest first body: camera, microphone, location, motion, biometrics, contacts, calendar, notifications, radios, files, and other apps.

iOS integration uses Swift, SwiftUI, App Intents, Foundation Models, Vision, and the rest of the system frameworks where they fit. The model adapter stays replaceable.

Android integration targets current platform APIs, with AppFunctions explored as a discovery surface and on-device inference where the device provides it. Compatibility and distribution baselines are recorded when a mobile crate exists, not before.

Web runs the core in a worker where possible, and behind a gateway where the browser cannot hold the capability. Desktop integrates with the local machine: files, terminal, tools, git, data, and local models, still under policy.

## 23. Seven planes

1. **Experience** — voice, UI, widgets, notifications, camera, wearables
2. **Intelligence** — intent, context, planner, reasoner, model router
3. **Capability** — registry, adapters, MCP, native and system APIs, other agents
4. **Governance** — identity, permission, policy, trust, risk, consent, audit
5. **Execution** — scheduler, transactions, retry, compensation, cancellation
6. **Memory** — working, session, episodic, preference, knowledge, context
7. **Evidence** — decision, action, outcome, verification, audit

## 24. Model routing

The router chooses a model from privacy, latency, cost, context size, reasoning, multimodal need, connectivity, battery, device, availability, and policy.

```text
Local first → edge when it helps → cloud when it is necessary
```

Preferred providers are replaceable: on-device system models, local open models, enterprise models, cloud models. Offline, the runtime keeps local reasoning, local memory, local tools, and a queue. On reconnect it synchronizes, reconciles, verifies, and continues.

## 25. Agentic UI and continuity

The surface shows the goal, context, plan, actions, progress, evidence, and outcome. States include understanding, planning, waiting for approval, executing, verifying, and completed. A person can intervene at each meaningful state.

A handoff to another device carries goal, memory, context, plan, permissions, execution state, evidence, and pending actions.

Delegation to another agent follows discover, negotiate, request, authorize, execute, report, verify, close. An agent contract states identity, purpose, goals, capabilities, required context, permissions, policies, constraints, memory scope, trust, delegation, and verification.

Agent identity is distinct from the user's: owner, issuer, capabilities, trust, credentials, scope, expiration, and delegation rights.

## 26. Security

Identity, authentication, authorization, capability policy, execution policy, verification, audit.

Requirements include secure storage, encryption, least privilege, isolation, sandboxing, credential isolation, signed tools, provenance, audit, and revocation.

Credentials stay in a broker. They do not enter the model context.

External content — pages, mail, documents, messages, codes, images, tool output, other agents — crosses an untrusted boundary, is classified, and meets policy before any tool is authorized.

> Data is not automatically an instruction.

Tool descriptions can be hostile. Tools have identity, signature, provenance, version, trust, and restrictions.

Memory is classified and released by agent, goal, purpose, policy, consent, and context.

Physical effects pass a safety controller and a sensor check. Software acceptance is not physical success.

## 27. Developer experience

Developers describe capabilities, constraints, and outcomes. The runtime orchestrates.

An agentic application manifest eventually declares goals, capabilities, context, memory, policies, and whether verification is required.

The SDK surface is agent, goal, context, memory, capability, tool, policy, permission, event, plan, execution, verification, evidence, and contract.

The illustrative call, not the current syntax:

```rust
let agent = Agent::builder()
    .name("personal-agent")
    .goal("prepare_my_day")
    .build();
```

Events enter the runtime, not a private handler beside it.

## 28. Demonstrations

**Prepare my day.** One sentence. Calendar, tasks, location, travel, weather, messages, documents, a plan, approval where required, action, verification. This is the first demo, and it runs in-process today.

**Continue on my laptop.** The phone captures intent. The laptop recovers the goal and uses desktop capabilities. Cloud analysis is optional. Results merge and verify.

**Prepare the house.** Location, arrival, weather, home state, energy, security, a plan, policy, actions, sensor verification.

## 29. Evaluation

Measure goal completion, efficiency, latency, cost, recovery, verification accuracy, safety, human intervention, failure, tool selection, and policy violations.

The question is not whether a model wrote a pleasing answer. The question is whether the agent safely achieved the outcome.

Test ambiguous intent, missing information, tool and network failure, permission denial, conflicting goals, malicious content, bad model output, state changes, interruption, duplicate events, partial success, and rollback.

A replay record eventually holds the goal, context, policy, capabilities, model configuration, actions, results, evidence, and final state.

## 30. Protocols and ecosystem

Future layers: application, agent, capability, context, evidence, and policy, over local IPC, HTTP, QUIC, WebSocket, WebTransport, Bluetooth, or MQTT.

MCP is one capability adapter. RAVEN adds intent, planning, policy, identity, memory, execution, verification, evidence, and autonomy.

Applications become capability providers. A personal agent may delegate to work, travel, and home agents. Trust is dynamic and limits visibility, autonomy, delegation, credentials, and execution.

Portable agents are a later goal: agent, memory, goals, policies, and capabilities moving between environments.

## 31. Privacy and minimization

Local by default. Minimal exposure. Explicit consent. Purpose limitation. Short retention. Encrypted memory. Isolated credentials. Auditable execution.

The model receives the context the goal requires, after policy filtering. It does not receive the database.

## 32. Definition

> RAVEN is a cross-platform agentic application runtime that enables software to understand intent, reason over context, formulate goals, discover capabilities, plan actions, execute them under explicit policy and permissions, verify outcomes, maintain governed memory, adapt to changing conditions, and collaborate with other agents across mobile, web, desktop, edge, cloud, and physical environments.

```text
RAVEN = Intent + Context + Goal + Agent + Capability
      + Policy + Memory + Execution + Verification
      + Evidence + Environment

Intent → Understand → Goal → Context → Plan → Authorize
      → Act → Observe → Verify → Adapt → Outcome
```

The programming move is from `if condition then function` toward:

```text
Achieve outcome
with constraints
using available capabilities
under policy
and verify the result
```

## 33. Invariants

These stay unless they are deliberately revised and marked as such:

- Rust core, edition 2024 as the target
- Intent-centric execution
- Goal-first architecture
- Capability abstraction
- Policy-governed autonomy
- Human authority
- Verification-driven completion
- Evidence-aware execution
- Local-first intelligence
- Model and provider independence
- Event-driven runtime
- Cross-platform operation
- Agent portability
- Multi-agent extensibility
- Physical-world readiness

Changes are classified as concept extension, architecture extension, implementation detail, platform adapter, experimental feature, or deprecated direction.

Mobile is the first body. Web and desktop are further environments. Edge is the local nervous system. Cloud is optional extended cognition. Capabilities are the hands. Sensors are the senses. Memory is continuity. Policy is restraint. Verification is reality. The agent decides. The human remains the authority.
