use crate::errors::ProofServiceError;
use crate::types::{ProofResult, ProvingType};
use blueprint_sdk::{debug, info};
use pico_sdk::client::DefaultProverClient;
use pico_vm::configs::stark_config::{KoalaBearBn254Poseidon2, KoalaBearPoseidon2};
use pico_vm::machine::proof::BaseProof;
use rand::Rng;
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
    let elf_contents = read_elf_file(elf_path)?;

    // 2. Initialize Prover Client (Default is KoalaBear)
    // Explicit types might be needed if inference fails, but DefaultProverClient should work.
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
            // prove_fast returns Result<MetaProof<KoalaBearPoseidon2>, Error>
            let riscv_proof = client.prove_fast().map_err(|e| {
                ProofServiceError::ProvingError(format!("Fast proving failed: {:?}", e))
            })?;

            // Extract public values (likely from riscv_proof.pv_stream)
            let pv = riscv_proof.pv_stream.clone().ok_or_else(|| {
                ProofServiceError::ProvingError(
                    "Fast proof missing public values stream".to_string(),
                )
            })?;

            // Extract proof data (likely the first proof in the MetaProof)
            // Assume the proof object itself can be SCALE encoded for serialization.
            let proof: BaseProof<KoalaBearPoseidon2> = riscv_proof
                .proofs()
                .first()
                .ok_or_else(|| {
                    ProofServiceError::ProvingError(
                        "Fast proof MetaProof contained no proofs".to_string(),
                    )
                })?
                .clone();
            // Serialize the proof
            let proof_data = serde_json::to_vec(&proof)?;

            info!("Fast proof generated successfully.");
            (proof_data, pv, None)
        }
        ProvingType::Full => {
            info!("Executing full proof (RECURSION phase)");
            // Create a specific output dir for this proof run
            let proof_output_dir = create_proof_output_dir(output_base_dir, "full")?;
            // prove returns Result<(MetaProof<KoalaBearPoseidon2>, MetaProof<KoalaBearBn254Poseidon2>), Error>
            let (riscv_proof, embed_proof) =
                client.prove(proof_output_dir.clone()).map_err(|e| {
                    ProofServiceError::ProvingError(format!("Full proving failed: {:?}", e))
                })?;

            // Extract public values from the RISCV proof part
            let pv = riscv_proof.pv_stream.clone().ok_or_else(|| {
                ProofServiceError::ProvingError(
                    "Full proof (RISCV part) missing public values stream".to_string(),
                )
            })?;

            // Extract proof data from the Embed proof part
            let proof: BaseProof<KoalaBearBn254Poseidon2> = embed_proof
                .proofs()
                .first()
                .ok_or_else(|| {
                    ProofServiceError::ProvingError(
                        "Full proof (Embed part) MetaProof contained no proofs".to_string(),
                    )
                })?
                .clone();
            let proof_data = serde_json::to_vec(&proof)?;

            info!("Full proof generated successfully.");
            (proof_data, pv, Some(proof_output_dir))
        }
        ProvingType::FullWithEvm => {
            info!("Executing full proof with EVM phase");
            let proof_output_dir = create_proof_output_dir(output_base_dir, "evm")?;

            // Check if setup is needed (basic check, still relies on Docker call robustness)
            let need_setup = !check_if_evm_setup_exists(&proof_output_dir);
            if need_setup {
                info!(
                    "Suggesting EVM PK/VK setup for output dir: {:?}",
                    proof_output_dir
                );
            }

            // Call prove_evm - this internally calls .prove() and then runs Docker commands.
            // DefaultProverClient is KoalaBear, so field_type is "kb".
            client
                .prove_evm(need_setup, proof_output_dir.clone(), "kb")
                .map_err(|e| {
                    ProofServiceError::ProvingError(format!("EVM proving failed: {:?}", e))
                })?;

            info!("EVM Docker commands completed (assumed). Reading artifacts...");

            // Read artifacts generated by Docker container in proof_output_dir.
            let proof_path = proof_output_dir.join("proof.data");
            // Docs mention pv_file, CLI output might use inputs.json. Check both.
            let pv_path_primary = proof_output_dir.join("pv_file");
            let pv_path_alt = proof_output_dir.join("inputs.json");

            let proof_data = tokio::fs::read(&proof_path).await.map_err(|e| {
                ProofServiceError::ProvingError(format!(
                    "Failed to read EVM proof file {:?}: {}",
                    proof_path, e
                ))
            })?;

            // Read public values file (try pv_file first, then inputs.json)
            let pv_content = std::fs::read_to_string(&pv_path_primary)
                .or_else(|_| std::fs::read_to_string(&pv_path_alt))
                .map_err(|e| {
                    ProofServiceError::ProvingError(format!(
                        "Failed to read EVM public values file ({:?} or {:?}): {}",
                        pv_path_primary, pv_path_alt, e
                    ))
                })?;

            // Public values can be hex in pv_file or JSON in inputs.json. Handle both.
            let pv_bytes = if pv_path_alt.exists() && pv_content.trim().starts_with('{') {
                // Assume inputs.json format: {"riscvVKey": "...", "proof": "...", "publicValues": "0x..."}
                let json_val: serde_json::Value =
                    serde_json::from_str(&pv_content).map_err(|e| {
                        ProofServiceError::ProvingError(format!(
                            "Failed to parse EVM public values JSON {:?}: {}",
                            pv_path_alt, e
                        ))
                    })?;
                let pv_hex = json_val["publicValues"].as_str().ok_or_else(|| {
                    ProofServiceError::ProvingError(
                        "Missing 'publicValues' field in inputs.json".to_string(),
                    )
                })?;
                // Remove "0x" prefix if present
                hex::decode(pv_hex.trim_start_matches("0x"))?
            } else {
                // Assume pv_file format (raw hex string)
                hex::decode(pv_content.trim())?
            };

            info!("EVM proof generated and artifacts read successfully.");
            (proof_data, pv_bytes, Some(proof_output_dir))
        }
    };

    let result = ProofResult {
        public_values: hex::encode(&public_values_bytes),
        proof: hex::encode(&proof_bytes), // Proof data is SCALE encoded then hex encoded
        proving_type: proving_type.clone(),
        output_dir: maybe_output_dir.map(|p| p.to_string_lossy().to_string()),
        // Populate other fields later in generate_proof job
        program_hash: String::new(), // Placeholder - To be filled by caller (generate_proof job)
        inputs: inputs_hex.to_string(), // Store original hex input
    };

    info!("Pico proving process completed successfully.");
    Ok(result)
}

fn read_elf_file(elf_path: &Path) -> Result<Vec<u8>, ProofServiceError> {
    let file = File::open(elf_path)?; // Use std::fs::File for blocking read is ok here
    let mut reader = std::io::BufReader::new(file);
    let mut buffer = Vec::new();
    reader.read_to_end(&mut buffer)?;
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
    // Add a random element for more robustness against collisions
    let random_suffix: String = rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(6)
        .map(char::from)
        .collect();
    let dir_name = format!("proof_{}_{}_{}", proof_type, timestamp, random_suffix);
    let output_dir = base_dir.join(dir_name);
    std::fs::create_dir_all(&output_dir)?; // Use std::fs here, blocking is fine
    Ok(output_dir)
}

// Basic placeholder check if EVM setup artifacts exist
fn check_if_evm_setup_exists(output_dir: &Path) -> bool {
    // Gnark PK/VK are generated according to docs (`prove --evm --setup`).
    // The `prove_evm` docker command likely checks for these itself.
    // A robust check would look for `pk` and `vk` files (exact names depend on gnark_cli).
    // Relying on the `prove_evm` command's behavior with `need_setup=true` is simpler.
    // Let's assume the check isn't strictly necessary here, the docker command handles it.
    let pk_exists = output_dir.join("proving.key").exists(); // Example filename
    let vk_exists = output_dir.join("verifying.key").exists(); // Example filename
    if pk_exists && vk_exists {
        debug!(
            "Found potential EVM setup files (pk/vk) in {:?}",
            output_dir
        );
        true
    } else {
        debug!(
            "Did not find potential EVM setup files (pk/vk) in {:?}",
            output_dir
        );
        false
    }
}
