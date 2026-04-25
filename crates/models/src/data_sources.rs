use poem_openapi::Object;
use serde::{Deserialize, Serialize};

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct DataSource {
    pub id: String,
    pub name: String,
    pub source_type: String,
    pub url: Option<String>,
    pub config_json: Option<String>,
    pub enabled: bool,
    pub poll_interval_seconds: i64,
    pub last_checked_at: Option<String>,
    pub last_success_at: Option<String>,
    pub last_error: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct CreateDataSourceRequest {
    pub name: String,
    pub source_type: String,
    pub url: Option<String>,
    pub config_json: Option<String>,
    pub enabled: bool,
    pub poll_interval_seconds: Option<i64>,
}

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct UpdateDataSourceRequest {
    pub name: Option<String>,
    pub source_type: Option<String>,
    pub url: Option<String>,
    pub config_json: Option<String>,
    pub enabled: Option<bool>,
    pub poll_interval_seconds: Option<i64>,
}

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct DataSourceItem {
    pub id: String,
    pub data_source_id: String,
    pub external_id: Option<String>,
    pub title: String,
    pub url: Option<String>,
    pub content: Option<String>,
    pub summary: Option<String>,
    pub raw_payload: Option<String>,
    pub published_at: Option<String>,
    pub discovered_at: String,
    pub created_at: String,
}

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct DataSourceEvent {
    pub id: String,
    pub data_source_id: Option<String>,
    pub event_type: String,
    pub message: String,
    pub payload: Option<String>,
    pub created_at: String,
}

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct DataSourceScript {
    pub data_source_id: String,
    pub language: String,
    pub script_text: String,
    pub script_hash: Option<String>,
    pub last_build_status: Option<String>,
    pub last_build_output: Option<String>,
    pub last_built_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct UpdateDataSourceScriptRequest {
    pub script_text: String,
}

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct BuildDataSourceScriptRequest {
    pub script_text: Option<String>,
}

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct BuildDataSourceScriptResponse {
    pub success: bool,
    pub status: String,
    pub output: String,
    pub script_hash: Option<String>,
}

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct TraderDataSourceAssignment {
    pub trader_id: String,
    pub data_source_id: String,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct UpdateTraderDataSourcesRequest {
    pub data_source_ids: Vec<String>,
}

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct TraderDataSourcesResponse {
    pub data_sources: Vec<DataSource>,
}

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct DataSourceItemsResponse {
    pub items: Vec<DataSourceItem>,
}

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct DataSourceEventsResponse {
    pub events: Vec<DataSourceEvent>,
}

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct EngineTraderDataSource {
    pub id: String,
    pub name: String,
    pub source_type: String,
}
