use poem_openapi::Object;
use serde::{Deserialize, Serialize};

use crate::data_sources::EngineTraderDataSource;

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct Trader {
    pub id: String,
    pub name: String,
    pub fundamental_perspective: String,
    pub freedom_level: String,
    pub status: String,
    pub default_paper_account_id: Option<String>,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
    pub started_at: Option<String>,
    pub stopped_at: Option<String>,
}

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct CreateTraderInfoSourceRequest {
    pub source_type: String,
    pub name: String,
    pub config_json: Option<String>,
    pub enabled: Option<bool>,
}

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct CreateTraderRequest {
    pub name: String,
    pub fundamental_perspective: String,
    pub freedom_level: String,
    pub default_paper_account_id: Option<String>,
    pub openai_api_key: String,
    pub info_sources: Vec<CreateTraderInfoSourceRequest>,
}

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct UpdateTraderRequest {
    pub name: Option<String>,
    pub fundamental_perspective: Option<String>,
    pub freedom_level: Option<String>,
    pub default_paper_account_id: Option<String>,
    pub openai_api_key: Option<String>,
    pub info_sources: Option<Vec<CreateTraderInfoSourceRequest>>,
}

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct TraderInfoSource {
    pub id: String,
    pub trader_id: String,
    pub source_type: String,
    pub name: String,
    pub config_json: Option<String>,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct TraderRuntimeState {
    pub trader_id: String,
    pub engine_name: Option<String>,
    pub last_heartbeat_at: Option<String>,
    pub last_evaluation_at: Option<String>,
    pub last_error: Option<String>,
    pub current_task: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct TraderEvent {
    pub id: String,
    pub trader_id: String,
    pub event_type: String,
    pub message: String,
    pub payload: Option<String>,
    pub created_at: String,
}

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct TraderTradeProposal {
    pub id: String,
    pub trader_id: String,
    pub symbol: String,
    pub side: String,
    pub quantity: f64,
    pub order_type: String,
    pub reason: String,
    pub confidence: Option<f64>,
    pub status: String,
    pub reviewed_by: Option<String>,
    pub reviewed_at: Option<String>,
    pub resulting_order_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct TraderDetail {
    pub trader: Trader,
    pub info_sources: Vec<TraderInfoSource>,
    pub runtime_state: Option<TraderRuntimeState>,
    pub recent_events: Vec<TraderEvent>,
}

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct TraderListResponse {
    pub traders: Vec<Trader>,
}

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct TraderEventsResponse {
    pub events: Vec<TraderEvent>,
}

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct TraderTradeProposalsResponse {
    pub proposals: Vec<TraderTradeProposal>,
}

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct EngineRunnableTrader {
    pub id: String,
    pub name: String,
    pub fundamental_perspective: String,
    pub freedom_level: String,
    pub default_paper_account_id: Option<String>,
    pub info_sources: Vec<TraderInfoSource>,
    pub data_sources: Vec<EngineTraderDataSource>,
    pub openai_api_key: String,
}

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct EngineTraderConfigResponse {
    pub traders: Vec<EngineRunnableTrader>,
}

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct UpsertTraderRuntimeStateRequest {
    pub engine_name: Option<String>,
    pub last_heartbeat_at: Option<String>,
    pub last_evaluation_at: Option<String>,
    pub last_error: Option<String>,
    pub current_task: Option<String>,
}

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct CreateTraderEventRequest {
    pub event_type: String,
    pub message: String,
    pub payload: Option<String>,
}

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct CreateTraderTradeProposalRequest {
    pub symbol: String,
    pub side: String,
    pub quantity: f64,
    pub order_type: Option<String>,
    pub reason: String,
    pub confidence: Option<f64>,
}
