use std::{
    path::PathBuf, process::{Command, Stdio}
};

use coprocessor_sdk::sdk::Builder;
use log::{error, info};
use trading_volumn_lib::prepare_test_receipts;

// test batchQueryAsync,test submit proof,
fn main() {
    // Initialize the logger with a default filter level
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info) // Set default log level to Info
        .init();

    // add data
    let test_receipts = prepare_test_receipts();
    let mut sdk = Builder::new().with_receipts(test_receipts.receipts).init(64, 0, 0);
    sdk.chain_id = 1;
    sdk.save_inputs(PathBuf::from("./example/trading_volumn_prover/inputs/"))
        .unwrap();

    // before you execute this code you must install brevis request bin first
    // cd network
    // cargo install --path 。

    info!("Starting brevis-requests process...");
    let request_brevis_cmd = Command::new("brevis-request")
        .env("REQUEST_DATA_FILE", "./example/trading_volumn_prover/inputs/request_prove_inputs.json")
        .stdout(Stdio::piped()) // Capture stdout for logging
        .stderr(Stdio::piped()) // Capture stderr for logging
        .spawn();

    match request_brevis_cmd {
        Ok(child) => {
            info!("brevis-request process started successfully.");

            // Wait for the process to complete and capture its output
            let output = child
                .wait_with_output()
                .expect("Failed to wait on child process");

            // Log the exit status
            if output.status.success() {
                info!("brevis-request process completed successfully.");
            } else {
                error!(
                    "brevis-request process failed with exit code: {:?}",
                    output.status.code()
                );
            }

            // Log the stdout and stderr
            if !output.stdout.is_empty() {
                info!(
                    "brevis-request stdout: {}",
                    String::from_utf8_lossy(&output.stdout)
                );
            }
            if !output.stderr.is_empty() {
                error!(
                    "brevis-request stderr: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
            }
        }
        Err(e) => {
            error!("Failed to execute brevis-request process: {}", e);
        }
    }
}
