use blueprint_sdk::alloy::primitives::Address;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use url::Url;

// --- Program Location ---
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub enum ProgramLocation {
    RemoteUrl(Url),
    LocalPath(PathBuf),
}

// --- Proving Options ---
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ProvingType {
    Fast,
    Full,
    FullWithEvm,
}

// --- Job Input Structure ---
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ProofRequest {
    /// SHA256 hash of the ELF program binary (hex encoded).
    pub program_hash: String,
    /// Input data for the zkVM program (hex encoded).
    pub inputs: String,
    /// Type of proof to generate.
    pub proving_type: ProvingType,
    /// Optional: Override program location (e.g., for testing with local files)
    #[serde(default)]
    pub program_location_override: Option<ProgramLocation>,
    /// Optional: Ethereum RPC URL override for this specific request
    #[serde(default)]
    pub eth_rpc_url_override: Option<String>,
    /// Optional: Registry contract address override for this specific request
    #[serde(default)]
    pub registry_address_override: Option<Address>,
}

// --- Job Output Structure ---
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ProofResult {
    /// Public values output by the program (hex encoded).
    pub public_values: String,
    /// The generated proof data (structure depends on proving type, hex encoded).
    pub proof: String,
    /// Type of proof generated.
    pub proving_type: ProvingType,
    /// Optional: Path to output directory used during proving (relative to some base or absolute).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_dir: Option<String>,
    /// Hash of the program that was proven.
    pub program_hash: String,
    /// Inputs provided to the program.
    pub inputs: String,
}
