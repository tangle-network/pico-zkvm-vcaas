// pico-coprocessor-service-lib/src/lib.rs

// Declare modules
mod context;
mod errors;
mod evm;
mod jobs;
mod pico;
mod program;
mod types;

#[cfg(test)]
mod tests;

// Publicly export key types, errors, context, and job functions
pub use context::ServiceContext;
pub use errors::ProofServiceError;
// Export new job function and request type
pub use jobs::{generate_coprocessor_proof, generate_proof};
// Export new request type
pub use types::{
    BlockchainData,
    CoprocessorProofRequest,
    MaxSizes, // Export new types
    ProgramLocation,
    ProofRequest,
    ProofResult,
    ProvingType,
    SerializableLog,
    SerializableReceipt,
    SerializableStorageSlot,
    SerializableTransaction, // Export data types
};

// Define Job IDs
pub const GENERATE_PROOF_JOB_ID: u32 = 1;
pub const GENERATE_COPROCESSOR_PROOF_JOB_ID: u32 = 2; // New Job ID
