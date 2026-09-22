# Prepare my day

The canonical RAVEN demonstration.

```bash
cargo run -p raven-cli -- demo prepare-my-day
cargo run -p raven-cli -- demo prepare-my-day --approve
```

The runtime receives one intent: "Prepare my day."

It then:

1. Discovers calendar, tasks, weather, location, and message capabilities.
2. Asks a replaceable `ModelAdapter` (`ScriptedModel` in this demo) for a plan.
3. Accepts only steps whose capabilities were discovered, then authorizes.
4. Allows the four observations (risk L0) and verifies each fixture result.
5. Stops at `send_message` (risk L2) with `AskUser`, holding a durable checkpoint.

Without `--approve`, the message tool is bound but never invoked.

With `--approve`, the host resumes the checkpoint. The pending step runs, verifies, and the goal completes.

The same checkpoint can be denied (goal fails) or cancelled. A `CancellationToken` can also stop the run between steps.

No calendar, network, or notification API is contacted.
