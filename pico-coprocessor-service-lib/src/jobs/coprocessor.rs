// pico-coprocessor-service-lib/src/jobs/generate_coprocessor_proof.rs
use crate::{
    context::ServiceContext,
    errors::ProofServiceError,
    evm, pico, program,
    types::{BlockchainData, CoprocessorProofRequest, MaxSizes, ProofResult},
};
use blueprint_sdk::{
    alloy::primitives::{Address, B256},
    error,
    extract::Context,
    info,
    tangle::extract::{TangleArg, TangleResult},
};
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, str::FromStr};
use tempfile::TempDir; // For serializing inputs

// Helper struct for managing temporary resources
pub struct CoprocessorProofResources {
    _elf_temp_dir: TempDir,
    elf_path: PathBuf,
    _output_temp_dir: TempDir,
    output_path: PathBuf,
}

// Define a structure to bundle inputs for SCALE encoding
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct CoprocessorInputBundle {
    pub data: BlockchainData,
    pub sizes: MaxSizes,
}

pub async fn generate_coprocessor_proof(
    Context(ctx): Context<ServiceContext>,
    TangleArg(request): TangleArg<CoprocessorProofRequest>,
) -> Result<TangleResult<ProofResult>, ProofServiceError> {
    info!(request = ?request, "Received generate_coprocessor_proof job request");

    // --- 1. Preparation ---
    // Validate program hash format
    let program_hash_bytes = match B256::from_str(&request.program_hash) {
        Ok(hash) => hash,
        Err(_) => {
            let err = ProofServiceError::InvalidInput(format!(
                "Invalid program_hash format (expected 32-byte hex): {}",
                request.program_hash
            ));
            error!("{}", err);
            return Err(err);
        }
    };

    // Validate max sizes (must be > 0 and multiple of 32 according to docs)
    if request.max_sizes.max_receipt_size == 0
        || request.max_sizes.max_receipt_size % 32 != 0
        || request.max_sizes.max_storage_size == 0
        || request.max_sizes.max_storage_size % 32 != 0
        || request.max_sizes.max_tx_size == 0
        || request.max_sizes.max_tx_size % 32 != 0
    {
        let err = ProofServiceError::InvalidInput(format!(
            "Invalid max_sizes: must be > 0 and multiple of 32. Got {:?}",
            request.max_sizes
        ));
        error!("{}", err);
        return Err(err);
    }

    // Create a temporary directory for proof outputs
    let output_temp_dir = match tempfile::Builder::new()
        .prefix("pico_coproc_out_")
        .tempdir_in(&ctx.temp_dir_base)
    {
        Ok(dir) => dir,
        Err(e) => {
            let err = ProofServiceError::TempDirError(format!(
                "Failed to create coprocessor output temp dir: {}",
                e
            ));
            error!("{}", err);
            return Err(err);
        }
    };
    let output_path = output_temp_dir.path().to_path_buf();

    // --- 2. Get Program ELF ---
    // Fetch the user's zkVM program (which should use coprocessor-sdk)
    let fetch_result = get_program_elf_for_coprocessor(&ctx, &request, &program_hash_bytes).await;
    let (elf_temp_dir, elf_path) = match fetch_result {
        Ok((dir, path)) => (dir, path),
        Err(e) => {
            error!("Failed to get coprocessor program ELF: {:?}", e);
            let _ = tokio::fs::remove_dir_all(output_path).await; // Cleanup output dir
            return Err(e);
        }
    };

    // Wrap resources for automatic cleanup
    let _resources = CoprocessorProofResources {
        // Variable binding ensures it lives long enough
        _elf_temp_dir: elf_temp_dir,
        elf_path: elf_path.clone(),
        _output_temp_dir: output_temp_dir,
        output_path: output_path.clone(),
    };

    // --- 3. Serialize Inputs for zkVM ---
    // The user's ELF program needs to deserialize this structure from stdin.
    let input_bundle = CoprocessorInputBundle {
        data: request.blockchain_data.clone(),
        sizes: request.max_sizes.clone(),
    };
    let serialized_inputs = serde_json::to_string(&input_bundle).unwrap();

    // --- 4. Execute Proving ---
    // Call the same underlying pico executor, but pass the serialized bundle as input.
    let proof_exec_result = pico::execute_pico_prove(
        &elf_path,
        &serialized_inputs, // Pass the encoded bundle
        &request.proving_type,
        &output_path,
    )
    .await;

    // --- 5. Handle Result ---
    match proof_exec_result {
        Ok(mut proof_result) => {
            // Populate remaining fields
            proof_result.program_hash = request.program_hash;
            // Store the hex of the SCALE encoded bundle as the "inputs" field
            proof_result.inputs = serialized_inputs;

            info!(result = ?proof_result, "Coprocessor proof generation successful");
            Ok(TangleResult(proof_result))
        }
        Err(e) => {
            error!("Coprocessor proof generation failed: {:?}", e);
            // Cleanup happens automatically via _resources drop
            Err(e)
        }
    }
}

// Helper function (similar to the one in generate_proof job)
async fn get_program_elf_for_coprocessor(
    ctx: &ServiceContext,
    request: &CoprocessorProofRequest,
    program_hash_bytes: &B256,
) -> Result<(TempDir, PathBuf), ProofServiceError> {
    let location = match &request.program_location_override {
        Some(loc) => {
            info!("Using coprocessor program location override: {:?}", loc);
            loc.clone()
        }
        None => {
            info!("Fetching coprocessor program location from registry...");
            evm::get_program_location_from_registry(ctx, program_hash_bytes).await?
        }
    };
    program::fetch_and_verify_program(ctx, &location, &request.program_hash).await
}
