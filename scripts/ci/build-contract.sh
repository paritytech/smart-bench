#!/bin/bash
#
# Write the original and optimized sizes of a Wasm file as
# a CSV format line to stdout.
#
# Usage: `./build-contract.sh <path_to_contract>`

set -eux
set -o pipefail

CONTRACT=$(basename $1)
SIZE_OUT=$(cargo +nightly contract build --release --manifest-path $1/Cargo.toml --output-json) || exit $?
OPTIMIZED_SIZE=$(echo $SIZE_OUT | jq '.optimization_result.optimized_size')

echo -n "${CONTRACT}, ${OPTIMIZED_SIZE}"
