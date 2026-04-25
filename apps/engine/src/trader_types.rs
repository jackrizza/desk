pub use models::channels::{
    ChannelMessage, CreateChannelMessageRequest, CreateTraderMemoryRequest, EngineChannelContext,
    TraderMemory,
};
pub use models::trader::{
    CreateTraderEventRequest, CreateTraderPortfolioProposalActionRequest,
    CreateTraderPortfolioProposalRequest, CreateTraderTradeProposalRequest, EngineRunnableTrader,
    EngineTraderConfigResponse, TraderPortfolioProposalDetail, TraderRuntimeState,
    TraderTradeProposal, UpsertTraderRuntimeStateRequest,
};

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize)]
pub struct TraderAiDecision {
    pub action: String,
    pub symbol: Option<String>,
    pub side: Option<String>,
    pub quantity: Option<f64>,
    pub reason: String,
    pub confidence: Option<f64>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct TraderAiPortfolioProposal {
    pub title: String,
    pub summary: String,
    pub thesis: String,
    pub confidence: Option<f64>,
    pub expected_duration_seconds: Option<i64>,
    pub market_basis: Option<serde_json::Value>,
    pub invalidation_conditions: Option<serde_json::Value>,
    pub change_thresholds: Option<serde_json::Value>,
    pub actions: Vec<TraderAiPortfolioProposalAction>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct TraderAiPortfolioProposalAction {
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
}

#[derive(Clone, Debug, Deserialize)]
pub struct TraderMemoryDraft {
    pub topic: String,
    pub summary: String,
    pub memory_type: String,
    pub importance: Option<i64>,
    pub confidence: Option<f64>,
}

#[derive(Serialize)]
pub struct OpenAiChatRequest {
    pub model: String,
    pub messages: Vec<OpenAiMessage>,
    pub response_format: OpenAiResponseFormat,
}

#[derive(Serialize)]
pub struct OpenAiTextChatRequest {
    pub model: String,
    pub messages: Vec<OpenAiMessage>,
}

#[derive(Serialize)]
pub struct OpenAiMessage {
    pub role: String,
    pub content: String,
}

#[derive(Serialize)]
pub struct OpenAiResponseFormat {
    pub r#type: String,
}

#[derive(Deserialize)]
pub struct OpenAiChatResponse {
    pub choices: Vec<OpenAiChoice>,
}

#[derive(Deserialize)]
pub struct OpenAiChoice {
    pub message: OpenAiMessageResponse,
}

#[derive(Deserialize)]
pub struct OpenAiMessageResponse {
    pub content: String,
}
