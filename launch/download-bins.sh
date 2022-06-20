#!/usr/bin/env bash

cumulus_version=0.9.24
polkadot_version=0.9.24

mkdir bin
curl https://github.com/paritytech/cumulus/releases/download/polkadot-v$cumulus_version/polkadot-parachain --output ./bin/polkadot-parachain --location
curl https://github.com/paritytech/polkadot/releases/download/v$polkadot_version/polkadot --output ./bin/polkadot --location

chmod +x ./bin/polkadot-parachain
chmod +x ./bin/polkadot