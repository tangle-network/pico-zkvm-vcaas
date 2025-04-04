// pico-coprocessor-service-lib/src/types.rs
use blueprint_sdk::alloy::primitives::{Address, B256, Bytes, U256};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use url::Url; // Use Alloy types

// --- Shared Types (ProgramLocation, ProvingType, ProofResult) ---
// Keep existing ProgramLocation, ProvingType, ProofResult definitions

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ProgramLocation {
    RemoteUrl(Url),
    LocalPath(PathBuf),
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Default)]
pub enum ProvingType {
    Fast,
    #[default]
    Full,
    FullWithEvm,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ProofResult {
    pub public_values: String, // hex encoded
    pub proof: String,         // hex encoded (SCALE encoded proof data)
    pub proving_type: ProvingType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_dir: Option<String>,
    pub program_hash: String, // hex encoded
    pub inputs: String,       // hex encoded (original inputs provided to the job)
}

// --- Generic Proof Job Input ---
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ProofRequest {
    pub program_hash: String, // hex encoded B256
    pub inputs: String,       // hex encoded bytes
    pub proving_type: ProvingType,
    #[serde(default)]
    pub program_location_override: Option<ProgramLocation>,
    #[serde(default)]
    pub eth_rpc_url_override: Option<String>,
    #[serde(default)]
    pub registry_address_override: Option<Address>,
}

// --- zkCoprocessor Specific Types ---

// Assume basic fields based on typical EVM data. Adapt if coprocessor-sdk specifics are known.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct SerializableReceipt {
    // Example fields - adjust based on actual coprocessor-sdk needs
    pub transaction_hash: B256,
    pub status: Option<U256>, // 1 for success, 0 for failure
    pub logs: Vec<SerializableLog>,
    // Add other relevant fields like gas_used, contract_address, etc.
    // Use hex encoding for byte fields if not using Bytes directly
    pub raw_data_hex: String, // Allow passing raw RLP or similar if needed
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct SerializableLog {
    // Example fields
    pub address: Address,
    pub topics: Vec<B256>,
    pub data_hex: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct SerializableStorageSlot {
    // Example fields
    pub address: Address,
    pub slot: B256,         // Storage key/slot hash
    pub value: B256,        // Storage value
    pub block_number: U256, // Block context might be needed
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct SerializableTransaction {
    // Example fields
    pub transaction_hash: B256,
    pub from: Address,
    pub to: Option<Address>,
    pub value: U256,
    pub input_data_hex: String,
    // Add other relevant fields like nonce, gas_price, gas_limit, etc.
    pub raw_data_hex: String, // Allow passing raw RLP or similar if needed
}

/// Container for blockchain data inputs to the coprocessor job.
#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub struct BlockchainData {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub receipts: Option<Vec<SerializableReceipt>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub storage_slots: Option<Vec<SerializableStorageSlot>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transactions: Option<Vec<SerializableTransaction>>,
}

/// Required max sizes for coprocessor SDK initialization.
#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub struct MaxSizes {
    pub max_receipt_size: usize,
    pub max_storage_size: usize,
    pub max_tx_size: usize,
}

/// Input structure for the zkCoprocessor proof generation job.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct CoprocessorProofRequest {
    /// Hash of the user's zkVM program (which uses coprocessor-sdk).
    pub program_hash: String, // hex encoded B256
    /// Blockchain data to be processed by the zkVM program.
    pub blockchain_data: BlockchainData,
    /// Max size configuration for the coprocessor SDK.
    pub max_sizes: MaxSizes,
    /// Type of proof to generate.
    pub proving_type: ProvingType,
    /// Optional override for program location.
    #[serde(default)]
    pub program_location_override: Option<ProgramLocation>,
    /// Optional override for Ethereum RPC URL.
    #[serde(default)]
    pub eth_rpc_url_override: Option<String>,
    /// Optional override for Registry contract address.
    #[serde(default)]
    pub registry_address_override: Option<Address>,
}
