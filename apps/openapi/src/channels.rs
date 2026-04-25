use database::Database;
use models::channels::{
    Channel, ChannelMessage, ChannelMessagesResponse, CreateChannelMessageRequest,
    CreateTraderMemoryRequest, CreateUserChannelMessageRequest, DataScientistProfile,
    EngineChannelContext, MdProfile, TraderMemory, TraderMemorySearchRequest,
    TraderMemorySearchResponse, TraderPersona, TraderPersonaUpdateRequest,
    UpdateDataScientistProfileRequest, UpdateMdProfileRequest, UpdateTraderMemoryRequest,
    UpdateUserInvestorProfileRequest, UserInvestorProfile,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelErrorKind {
    BadRequest,
    NotFound,
    Internal,
}

#[derive(Debug)]
pub struct ChannelApiError {
    pub kind: ChannelErrorKind,
    pub message: String,
}

impl ChannelApiError {
    fn bad_request(message: impl Into<String>) -> Self {
        Self {
            kind: ChannelErrorKind::BadRequest,
            message: message.into(),
        }
    }

    fn not_found(message: impl Into<String>) -> Self {
        Self {
            kind: ChannelErrorKind::NotFound,
            message: message.into(),
        }
    }

    fn internal(message: impl Into<String>) -> Self {
        Self {
            kind: ChannelErrorKind::Internal,
            message: message.into(),
        }
    }
}

pub async fn list_channels(database: &Database) -> Result<Vec<Channel>, ChannelApiError> {
    database
        .list_channels()
        .await
        .map_err(|err| ChannelApiError::internal(format!("failed to list channels: {err}")))
}

pub async fn list_messages(
    database: &Database,
    channel_id: &str,
    limit: Option<i64>,
    before: Option<&str>,
    after: Option<&str>,
) -> Result<ChannelMessagesResponse, ChannelApiError> {
    let channel = database
        .get_channel(channel_id)
        .await
        .map_err(|err| ChannelApiError::internal(format!("failed to load channel: {err}")))?
        .ok_or_else(|| ChannelApiError::not_found("channel not found"))?;
    let messages = database
        .list_channel_messages(&channel.id, limit, before, after)
        .await
        .map_err(|err| ChannelApiError::internal(format!("failed to list messages: {err}")))?;
    Ok(ChannelMessagesResponse { messages })
}

pub async fn create_user_message(
    database: &Database,
    channel_id: &str,
    request: CreateUserChannelMessageRequest,
) -> Result<ChannelMessage, ChannelApiError> {
    let channel = database
        .get_channel(channel_id)
        .await
        .map_err(|err| ChannelApiError::internal(format!("failed to load channel: {err}")))?
        .ok_or_else(|| ChannelApiError::not_found("channel not found"))?;
    let content = request.content_markdown.trim();
    if content.is_empty() {
        return Err(ChannelApiError::bad_request("message content is required"));
    }
    let profile = database.get_user_investor_profile().await.map_err(|err| {
        ChannelApiError::internal(format!("failed to load investor profile: {err}"))
    })?;
    let author_name = profile
        .name
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("User");
    database
        .create_channel_message(
            &channel.id,
            &CreateChannelMessageRequest {
                author_type: "user".to_string(),
                author_id: None,
                author_name: Some(author_name.to_string()),
                role: Some("message".to_string()),
                content_markdown: content.to_string(),
                metadata_json: None,
            },
        )
        .await
        .map_err(|err| ChannelApiError::internal(format!("failed to create message: {err}")))
}

pub async fn create_engine_message(
    database: &Database,
    channel_name: &str,
    request: CreateChannelMessageRequest,
) -> Result<ChannelMessage, ChannelApiError> {
    let channel = database
        .get_channel(channel_name)
        .await
        .map_err(|err| ChannelApiError::internal(format!("failed to load channel: {err}")))?
        .ok_or_else(|| ChannelApiError::not_found("channel not found"))?;

    validate_engine_message(&request)?;
    database
        .create_channel_message(&channel.id, &request)
        .await
        .map_err(|err| ChannelApiError::internal(format!("failed to create message: {err}")))
}

pub async fn clear_messages(database: &Database) -> Result<u64, ChannelApiError> {
    database.clear_channel_messages().await.map_err(|err| {
        ChannelApiError::internal(format!("failed to clear channel messages: {err}"))
    })
}

pub async fn list_trader_memories(
    database: &Database,
    trader_id: &str,
    status: Option<&str>,
    memory_type: Option<&str>,
    topic: Option<&str>,
) -> Result<Vec<TraderMemory>, ChannelApiError> {
    database
        .list_trader_memories(
            trader_id,
            status.map(|value| {
                let trimmed = value.trim();
                if trimmed.is_empty() {
                    "__all__"
                } else {
                    trimmed
                }
            }),
            memory_type.and_then(blank_to_none),
            topic.and_then(blank_to_none),
        )
        .await
        .map_err(|err| ChannelApiError::internal(format!("failed to list memories: {err}")))
}

pub async fn create_trader_memory(
    database: &Database,
    trader_id: &str,
    request: CreateTraderMemoryRequest,
) -> Result<TraderMemory, ChannelApiError> {
    validate_memory_create(&request)?;
    database
        .create_trader_memory(trader_id, &request)
        .await
        .map_err(|err| ChannelApiError::internal(format!("failed to create memory: {err}")))
}

pub async fn update_trader_memory(
    database: &Database,
    trader_id: &str,
    memory_id: &str,
    request: UpdateTraderMemoryRequest,
) -> Result<TraderMemory, ChannelApiError> {
    validate_memory_update(&request)?;
    database
        .update_trader_memory(trader_id, memory_id, &request)
        .await
        .map_err(|err| ChannelApiError::internal(format!("failed to update memory: {err}")))?
        .ok_or_else(|| ChannelApiError::not_found("memory not found"))
}

pub async fn archive_trader_memory(
    database: &Database,
    trader_id: &str,
    memory_id: &str,
) -> Result<TraderMemory, ChannelApiError> {
    database
        .archive_trader_memory(trader_id, memory_id)
        .await
        .map_err(|err| ChannelApiError::internal(format!("failed to archive memory: {err}")))?
        .ok_or_else(|| ChannelApiError::not_found("memory not found"))
}

pub async fn search_trader_memories(
    database: &Database,
    trader_id: &str,
    request: TraderMemorySearchRequest,
) -> Result<TraderMemorySearchResponse, ChannelApiError> {
    database
        .search_trader_memories(trader_id, request.query.trim(), request.limit)
        .await
        .map_err(|err| ChannelApiError::internal(format!("failed to search memories: {err}")))
}

pub async fn mark_trader_memory_used(
    database: &Database,
    trader_id: &str,
    memory_id: &str,
) -> Result<TraderMemory, ChannelApiError> {
    database
        .mark_trader_memory_used(trader_id, memory_id)
        .await
        .map_err(|err| ChannelApiError::internal(format!("failed to mark memory used: {err}")))?
        .ok_or_else(|| ChannelApiError::not_found("memory not found"))
}

pub async fn get_trader_persona(
    database: &Database,
    trader_id: &str,
) -> Result<TraderPersona, ChannelApiError> {
    database
        .get_trader_persona(trader_id)
        .await
        .map_err(|err| ChannelApiError::internal(format!("failed to load persona: {err}")))?
        .ok_or_else(|| ChannelApiError::not_found("trader not found"))
}

pub async fn update_trader_persona(
    database: &Database,
    trader_id: &str,
    request: TraderPersonaUpdateRequest,
) -> Result<TraderPersona, ChannelApiError> {
    database
        .update_trader_persona(trader_id, &request)
        .await
        .map_err(|err| ChannelApiError::internal(format!("failed to update persona: {err}")))?
        .ok_or_else(|| ChannelApiError::not_found("trader not found"))
}

pub async fn get_md_profile(database: &Database) -> Result<MdProfile, ChannelApiError> {
    database
        .get_md_profile()
        .await
        .map_err(|err| ChannelApiError::internal(format!("failed to load MD profile: {err}")))
}

pub async fn update_md_profile(
    database: &Database,
    request: UpdateMdProfileRequest,
) -> Result<MdProfile, ChannelApiError> {
    database
        .update_md_profile(&request)
        .await
        .map_err(|err| ChannelApiError::internal(format!("failed to update MD profile: {err}")))
}

pub async fn get_data_scientist_profile(
    database: &Database,
) -> Result<DataScientistProfile, ChannelApiError> {
    database.get_data_scientist_profile().await.map_err(|err| {
        ChannelApiError::internal(format!("failed to load Data Scientist profile: {err}"))
    })
}

pub async fn update_data_scientist_profile(
    database: &Database,
    request: UpdateDataScientistProfileRequest,
) -> Result<DataScientistProfile, ChannelApiError> {
    database
        .update_data_scientist_profile(&request)
        .await
        .map_err(|err| {
            ChannelApiError::internal(format!("failed to update Data Scientist profile: {err}"))
        })
}

pub async fn get_investor_profile(
    database: &Database,
) -> Result<UserInvestorProfile, ChannelApiError> {
    database
        .get_user_investor_profile()
        .await
        .map_err(|err| ChannelApiError::internal(format!("failed to load investor profile: {err}")))
}

pub async fn update_investor_profile(
    database: &Database,
    request: UpdateUserInvestorProfileRequest,
) -> Result<UserInvestorProfile, ChannelApiError> {
    database
        .update_user_investor_profile(&request)
        .await
        .map_err(|err| {
            ChannelApiError::internal(format!("failed to update investor profile: {err}"))
        })
}

pub async fn engine_context(database: &Database) -> Result<EngineChannelContext, ChannelApiError> {
    database
        .engine_channel_context()
        .await
        .map_err(|err| ChannelApiError::internal(format!("failed to load channel context: {err}")))
}

fn validate_engine_message(request: &CreateChannelMessageRequest) -> Result<(), ChannelApiError> {
    match request.author_type.as_str() {
        "trader" | "md" | "system" => {}
        _ => {
            return Err(ChannelApiError::bad_request(
                "engine messages must be from trader, md, or system",
            ));
        }
    }
    match request.role.as_deref().unwrap_or("message") {
        "message" | "question" | "answer" | "alert" | "proposal" | "review" | "system" => {}
        _ => return Err(ChannelApiError::bad_request("invalid message role")),
    }
    if request.content_markdown.trim().is_empty() {
        return Err(ChannelApiError::bad_request("message content is required"));
    }
    Ok(())
}

fn validate_memory_create(request: &CreateTraderMemoryRequest) -> Result<(), ChannelApiError> {
    validate_memory_type(&request.memory_type)?;
    validate_topic_summary(&request.topic, &request.summary)?;
    validate_importance(request.importance.unwrap_or(3))?;
    if let Some(confidence) = request.confidence {
        validate_confidence(confidence)?;
    }
    Ok(())
}

fn validate_memory_update(request: &UpdateTraderMemoryRequest) -> Result<(), ChannelApiError> {
    if let Some(memory_type) = &request.memory_type {
        validate_memory_type(memory_type)?;
    }
    if let Some(status) = &request.status {
        if !matches!(status.as_str(), "active" | "archived" | "superseded") {
            return Err(ChannelApiError::bad_request("invalid memory status"));
        }
    }
    if let Some(topic) = &request.topic {
        if topic.trim().is_empty() {
            return Err(ChannelApiError::bad_request("memory topic is required"));
        }
    }
    if let Some(summary) = &request.summary {
        if summary.trim().is_empty() {
            return Err(ChannelApiError::bad_request("memory summary is required"));
        }
        if summary.len() > 1000 {
            return Err(ChannelApiError::bad_request("memory summary is too long"));
        }
    }
    if let Some(importance) = request.importance {
        validate_importance(importance)?;
    }
    if let Some(confidence) = request.confidence {
        validate_confidence(confidence)?;
    }
    Ok(())
}

fn validate_memory_type(memory_type: &str) -> Result<(), ChannelApiError> {
    if matches!(
        memory_type,
        "answer"
            | "review"
            | "decision"
            | "user_preference"
            | "risk_note"
            | "data_note"
            | "proposal_note"
            | "channel_resolution"
    ) {
        Ok(())
    } else {
        Err(ChannelApiError::bad_request("invalid memory type"))
    }
}

fn validate_topic_summary(topic: &str, summary: &str) -> Result<(), ChannelApiError> {
    if topic.trim().is_empty() {
        return Err(ChannelApiError::bad_request("memory topic is required"));
    }
    if summary.trim().is_empty() {
        return Err(ChannelApiError::bad_request("memory summary is required"));
    }
    if summary.len() > 1000 {
        return Err(ChannelApiError::bad_request("memory summary is too long"));
    }
    Ok(())
}

fn validate_importance(importance: i64) -> Result<(), ChannelApiError> {
    if (1..=5).contains(&importance) {
        Ok(())
    } else {
        Err(ChannelApiError::bad_request(
            "importance must be between 1 and 5",
        ))
    }
}

fn validate_confidence(confidence: f64) -> Result<(), ChannelApiError> {
    if (0.0..=1.0).contains(&confidence) {
        Ok(())
    } else {
        Err(ChannelApiError::bad_request(
            "confidence must be between 0 and 1",
        ))
    }
}

fn blank_to_none(value: &str) -> Option<&str> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}
