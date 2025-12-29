#!/bin/bash
set -euox pipefail

cat > .env <<EOF
UID=$(id -u)
GID=$(id -g)
EOF
