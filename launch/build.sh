#!/usr/bin/env bash
set -euo pipefail

SCRIPT_NAME="${BASH_SOURCE[0]}"
SCRIPT_PATH=$(dirname "$(realpath -s "${BASH_SOURCE[0]}")")
THIS_GIT_REPOSITORY_ROOT=$(git rev-parse --show-toplevel)
VERSION=${VERSION:-latest}
NAME=smart-bench
IMAGE="${NAME}:${VERSION}"

(cd "${THIS_GIT_REPOSITORY_ROOT}" &&
   DOCKER_BUILDKIT=1 docker build \
     --build-arg DOCKERFILE_DIR="$(realpath --relative-to="${THIS_GIT_REPOSITORY_ROOT}" "${SCRIPT_PATH}")" \
     -f "${SCRIPT_PATH}/smart_bench.Dockerfile" -t "${IMAGE}" .
)
