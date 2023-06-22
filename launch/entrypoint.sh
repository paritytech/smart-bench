#!/usr/bin/env bash
set -euo pipefail

exec 3>&1
exec 2>&3
exec > /dev/null

ZOMBIENET_CONFIG=$(realpath -s "$1")
CONTRACTS_DIR=$(realpath -s "$2")
shift 2

parachain_ws_port=$(grep ws_port "${ZOMBIENET_CONFIG}" | tr -d '[:space:]' | cut -f2 -d'=')
zombienet -p native spawn "${ZOMBIENET_CONFIG}" &
zombienet_pid=$!

wait_for_parachain_node() {
  local pid=$1
  local port=$2
  while ! echo q | nc localhost "${port}"; do
    if ! kill -0 "${pid}"; then
      exit 1
    fi
  sleep 1;
  done
}
wait_for_parachain_node "${zombienet_pid}" "${parachain_ws_port}"

CARGO_MANIFEST_DIR=$(dirname "${CONTRACTS_DIR}") smart-bench "${@}" --url "ws://localhost:${parachain_ws_port}" 1>&3
