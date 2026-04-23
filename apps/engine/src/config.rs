use std::env;

use anyhow::{Context, Result};

#[derive(Debug, Clone)]
pub struct EngineConfig {
    pub engine_name: String,
    pub openapi_base_url: String,
    pub poll_interval_seconds: u64,
    pub enable_test_paper_orders: bool,
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
        let enable_test_paper_orders = env::var("ENGINE_ENABLE_TEST_PAPER_ORDERS")
            .unwrap_or_else(|_| "false".to_string())
            .parse::<bool>()
            .context("ENGINE_ENABLE_TEST_PAPER_ORDERS must be true or false")?;

        Ok(Self {
            engine_name,
            openapi_base_url: openapi_base_url.trim_end_matches('/').to_string(),
            poll_interval_seconds,
            enable_test_paper_orders,
        })
    }
}
