// pico-coprocessor-service-lib/src/context.rs
use crate::errors::ProofServiceError;
use blueprint_sdk::alloy::primitives::Address;
use std::path::PathBuf;
use url::Url;

#[derive(Clone)]
pub struct ServiceContext {
    // Client for downloading ELF files
    pub http_client: reqwest::Client,
    // Default configuration for interacting with Ethereum node and registry contract
    pub eth_rpc_url: Url,
    pub registry_contract_address: Address,
    // Base path for storing temporary files (downloaded ELFs, proof outputs)
    pub temp_dir_base: PathBuf,
}

impl ServiceContext {
    pub fn new(
        default_eth_rpc_url: Url,
        default_registry_contract_address: Address,
        temp_dir_base: PathBuf,
    ) -> Result<Self, ProofServiceError> {
        // Validate temp dir exists and is writable? Or create if not exists?
        if !temp_dir_base.exists() {
            std::fs::create_dir_all(&temp_dir_base).map_err(|e| {
                ProofServiceError::ConfigError(format!(
                    "Failed to create temp base dir {:?}: {}",
                    temp_dir_base, e
                ))
            })?;
        } else if !temp_dir_base.is_dir() {
            return Err(ProofServiceError::ConfigError(format!(
                "Temp base path {:?} is not a directory",
                temp_dir_base
            )));
        }

        let http_c = reqwest::Client::builder().build().map_err(|e| {
            ProofServiceError::ConfigError(format!("Failed to build HTTP client: {}", e))
        })?;

        Ok(Self {
            http_client: http_c,
            eth_rpc_url: default_eth_rpc_url,
            registry_contract_address: default_registry_contract_address,
            temp_dir_base,
        })
    }

    // Helper to get registry address, considering override
    pub fn get_registry_address(&self) -> Address {
        self.registry_contract_address
    }
}
