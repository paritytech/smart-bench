## Overview

This directory contains docker related scripts and configurations to run smart-bench within consistent environment.

Use a simple wrapper script for one-shot smart-bench usage and it will take care of building smart-bench software, run zombienet network and smart-bench itself and generate benchmarking results

The tools within this directory are also meant to be used by CI pipelines

## Usage
### Build

downloads dependencies
```
./downloads-bins.sh
```

builds smart-bench:latest image
```
./build.sh
```

or build using manual VERSION, eg. smart-bench:1.0 image
```
VERSION=1.0 ./build.sh
```


### Run 
```
Usage: ./run.sh OPTION -- ARGUMENTS_TO_SMART_BENCH

OPTION
 -b, --binaries-dir   (Optional) Path to directory that contains binaries to mount into the container (eg. polkadot, zombienet, moonbeam)
                      List of binaries being used depends on config provided. Default set of binaries is available within the image
 -t, --contracts-dir  (Optional) Path to directory that contains compiled smart contracts. Default set of compiled smart contracts is available within the image
 -u, --configs-dir    (Optional) Path to directory that contains zombienet config files. Default set of configs files is available within the image
 -h, --help           Print this help message

ARGUMENTS_TO_SMART_BENCH
  smart-bench specific parameters (NOTE: do not provide --url param as it is managed by this tool)

EXAMPLES
./run.sh -- evm erc20 --instance-count 1 --call-count 10
./run.sh -- ink-wasm erc20 --instance-count 1 --call-count 10
./run.sh -- sol-wasm erc20 --instance-count 1 --call-count 10
./run.sh --binaries-dir=./bin -- sol-wasm erc20 --instance-count 1 --call-count 10
./run.sh --contracts-dir=../contracts -- sol-wasm erc20 --instance-count 1 --call-count 10
./run.sh --configs-dir=./configs -- sol-wasm erc20 --instance-count 1 --call-count 10
```

## Raw Docker runs

Running smart-bench without overriding any binaries or configurations:
```
docker run --rm -it --init smart-bench:latest evm erc20 --instance-count 1 --call-count 10
```

Override binaries:
```
docker run --rm -it --init -v $PWD/bin:/usr/local/smart-bench/bin smart-bench:latest sol-wasm erc20 --instance-count 1 --call-count 10
```

Override contracts
NOTE: please note that smart-bench expects some particular files hierarchy for contracts directory, you need to re-create such hierarchy to override files within container
```
docker run --rm -it --init -v $PWD/../contracts:/usr/local/smart-bench/contracts smart-bench:latest sol-wasm erc20 --instance-count 1 --call-count 10
```

Override configs
```
docker run --rm -it --init -v $PWD/configs:/usr/local/smart-bench/config smart-bench:latest sol-wasm erc20 --instance-count 1 --call-count 10
```

## Moonbeam with Dev RPC module enabled build recipe 
Following is an example recipe how to build moonbeam binary with Dev RPC module enabled
```
git clone https://github.com/PureStake/moonbeam.git
cd moonbeam && git fetch https://github.com/karolk91/moonbeam master && git cherry-pick decd877
docker run -it --rm -v $PWD:/moonbeam docker.io/paritytech/ci-linux:production /bin/bash
cd /moonbeam
rustup toolchain install 1.69
rustup default 1.69
rustup override set 1.69
rustup target add wasm32-unknown-unknown --toolchain 1.69
cargo build --release
```
