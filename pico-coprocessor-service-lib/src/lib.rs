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
pub use jobs::generate_proof;
pub use types::{ProgramLocation, ProofRequest, ProofResult, ProvingType};

pub const GENERATE_PROOF_JOB_ID: u32 = 1;
