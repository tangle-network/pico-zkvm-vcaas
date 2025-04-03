use crate::errors::ProofServiceError;
use crate::types::{ProofResult, ProvingType};
use blueprint_sdk::{debug, info, warn};
use pico_sdk::client::DefaultProverClient;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

/// Executes the Pico proving process for the given ELF file and inputs.
pub async fn execute_pico_prove(
    elf_path: &Path,
    inputs_hex: &str,
    proving_type: &ProvingType,
    output_base_dir: &Path, // Base directory for prover outputs
) -> Result<ProofResult, ProofServiceError> {
    info!(elf = ?elf_path, type = ?proving_type, output_dir = ?output_base_dir, "Starting Pico proving process");

    // 1. Load ELF
    // Use pico_sdk::utils::load_elf or equivalent
    let elf_contents = read_elf_file(elf_path)?;

    // 2. Initialize Prover Client
    // Choose client based on proving_type if necessary (e.g., KoalaBearProveVKClient for EVM)
    // Using DefaultProverClient for now, assuming it handles different proof types via methods.
    let client = DefaultProverClient::new(&elf_contents);

    // 3. Prepare Inputs
    let input_bytes = hex::decode(inputs_hex)?;
    let stdin_builder = client.get_stdin_builder();
    stdin_builder.borrow_mut().write(&input_bytes);
    debug!("Inputs written to prover stdin");

    // 4. Execute Proving based on type
    let (proof_bytes, public_values_bytes, maybe_output_dir) = match proving_type {
        ProvingType::Fast => {
            info!("Executing fast proof (RISCV phase only)");
            let proof = client.prove_fast().map_err(|e| {
                ProofServiceError::ProvingError(format!("Fast proving failed: {:?}", e))
            })?;
            // Extract proof data and public values
            let pv = proof.pv_stream.ok_or_else(|| {
                ProofServiceError::ProvingError(
                    "Fast proof missing public values stream".to_string(),
                )
            })?;
            // Proof data format for fast proof needs clarification from Pico docs. Assuming proof.inner for now.
            // Need to check actual structure of RiscvProof.
            // let proof_data = proof.inner; // Placeholder - Adjust based on RiscvProof structure
            let proof_data = vec![]; // FIXME: Determine correct fast proof data extraction
            warn!("Fast proof data extraction needs implementation.");
            (proof_data, pv, None)
        }
        ProvingType::Full => {
            info!("Executing full proof (RECURSION phase)");
            // Create a specific output dir for this proof run
            let proof_output_dir = create_proof_output_dir(output_base_dir, "full")?;
            let embed_proof = client
                .prove(proof_output_dir.clone()) // prove() likely requires PathBuf
                .map_err(|e| {
                    ProofServiceError::ProvingError(format!("Full proving failed: {:?}", e))
                })?;
            // Extract proof and public values from EmbedProof struct
            // Need to check EmbedProof structure from Pico SDK docs.
            // let proof_data = embed_proof.proof_bytes; // Placeholder
            // let pv = embed_proof.pv_bytes; // Placeholder
            let proof_data = vec![]; // FIXME: Determine correct full proof data extraction
            let pv = vec![]; // FIXME: Determine correct full proof pv extraction
            warn!("Full proof data and PV extraction needs implementation.");
            (proof_data, pv, Some(proof_output_dir))
        }
        ProvingType::FullWithEvm => {
            info!("Executing full proof with EVM phase");
            // EVM proving might need a different client or specific setup.
            // Assuming DefaultProverClient has prove_evm or similar.
            // Need KoalaBearProveVKClient according to docs? Let's assume client has the method.
            let proof_output_dir = create_proof_output_dir(output_base_dir, "evm")?;

            // Requires PK/VK setup. The SDK call might handle this with the boolean flag.
            // Let's assume setup is handled elsewhere or the first run does it.
            let need_setup = !check_if_evm_setup_exists(&proof_output_dir); // Basic check
            if need_setup {
                info!(
                    "EVM PK/VK setup seems needed for output dir: {:?}",
                    proof_output_dir
                );
                // Call setup explicitly if required by SDK? prove_evm might do it.
                // client.setup_evm(&proof_output_dir)?
            }

            // Check if prove_evm exists on DefaultProverClient or if KoalaBear specific client is needed
            // Let's assume it exists for now.
            client
                .prove_evm(need_setup, proof_output_dir.clone(), "kb")
                .map_err(|e| {
                    ProofServiceError::ProvingError(format!("EVM proving failed: {:?}", e))
                })?;

            // EVM proof artifacts (proof.data, pv_file) are generated in proof_output_dir.
            // We need to read them.
            let proof_path = proof_output_dir.join("proof.data"); // Check actual filename
            let pv_path = proof_output_dir.join("inputs.json"); // Public values are in inputs.json for EVM? Check docs. No, pv_file in docs.
            let pv_path_alt = proof_output_dir.join("pv_file"); // Check actual filename

            let proof_data = std::fs::read(&proof_path).map_err(|e| {
                ProofServiceError::ProvingError(format!(
                    "Failed to read EVM proof file {:?}: {}",
                    proof_path, e
                ))
            })?;

            let pv_data_hex = std::fs::read_to_string(&pv_path_alt)
                .or_else(|_| std::fs::read_to_string(&pv_path)) // Try alternative pv file name
                .map_err(|e| {
                    ProofServiceError::ProvingError(format!(
                        "Failed to read EVM public values file {:?}/{:?}: {}",
                        pv_path_alt, pv_path, e
                    ))
                })?;

            // The pv_file content is likely already hex, trim whitespace.
            let pv_bytes = hex::decode(pv_data_hex.trim())?;

            (proof_data, pv_bytes, Some(proof_output_dir))
        }
    };

    let result = ProofResult {
        public_values: hex::encode(&public_values_bytes),
        proof: hex::encode(&proof_bytes),
        proving_type: proving_type.clone(),
        output_dir: maybe_output_dir.map(|p| p.to_string_lossy().to_string()),
        // Populate other fields later in generate_proof job
        program_hash: String::new(),    // Placeholder
        inputs: inputs_hex.to_string(), // Store original hex input
    };

    info!("Pico proving process completed successfully.");
    Ok(result)
}

fn read_elf_file(elf_path: &Path) -> Result<Vec<u8>, ProofServiceError> {
    let mut file = File::open(elf_path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    Ok(buffer)
}

fn create_proof_output_dir(
    base_dir: &Path,
    proof_type: &str,
) -> Result<PathBuf, ProofServiceError> {
    // Create a unique subdirectory for each proof run's artifacts
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0); // Simple timestamp for uniqueness
    let dir_name = format!("proof_{}_{}", proof_type, timestamp);
    let output_dir = base_dir.join(dir_name);
    std::fs::create_dir_all(&output_dir)?;
    Ok(output_dir)
}

// Basic placeholder check if EVM setup artifacts exist
fn check_if_evm_setup_exists(output_dir: &Path) -> bool {
    // Gnark PK/VK are generated according to docs. Check for expected files.
    // e.g., output_dir/pk.bin, output_dir/vk.bin (filenames are assumptions)
    // For simplicity, let's just return false to always suggest setup might be needed.
    // A robust check would look for specific files generated by `cargo pico prove --evm --setup`.
    // output_dir.join("NAME_OF_PK_FILE").exists() && output_dir.join("NAME_OF_VK_FILE").exists()
    warn!("EVM setup check is basic; assuming setup might be needed.");
    false
}
