#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

env PATH=/tmp/cargo/bin:/usr/bin:/bin CARGO_HOME=/tmp/cargo RUSTUP_HOME=/tmp/rustup \
  /tmp/cargo/bin/cargo build --release

JAVA_HOME="${GUI_JAVA_HOME:-/home/dbc211/.sdkman/candidates/java/21.0.9-tem}"
export JAVA_HOME
export PATH="$JAVA_HOME/bin:/home/dbc211/.sdkman/candidates/maven/current/bin:/usr/bin:/bin"

cd "$ROOT/gui"
mvn -q -DskipTests package
mvn -q dependency:build-classpath -Dmdep.outputFile=/tmp/audio-engine-gui-cp.txt

java -Djna.library.path="$ROOT/target/release" \
  -cp "target/classes:$(cat /tmp/audio-engine-gui-cp.txt)" \
  com.losshifi.audioengine.FfiSmoke "$ROOT/test.flac" /tmp/audio-engine-gui-smoke.wav
