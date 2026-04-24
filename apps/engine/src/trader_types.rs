pub use models::trader::{
    CreateTraderEventRequest, CreateTraderTradeProposalRequest, EngineRunnableTrader,
    EngineTraderConfigResponse, TraderRuntimeState, TraderTradeProposal,
    UpsertTraderRuntimeStateRequest,
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

#[derive(Serialize)]
pub struct OpenAiChatRequest {
    pub model: String,
    pub messages: Vec<OpenAiMessage>,
    pub response_format: OpenAiResponseFormat,
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
