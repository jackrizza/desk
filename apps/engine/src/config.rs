use std::env;

use anyhow::{Context, Result};

#[derive(Debug, Clone)]
pub struct EngineConfig {
    pub engine_name: String,
    pub openapi_base_url: String,
    pub poll_interval_seconds: u64,
    pub trader_proposal_interval_seconds: u64,
    pub enable_test_paper_orders: bool,
    pub channels_enabled: bool,
    pub md_enabled: bool,
    pub channel_check_interval_seconds: u64,
}

impl EngineConfig {
    pub fn from_env() -> Result<Self> {
        let engine_name = env::var("ENGINE_NAME").unwrap_or_else(|_| "engine-1".to_string());
        let openapi_base_url =
            env::var("OPENAPI_BASE_URL").unwrap_or_else(|_| "http://openapi:3000/api".to_string());
        let poll_interval_seconds = env::var("ENGINE_POLL_INTERVAL_SECONDS")
            .unwrap_or_else(|_| "30".to_string())
            .parse::<u64>()
            .context("ENGINE_POLL_INTERVAL_SECONDS must be a positive integer")?;
        let trader_proposal_interval_seconds = env::var("TRADER_PROPOSAL_INTERVAL_SECONDS")
            .unwrap_or_else(|_| "300".to_string())
            .parse::<u64>()
            .context("TRADER_PROPOSAL_INTERVAL_SECONDS must be a positive integer")?;
        let enable_test_paper_orders = env::var("ENGINE_ENABLE_TEST_PAPER_ORDERS")
            .unwrap_or_else(|_| "false".to_string())
            .parse::<bool>()
            .context("ENGINE_ENABLE_TEST_PAPER_ORDERS must be true or false")?;
        let channels_enabled = env::var("ENGINE_CHANNELS_ENABLED")
            .unwrap_or_else(|_| "true".to_string())
            .parse::<bool>()
            .context("ENGINE_CHANNELS_ENABLED must be true or false")?;
        let md_enabled = env::var("ENGINE_MD_ENABLED")
            .unwrap_or_else(|_| "true".to_string())
            .parse::<bool>()
            .context("ENGINE_MD_ENABLED must be true or false")?;
        let channel_check_interval_seconds = env::var("ENGINE_CHANNEL_CHECK_INTERVAL_SECONDS")
            .unwrap_or_else(|_| "60".to_string())
            .parse::<u64>()
            .context("ENGINE_CHANNEL_CHECK_INTERVAL_SECONDS must be a positive integer")?;

        Ok(Self {
            engine_name,
            openapi_base_url: openapi_base_url.trim_end_matches('/').to_string(),
            poll_interval_seconds,
            trader_proposal_interval_seconds,
            enable_test_paper_orders,
            channels_enabled,
            md_enabled,
            channel_check_interval_seconds,
        })
    }
}
