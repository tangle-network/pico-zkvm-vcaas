// tests/integration_tests.rs
use crate::{
    GENERATE_PROOF_JOB_ID, ProgramLocation, ProofRequest, ProofServiceError, ProvingType,
    ServiceContext, generate_proof,
};
use blueprint_sdk::{
    alloy::primitives::Address,
    extract::Context,
    tangle::extract::{Optional, TangleArg, TangleResult}, // Make sure extractors are public or re-exported if needed
};
use std::path::PathBuf;
use std::str::FromStr;
use tempfile::tempdir;
use url::Url;

// Helper to create a realistic context for testing
// May require mocking HTTP/EVM in the future
fn setup_test_context() -> ServiceContext {
    let temp_base = tempdir()
        .expect("Failed to create base temp dir for tests")
        .into_path();
    // Use a placeholder RPC and address for now. Real tests need mocking or a testnet.
    let rpc_url = Url::parse("http://localhost:8545").unwrap();
    let registry_addr = Address::from_str("0x0000000000000000000000000000000000000000").unwrap();

    ServiceContext::new(rpc_url, registry_addr, temp_base)
        .expect("Failed to create test ServiceContext")
}

#[tokio::test]
async fn test_generate_proof_job_invalid_hash() {
    let ctx = setup_test_context();
    let request = ProofRequest {
        program_hash: "invalid-hash-format".to_string(), // Invalid hash
        inputs: "00".to_string(),
        proving_type: ProvingType::Fast,
        program_location_override: None,
        eth_rpc_url_override: None,
        registry_address_override: None,
    };

    let tangle_arg = TangleArg(request);
    let job_context = Context(ctx);

    let result: TangleResult<Result<_, ProofServiceError>> =
        generate_proof(job_context, tangle_arg).await;

    assert!(result.0.is_err());
    match result.0.err().unwrap() {
        ProofServiceError::InvalidInput(msg) => {
            assert!(msg.contains("Invalid program_hash format"));
        }
        _ => panic!("Expected InvalidInput error"),
    }
}

// --- TODO: More Tests ---
// - test_generate_proof_job_program_not_found (requires mocking EVM call)
// - test_generate_proof_job_download_fails (requires mocking HTTP call)
// - test_generate_proof_job_hash_mismatch (requires local file setup or HTTP mock)
// - test_generate_proof_job_pico_prover_error (requires mocking pico::execute_pico_prove or running a dummy ELF)
// - test_generate_proof_job_success_local_file (requires a dummy ELF and local path override)
// - test_generate_proof_job_success_remote_file (requires HTTP mock server)
// - test_generate_proof_job_success_evm (most complex, needs Pico mock/dummy and EVM mock)
