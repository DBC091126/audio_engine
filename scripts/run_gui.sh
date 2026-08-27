#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT/gui"

JAVA_HOME="${GUI_JAVA_HOME:-/home/dbc211/.sdkman/candidates/java/21.0.9-tem}"
export JAVA_HOME
export PATH="$JAVA_HOME/bin:/home/dbc211/.sdkman/candidates/maven/current/bin:$PATH"

mvn -q javafx:run
