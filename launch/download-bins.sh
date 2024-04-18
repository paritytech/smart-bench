#!/usr/bin/env bash
set -euo pipefail

polkadot_sdk_version=1.7.0
polkadot_sdk_url="https://github.com/paritytech/polkadot-sdk/releases/download/polkadot-v${polkadot_sdk_version}"

zombienet_version=1.3.100
moonbeam_version=13_12_2023

packages_to_download=$(cat << EOF
[
  {
    "target":"polkadot-parachain",
    "url": "${polkadot_sdk_url}/polkadot-parachain",
    "format": "bin",
    "sha256": "${polkadot_sdk_url}/polkadot-parachain.sha256"
  },
  {
    "target":"polkadot-execute-worker",
    "url": "${polkadot_sdk_url}/polkadot-execute-worker",
    "format": "bin",
    "sha256": "${polkadot_sdk_url}/polkadot-execute-worker.sha256"
  },
  {
    "target":"polkadot-prepare-worker",
    "url": "${polkadot_sdk_url}/polkadot-prepare-worker",
    "format": "bin",
    "sha256": "${polkadot_sdk_url}/polkadot-prepare-worker.sha256"
  },
  {
    "target":"polkadot",
    "url": "${polkadot_sdk_url}/polkadot",
    "format": "bin", 
    "sha256": "${polkadot_sdk_url}/polkadot.sha256"
  },
  {
    "target":"zombienet",
    "url": "https://github.com/paritytech/zombienet/releases/download/v$zombienet_version/zombienet-linux-x64",
    "format": "bin",
    "sha256": "7be7d913cbb1f77e309d6a72c3b5342daca1406a0c1c452e3113120bb5feb007"
  },
  {
    "target":"moonbeam",
    "url": "https://github.com/karolk91/moonbeam/releases/download/$moonbeam_version/moonbeam.gz",
    "format": "gz",
    "sha256": "8402a01a0fdadaf7ffbae59e84982f3186e73de8d3ea9a0cb67aaa81b90a7f48"
  }
]
EOF
)

download_package() {
  local package_json=$1
  local url; url=$(echo "${package_json}" | jq -r '.url')
  local target; target=$(echo "${package_json}" | jq -r '.target')
  local format; format=$(echo "${package_json}" | jq -r '.format')
  local sha256; sha256=$(echo "${package_json}" | jq -r '.sha256')

  case "${sha256}" in
    "http"*)
      curl --location "${sha256}" --output "${target}.sha256"
      sha256=$(cut -d' ' -f1 "${target}.sha256")
      rm "${target}.sha256"
    ;;
  esac

  curl --location "${url}" --output "_${target}"
  echo "${sha256} _${target}" | sha256sum --check
  
  if [ "${format}" = "gz" ]; then
    tar xzf "_${target}"
    rm "_${target}"
  else
    mv "_${target}" "${target}"
  fi

  chmod +x "${target}"
}


mkdir -p bin

item_count=$(echo "${packages_to_download}" | jq length)
(cd bin && for i in $(seq 0 $((item_count-1)) ); do 
  item=$(echo "$packages_to_download" | jq -r ".[$i]")
  target=$(echo "${item}" | jq -r '.target')
  if [ -f "${target}" ]; then
    echo "[${target}] already exists"
  else
    echo "[${target}] downloading"
    download_package "${item}"
    echo "[${target}] downloaded"
  fi
done)
