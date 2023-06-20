## Overview

This directory contains docker related scripts and configurations to run smart-bench within consistent environment.

Use a simple wrapper script for one-shot smart-bench usage and it will take care of building smart-bench software, run zombienet network and smart-bench itself and generate benchmarking results

The tools within this directory are also meant to be used by CI pipelines

## Usage
```
Usage: ./run.sh OPTION -- ARGUMENTS_TO_SMART_BENCH

OPTION
 -b, --binaries-dir   Path to directory that contains all required binaries (eg. polkadot, zombienet, moonbeam)
                      List of required binaries depends on config provided
 -c, --config         Path to zombienet config file
 -h, --help           Print this help message

ARGUMENTS_TO_SMART_BENCH
  smart-bench specific parameters (NOTE: do not provide --url param as it is managed by this tool)

EXAMPLES
./run.sh --binaries-dir="bin/" --config="configs/network_native_moonbeam.toml" -- evm erc20 --instance-count 1 --call-count 10
```
