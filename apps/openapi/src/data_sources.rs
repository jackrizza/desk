use database::Database;
use models::data_sources::{
    BuildDataSourceScriptRequest, BuildDataSourceScriptResponse, CreateDataSourceRequest,
    DataSource, DataSourceEventsResponse, DataSourceItemsResponse, DataSourceScript,
    TraderDataSourcesResponse, UpdateDataSourceRequest, UpdateDataSourceScriptRequest,
    UpdateTraderDataSourcesRequest,
};
use sha2::{Digest, Sha256};
use std::io::Write;
use std::process::{Command, Stdio};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataSourceErrorKind {
    BadRequest,
    NotFound,
    Internal,
}

#[derive(Debug)]
pub struct DataSourceApiError {
    pub kind: DataSourceErrorKind,
    pub message: String,
}

impl DataSourceApiError {
    fn bad_request(message: impl Into<String>) -> Self {
        Self {
            kind: DataSourceErrorKind::BadRequest,
            message: message.into(),
        }
    }

    fn not_found(message: impl Into<String>) -> Self {
        Self {
            kind: DataSourceErrorKind::NotFound,
            message: message.into(),
        }
    }

    fn internal(message: impl Into<String>) -> Self {
        Self {
            kind: DataSourceErrorKind::Internal,
            message: message.into(),
        }
    }
}

pub async fn create(
    database: &Database,
    request: CreateDataSourceRequest,
) -> Result<DataSource, DataSourceApiError> {
    validate_create(&request)?;
    database
        .create_data_source(&request)
        .await
        .map_err(|err| DataSourceApiError::internal(format!("failed to create data source: {err}")))
}

pub async fn list(database: &Database) -> Result<Vec<DataSource>, DataSourceApiError> {
    database
        .list_data_sources()
        .await
        .map_err(|err| DataSourceApiError::internal(format!("failed to list data sources: {err}")))
}

pub async fn get(database: &Database, source_id: &str) -> Result<DataSource, DataSourceApiError> {
    database
        .get_data_source(source_id)
        .await
        .map_err(|err| DataSourceApiError::internal(format!("failed to load data source: {err}")))?
        .ok_or_else(|| DataSourceApiError::not_found("data source not found"))
}

pub async fn update(
    database: &Database,
    source_id: &str,
    request: UpdateDataSourceRequest,
) -> Result<DataSource, DataSourceApiError> {
    validate_update(&request)?;
    database
        .update_data_source(source_id, &request)
        .await
        .map_err(|err| {
            DataSourceApiError::internal(format!("failed to update data source: {err}"))
        })?
        .ok_or_else(|| DataSourceApiError::not_found("data source not found"))
}

pub async fn delete(database: &Database, source_id: &str) -> Result<(), DataSourceApiError> {
    match database.disable_data_source(source_id).await {
        Ok(true) => Ok(()),
        Ok(false) => Err(DataSourceApiError::not_found("data source not found")),
        Err(err) => Err(DataSourceApiError::internal(format!(
            "failed to disable data source: {err}"
        ))),
    }
}

pub async fn items(
    database: &Database,
    source_id: &str,
) -> Result<DataSourceItemsResponse, DataSourceApiError> {
    let _ = get(database, source_id).await?;
    Ok(DataSourceItemsResponse {
        items: database
            .list_data_source_items(source_id, 100)
            .await
            .map_err(|err| DataSourceApiError::internal(format!("failed to list items: {err}")))?,
    })
}

pub async fn events(
    database: &Database,
    source_id: &str,
) -> Result<DataSourceEventsResponse, DataSourceApiError> {
    let _ = get(database, source_id).await?;
    Ok(DataSourceEventsResponse {
        events: database
            .list_data_source_events(source_id, 100)
            .await
            .map_err(|err| DataSourceApiError::internal(format!("failed to list events: {err}")))?,
    })
}

pub async fn trader_sources(
    database: &Database,
    trader_id: &str,
) -> Result<TraderDataSourcesResponse, DataSourceApiError> {
    Ok(TraderDataSourcesResponse {
        data_sources: database
            .list_trader_data_sources(trader_id)
            .await
            .map_err(|err| {
                DataSourceApiError::internal(format!("failed to list trader data sources: {err}"))
            })?,
    })
}

pub async fn replace_trader_sources(
    database: &Database,
    trader_id: &str,
    request: UpdateTraderDataSourcesRequest,
) -> Result<TraderDataSourcesResponse, DataSourceApiError> {
    database
        .replace_trader_data_sources(trader_id, &request.data_source_ids)
        .await
        .map_err(|err| {
            DataSourceApiError::internal(format!("failed to replace trader data sources: {err}"))
        })?;
    trader_sources(database, trader_id).await
}

pub async fn get_script(
    database: &Database,
    source_id: &str,
) -> Result<DataSourceScript, DataSourceApiError> {
    let source = get_python_script_source(database, source_id).await?;
    if let Some(script) = database
        .get_data_source_script(&source.id)
        .await
        .map_err(|err| DataSourceApiError::internal(format!("failed to load script: {err}")))?
    {
        return Ok(script);
    }

    database
        .upsert_data_source_script(
            &source.id,
            DEFAULT_PYTHON_SCRIPT,
            &script_hash(DEFAULT_PYTHON_SCRIPT),
        )
        .await
        .map_err(|err| {
            DataSourceApiError::internal(format!("failed to create default script: {err}"))
        })
}

pub async fn engine_script(
    database: &Database,
    source_id: &str,
) -> Result<DataSourceScript, DataSourceApiError> {
    let source = get_python_script_source(database, source_id).await?;
    if !source.enabled {
        return Err(DataSourceApiError::not_found(
            "data source script not found",
        ));
    }
    get_script(database, source_id).await
}

pub async fn update_script(
    database: &Database,
    source_id: &str,
    request: UpdateDataSourceScriptRequest,
) -> Result<DataSourceScript, DataSourceApiError> {
    let source = get_python_script_source(database, source_id).await?;
    database
        .upsert_data_source_script(
            &source.id,
            &request.script_text,
            &script_hash(&request.script_text),
        )
        .await
        .map_err(|err| DataSourceApiError::internal(format!("failed to save script: {err}")))
}

pub async fn build_script(
    database: &Database,
    source_id: &str,
    request: BuildDataSourceScriptRequest,
) -> Result<BuildDataSourceScriptResponse, DataSourceApiError> {
    let saved_script = get_script(database, source_id).await?;
    let script = if let Some(script_text) = request.script_text {
        script_text
    } else {
        saved_script.script_text
    };
    let hash = script_hash(&script);
    let validation = validate_python_script(&script);
    let status = if validation.success {
        "success"
    } else {
        "failed"
    };

    let _ = database
        .update_data_source_script_build(source_id, status, &validation.output, Some(&hash))
        .await
        .map_err(|err| {
            DataSourceApiError::internal(format!("failed to persist build result: {err}"))
        })?;

    Ok(BuildDataSourceScriptResponse {
        success: validation.success,
        status: status.to_string(),
        output: validation.output,
        script_hash: Some(hash),
    })
}

fn validate_create(request: &CreateDataSourceRequest) -> Result<(), DataSourceApiError> {
    if request.name.trim().is_empty() {
        return Err(DataSourceApiError::bad_request("name must be non-empty"));
    }
    validate_source_type(&request.source_type)?;
    validate_poll_interval(request.poll_interval_seconds.unwrap_or(30))?;
    validate_url(&request.source_type, request.url.as_deref())
}

fn validate_update(request: &UpdateDataSourceRequest) -> Result<(), DataSourceApiError> {
    if request
        .name
        .as_deref()
        .map(str::trim)
        .is_some_and(str::is_empty)
    {
        return Err(DataSourceApiError::bad_request("name must be non-empty"));
    }
    if let Some(source_type) = &request.source_type {
        validate_source_type(source_type)?;
        validate_url(source_type, request.url.as_deref())?;
    }
    if let Some(interval) = request.poll_interval_seconds {
        validate_poll_interval(interval)?;
    }
    Ok(())
}

fn validate_source_type(source_type: &str) -> Result<(), DataSourceApiError> {
    match source_type.trim() {
        "rss" | "web_page" | "manual_note" | "placeholder_api" | "python_script" => Ok(()),
        _ => Err(DataSourceApiError::bad_request("invalid source_type")),
    }
}

fn validate_poll_interval(interval: i64) -> Result<(), DataSourceApiError> {
    if interval < 30 {
        return Err(DataSourceApiError::bad_request(
            "poll_interval_seconds must be at least 30",
        ));
    }
    Ok(())
}

async fn get_python_script_source(
    database: &Database,
    source_id: &str,
) -> Result<DataSource, DataSourceApiError> {
    let source = get(database, source_id).await?;
    if source.source_type != "python_script" {
        return Err(DataSourceApiError::bad_request(
            "data source is not a python_script source",
        ));
    }
    Ok(source)
}

struct ScriptValidation {
    success: bool,
    output: String,
}

fn validate_python_script(script: &str) -> ScriptValidation {
    let probe = r#"
import ast
import sys

script = sys.stdin.read()
try:
    tree = ast.parse(script, filename="<data-source-script>")
except SyntaxError as exc:
    print(f"SyntaxError on line {exc.lineno}: {exc.msg}")
    sys.exit(1)

for node in tree.body:
    if isinstance(node, ast.FunctionDef) and node.name == "collect":
        if len(node.args.args) == 1 and node.args.args[0].arg == "context":
            print("Build successful.\ncollect(context) function found.")
            sys.exit(0)
        print("Build failed: collect must accept exactly one argument named context.")
        sys.exit(1)

print("Build failed: script must define collect(context).")
sys.exit(1)
"#;
    let mut child = match Command::new("python")
        .arg("-c")
        .arg(probe)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .or_else(|_| {
            Command::new("python3")
                .arg("-c")
                .arg(probe)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
        }) {
        Ok(child) => child,
        Err(_) => {
            return fallback_validate_python_script(script);
        }
    };

    if let Some(mut stdin) = child.stdin.take() {
        if let Err(err) = stdin.write_all(script.as_bytes()) {
            return ScriptValidation {
                success: false,
                output: format!("Build failed: could not send script to Python: {err}"),
            };
        }
    }

    match child.wait_with_output() {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            let message = if stdout.is_empty() { stderr } else { stdout };
            ScriptValidation {
                success: output.status.success(),
                output: if message.is_empty() {
                    "Build failed: Python validation produced no output.".to_string()
                } else {
                    message
                },
            }
        }
        Err(err) => ScriptValidation {
            success: false,
            output: format!("Build failed: Python validation failed to run: {err}"),
        },
    }
}

fn fallback_validate_python_script(script: &str) -> ScriptValidation {
    let has_collect = script.lines().any(|line| {
        let trimmed = line.trim_start();
        trimmed.starts_with("def collect(context)") || trimmed.starts_with("def collect(context:")
    });
    if has_collect {
        ScriptValidation {
            success: true,
            output: "Build successful.\ncollect(context) function found. Python syntax validation was skipped because python was not available.".to_string(),
        }
    } else {
        ScriptValidation {
            success: false,
            output: "Build failed: script must define collect(context).".to_string(),
        }
    }
}

fn script_hash(script: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(script.as_bytes());
    format!("{:x}", hasher.finalize())
}

pub const DEFAULT_PYTHON_SCRIPT: &str = r#"def collect(context):
    return {
        "items": [
            {
                "external_id": "example-item",
                "title": "Example item",
                "url": None,
                "content": "Edit this script to collect data.",
                "summary": "Example Python data source item.",
                "published_at": None,
            }
        ]
    }
"#;

fn validate_url(source_type: &str, url: Option<&str>) -> Result<(), DataSourceApiError> {
    if source_type == "rss" || source_type == "web_page" {
        let Some(url) = url.map(str::trim).filter(|value| !value.is_empty()) else {
            return Err(DataSourceApiError::bad_request("url is required"));
        };
        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Err(DataSourceApiError::bad_request(
                "url must start with http:// or https://",
            ));
        }
    }
    Ok(())
}
