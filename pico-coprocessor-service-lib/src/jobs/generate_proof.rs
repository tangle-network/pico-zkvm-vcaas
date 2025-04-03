// pico-coprocessor-service-lib/src/jobs/generate_proof.rs
use crate::{
    context::ServiceContext,
    errors::ProofServiceError,
    evm, pico, program,
    types::{ProofRequest, ProofResult},
};
use blueprint_sdk::{
    alloy::primitives::B256,
    error,
    extract::Context,
    info,
    tangle::extract::{TangleArg, TangleResult},
};
use std::{path::PathBuf, str::FromStr};
use tempfile::TempDir; // To manage temporary directories

// Wrapper struct to hold temporary resources and ensure cleanup
struct ProofResources {
    elf_temp_dir: TempDir, // Holds the temp dir containing the ELF, cleans up on drop
    elf_path: PathBuf,
    output_temp_dir: TempDir, // Holds the temp dir for proof outputs, cleans up on drop
    output_path: PathBuf,
}

pub async fn generate_proof(
    Context(ctx): Context<ServiceContext>,
    TangleArg(request): TangleArg<ProofRequest>,
) -> TangleResult<Result<ProofResult, ProofServiceError>> {
    info!(request = ?request, "Received generate_proof job request");

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
            return TangleResult(Err(err));
        }
    };

    // Validate input hex format
    if hex::decode(&request.inputs).is_err() {
        let err = ProofServiceError::InvalidInput(format!(
            "Invalid inputs format (expected hex): {}",
            request.inputs
        ));
        error!("{}", err);
        return TangleResult(Err(err));
    }

    // Create a temporary directory for proof outputs for this specific job
    let output_temp_dir = match tempfile::Builder::new()
        .prefix("pico_output_")
        .tempdir_in(&ctx.temp_dir_base)
    {
        Ok(dir) => dir,
        Err(e) => {
            let err = ProofServiceError::TempDirError(format!(
                "Failed to create proof output temp dir: {}",
                e
            ));
            error!("{}", err);
            return TangleResult(Err(err));
        }
    };
    let output_path = output_temp_dir.path().to_path_buf();

    // --- 2. Get Program ---
    let fetch_result = get_program_elf(&ctx, &request, &program_hash_bytes).await;
    let (elf_temp_dir, elf_path) = match fetch_result {
        Ok((dir, path)) => (dir, path),
        Err(e) => {
            error!("Failed to get program ELF: {:?}", e);
            // Cleanup output dir if program fetch failed
            let _ = tokio::fs::remove_dir_all(output_path).await;
            return TangleResult(Err(e));
        }
    };

    // Wrap resources for automatic cleanup
    let _resources = ProofResources {
        // Variable binding ensures it lives long enough
        elf_temp_dir,                     // Transfer ownership
        elf_path: elf_path.clone(),       // Clone path for use
        output_temp_dir,                  // Transfer ownership
        output_path: output_path.clone(), // Clone path for use
    };

    // --- 3. Execute Proving ---
    let proof_exec_result = pico::execute_pico_prove(
        &elf_path, // Path from fetch_result
        &request.inputs,
        &request.proving_type,
        &output_path, // Use the dedicated output dir for this job
    )
    .await;

    // --- 4. Handle Result ---
    match proof_exec_result {
        Ok(mut proof_result) => {
            // Populate remaining fields
            proof_result.program_hash = request.program_hash;
            // Input is already hex, stored in pico::execute_pico_prove
            // proof_result.inputs = request.inputs; // Already set inside execute_pico_prove

            info!(result = ?proof_result, "Proof generation successful");
            TangleResult(Ok(proof_result))
        }
        Err(e) => {
            error!("Proof generation failed: {:?}", e);
            // Temp dirs (_elf_temp_dir, output_temp_dir) are cleaned up automatically when _resources goes out of scope
            TangleResult(Err(e))
        }
    }
}

// Helper function to manage program fetching logic
async fn get_program_elf(
    ctx: &ServiceContext,
    request: &ProofRequest,
    program_hash_bytes: &B256,
) -> Result<(TempDir, PathBuf), ProofServiceError> {
    // Determine location: Override > Registry
    let location = match &request.program_location_override {
        Some(loc) => {
            info!("Using program location override: {:?}", loc);
            loc.clone()
        }
        None => {
            info!("Fetching program location from registry...");
            evm::get_program_location_from_registry(ctx, program_hash_bytes).await?
        }
    };

    // Fetch and verify
    program::fetch_and_verify_program(ctx, &location, &request.program_hash).await
}
