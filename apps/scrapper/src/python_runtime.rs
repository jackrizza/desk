use std::{ffi::CString, fs, path::Path, process::Command, time::Duration};

use anyhow::{Context, Result};
use models::data_sources::{DataSource, DataSourceScript};
use pyo3::{prelude::*, types::PyDict};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use tokio::time;
use tracing::{info, warn};

use crate::types::ScrappedItem;

#[derive(Clone, Debug)]
pub struct PythonRuntimeConfig {
    pub venv_path: String,
    pub max_items: usize,
    pub timeout_seconds: u64,
}

#[derive(Clone)]
pub struct PythonRuntime {
    config: PythonRuntimeConfig,
}

#[derive(Deserialize)]
struct PythonCollectResult {
    items: Vec<PythonCollectedItem>,
}

#[derive(Deserialize)]
struct PythonCollectedItem {
    external_id: Option<String>,
    title: Option<String>,
    url: Option<String>,
    content: Option<String>,
    summary: Option<String>,
    published_at: Option<String>,
}

impl PythonRuntime {
    pub fn new(config: PythonRuntimeConfig) -> Result<Self> {
        ensure_venv(&config.venv_path)?;
        info!(
            python_venv_path = %config.venv_path,
            max_items = config.max_items,
            timeout_seconds = config.timeout_seconds,
            "python script runtime initialized"
        );
        Ok(Self { config })
    }

    pub async fn collect(
        &self,
        source: &DataSource,
        script: &DataSourceScript,
    ) -> Result<Vec<ScrappedItem>> {
        let source = source.clone();
        let script_text = script.script_text.clone();
        let max_items = self.config.max_items;
        let timeout = Duration::from_secs(self.config.timeout_seconds);
        let task =
            tokio::task::spawn_blocking(move || execute_script(&source, &script_text, max_items));

        match time::timeout(timeout, task).await {
            Ok(Ok(result)) => result,
            Ok(Err(err)) => Err(err.into()),
            Err(_) => anyhow::bail!(
                "python script timed out after {} seconds",
                timeout.as_secs()
            ),
        }
    }
}

fn ensure_venv(path: &str) -> Result<()> {
    let marker = Path::new(path).join("pyvenv.cfg");
    if marker.exists() {
        return Ok(());
    }

    if let Some(parent) = Path::new(path).parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create venv parent {}", parent.display()))?;
    }

    let status = Command::new("python3")
        .args(["-m", "venv", path])
        .status()
        .or_else(|_| Command::new("python").args(["-m", "venv", path]).status())
        .with_context(|| format!("failed to create python venv at {path}"))?;

    if !status.success() {
        anyhow::bail!("python venv creation failed at {path}");
    }

    Ok(())
}

fn execute_script(
    source: &DataSource,
    script: &str,
    max_items: usize,
) -> Result<Vec<ScrappedItem>> {
    Python::attach(|py| -> Result<Vec<ScrappedItem>> {
        let locals = PyDict::new(py);
        let context = serde_json::json!({
            "data_source_id": source.id,
            "name": source.name,
            "source_type": source.source_type,
            "url": source.url,
            "config": source.config_json
                .as_deref()
                .and_then(|value| serde_json::from_str::<serde_json::Value>(value).ok())
                .unwrap_or_else(|| serde_json::json!({})),
            "last_checked_at": source.last_checked_at,
        });
        locals.set_item("_context_json", context.to_string())?;
        let bootstrap = CString::new("import json\ncontext = json.loads(_context_json)")?;
        py.run(bootstrap.as_c_str(), Some(&locals), Some(&locals))?;
        let script_code = CString::new(script).context("python script contains a null byte")?;
        py.run(script_code.as_c_str(), Some(&locals), Some(&locals))
            .context("python script execution failed")?;
        let collect_code = CString::new(
            r#"
if "collect" not in globals() or not callable(collect):
    raise ValueError("script must define collect(context)")
_result = collect(context)
_json_result = json.dumps(_result)
"#,
        )?;
        py.run(collect_code.as_c_str(), Some(&locals), Some(&locals))
            .context("python collect(context) failed")?;

        let result_json: String = locals
            .get_item("_json_result")?
            .context("python script did not produce a result")?
            .extract()?;
        parse_collect_result(&result_json, max_items)
    })
}

fn parse_collect_result(result_json: &str, max_items: usize) -> Result<Vec<ScrappedItem>> {
    let result: PythonCollectResult = serde_json::from_str(result_json)
        .context("collect(context) must return a dict with items")?;
    let mut items = Vec::new();
    for item in result.items.into_iter().take(max_items) {
        let title = item
            .title
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .context("python script returned an item without a title")?;
        let content = truncate_optional(item.content, 64_000);
        let summary = truncate_optional(item.summary, 8_000);
        let external_id = item
            .external_id
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| hash_item(&title, item.url.as_deref(), content.as_deref()));
        let raw_payload = serde_json::json!({
            "external_id": external_id,
            "title": title,
            "url": item.url,
            "content": content,
            "summary": summary,
            "published_at": item.published_at,
        })
        .to_string();
        items.push(ScrappedItem {
            external_id,
            title,
            url: item.url,
            content,
            summary,
            raw_payload: Some(raw_payload),
            published_at: item.published_at,
        });
    }

    if items.len() == max_items {
        warn!(max_items, "python script result was truncated");
    }

    Ok(items)
}

fn truncate_optional(value: Option<String>, max_len: usize) -> Option<String> {
    value.map(|text| {
        if text.len() <= max_len {
            text
        } else {
            text.chars().take(max_len).collect()
        }
    })
}

fn hash_item(title: &str, url: Option<&str>, content: Option<&str>) -> String {
    let mut hasher = Sha256::new();
    hasher.update(title.as_bytes());
    if let Some(url) = url {
        hasher.update(url.as_bytes());
    }
    if let Some(content) = content {
        hasher.update(content.as_bytes());
    }
    format!("{:x}", hasher.finalize())
}
