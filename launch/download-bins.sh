#!/usr/bin/env bash
set -euo pipefail

cumulus_version=0.9.420
polkadot_version=0.9.42
zombienet_version=1.3.58
moonbeam_version=30_06_2023

if [ ! -d bin ]; then
  mkdir bin
  curl https://github.com/paritytech/cumulus/releases/download/polkadot-v$cumulus_version/polkadot-parachain --output ./bin/polkadot-parachain --location
  curl https://github.com/paritytech/polkadot/releases/download/v$polkadot_version/polkadot --output ./bin/polkadot --location
  curl https://github.com/paritytech/zombienet/releases/download/v$zombienet_version/zombienet-linux-x64 --output ./bin/zombienet --location
  curl https://github.com/karolk91/moonbeam/releases/download/$moonbeam_version/moonbeam.gz --output ./bin/moonbeam.gz --location && (cd bin && tar zxvf moonbeam.gz && rm moonbeam.gz)

  chmod +x ./bin/polkadot-parachain
  chmod +x ./bin/polkadot
  chmod +x ./bin/zombienet
  chmod +x ./bin/moonbeam
fi
