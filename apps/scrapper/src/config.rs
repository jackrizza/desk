use anyhow::{Context, Result};
use std::env;

#[derive(Clone, Debug)]
pub struct ScrapperConfig {
    pub scrapper_name: String,
    pub database_url: String,
    pub openapi_base_url: String,
    pub poll_interval_seconds: u64,
    pub python_venv_path: String,
    pub python_max_items: usize,
    pub python_timeout_seconds: u64,
}

impl ScrapperConfig {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            scrapper_name: env::var("SCRAPPER_NAME").unwrap_or_else(|_| "scrapper-1".to_string()),
            database_url: env::var("SCRAPPER_DATABASE_URL")
                .or_else(|_| env::var("DATABASE_URL"))
                .context("SCRAPPER_DATABASE_URL or DATABASE_URL must be set")?,
            openapi_base_url: env::var("OPENAPI_BASE_URL")
                .unwrap_or_else(|_| "http://openapi:3000/api".to_string()),
            poll_interval_seconds: env::var("SCRAPPER_POLL_INTERVAL_SECONDS")
                .ok()
                .and_then(|value| value.parse::<u64>().ok())
                .unwrap_or(30)
                .max(30),
            python_venv_path: env::var("SCRAPPER_PYTHON_VENV_PATH")
                .unwrap_or_else(|_| "/app/.venv".to_string()),
            python_max_items: env::var("SCRAPPER_PYTHON_MAX_ITEMS")
                .ok()
                .and_then(|value| value.parse::<usize>().ok())
                .unwrap_or(100)
                .max(1),
            python_timeout_seconds: env::var("SCRAPPER_PYTHON_TIMEOUT_SECONDS")
                .ok()
                .and_then(|value| value.parse::<u64>().ok())
                .unwrap_or(20)
                .max(1),
        })
    }
}
