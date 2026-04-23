use poem_openapi::Object;
use serde::{Deserialize, Serialize};

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct EngineHealthResponse {
    pub status: String,
}

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct ActiveSymbolsResponse {
    pub symbols: Vec<String>,
}

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct ActiveSymbol {
    pub id: String,
    pub symbol: String,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct EngineHeartbeatRequest {
    pub engine_name: String,
    pub status: String,
    pub timestamp: String,
}

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct EngineHeartbeat {
    pub id: String,
    pub engine_name: String,
    pub status: String,
    pub timestamp: String,
    pub created_at: String,
}

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct EngineEventRequest {
    pub engine_name: String,
    pub event_type: String,
    pub symbol: Option<String>,
    pub message: String,
    pub timestamp: String,
}

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct EngineEvent {
    pub id: String,
    pub engine_name: String,
    pub event_type: String,
    pub symbol: Option<String>,
    pub message: String,
    pub timestamp: String,
    pub created_at: String,
}
