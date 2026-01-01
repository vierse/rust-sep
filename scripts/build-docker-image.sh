#!/bin/bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

set -x
docker build --file "$PROJECT_ROOT/docker/app.Dockerfile" --tag shorten-app "$PROJECT_ROOT"