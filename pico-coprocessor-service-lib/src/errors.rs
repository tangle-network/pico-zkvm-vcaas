// pico-coprocessor-service-lib/src/errors.rs
use blueprint_sdk::Error as BlueprintSdkError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ProofServiceError {
    #[error("Configuration Error: {0}")]
    ConfigError(String),
    #[error("Contract Call Error: {0}")]
    ContractCallError(#[from] blueprint_sdk::alloy::contract::Error),
    #[error("Network Error: {0}")]
    NetworkError(#[from] reqwest::Error),
    #[error("Filesystem Error: {0}")]
    IoError(String),
    #[error("Program Not Found in Registry: Hash {0}")]
    ProgramNotFoundInRegistry(String),
    #[error("Program Download Failed: {0}")]
    ProgramDownloadFailed(String),
    #[error("Program Verification Failed: Hash Mismatch (Expected {expected}, Got {got})")]
    ProgramHashMismatch { expected: String, got: String },
    #[error("Invalid Input Data: {0}")]
    InvalidInput(String),
    #[error("Proving Error: {0}")]
    ProvingError(String),
    #[error("Serialization/Deserialization Error: {0}")]
    SerdeError(#[from] serde_json::Error),
    #[error("Blockchain Interaction Error: {0}")]
    BlockchainError(String),
    #[error("Invalid Program Location URL: {0}")]
    InvalidUrl(#[from] url::ParseError),
    #[error("Unsupported Proving Type: {0}")]
    UnsupportedProvingType(String),
    #[error("Temporary Directory Error: {0}")]
    TempDirError(String),
    #[error("Hex Decoding Error: {0}")]
    HexError(#[from] hex::FromHexError),
    #[error("Internal Error: {0}")]
    InternalError(String),
    #[error("Blueprint SDK Error: {0}")]
    BlueprintSdkError(#[from] BlueprintSdkError),
}

impl From<std::io::Error> for ProofServiceError {
    fn from(e: std::io::Error) -> Self {
        ProofServiceError::IoError(e.to_string())
    }
}
