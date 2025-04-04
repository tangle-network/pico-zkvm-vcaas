// pico-coprocessor-service-bin/src/main.rs
use blueprint_sdk::{
    Job,
    Router,                     // Ensure Job and Router are imported
    alloy::primitives::Address, // Import Address
    contexts::tangle::TangleClientContext,
    crypto::{sp_core::SpSr25519, tangle_pair_signer::TanglePairSigner},
    keystore::backends::Backend,
    runner::{BlueprintRunner, config::BlueprintEnvironment, tangle::config::TangleConfig},
    tangle::{
        consumer::TangleConsumer, filters::MatchesServiceId, layers::TangleLayer,
        producer::TangleProducer,
    },
};
// Import new types and jobs from lib
use pico_coprocessor_service_blueprint_lib::{
    GENERATE_COPROCESSOR_PROOF_JOB_ID,
    GENERATE_PROOF_JOB_ID,
    ServiceContext,
    generate_coprocessor_proof,
    generate_proof,
    say_hello, // Jobs
};
use std::{path::PathBuf, str::FromStr}; // For PathBuf and FromStr
use tower::filter::FilterLayer;
use tracing::error;
use tracing::level_filters::LevelFilter;
use url::Url; // For parsing RPC URL

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Use Box<dyn Error> for broader error handling
    setup_log();
    tracing::info!("Starting Pico Coprocessor Service Blueprint Runner...");

    // --- Load Configuration ---
    let env = BlueprintEnvironment::load()
        .map_err(|e| format!("Failed to load blueprint environment: {}", e))?;

    // Tangle Signer Setup
    let sr25519_signer = env.keystore().first_local::<SpSr25519>()?;
    let sr25519_pair = env.keystore().get_secret::<SpSr25519>(&sr25519_signer)?;
    let tangle_signer = TanglePairSigner::new(sr25519_pair.0);
    tracing::info!("Tangle signer configured.");

    // Tangle Client Setup
    let tangle_client = env.tangle_client().await?;
    let tangle_producer =
        TangleProducer::finalized_blocks(tangle_client.rpc_client.clone()).await?;
    let tangle_consumer = TangleConsumer::new(tangle_client.rpc_client.clone(), tangle_signer);
    tracing::info!("Tangle producer and consumer configured.");

    let tangle_config = env.protocol_settings.tangle()?.clone(); // Use loaded config
    let service_id = tangle_config
        .service_id
        .ok_or("Tangle Service ID not configured")?;
    tracing::info!(%service_id, "Using Tangle Service ID");

    // --- Service Specific Configuration ---
    // Get these from environment variables or a config file via BlueprintEnvironment extensions
    // Example using environment variables (add error handling)
    let eth_rpc_env =
        std::env::var("ETH_RPC_URL").map_err(|_| "ETH_RPC_URL environment variable not set")?;
    let eth_rpc_url =
        Url::parse(&eth_rpc_env).map_err(|e| format!("Invalid ETH_RPC_URL: {}", e))?;

    let registry_addr_env = std::env::var("REGISTRY_CONTRACT_ADDRESS")
        .map_err(|_| "REGISTRY_CONTRACT_ADDRESS environment variable not set")?;
    let registry_contract_address = Address::from_str(&registry_addr_env)
        .map_err(|e| format!("Invalid REGISTRY_CONTRACT_ADDRESS: {}", e))?;

    let temp_dir_base_env =
        std::env::var("TEMP_DIR_BASE").unwrap_or_else(|_| "/tmp/pico-service".to_string());
    let temp_dir_base = PathBuf::from(temp_dir_base_env);

    tracing::info!(rpc_url = %eth_rpc_url, registry = %registry_contract_address, temp_dir = ?temp_dir_base, "Service configuration loaded");

    // --- Create Service Context ---
    let service_context =
        ServiceContext::new(eth_rpc_url, registry_contract_address, temp_dir_base)
            .map_err(|e| format!("Failed to create service context: {:?}", e))?;
    tracing::info!("Service context created.");

    // --- Build Router ---
    let router = Router::new()
        // Add routes for each job ID
        .route(GENERATE_PROOF_JOB_ID, generate_proof.layer(TangleLayer))
        .route(
            GENERATE_COPROCESSOR_PROOF_JOB_ID,
            generate_coprocessor_proof.layer(TangleLayer),
        ) // Add new route
        // Global filter layer
        .layer(FilterLayer::new(MatchesServiceId(service_id)))
        // Add the shared context
        .with_context(service_context);
    tracing::info!("Router configured with {} jobs.", 3); // Update count

    // --- Build and Run Runner ---
    let runner_result = BlueprintRunner::builder(tangle_config, env)
        .router(router)
        .producer(tangle_producer)
        .consumer(tangle_consumer)
        .with_shutdown_handler(async { println!("Shutting down Pico Coprocessor Service!") })
        .run()
        .await;

    if let Err(e) = runner_result {
        error!("Blueprint runner failed: {:?}", e);
        // Convert specific blueprint errors if needed, otherwise return the boxed error
        return Err(e.into());
    }

    tracing::info!("Blueprint runner finished successfully.");
    Ok(())
}

pub fn setup_log() {
    use tracing_subscriber::util::SubscriberInitExt;
    let filter = tracing_subscriber::EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env_lossy();

    let _ = tracing_subscriber::fmt() //.SubscriberBuilder::default()
        // .without_time() // Keep time for debugging
        .with_span_events(tracing_subscriber::fmt::format::FmtSpan::CLOSE) // Show span duration
        .with_env_filter(filter)
        // .finish() // finish called by init
        .try_init(); // Use try_init to avoid panic if already initialized
    tracing::info!("Logging initialized.");
}
