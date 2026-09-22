# RavenKit

Swift package that links the RAVEN UniFFI bridge into iOS apps.

## Build the native library

From the repository root (requires Xcode and the `aarch64-apple-ios` / `aarch64-apple-ios-sim` Rust targets):

```bash
./scripts/build-ios-ffi.sh
```

That produces:

- `RavenFFI.xcframework`
- `Generated/RavenFFI.swift` (UniFFI bindings)

Copy or leave the generated Swift file where the app target can compile it. The sample app includes it via the Xcode project.

## Planner

`DayPlanner` is the replaceable host planner.

- `FoundationModelsDayPlanner` — Apple Foundation Models (`SystemLanguageModel`) with guided generation; falls back when unavailable
- `ScriptedDayPlanner` — deterministic plan matching the Rust demo

Requires iOS 26+.

