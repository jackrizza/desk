use poem_openapi::Object;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct ChatCommandRequest {
    pub message: String,
    pub context: Option<Value>,
    pub confirmation_token: Option<String>,
    pub confirmed: Option<bool>,
}

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct ChatCommandIntent {
    pub action: String,
    pub entity: String,
    pub parameters: Value,
    pub confidence: f64,
    pub requires_confirmation: bool,
}

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct ChatCommandAction {
    pub r#type: String,
    pub entity_id: Option<String>,
    pub message: Option<String>,
}

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct ChatCommandResponse {
    pub reply: String,
    pub actions: Vec<ChatCommandAction>,
    pub handled: bool,
    pub requires_confirmation: bool,
    pub confirmation_token: Option<String>,
    pub intent: Option<ChatCommandIntent>,
}
