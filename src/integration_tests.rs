// Copyright 2018-2023 Parity Technologies (UK) Ltd.
// This file is part of cargo-contract.
//
// smart-bench is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// smart-bench is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with smart-bench.  If not, see <http://www.gnu.org/licenses/>.

use anyhow::Result;
use serial_test::serial;
use std::{ffi::OsStr, process, str, thread, time};
use subxt::{OnlineClient, PolkadotConfig as DefaultConfig};

// Check if str match with pattern
fn is_match(stdout: &str, pattern: &str) -> bool {
    let regex = regex::Regex::new(pattern).unwrap();
    regex.is_match(stdout)
}

const SMART_BENCH_STATS_PATTERN: &str = r"[0-9]+: PoV Size=[0-9]+KiB\([0-9]+%\) Weight RefTime=[0-9]+ms\([0-9]+%\) Weight ProofSize=[0-9]+KiB\([0-9]+%\) Witness=[0-9]+KiB Block=[0-9]+KiB NumExtrinsics=[0-9]+";
const CONTRACTS_NODE_WASM: &str = "substrate-contracts-node";
const CONTRACTS_NODE_EVM: &str = "moonbeam";

// Copied from crate cargo-contract integration_tests.rs
/// Spawn and manage an instance of a compatible contracts enabled chain node.
#[allow(dead_code)]
struct ContractsNodeProcess {
    proc: process::Child,
    tmp_dir: tempfile::TempDir,
    client: OnlineClient<DefaultConfig>,
}

impl Drop for ContractsNodeProcess {
    fn drop(&mut self) {
        self.kill()
    }
}

impl ContractsNodeProcess {
    async fn spawn<S>(program: S, args: &[&str]) -> Result<Self>
    where
        S: AsRef<OsStr>,
    {
        let tmp_dir = tempfile::Builder::new()
            .prefix("cargo-contract.cli.test.node")
            .tempdir()?;

        let mut cmd = process::Command::new(program);
        cmd.env("RUST_LOG", "error")
            .arg("--dev")
            .arg(format!("--base-path={}", tmp_dir.path().to_string_lossy()));
        args.into_iter().for_each({
            |&e| {
                match e {
                    args if args.contains(' ') => {
                        cmd.args(args.split(' ').collect::<Vec<_>>().as_slice())
                    }
                    arg => cmd.arg(arg),
                };
            }
        });

        let mut proc = cmd.spawn()?;
        // wait for rpc to be initialized
        const MAX_ATTEMPTS: u32 = 20;
        let mut attempts = 1;
        let client = loop {
            thread::sleep(time::Duration::from_secs(1));
            tracing::debug!(
                "Connecting to contracts enabled node, attempt {}/{}",
                attempts,
                MAX_ATTEMPTS
            );
            let result = OnlineClient::new().await;
            if let Ok(client) = result {
                break Ok(client);
            }
            if attempts < MAX_ATTEMPTS {
                attempts += 1;
                continue;
            }
            if let Err(err) = result {
                break Err(err);
            }
        };
        match client {
            Ok(client) => Ok(Self {
                proc,
                client,
                tmp_dir,
            }),
            Err(err) => {
                let err = anyhow::anyhow!(
                    "Failed to connect to node rpc after {} attempts: {}",
                    attempts,
                    err
                );
                tracing::error!("{}", err);
                proc.kill()?;
                Err(err)
            }
        }
    }

    fn kill(&mut self) {
        tracing::debug!("Killing contracts node process {}", self.proc.id());
        if let Err(err) = self.proc.kill() {
            tracing::error!(
                "Error killing contracts node process {}: {}",
                self.proc.id(),
                err
            )
        }
    }
}

/// Init a tracing subscriber for logging in tests.
///
/// Be aware that this enables `TRACE` by default. It also ignores any error
/// while setting up the logger.
///
/// The logs are not shown by default, logs are only shown when the test fails
/// or if [`nocapture`](https://doc.rust-lang.org/cargo/commands/cargo-test.html#display-options)
/// is being used.
#[cfg(feature = "integration-tests")]
pub fn init_tracing_subscriber() {
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::TRACE)
        .with_test_writer()
        .try_init();
}

/// Create a `smart-bench` command
fn smart_bench() -> assert_cmd::Command {
    let cmd = assert_cmd::Command::cargo_bin("smart-bench").unwrap();
    cmd
}

/// Tests Ink! wasm contract
///
/// # Note
///
/// Requires [`substrate-contracts-node`](https://github.com/paritytech/substrate-contracts-node/) to
/// be installed and available on the `PATH`, and the no other process running using the default
/// port `9944`.
#[async_std::test]
#[serial]
async fn test_ink_contract_success() {
    init_tracing_subscriber();
    let node_process = ContractsNodeProcess::spawn(CONTRACTS_NODE_WASM, &[])
        .await
        .expect("Error spawning contracts node");

    let output = smart_bench()
        .arg("ink-wasm")
        .arg("flipper")
        .args(["--instance-count", "1"])
        .args(["--call-count", "1"])
        .args(["--url", "ws://localhost:9944"])
        .timeout(std::time::Duration::from_secs(5))
        .output()
        .expect("failed to execute process");
    let stderr = str::from_utf8(&output.stderr).unwrap();
    let stdout = str::from_utf8(&output.stdout).unwrap();

    assert!(
        output.status.success(),
        "smart-bench ink-wasm test failed: {stderr}"
    );
    let lines: Vec<&str> = stdout.lines().collect();
    assert!(
        is_match(lines.last().unwrap(), SMART_BENCH_STATS_PATTERN),
        "Incorrect output with stats: {stdout}"
    );
    // prevent the node_process from being dropped and killed
    let _ = node_process;
}

/// Tests solidity wasm contract (solang)
///
/// # Note
///
/// Requires [`substrate-contracts-node`](https://github.com/paritytech/substrate-contracts-node/) to
/// be installed and available on the `PATH`, and the no other process running using the default
/// port `9944`.
#[async_std::test]
#[serial]
async fn test_solidity_wasm_contract_success() {
    init_tracing_subscriber();
    let node_process = ContractsNodeProcess::spawn(CONTRACTS_NODE_WASM, &[])
        .await
        .expect("Error spawning contracts node");

    let output = smart_bench()
        .arg("sol-wasm")
        .arg("flipper")
        .args(["--instance-count", "1"])
        .args(["--call-count", "1"])
        .args(["--url", "ws://localhost:9944"])
        .timeout(std::time::Duration::from_secs(5))
        .output()
        .expect("failed to execute process");
    let stderr = str::from_utf8(&output.stderr).unwrap();
    let stdout = str::from_utf8(&output.stdout).unwrap();

    assert!(
        output.status.success(),
        "smart-bench sol-wasm test failed: {stderr}"
    );
    let lines: Vec<&str> = stdout.lines().collect();
    assert!(
        is_match(lines.last().unwrap(), SMART_BENCH_STATS_PATTERN),
        "Incorrect output with stats: {stdout}"
    );

    // prevent the node_process from being dropped and killed
    let _ = node_process;
}

/// Tests solidity evm contract
///
/// # Note
///
/// Requires [`moonbeam`](https://github.com/PureStake/moonbeam/) with enabled
/// [`dev RPC`](https://github.com/paritytech/substrate-contracts-node/blob/539cf0271090f406cb3337e4d97680a6a63bcd2f/node/src/rpc.rs#L60)
/// to be installed and available on the `PATH`, and the no other process running using the default
/// port `9944`.
#[async_std::test]
#[serial]
async fn test_solidity_evm_contract_success() {
    init_tracing_subscriber();
    let node_process = ContractsNodeProcess::spawn(CONTRACTS_NODE_EVM, &["--sealing instant"])
        .await
        .expect("Error spawning contracts node");

    let output = smart_bench()
        .arg("evm")
        .arg("flipper")
        .args(["--instance-count", "1"])
        .args(["--call-count", "1"])
        .args(["--url", "ws://localhost:9944"])
        .timeout(std::time::Duration::from_secs(5))
        .output()
        .expect("failed to execute process");
    let stderr = str::from_utf8(&output.stderr).unwrap();
    let stdout = str::from_utf8(&output.stdout).unwrap();

    assert!(
        output.status.success(),
        "smart-bench evm test failed: {stderr}"
    );
    let lines: Vec<&str> = stdout.lines().collect();
    assert!(
        is_match(lines.last().unwrap(), SMART_BENCH_STATS_PATTERN),
        "Incorrect output with stats: {stdout}"
    );

    // prevent the node_process from being dropped and killed
    let _ = node_process;
}

/// Test for not existing contract name
#[async_std::test]
async fn test_bad_contract_name_fail() {
    smart_bench()
        .arg("ink-wasm")
        .arg("Badflipper")
        .args(["--instance-count", "1"])
        .args(["--call-count", "1"])
        .args(["--url", "ws://localhost:9944"])
        .assert()
        .failure();
}

/// Test for wrong port
#[async_std::test]
#[serial]
async fn test_bad_contract_node_port_fail() {
    //Node is not started so any port is bad
    smart_bench()
        .arg("ink-wasm")
        .arg("flipper")
        .args(["--instance-count", "1"])
        .args(["--call-count", "1"])
        .args(["--url", "ws://localhost:9944"])
        .assert()
        .failure();
}
