#!/usr/bin/env bash
# Build RavenFFI.xcframework and Swift bindings for the sample iOS app.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

# Prefer an explicit workspace target dir so Cursor sandbox caches do not
# silently relocate artifacts away from apps/ios.
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target}"
TARGET_DIR="$CARGO_TARGET_DIR"

OUT="$ROOT/apps/ios/RavenKit"
GEN="$OUT/Generated"
HEADERS="$OUT/.headers"
XCFRAMEWORK="$OUT/RavenFFI.xcframework"
LIB_NAME="libraven_ffi.a"

mkdir -p "$GEN" "$HEADERS" "$TARGET_DIR"

echo "→ Rust static libraries (iOS device + simulator)"
echo "  CARGO_TARGET_DIR=$TARGET_DIR"
cargo build -p raven-ffi --release --target aarch64-apple-ios
cargo build -p raven-ffi --release --target aarch64-apple-ios-sim

DEVICE_LIB="$TARGET_DIR/aarch64-apple-ios/release/$LIB_NAME"
SIM_LIB="$TARGET_DIR/aarch64-apple-ios-sim/release/$LIB_NAME"

if [[ ! -f "$DEVICE_LIB" || ! -f "$SIM_LIB" ]]; then
  echo "missing static libraries:"
  echo "  $DEVICE_LIB"
  echo "  $SIM_LIB"
  exit 1
fi

echo "→ UniFFI Swift bindings"
cargo run -p raven-ffi --bin uniffi-bindgen -- generate \
  --library "$SIM_LIB" \
  --language swift \
  --out-dir "$GEN"

# UniFFI emits raven_ffi.swift + raven_ffiFFI.h (+ modulemap). Normalize names.
if [[ -f "$GEN/raven_ffi.swift" ]]; then
  mv -f "$GEN/raven_ffi.swift" "$GEN/RavenFFI.swift"
fi
if [[ -f "$GEN/raven_ffiFFI.h" ]]; then
  cp "$GEN/raven_ffiFFI.h" "$HEADERS/raven_ffiFFI.h"
elif [[ -f "$GEN/RavenFFIFFI.h" ]]; then
  cp "$GEN/RavenFFIFFI.h" "$HEADERS/raven_ffiFFI.h"
fi

# XCFramework module map pointing at the FFI header.
# UniFFI Swift imports `raven_ffiFFI`, so the module name must match.
cat > "$HEADERS/module.modulemap" <<'EOF'
module raven_ffiFFI {
    header "raven_ffiFFI.h"
    export *
}
EOF

rm -rf "$XCFRAMEWORK"
echo "→ XCFramework"
xcodebuild -create-xcframework \
  -library "$DEVICE_LIB" -headers "$HEADERS" \
  -library "$SIM_LIB" -headers "$HEADERS" \
  -output "$XCFRAMEWORK"

# Keep generated Swift inside the package sources so RavenKit compiles it.
mkdir -p "$OUT/Sources/RavenKit/Generated"
if [[ -f "$GEN/RavenFFI.swift" ]]; then
  cp "$GEN/RavenFFI.swift" "$OUT/Sources/RavenKit/Generated/RavenFFI.swift"
elif [[ -f "$GEN/raven_ffi.swift" ]]; then
  cp "$GEN/raven_ffi.swift" "$OUT/Sources/RavenKit/Generated/RavenFFI.swift"
fi
rm -f "$GEN"/*.h "$GEN"/*.modulemap

echo "Built $XCFRAMEWORK"
echo "Swift bindings at $OUT/Sources/RavenKit/Generated/RavenFFI.swift"
