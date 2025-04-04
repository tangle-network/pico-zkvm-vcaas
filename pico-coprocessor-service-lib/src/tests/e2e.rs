// tests/integration_tests.rs
use crate::{
    BlockchainData, CoprocessorProofRequest, GENERATE_COPROCESSOR_PROOF_JOB_ID,
    GENERATE_PROOF_JOB_ID, MaxSizes, ProgramLocation, ProofRequest, ProofResult, ProofServiceError,
    ProvingType, SerializableLog, SerializableReceipt, ServiceContext, generate_coprocessor_proof,
    generate_proof, jobs::coprocessor::CoprocessorInputBundle,
};
use blueprint_sdk::{
    alloy::primitives::{Address, B256, U256, keccak256}, // Import alloy types
    extract::Context,
    tangle::extract::{Optional, TangleArg, TangleResult},
};
use hex::FromHex;
use std::path::PathBuf;
use std::str::FromStr;
use tempfile::tempdir;
use url::Url; // For checking input encoding

// Helper to create a realistic context for testing
fn setup_test_context() -> ServiceContext {
    let temp_base = tempdir()
        .expect("Failed to create base temp dir for tests")
        .into_path();
    let rpc_url = Url::parse("http://localhost:8545").unwrap(); // Placeholder
    let registry_addr = Address::from_str("0x1111111111111111111111111111111111111111").unwrap(); // Placeholder

    ServiceContext::new(rpc_url, registry_addr, temp_base)
        .expect("Failed to create test ServiceContext")
}

// --- generate_proof Tests ---
#[tokio::test]
async fn test_generate_proof_job_invalid_hash_format() {
    let ctx = setup_test_context();
    let request = ProofRequest {
        program_hash: "invalid-hash-format".to_string(),
        inputs: "00".to_string(),
        proving_type: ProvingType::Fast,
        program_location_override: None,
        eth_rpc_url_override: None,
        registry_address_override: None,
    };
    let tangle_arg = TangleArg(request);
    let job_context = Context(ctx);
    let result = generate_proof(job_context, tangle_arg).await;
    assert!(result.is_err());
    assert!(
        matches!(result.err().unwrap(), ProofServiceError::InvalidInput(msg) if msg.contains("Invalid program_hash format"))
    );
}

#[tokio::test]
async fn test_generate_proof_job_invalid_input_hex() {
    let ctx = setup_test_context();
    let request = ProofRequest {
        program_hash: B256::ZERO.to_string(), // Valid hash format
        inputs: "invalid-hex".to_string(),    // Invalid hex
        proving_type: ProvingType::Fast,
        program_location_override: None,
        eth_rpc_url_override: None,
        registry_address_override: None,
    };
    let tangle_arg = TangleArg(request);
    let job_context = Context(ctx);

    // Need to mock `get_program_elf` or `program::fetch_and_verify_program` for this test
    // to prevent it from failing before checking input hex inside pico::execute_pico_prove.
    // For now, we expect this specific setup to fail early during hex::decode in pico.rs
    // TODO: Add mocking to test input validation path more directly.
    // Let's assume pico::execute_pico_prove is called and fails on hex decode for now.

    // Mocking setup would go here...
    // let mocked_pico_result = Err(ProofServiceError::HexError("mocked".to_string()));

    let result = generate_proof(job_context, tangle_arg).await;

    // Without mocking, it likely fails finding the program 0x000... in the registry.
    // Let's assert for *either* ProgramNotFound *or* HexError if pico.rs is reached.
    match result {
        Ok(TangleResult(proof_result)) => {
            assert!(
                false,
                "Expected error, got proof result: {:?}",
                proof_result
            );
        }
        Err(e) => {
            assert!(matches!(
                e,
                ProofServiceError::ProgramNotFoundInRegistry(_) | ProofServiceError::HexError(_)
            ));
        }
    }
}

// --- generate_coprocessor_proof Tests ---

#[tokio::test]
async fn test_coprocessor_job_invalid_hash_format() {
    let ctx = setup_test_context();
    let request = CoprocessorProofRequest {
        program_hash: "invalid-hash".to_string(), // Invalid
        blockchain_data: BlockchainData::default(),
        max_sizes: MaxSizes {
            max_receipt_size: 32,
            max_storage_size: 32,
            max_tx_size: 32,
        },
        proving_type: ProvingType::Fast,
        program_location_override: None,
        eth_rpc_url_override: None,
        registry_address_override: None,
    };
    let tangle_arg = TangleArg(request);
    let job_context = Context(ctx);
    let result = generate_coprocessor_proof(job_context, tangle_arg).await;
    assert!(result.is_err());
    assert!(
        matches!(result.err().unwrap(), ProofServiceError::InvalidInput(msg) if msg.contains("Invalid program_hash format"))
    );
}

#[tokio::test]
async fn test_coprocessor_job_invalid_max_sizes_zero() {
    let ctx = setup_test_context();
    let request = CoprocessorProofRequest {
        program_hash: B256::ZERO.to_string(),
        blockchain_data: BlockchainData::default(),
        max_sizes: MaxSizes {
            max_receipt_size: 0,
            max_storage_size: 32,
            max_tx_size: 32,
        }, // Invalid (zero)
        proving_type: ProvingType::Fast,
        program_location_override: None,
        eth_rpc_url_override: None,
        registry_address_override: None,
    };
    let tangle_arg = TangleArg(request);
    let job_context = Context(ctx);
    let result = generate_coprocessor_proof(job_context, tangle_arg).await;
    assert!(result.is_err());
    assert!(
        matches!(result.err().unwrap(), ProofServiceError::InvalidInput(msg) if msg.contains("Invalid max_sizes"))
    );
}

#[tokio::test]
async fn test_coprocessor_job_invalid_max_sizes_multiple() {
    let ctx = setup_test_context();
    let request = CoprocessorProofRequest {
        program_hash: B256::ZERO.to_string(),
        blockchain_data: BlockchainData::default(),
        max_sizes: MaxSizes {
            max_receipt_size: 33,
            max_storage_size: 32,
            max_tx_size: 32,
        }, // Invalid (not multiple of 32)
        proving_type: ProvingType::Fast,
        program_location_override: None,
        eth_rpc_url_override: None,
        registry_address_override: None,
    };
    let tangle_arg = TangleArg(request);
    let job_context = Context(ctx);
    let result = generate_coprocessor_proof(job_context, tangle_arg).await;
    assert!(result.is_err());
    assert!(
        matches!(result.err().unwrap(), ProofServiceError::InvalidInput(msg) if msg.contains("Invalid max_sizes"))
    );
}

// Example test demonstrating input bundle serialization (doesn't call job)
#[test]
fn test_coprocessor_input_bundle_serialization() {
    let bundle = CoprocessorInputBundle {
        data: BlockchainData {
            receipts: Some(vec![SerializableReceipt {
                transaction_hash: B256::from_str("0xdeadbeef...").unwrap_or_default(), // Example hash
                status: Some(U256::from(1)),
                logs: vec![SerializableLog {
                    address: Address::from_str("0x...").unwrap_or_default(),
                    topics: vec![B256::from_str("0x...").unwrap_or_default()],
                    data_hex: "0123".to_string(),
                }],
                raw_data_hex: "f8...".to_string(),
            }]),
            storage_slots: None,
            transactions: None,
        },
        sizes: MaxSizes {
            max_receipt_size: 64,
            max_storage_size: 32,
            max_tx_size: 32,
        },
    };

    let encoded = serde_json::to_vec(&bundle).unwrap();
    assert!(encoded.len() > 0); // Basic check that encoding produces bytes

    // Optional: Decode back to verify
    let decoded = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(bundle, decoded);
}

// --- process_coprocessor_proof Full E2E Test ---

#[tokio::test]
async fn test_coprocessor_job_trading_volume_e2e() {
    let ctx = setup_test_context();
    let elf_path = PathBuf::from("./tests/fixtures/trading_volume.elf");

    // --- Test Setup ---

    // 1. Load ELF and calculate hash
    let elf_bytes = std::fs::read(&elf_path)
        .expect("Failed to read test ELF file at tests/fixtures/trading_volume.elf. Make sure you compiled and copied it.");
    let elf_hash = keccak256(&elf_bytes);
    let program_hash = hex::encode(elf_hash);
    println!("Test ELF Hash: {}", program_hash);

    // 2. Prepare Input Data (mimicking prepare_test_receipts)
    let max_receipts_for_test = 4; // Use a smaller number for faster testing
    let (blockchain_data, expected_volume) = prepare_test_blockchain_data(max_receipts_for_test);

    let max_sizes = MaxSizes {
        max_receipt_size: max_receipts_for_test * 32, // Example sizing, needs adjustment based on actual data
        max_storage_size: 32,                         // Minimal size if not used
        max_tx_size: 32,                              // Minimal size if not used
    };
    // Ensure sizes are valid
    assert!(max_sizes.max_receipt_size > 0 && max_sizes.max_receipt_size % 32 == 0);

    // 3. Construct Request using LocalPath override
    let request = CoprocessorProofRequest {
        program_hash,
        blockchain_data: blockchain_data.clone(), // Clone data for potential later use/assertion
        max_sizes: max_sizes.clone(),             // Clone sizes
        proving_type: ProvingType::Fast,          // Use Fast for testing (no Docker needed)
        program_location_override: Some(ProgramLocation::LocalPath(elf_path)), // Override location
        eth_rpc_url_override: None,
        registry_address_override: None,
    };

    // --- Execute Job ---
    let tangle_arg = TangleArg(request.clone()); // Clone request
    let job_context = Context(ctx);
    let result = generate_coprocessor_proof(job_context, tangle_arg).await;

    // --- Assertions ---
    println!("Job Result: {:?}", result);
    assert!(result.is_ok(), "Job failed: {:?}", result.err());

    let proof_result = result.unwrap().0;

    // Verify Proving Type and Hash
    assert_eq!(proof_result.proving_type, ProvingType::Fast);
    assert_eq!(proof_result.program_hash, request.program_hash);

    // Verify Inputs field (should be hex of SCALE encoded CoprocessorInputBundle)
    let expected_input_bundle = CoprocessorInputBundle {
        data: blockchain_data, // Use the same data used in the request
        sizes: max_sizes,      // Use the same sizes used in the request
    };
    let expected_input_hex = hex::encode(serde_json::to_vec(&expected_input_bundle).unwrap());
    assert_eq!(proof_result.inputs, expected_input_hex);

    // Verify Public Values (should be hex of volume.to_be_bytes())
    // Decode the hex public values back into bytes, then into U256
    let public_value_bytes =
        hex::decode(&proof_result.public_values).expect("Failed to decode public_values hex");
    assert_eq!(
        public_value_bytes.len(),
        32,
        "Public value should be 32 bytes for U256"
    );
    let result_volume = U256::from_be_slice(&public_value_bytes);

    println!("Expected Volume: {}", expected_volume);
    println!("Result Volume:   {}", result_volume);
    assert_eq!(
        result_volume, expected_volume,
        "Public value (volume) does not match expected"
    );

    // Optional: Verify proof field is non-empty hex (content depends on prover)
    assert!(!proof_result.proof.is_empty());
    assert!(hex::decode(&proof_result.proof).is_ok());
}

// Helper function to create test BlockchainData mimicking trading_volumn_lib
// Returns the data and the expected final volume U256
fn prepare_test_blockchain_data(num_receipts: usize) -> (BlockchainData, U256) {
    // Data from trading_volumn_lib
    let transaction_hash_hex = "0xd97c7863076f6b8a2430f3cc363220a1d67ee990d2673c927c93822fa541d39c";
    let transaction_hash = B256::from_str(transaction_hash_hex).unwrap();
    let usdc_pool_hex = "0x88e6a0c2ddd26feeb64f039a2c41296fcb3f5640";
    let usdc_pool = Address::from_str(usdc_pool_hex).unwrap();
    let event_swap_hex = "0xc42079f94a6350d7e6235f29174924f928cc2ac818eb64fed8004e115fbcca67";
    let event_swap = B256::from_str(event_swap_hex).unwrap();
    let value_hex = "0000000000000000000000000000000000000000000000010d12bdb167e201e0";
    let value = U256::from_str(value_hex).unwrap();
    let user_addr_hex = "0000000000000000000000006a000f20005980200259b80c5102003040001068";
    let user_addr = B256::from_str(user_addr_hex).unwrap();

    // Create the log data roughly matching field_0 and field_1 structure from the example
    // Note: We need SerializableLog, not SdkLogFieldData here if using wrappers.
    let log_0 = SerializableLog {
        address: usdc_pool,
        topics: vec![event_swap],        // Assuming topic is in topics vec
        data_hex: value_hex.to_string(), // Assuming value maps to data_hex
    };
    let log_1 = SerializableLog {
        address: usdc_pool,
        topics: vec![event_swap, user_addr], // Assuming user addr is topic 1
        data_hex: "".to_string(),            // No data part for this log field in example
    };

    let mut test_receipts = Vec::with_capacity(num_receipts);
    for _ in 0..num_receipts {
        // Reconstruct SerializableReceipt based on what the test program expects
        // We need to ensure the structure matches what the zkVM program converts from/to.
        // This mapping is crucial and depends on the exact fields used.
        test_receipts.push(SerializableReceipt {
            transaction_hash: transaction_hash,
            status: Some(U256::from(1)), // Assume success
            // Combine logs similar to how fields were combined in original example
            // The test zkVM program expects specific logs/fields.
            logs: vec![log_0.clone(), log_1.clone()], // Example pairing
            raw_data_hex: "".to_string(),             // Not used in simplified zkVM logic
        });
    }

    let blockchain_data = BlockchainData {
        receipts: Some(test_receipts),
        storage_slots: None,
        transactions: None,
    };

    // Calculate expected volume (sum of 'value' from log_0 for each receipt)
    let expected_volume = value.checked_mul(U256::from(num_receipts as u64)).unwrap();

    (blockchain_data, expected_volume)
}
