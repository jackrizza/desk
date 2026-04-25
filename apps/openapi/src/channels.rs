use database::Database;
use models::channels::{
    Channel, ChannelMessage, ChannelMessagesResponse, CreateChannelMessageRequest,
    CreateUserChannelMessageRequest, DataScientistProfile, EngineChannelContext, MdProfile,
    TraderPersona, TraderPersonaUpdateRequest, UpdateDataScientistProfileRequest,
    UpdateMdProfileRequest, UpdateUserInvestorProfileRequest, UserInvestorProfile,
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
    database
        .create_channel_message(
            &channel.id,
            &CreateChannelMessageRequest {
                author_type: "user".to_string(),
                author_id: None,
                author_name: Some("User".to_string()),
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
