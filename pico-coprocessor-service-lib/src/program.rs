// pico-coprocessor-service-lib/src/program.rs
use crate::context::ServiceContext;
use crate::errors::ProofServiceError;
use crate::types::ProgramLocation;
use blueprint_sdk::{debug, error, info};
use futures::StreamExt;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use tempfile::{self, TempDir};
use tokio::fs::File;
use tokio::io::{AsyncWriteExt, BufWriter};
use url::Url;

/// Fetches the program ELF binary, verifies its hash, saves it to a temporary directory.
/// Returns the TempDir handle (for cleanup) and the path to the temporary file.
pub async fn fetch_and_verify_program(
    ctx: &ServiceContext,
    location: &ProgramLocation,
    expected_hash_hex: &str,
) -> Result<(TempDir, PathBuf), ProofServiceError> {
    // Return tuple
    let temp_dir = tempfile::Builder::new()
        .prefix("pico_elf_")
        .tempdir_in(&ctx.temp_dir_base)
        .map_err(|e| {
            ProofServiceError::TempDirError(format!("Failed to create temp dir for ELF: {}", e))
        })?;

    let elf_path = temp_dir.path().join("program.elf");

    let actual_hash_hex = match location {
        ProgramLocation::RemoteUrl(url) => download_and_hash(ctx, url, &elf_path).await?,
        ProgramLocation::LocalPath(path) => {
            if !path.exists() {
                return Err(ProofServiceError::IoError(format!(
                    "Local program path not found: {:?}",
                    path
                )));
            }
            // Copying might be slow for large files, consider alternatives if needed
            let bytes_copied = tokio::fs::copy(path, &elf_path).await?;
            debug!(
                "Copied {} bytes from local path {:?} to {:?}",
                bytes_copied, path, elf_path
            );
            calculate_file_hash(&elf_path).await?
        }
    };

    // Verify hash
    if actual_hash_hex.eq_ignore_ascii_case(expected_hash_hex) {
        info!(expected = %expected_hash_hex, actual = %actual_hash_hex, path = ?elf_path, "Program hash verified successfully");
        // Return the TempDir handle AND the path
        Ok((temp_dir, elf_path))
    } else {
        error!(expected = %expected_hash_hex, actual = %actual_hash_hex, "Program hash mismatch!");
        // TempDir cleans up automatically when dropped, no need for manual remove_dir_all here
        Err(ProofServiceError::ProgramHashMismatch {
            expected: expected_hash_hex.to_string(),
            got: actual_hash_hex,
        })
    }
}

// download_and_hash and calculate_file_hash remain the same
async fn download_and_hash(
    ctx: &ServiceContext,
    url: &Url,
    dest_path: &Path,
) -> Result<String, ProofServiceError> {
    info!(%url, dest = ?dest_path, "Downloading program ELF");
    let response = ctx.http_client.get(url.clone()).send().await?;

    if !response.status().is_success() {
        return Err(ProofServiceError::ProgramDownloadFailed(format!(
            "Failed to download from {}: Status {}",
            url,
            response.status()
        )));
    }

    let mut file = BufWriter::new(File::create(dest_path).await?);
    let mut hasher = Sha256::new();
    let mut stream = response.bytes_stream();

    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result?;
        hasher.update(&chunk);
        file.write_all(&chunk).await?;
    }

    file.flush().await?; // Ensure all bytes are written

    let hash_bytes = hasher.finalize();
    let hash_hex = hex::encode(hash_bytes);
    debug!(%url, %hash_hex, "Finished downloading and hashing");
    Ok(hash_hex)
}

async fn calculate_file_hash(path: &Path) -> Result<String, ProofServiceError> {
    let mut file = tokio::fs::File::open(path).await?;
    let mut hasher = Sha256::new();
    let mut buffer = [0; 4096]; // Slightly larger buffer

    loop {
        let n = tokio::io::AsyncReadExt::read(&mut file, &mut buffer).await?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }

    let hash_bytes = hasher.finalize();
    Ok(hex::encode(hash_bytes))
}
