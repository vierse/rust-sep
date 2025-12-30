#!/bin/bash
set -euox pipefail

docker run -p 3000:3000 --rm shorten-app