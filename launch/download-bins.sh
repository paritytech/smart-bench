#!/usr/bin/env bash
set -euo pipefail

cumulus_version=0.9.420
polkadot_version=0.9.42
zombienet_version=1.3.58
moonbeam_version=30_06_2023

packages_to_download=$(cat << EOF
[
  {
    "target":"polkadot-parachain",
    "url": "https://github.com/paritytech/cumulus/releases/download/v${cumulus_version}/polkadot-parachain",
    "format": "bin",
    "sha256": "https://github.com/paritytech/cumulus/releases/download/v${cumulus_version}/polkadot-parachain.sha256"
  },
  {
    "target":"polkadot",
    "url": "https://github.com/paritytech/polkadot/releases/download/v$polkadot_version/polkadot",
    "format": "bin", 
    "sha256": "https://github.com/paritytech/polkadot/releases/download/v$polkadot_version/polkadot.sha256"
  },
  {
    "target":"zombienet",
    "url": "https://github.com/paritytech/zombienet/releases/download/v$zombienet_version/zombienet-linux-x64",
    "format": "bin",
    "sha256": "e49b6f15c8aa304e38ad8819c853d721f2f580f3906e6e03601b6824de6964fc"
  },
  {
    "target":"moonbeam",
    "url": "https://github.com/karolk91/moonbeam/releases/download/$moonbeam_version/moonbeam.gz",
    "format": "gz",
    "sha256": "ef23292bbe301b51ed9ecfd34be61c5d26cf68cc7e262fd5bc20987f93eabe72"
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
