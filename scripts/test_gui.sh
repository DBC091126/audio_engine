#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

env PATH=/tmp/cargo/bin:/usr/bin:/bin CARGO_HOME=/tmp/cargo RUSTUP_HOME=/tmp/rustup \
  /tmp/cargo/bin/cargo build --release

if [ -n "${GUI_JAVA_HOME:-}" ]; then
  JAVA_HOME="$GUI_JAVA_HOME"
elif [ -x "/home/dbc211/.sdkman/candidates/java/21.0.9-tem/bin/java" ]; then
  JAVA_HOME="/home/dbc211/.sdkman/candidates/java/21.0.9-tem"
elif command -v java >/dev/null 2>&1; then
  JAVA_BIN="$(command -v java)"
  JAVA_HOME="$(cd "$(dirname "$JAVA_BIN")/.." && pwd)"
else
  echo "java not found; set GUI_JAVA_HOME or JAVA_HOME" >&2
  exit 2
fi
export JAVA_HOME
export PATH="$JAVA_HOME/bin:$PATH"

cd "$ROOT/gui"
if command -v mvn >/dev/null 2>&1; then
  MVN="mvn"
elif [ -x ./mvnw ]; then
  MVN="./mvnw"
else
  echo "mvn not found" >&2
  exit 2
fi

"$MVN" -q -DskipTests package
"$MVN" -q dependency:build-classpath -Dmdep.outputFile=/tmp/audio-engine-gui-cp.txt

java -Djna.library.path="$ROOT/target/release" \
  -cp "target/classes:$(cat /tmp/audio-engine-gui-cp.txt)" \
  com.losshifi.audioengine.FfiSmoke "$ROOT/test.flac" /tmp/audio-engine-gui-smoke.wav
