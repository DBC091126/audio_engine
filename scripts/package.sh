#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

if ! command -v cargo >/dev/null 2>&1 && [ -x /tmp/cargo/bin/cargo ]; then
  export PATH="/tmp/cargo/bin:$PATH"
  export CARGO_HOME="${CARGO_HOME:-/tmp/cargo}"
  export RUSTUP_HOME="${RUSTUP_HOME:-/tmp/rustup}"
fi

OS="$(uname -s)"
MACHINE="$(uname -m)"
if [ -n "${RUST_TARGET:-}" ]; then
  TARGET="$RUST_TARGET"
else
  case "$OS-$MACHINE" in
    Linux-x86_64) TARGET="x86_64-unknown-linux-gnu" ;;
    Linux-aarch64|Linux-arm64) TARGET="aarch64-unknown-linux-gnu" ;;
    Darwin-x86_64) TARGET="x86_64-apple-darwin" ;;
    Darwin-arm64|Darwin-aarch64) TARGET="aarch64-apple-darwin" ;;
    MINGW*-x86_64|MSYS*-x86_64) TARGET="x86_64-pc-windows-gnu" ;;
    *)
      echo "unsupported platform: $OS $MACHINE" >&2
      exit 2
      ;;
  esac
fi

NATIVE_DIR="$ROOT/target/$TARGET/release"
if [ -n "${RUST_TARGET:-}" ]; then
  rustup target add "$TARGET"
  (cd "$ROOT" && cargo build --release --target "$TARGET")
else
  (cd "$ROOT" && cargo build --release)
  NATIVE_DIR="$ROOT/target/release"
fi

JAVA_HOME="${GUI_JAVA_HOME:-/home/dbc211/.sdkman/candidates/java/21.0.9-tem}"
export JAVA_HOME
export PATH="$JAVA_HOME/bin:/home/dbc211/.sdkman/candidates/maven/current/bin:/usr/bin:/bin"

cd "$ROOT/gui"
mvn -q -DskipTests package -Daudio.engine.native.dir="$NATIVE_DIR"

DIST="$ROOT/gui/target/audio-engine-gui-0.1.0-dist"
OUT="$ROOT/dist"
rm -rf "$OUT"
mkdir -p "$OUT"

PACKAGE_TYPE="${PACKAGE_TYPE:-}"
if [ -z "$PACKAGE_TYPE" ]; then
  case "$OS" in
    Linux) PACKAGE_TYPE="deb" ;;
    Darwin) PACKAGE_TYPE="dmg" ;;
    MINGW*|MSYS*) PACKAGE_TYPE="exe" ;;
    *) PACKAGE_TYPE="app-image" ;;
  esac
fi

JPACKAGE_ARGS=(
  --type "$PACKAGE_TYPE"
  --input "$DIST"
  --dest "$OUT"
  --name "AudioEngine"
  --app-version "0.1.0"
  --main-jar "audio-engine-gui-0.1.0.jar"
  --main-class "com.losshifi.audioengine.Main"
  --module-path "$DIST"
  --add-modules "javafx.controls"
  --java-options "-Dprism.order=sw"
)

if [ "$PACKAGE_TYPE" = "deb" ]; then
  JPACKAGE_ARGS+=(
    --linux-package-name "audio-engine"
    --linux-deb-maintainer "Audio Engine <audio@example.invalid>"
  )
fi

"$JAVA_HOME/bin/jpackage" "${JPACKAGE_ARGS[@]}"
echo "package output: $OUT"
