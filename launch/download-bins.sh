#!/usr/bin/env bash

cumulus_version=0.9.19
polkadot_version=0.9.19

mkdir bin
curl https://github.com/paritytech/cumulus/releases/download/polkadot-v$cumulus_version/polkadot-collator --output ./bin/polkadot-collator --location
curl https://github.com/paritytech/polkadot/releases/download/v$polkadot_version/polkadot --output ./bin/polkadot --location

chmod +x ./bin/polkadot-collator
chmod +x ./bin/polkadot