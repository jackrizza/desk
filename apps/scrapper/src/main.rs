mod config;
mod db;
mod sources;
mod types;
mod worker;

use anyhow::Result;
use tracing::{error, info};
use tracing_subscriber::{EnvFilter, fmt};

use crate::{config::ScrapperConfig, worker::run_worker};

#[tokio::main]
async fn main() -> Result<()> {
    initialize_tracing();
    let config = ScrapperConfig::from_env()?;
    info!("scrapper configuration loaded");
    if let Err(err) = run_worker(config).await {
        error!(error = %err, "scrapper exited with error");
        return Err(err);
    }
    Ok(())
}

fn initialize_tracing() {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    fmt().with_env_filter(env_filter).compact().init();
}
