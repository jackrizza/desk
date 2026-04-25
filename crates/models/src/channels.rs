use poem_openapi::Object;
use serde::{Deserialize, Serialize};

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct Channel {
    pub id: String,
    pub name: String,
    pub display_name: String,
    pub description: Option<String>,
    pub is_system: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct ChannelMessage {
    pub id: String,
    pub channel_id: String,
    pub author_type: String,
    pub author_id: Option<String>,
    pub author_name: String,
    pub role: String,
    pub content_markdown: String,
    pub metadata_json: Option<String>,
    pub created_at: String,
}

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct CreateChannelMessageRequest {
    pub author_type: String,
    pub author_id: Option<String>,
    pub author_name: Option<String>,
    pub role: Option<String>,
    pub content_markdown: String,
    pub metadata_json: Option<String>,
}

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct CreateUserChannelMessageRequest {
    pub content_markdown: String,
}

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct TraderPersonaUpdateRequest {
    pub persona: Option<String>,
    pub tone: Option<String>,
    pub communication_style: Option<String>,
}

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct MdProfile {
    pub id: String,
    pub name: String,
    pub persona: String,
    pub tone: String,
    pub communication_style: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct UpdateMdProfileRequest {
    pub name: Option<String>,
    pub persona: Option<String>,
    pub tone: Option<String>,
    pub communication_style: Option<String>,
    pub openai_api_key: Option<String>,
}

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct DataScientistProfile {
    pub id: String,
    pub name: String,
    pub persona: String,
    pub tone: String,
    pub communication_style: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct UpdateDataScientistProfileRequest {
    pub name: Option<String>,
    pub persona: Option<String>,
    pub tone: Option<String>,
    pub communication_style: Option<String>,
    pub openai_api_key: Option<String>,
}

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct AgentChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct MdChatRequest {
    pub message: String,
    pub conversation: Option<Vec<AgentChatMessage>>,
}

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct MdChatResponse {
    pub reply: String,
    pub referenced_channels: Vec<String>,
    pub referenced_traders: Vec<String>,
    pub referenced_events: Vec<String>,
}

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct DataScientistChatRequest {
    pub message: String,
    pub conversation: Option<Vec<AgentChatMessage>>,
}

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct DataScientistChatAction {
    pub r#type: String,
    pub entity_id: Option<String>,
    pub name: Option<String>,
    pub source_type: Option<String>,
    pub url: Option<String>,
    pub build_status: Option<String>,
    pub build_output: Option<String>,
}

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct DataScientistChatResponse {
    pub reply: String,
    pub actions: Vec<DataScientistChatAction>,
}

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct UserInvestorProfile {
    pub id: String,
    pub name: Option<String>,
    pub age: Option<i64>,
    pub about: Option<String>,
    pub investment_goals: Option<String>,
    pub risk_tolerance: Option<String>,
    pub time_horizon: Option<String>,
    pub liquidity_needs: Option<String>,
    pub income_needs: Option<String>,
    pub investment_experience: Option<String>,
    pub restrictions: Option<String>,
    pub preferred_sectors: Option<String>,
    pub avoided_sectors: Option<String>,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct UpdateUserInvestorProfileRequest {
    pub name: Option<String>,
    pub age: Option<i64>,
    pub about: Option<String>,
    pub investment_goals: Option<String>,
    pub risk_tolerance: Option<String>,
    pub time_horizon: Option<String>,
    pub liquidity_needs: Option<String>,
    pub income_needs: Option<String>,
    pub investment_experience: Option<String>,
    pub restrictions: Option<String>,
    pub preferred_sectors: Option<String>,
    pub avoided_sectors: Option<String>,
    pub notes: Option<String>,
}

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct ChannelMessagesResponse {
    pub messages: Vec<ChannelMessage>,
}

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct TraderPersona {
    pub trader_id: String,
    pub persona: Option<String>,
    pub tone: Option<String>,
    pub communication_style: Option<String>,
}

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct TraderMemory {
    pub id: String,
    pub trader_id: String,
    pub memory_type: String,
    pub topic: String,
    pub summary: String,
    pub source_channel_id: Option<String>,
    pub source_message_id: Option<String>,
    pub confidence: Option<f64>,
    pub importance: i64,
    pub status: String,
    pub last_used_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct CreateTraderMemoryRequest {
    pub memory_type: String,
    pub topic: String,
    pub summary: String,
    pub source_channel_id: Option<String>,
    pub source_message_id: Option<String>,
    pub confidence: Option<f64>,
    pub importance: Option<i64>,
}

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct UpdateTraderMemoryRequest {
    pub memory_type: Option<String>,
    pub topic: Option<String>,
    pub summary: Option<String>,
    pub confidence: Option<f64>,
    pub importance: Option<i64>,
    pub status: Option<String>,
}

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct TraderMemorySearchRequest {
    pub query: String,
    pub limit: Option<i64>,
}

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct TraderMemorySearchResponse {
    pub memories: Vec<TraderMemory>,
}

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct EngineChannelContext {
    pub channels: Vec<Channel>,
    pub recent_messages: Vec<ChannelMessage>,
    pub md_profile: MdProfile,
    pub md_openai_api_key: Option<String>,
    pub user_investor_profile: UserInvestorProfile,
    pub trader_personas: Vec<TraderPersona>,
    pub trader_memories: Vec<TraderMemory>,
}
