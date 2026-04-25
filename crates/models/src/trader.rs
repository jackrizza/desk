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
    pub persona: Option<String>,
    pub tone: Option<String>,
    pub communication_style: Option<String>,
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
pub struct TraderSymbol {
    pub id: String,
    pub trader_id: String,
    pub symbol: String,
    pub asset_type: String,
    pub name: Option<String>,
    pub exchange: Option<String>,
    pub sector: Option<String>,
    pub industry: Option<String>,
    pub notes: Option<String>,
    pub thesis: Option<String>,
    pub fit_score: Option<f64>,
    pub status: String,
    pub source: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct CreateTraderSymbolRequest {
    pub symbol: String,
    pub asset_type: Option<String>,
    pub name: Option<String>,
    pub exchange: Option<String>,
    pub sector: Option<String>,
    pub industry: Option<String>,
    pub notes: Option<String>,
    pub thesis: Option<String>,
    pub fit_score: Option<f64>,
    pub status: Option<String>,
    pub source: Option<String>,
}

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct UpdateTraderSymbolRequest {
    pub asset_type: Option<String>,
    pub name: Option<String>,
    pub exchange: Option<String>,
    pub sector: Option<String>,
    pub industry: Option<String>,
    pub notes: Option<String>,
    pub thesis: Option<String>,
    pub fit_score: Option<f64>,
    pub status: Option<String>,
}

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct BulkUpsertTraderSymbolsRequest {
    pub symbols: Vec<CreateTraderSymbolRequest>,
}

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct SuggestTraderSymbolsRequest {
    pub max_symbols: Option<i64>,
    pub include_etfs: Option<bool>,
    pub include_stocks: Option<bool>,
    pub focus: Option<String>,
}

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct TraderSymbolsResponse {
    pub symbols: Vec<TraderSymbol>,
}

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct SuggestTraderSymbolsResponse {
    pub suggestions: Vec<TraderSymbol>,
}

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct TraderPortfolioProposal {
    pub id: String,
    pub trader_id: String,
    pub paper_account_id: Option<String>,
    pub title: String,
    pub summary: String,
    pub thesis: String,
    pub status: String,
    pub plan_state: String,
    pub confidence: Option<f64>,
    pub proposed_actions_json: String,
    pub source_snapshot_json: Option<String>,
    pub risk_snapshot_json: Option<String>,
    pub market_snapshot_json: Option<String>,
    pub market_basis_json: Option<String>,
    pub invalidation_conditions_json: Option<String>,
    pub change_thresholds_json: Option<String>,
    pub replacement_reason: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub reviewed_at: Option<String>,
    pub review_note: Option<String>,
    pub accepted_at: Option<String>,
    pub active_until: Option<String>,
    pub expected_duration_seconds: Option<i64>,
}

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct TraderPortfolioProposalAction {
    pub id: String,
    pub proposal_id: String,
    pub trader_id: String,
    pub symbol: Option<String>,
    pub action_type: String,
    pub side: Option<String>,
    pub quantity: Option<f64>,
    pub order_type: Option<String>,
    pub entry_price: Option<f64>,
    pub exit_price: Option<f64>,
    pub limit_price: Option<f64>,
    pub stop_price: Option<f64>,
    pub expected_duration_seconds: Option<i64>,
    pub enact_by: Option<String>,
    pub market_price_at_creation: Option<f64>,
    pub rationale: String,
    pub confidence: Option<f64>,
    pub risk_decision: Option<String>,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct CreateTraderPortfolioProposalActionRequest {
    pub symbol: Option<String>,
    pub action_type: String,
    pub side: Option<String>,
    pub quantity: Option<f64>,
    pub order_type: Option<String>,
    pub entry_price: Option<f64>,
    pub exit_price: Option<f64>,
    pub limit_price: Option<f64>,
    pub stop_price: Option<f64>,
    pub expected_duration_seconds: Option<i64>,
    pub enact_by: Option<String>,
    pub market_price_at_creation: Option<f64>,
    pub rationale: String,
    pub confidence: Option<f64>,
    pub risk_decision: Option<String>,
}

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct CreateTraderPortfolioProposalRequest {
    pub trader_id: String,
    pub paper_account_id: Option<String>,
    pub title: String,
    pub summary: String,
    pub thesis: String,
    pub confidence: Option<f64>,
    pub expected_duration_seconds: Option<i64>,
    pub proposed_actions: Vec<CreateTraderPortfolioProposalActionRequest>,
    pub source_snapshot_json: Option<String>,
    pub risk_snapshot_json: Option<String>,
    pub market_snapshot_json: Option<String>,
    pub market_basis_json: Option<String>,
    pub invalidation_conditions_json: Option<String>,
    pub change_thresholds_json: Option<String>,
    pub replacement_reason: Option<String>,
}

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct ReviewTraderPortfolioProposalRequest {
    pub status: String,
    pub review_note: Option<String>,
}

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct TraderPortfolioProposalDetail {
    pub proposal: TraderPortfolioProposal,
    pub actions: Vec<TraderPortfolioProposalAction>,
}

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct TraderPortfolioProposalsResponse {
    pub proposals: Vec<TraderPortfolioProposalDetail>,
}

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct TraderDetail {
    pub trader: Trader,
    pub info_sources: Vec<TraderInfoSource>,
    pub runtime_state: Option<TraderRuntimeState>,
    pub recent_events: Vec<TraderEvent>,
    pub tracked_symbols: Vec<TraderSymbol>,
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
    pub persona: Option<String>,
    pub tone: Option<String>,
    pub communication_style: Option<String>,
    pub info_sources: Vec<TraderInfoSource>,
    pub data_sources: Vec<EngineTraderDataSource>,
    pub tracked_symbols: Vec<TraderSymbol>,
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

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct TraderChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct TraderChatRequest {
    pub message: String,
    pub conversation: Option<Vec<TraderChatMessage>>,
}

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct TraderChatResponse {
    pub reply: String,
    pub trader_id: String,
    pub trader_name: String,
    pub referenced_events: Vec<String>,
    pub referenced_proposals: Vec<String>,
    pub referenced_orders: Vec<String>,
    pub actions: Vec<TraderChatAction>,
}

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct TraderChatAction {
    pub r#type: String,
    pub entity_id: Option<String>,
    pub title: Option<String>,
    pub status: Option<String>,
}
