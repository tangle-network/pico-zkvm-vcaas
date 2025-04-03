use crate::{ServiceContext, errors::ProofServiceError, types::ProgramLocation};
use blueprint_sdk::{
    alloy::{primitives::B256, sol},
    evm::util::get_provider_http,
};
use blueprint_sdk::{debug, info};
use url::Url;

sol!(
    #[sol(rpc)]
    #[derive(Debug)]
    ProgramRegistry,
    "../contracts/out/ProgramRegistry.sol/ProgramRegistry.json"
);

/// Fetches the program location from the EVM registry contract.
pub async fn get_program_location_from_registry(
    context: &ServiceContext,
    program_hash: &B256,
) -> Result<ProgramLocation, ProofServiceError> {
    let registry_address = context.get_registry_address();
    debug!(%registry_address, %program_hash, "Querying ProgramRegistry contract for location");

    // Create a contract instance
    let provider = get_provider_http(context.eth_rpc_url.as_str());
    let contract = ProgramRegistry::new(registry_address, provider);

    // Prepare the call object for getProgramLocation
    let call = contract.getProgramLocation(*program_hash);

    // Execute the call
    let result = call.call().await?;
    // Success: result is ProgramRegistry::getProgramLocationReturn { location: String }
    let location_string = result.location; // Access the named field
    info!(%program_hash, %location_string, "Found program location in registry");

    // Attempt to parse as URL. Need robust handling for other schemes (ipfs://)
    // This basic parsing assumes http/https.
    let url = Url::parse(&location_string).map_err(|e| ProofServiceError::InvalidUrl(e))?;
    Ok(ProgramLocation::RemoteUrl(url))
}
