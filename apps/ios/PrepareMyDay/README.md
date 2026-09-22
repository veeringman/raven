# Prepare My Day (iOS)

Sample iOS app that runs the RAVEN prepare-my-day loop end to end on device/simulator.

## Prerequisites

- Xcode 15+
- Rust with `aarch64-apple-ios` and `aarch64-apple-ios-sim` targets
- [XcodeGen](https://github.com/yonaskolb/XcodeGen) (`brew install xcodegen`) if you regenerate the project

## Build the Rust bridge

From the repository root:

```bash
./scripts/build-ios-ffi.sh
```

This builds `apps/ios/RavenKit/RavenFFI.xcframework` and UniFFI Swift bindings.

## Open and run

```bash
cd apps/ios/PrepareMyDay
xcodegen generate   # only if project.yml changed
open PrepareMyDay.xcodeproj
```

Select an iPhone simulator and run. Flow:

1. Tap **Run intent**
2. Calendar, tasks, weather, and location verify
3. `send_message` stops for approval
4. Tap **Approve send**, **Deny**, or **Cancel run**

Planning uses `FoundationModelsDayPlanner` (Apple on-device system model) through the replaceable `DayPlanner` protocol. If the model is unavailable, it falls back to `ScriptedDayPlanner`. The Rust runtime still authorizes and verifies; the model never invokes tools.
