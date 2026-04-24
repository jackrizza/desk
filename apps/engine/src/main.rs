mod client;
mod config;
mod indicators;
mod risk;
mod risk_guard;
mod strategy_eval;
mod strategy_runner;
mod trader_client;
mod trader_runner;
mod trader_types;
mod types;
mod worker;

use anyhow::Result;
use tracing::{error, info};
use tracing_subscriber::{EnvFilter, fmt};

use crate::{config::EngineConfig, worker::run_engine};

#[tokio::main]
async fn main() -> Result<()> {
    initialize_tracing();

    let config = EngineConfig::from_env()?;
    info!("engine configuration loaded");

    if let Err(err) = run_engine(config).await {
        error!(error = %err, "engine exited with error");
        return Err(err);
    }

    Ok(())
}

fn initialize_tracing() {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    fmt().with_env_filter(env_filter).compact().init();
}
