#!/usr/bin/env bash
set -euo pipefail

exec 3>&1
exec 2>&3
exec > /dev/null

if echo "${@}" | grep -q evm; then
  zombienet_config=$(realpath -s "${CONFIGS_DIR}/network_native_moonbeam.json")
elif echo "${@}" | grep -q wasm; then
  zombienet_config=$(realpath -s "${CONFIGS_DIR}/network_native_wasm.json")
else
  exit 1
fi

parachain_ws_port=$(grep ws_port "${zombienet_config}" | tr -d '[:space:]' | cut -f2 -d':' | tr -d ',')
PATH="${BINARIES_DIR}:${PATH}" zombienet -p native spawn "${zombienet_config}" &
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

PATH="${BINARIES_DIR}:${PATH}" CARGO_MANIFEST_DIR=$(dirname "${CONTRACTS_DIR}") smart-bench "${@}" --url "ws://localhost:${parachain_ws_port}" 1>&3
