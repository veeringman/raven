# Prepare my day

The canonical RAVEN demonstration.

```bash
cargo run -p raven-cli -- demo prepare-my-day
```

The runtime receives one intent: "Prepare my day."

It then:

1. Discovers calendar, tasks, weather, location, and message capabilities.
2. Plans those steps in that order.
3. Allows the four observations (risk L0) and verifies each fixture result.
4. Stops at `send_message` (risk L2) with `AskUser`.

The message tool is bound and would return evidence if policy allowed it. Policy does not allow it at the default autonomy, so `invoke` is never called.

No calendar, network, or notification API is contacted.
