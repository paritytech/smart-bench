# smart-bench :brain:

> Measure the end-to-end throughput of smart contracts.

## Usage

### Installation 

Currently, this tool must be run directly via cargo, because it requires access to the predefined wasm contract binaries in the `contracts` directory. 

So first clone this repo and run: `cargo run --release -- --help` which should give info for the args:

```
USAGE:
    smart-bench [OPTIONS] --instance-count <INSTANCE_COUNT> --call-count <CALL_COUNT> <CHAIN> [CONTRACTS]...

ARGS:
    <CHAIN>           the smart contract platform to benchmark [possible values: ink-wasm, sol-wasm, evm]
    <CONTRACTS>...    the list of contracts to benchmark with [possible values: erc20, flipper,
                      incrementer, erc721, erc1155, odd-product, triangle-number, storage-read,
                      storage-write, storage-read-write]

OPTIONS:
    -c, --call-count <CALL_COUNT>
            the number of calls to make to each contract

    -h, --help
            Print help information

    -i, --instance-count <INSTANCE_COUNT>
            the number of each contract to instantiate

        --url <url>
            the url of the substrate node for submitting the extrinsics [default:
            ws://localhost:9944]

    -V, --version
            Print version information

```

### Node binaries

- For Wasm contracts on a local pallet-contracts enabled parachain, first [download](./launch/download-bins.sh) (or build from source) the `polkadot` and `polkadot-parachain`
binaries, and make sure they are present in the `launch/bin` directory.
- For a local `moonbeam` parachain setup for EVM contracts, build the node from source from [this fork](https://github.com/ascjones/moonbeam) which has the required dev RPC endpoint enabled. This will also require building the `polkadot` relay-chain node from source at the same commit as the polkadot dependencies for the `moonbeam` binary. The resulting two binaries should be copied to the `launch/bin/moonbeam` directory.

### Launching the local test network

- Install https://github.com/paritytech/polkadot-launch
- Launch the local network
  - Wasm contracts with pallet-contracts: `polkadot-launch launch/contracts-rococo-local.json`
  - EVM contracts on a moonbeam node: `polkadot-launch launch/moonbase-local.json`
- Wait for `POLKADOT LAUNCH COMPLETE`.

### Running benchmarks

`smart-bench` works on a pre-defined set of contracts, and the user can specify which contract(s) should be tested, and how many instances and number of calls should be executed. e.g.

`cargo run --release -- ink-wasm erc20 erc1155 --instance-count 10 --call-count 20 --url ws://localhost:9988`

The above will create 10 instances of each of the `erc20` and `erc1155` contracts, and call each of those instances 20 times (400 total calls). Once all the calls have been submitted, the block stats should appear on the console e.g.

```
0005: PoV Size=0130KiB(005%) Weight RefTime=0000088ms(017%) Weight ProofSize=3277KiB(064%) Witness=0119KiB Block=0011KiB NumExtrinsics=0048
0006: PoV Size=0130KiB(005%) Weight RefTime=0000088ms(017%) Weight ProofSize=3277KiB(064%) Witness=0118KiB Block=0011KiB NumExtrinsics=0048
0007: PoV Size=0130KiB(005%) Weight RefTime=0000088ms(017%) Weight ProofSize=3277KiB(064%) Witness=0119KiB Block=0011KiB NumExtrinsics=0048
0008: PoV Size=0130KiB(005%) Weight RefTime=0000088ms(017%) Weight ProofSize=3277KiB(064%) Witness=0118KiB Block=0011KiB NumExtrinsics=0048
```
One row per block, showing the % usage of the PoV size and the block weight, as well as the number of extrinsics executed per block. Note the Weight % is expected to max out at 75%, since that is the ratio of the total block weight assigned to "normal" i.e. the user submitted/non-operational class of extrinsics.

#### Ink!/Wasm contracts

Currently the Wasm contracts are the `contracts/ink/*.contract` files, some of which have been compiled from https://github.com/paritytech/ink/tree/master/examples and committed to this repository. So in order to modify these they can compiled locally and copied over to the `contracts/ink` dir. There are also two locally defined custom contracts in the `contracts/ink` folder: `computation` and `storage` for testing pure computation and storage operations.

#### Solidity/EVM contracts

Before running the benchmarks against a `pallet-evm` enabled network, the solidity contracts must first be compiled:

- Install hardhat https://hardhat.org/getting-started
- `cd contracts/solidity && npx hardhat compile`

Now make sure the target EVM enabled network is up and running as specified above, and this time change the value of the first argument to `evm`:

`cargo run --release -- evm erc20 erc1155 --instance-count 10 --call-count 20 --url ws://localhost:9988`

#### Solang - Solidity/Wasm contracts

Before running benchmark against a `pallet-contract` enabled network, Solang contract needs to be compiled.
The easiest way to compile the contracts is to do this having Solidity/EVM compiled first.
After this the `openzeppelin_solang.patch` needs to be applied:
`cd contracts/solidity/node_modules/@openzeppelin && patch -p1 < ../../openzeppelin_solang.patch`
Finally a Solang contract can be compiled using command:
`cd contracts/solidity/wasm/ && solang compile --target polkadot --importmap @openzeppelin=../node_modules/@openzeppelin/   ./../contracts/BenchERC1155.sol`
Currently [`solang`](https://github.com/hyperledger/solang) compiler needs to be built from sources including [`U256 type fix commit`](https://github.com/smiasojed/solang/commit/467b25ab3d44884e643e3217ac16c56c5788dccc)


### Integration tests

Smart-bench contains integrations tests, which can be run using command `cargo test`.
Before running tests, smart-bench needs to be build using `cargo build` command.
Integration tests requires two types of nodes to be installed and available on `PATH`.
- [`moonbeam`](https://github.com/PureStake/moonbeam/) with enabled [`dev RPC`](https://github.com/paritytech/substrate-contracts-node/blob/539cf0271090f406cb3337e4d97680a6a63bcd2f/node/src/rpc.rs#L60) for Solidity/EVM contracts
- [`substrate-contracts-node`](https://github.com/paritytech/substrate-contracts-node/) for Ink! and Solang (Solidity/Wasm) contracts
