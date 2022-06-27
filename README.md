# smart-bench :brain: :bench:

> Measure the end-to-end throughput of smart contracts.

## Usage

### Installation 

Currently, this tool must be run directly via cargo, because it requires access to the predefined wasm contract binaries in the `contracts` directory. 

So first clone this repo and run: `cargo run --release -- --help` which should give info for the args:

```
USAGE:
    smart-bench [OPTIONS] --instance-count <INSTANCE_COUNT> --call-count <CALL_COUNT> <CHAIN> [CONTRACTS]...

ARGS:
    <CHAIN>           the smart contract platform to benchmark [possible values: wasm, evm]
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

`cargo run --release -- wasm erc20 erc1155 --instance-count 10 --call-count 20 --url ws://localhost:9988`

The above will create 10 instances of each of the `erc20` and `erc1155` contracts, and call each of those instances 20 times (400 total calls). Once all the calls have been submitted, the block stats should appear on the console e.g.

```
0019: PoV Size=0100KiB(003%) Weight=0000042ms(008%) Witness=0005KiB Block=0094KiB NumExtrinsics=0012
0020: PoV Size=0415KiB(016%) Weight=0000374ms(074%) Witness=0111KiB Block=0303KiB NumExtrinsics=0164
0021: PoV Size=0145KiB(005%) Weight=0000376ms(075%) Witness=0110KiB Block=0034KiB NumExtrinsics=0177
0022: PoV Size=0126KiB(004%) Weight=0000160ms(032%) Witness=0110KiB Block=0016KiB NumExtrinsics=0075

```
One row per block, showing the % usage of the PoV size and the block weight, as well as the number of extrinsics executed per block. Note the Weight % is expected to max out at 75%, since that is the ratio of the total block weight assigned to "normal" i.e. the user submitted/non-operational class of extrinsics.

#### Wasm contracts

Currently the Wasm contracts are the `contracts/*.contract` files, some of which have been compiled from https://github.com/paritytech/ink/tree/master/examples and committed to this repository. So in order to modify these they can compiled locally and copied over to the `contracts` dir. There are also two locally defined custom contracts in the `contracts` folder: `computation` and `storage` for testing pure computation and storage operations.

#### Solidity/EVM contracts

Before running the benchmarks against a `pallet-evm` enabled network, the solidity contracts must first be compiled:

- Install hardhat https://hardhat.org/getting-started
- `cd contracts/solidity && npx hardhat compile`

Now make sure the target EVM enabled network is up and running as specified above, and this time change the value of the first argument to `evm`:

`cargo run --release -- evm erc20 erc1155 --instance-count 10 --call-count 20 --url ws://localhost:9988`





