use std::{
    collections::hash_map::DefaultHasher,
    env,
    hash::{Hash, Hasher},
    time::Duration,
};

use database::Database;
use models::{
    chat_commands::{
        ChatCommandAction, ChatCommandIntent, ChatCommandRequest, ChatCommandResponse,
    },
    data_sources::{
        CreateDataSourceRequest, UpdateDataSourceRequest, UpdateTraderDataSourcesRequest,
    },
    trader::{CreateTraderRequest, UpdateTraderRequest},
};
use serde_json::{Value, json};

use crate::{data_sources, traders};

pub async fn handle(
    database: &Database,
    request: ChatCommandRequest,
) -> Result<ChatCommandResponse, String> {
    let message = request.message.trim();
    if message.is_empty() {
        return Ok(not_handled("Tell me what you want to do."));
    }

    if request.confirmed.unwrap_or(false) {
        let token = request.confirmation_token.as_deref().unwrap_or("");
        let Some(intent) = intent_from_token(token) else {
            return Ok(ChatCommandResponse {
                reply: "I could not find the pending command to confirm. Please send the request again.".to_string(),
                handled: true,
                requires_confirmation: false,
                confirmation_token: None,
                actions: vec![],
                intent: None,
            });
        };
        return execute_intent(database, intent, true).await;
    }

    let Some(intent) = parse_command(message).or_else(|| None) else {
        let Some(intent) = parse_command_with_openai(message).await else {
            return Ok(not_handled(""));
        };
        let intent = apply_confirmation_rules(intent);
        if intent.action == "none" || intent.entity == "none" || intent.confidence < 0.65 {
            return Ok(not_handled(""));
        }
        if intent.requires_confirmation {
            let token = token_for_intent(&intent);
            return Ok(ChatCommandResponse {
                reply: confirmation_message(&intent),
                actions: vec![],
                handled: true,
                requires_confirmation: true,
                confirmation_token: Some(token),
                intent: Some(intent),
            });
        }
        return execute_intent(database, intent, false).await;
    };
    let intent = apply_confirmation_rules(intent);

    if intent.action == "none" || intent.entity == "none" || intent.confidence < 0.65 {
        return Ok(not_handled(""));
    };

    if intent.requires_confirmation {
        let token = token_for_intent(&intent);
        return Ok(ChatCommandResponse {
            reply: confirmation_message(&intent),
            actions: vec![],
            handled: true,
            requires_confirmation: true,
            confirmation_token: Some(token),
            intent: Some(intent),
        });
    }

    execute_intent(database, intent, false).await
}

fn not_handled(reply: impl Into<String>) -> ChatCommandResponse {
    ChatCommandResponse {
        reply: reply.into(),
        actions: vec![],
        handled: false,
        requires_confirmation: false,
        confirmation_token: None,
        intent: None,
    }
}

async fn execute_intent(
    database: &Database,
    intent: ChatCommandIntent,
    confirmed: bool,
) -> Result<ChatCommandResponse, String> {
    if intent.requires_confirmation && !confirmed {
        return Ok(ChatCommandResponse {
            reply: confirmation_message(&intent),
            actions: vec![],
            handled: true,
            requires_confirmation: true,
            confirmation_token: Some(token_for_intent(&intent)),
            intent: Some(intent),
        });
    }

    match (intent.entity.as_str(), intent.action.as_str()) {
        ("trader", "create") => create_trader(database, intent).await,
        ("trader", "list") => list_traders(database, intent).await,
        ("trader", "start") => set_trader_status(database, intent, "running").await,
        ("trader", "stop") => set_trader_status(database, intent, "stopped").await,
        ("trader", "pause") => set_trader_status(database, intent, "paused").await,
        ("trader", "update") => update_trader(database, intent).await,
        ("trader", "delete") => delete_trader(database, intent).await,
        ("trader", "show_status") => show_trader_status(database, intent).await,
        ("data_source", "create") => create_data_source(database, intent).await,
        ("data_source", "update") => update_data_source(database, intent).await,
        ("data_source", "list") => list_data_sources(database, intent).await,
        ("data_source", "delete") => delete_data_source(database, intent).await,
        ("data_source", "show_status") => show_data_source(database, intent).await,
        ("trader_data_source_assignment", "assign") => assign_sources(database, intent).await,
        ("trader_data_source_assignment", "unassign") => unassign_sources(database, intent).await,
        ("trader_data_source_assignment", "list") => list_trader_sources(database, intent).await,
        _ => Ok(not_handled("")),
    }
}

fn parse_command(message: &str) -> Option<ChatCommandIntent> {
    let lower = message.to_ascii_lowercase();
    if (lower.starts_with("delete data source ") || lower.starts_with("disable data source "))
        && !lower.contains("create")
    {
        let name = message
            .split("data source")
            .nth(1)
            .unwrap_or("")
            .trim()
            .trim_end_matches('.');
        let mut intent = named_intent("delete", "data_source", name);
        intent.requires_confirmation = true;
        return Some(intent);
    }
    if lower.contains("unassign ") && lower.contains(" from ") {
        let mut intent = parse_unassignment(message);
        intent.action = "unassign".to_string();
        return Some(intent);
    }
    if is_trader_update(&lower) {
        return Some(parse_update_trader(message));
    }
    if is_data_source_update(&lower) {
        return Some(parse_update_data_source(message));
    }
    if lower.contains("assign ") && lower.contains(" to ") {
        return Some(parse_assignment(message));
    }
    if is_trader_create(&lower) {
        return Some(parse_create_trader(message));
    }
    if is_data_source_create(&lower) {
        return Some(parse_create_data_source(message));
    }
    if lower.starts_with("start ") {
        return Some(named_intent(
            "start",
            "trader",
            message.trim_start_matches_word("Start").trim(),
        ));
    }
    if lower.starts_with("stop ") {
        return Some(named_intent(
            "stop",
            "trader",
            message.trim_start_matches_word("Stop").trim(),
        ));
    }
    if lower.starts_with("pause ") {
        return Some(named_intent(
            "pause",
            "trader",
            message.trim_start_matches_word("Pause").trim(),
        ));
    }
    if lower.starts_with("delete ") {
        let mut intent = named_intent(
            "delete",
            "trader",
            message.trim_start_matches_word("Delete").trim(),
        );
        intent.requires_confirmation = true;
        return Some(intent);
    }
    if lower.starts_with("is ") && lower.ends_with(" running?") {
        let name = message
            .trim_start_matches_word("Is")
            .trim()
            .trim_end_matches(" running?")
            .trim();
        return Some(named_intent("show_status", "trader", name));
    }
    if lower.contains("running traders")
        || lower.contains("show all traders")
        || lower == "show traders"
    {
        return Some(ChatCommandIntent {
            action: "list".to_string(),
            entity: "trader".to_string(),
            parameters: json!({ "status": if lower.contains("running") { "running" } else { "" } }),
            confidence: 0.88,
            requires_confirmation: false,
        });
    }
    if lower.contains("all data sources")
        || lower.contains("failed data sources")
        || lower == "show data sources"
    {
        return Some(ChatCommandIntent {
            action: "list".to_string(),
            entity: "data_source".to_string(),
            parameters: json!({ "failed_only": lower.contains("failed") }),
            confidence: 0.88,
            requires_confirmation: false,
        });
    }
    if lower.contains("sources does") && lower.contains(" use") {
        let name = message
            .split("does")
            .nth(1)
            .and_then(|value| value.split("use").next())
            .unwrap_or("")
            .trim();
        return Some(ChatCommandIntent {
            action: "list".to_string(),
            entity: "trader_data_source_assignment".to_string(),
            parameters: json!({ "trader_name": name }),
            confidence: 0.82,
            requires_confirmation: false,
        });
    }
    if lower.contains("recent items from ") {
        let name = message
            .split("from")
            .nth(1)
            .unwrap_or("")
            .trim()
            .trim_end_matches('.');
        return Some(ChatCommandIntent {
            action: "show_status".to_string(),
            entity: "data_source".to_string(),
            parameters: json!({ "name": name, "include_items": true }),
            confidence: 0.8,
            requires_confirmation: false,
        });
    }
    None
}

async fn parse_command_with_openai(message: &str) -> Option<ChatCommandIntent> {
    let api_key = env::var("CHAT_COMMAND_OPENAI_API_KEY")
        .or_else(|_| env::var("OPENAI_API_KEY"))
        .ok()?;
    let model = env::var("CHAT_COMMAND_MODEL").unwrap_or_else(|_| "gpt-5.2".to_string());
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(8))
        .build()
        .ok()?;
    let prompt = r#"Return only JSON with this exact shape:
{"action":"create|update|delete|list|start|stop|pause|assign|unassign|show_status|none","entity":"trader|data_source|trader_data_source_assignment|none","parameters":{},"confidence":0.0,"requires_confirmation":false}

Parse only app-management requests for Traders, Data Sources, and Trader/Data Source assignments.
Use trader freedom levels analyst, junior_trader, senior_trader.
Use data source types rss, web_page, manual_note, placeholder_api, python_script.
For ordinary explanatory chat, return action none and entity none.
Do not include or request API keys. If the user asks to paste or replace an API key in chat, return action none."#;
    let body = json!({
        "model": model,
        "messages": [
            { "role": "system", "content": prompt },
            { "role": "user", "content": message }
        ],
        "response_format": { "type": "json_object" }
    });
    let response: Value = client
        .post("https://api.openai.com/v1/chat/completions")
        .bearer_auth(api_key)
        .json(&body)
        .send()
        .await
        .ok()?
        .error_for_status()
        .ok()?
        .json()
        .await
        .ok()?;
    let content = response
        .get("choices")?
        .as_array()?
        .first()?
        .get("message")?
        .get("content")?
        .as_str()?;
    serde_json::from_str::<ChatCommandIntent>(content).ok()
}

fn apply_confirmation_rules(mut intent: ChatCommandIntent) -> ChatCommandIntent {
    if (intent.entity == "trader" && intent.action == "delete")
        || (intent.entity == "data_source" && intent.action == "delete")
        || (intent.entity == "trader"
            && intent.action == "update"
            && intent
                .parameters
                .get("freedom_level")
                .and_then(Value::as_str)
                == Some("senior_trader"))
    {
        intent.requires_confirmation = true;
    }
    let assignment_count = intent
        .parameters
        .get("data_source_names")
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or(0);
    if intent.entity == "trader_data_source_assignment" && assignment_count > 5 {
        intent.requires_confirmation = true;
    }
    intent
}

fn parse_create_trader(message: &str) -> ChatCommandIntent {
    let lower = message.to_ascii_lowercase();
    let name = extract_after_named(message)
        .or_else(|| extract_called(message))
        .unwrap_or_else(|| "New Trader".to_string());
    let freedom_level = if lower.contains("senior") {
        "senior_trader"
    } else if lower.contains("junior") {
        "junior_trader"
    } else {
        "analyst"
    };
    let data_source_names = extract_trader_source_names(message);
    ChatCommandIntent {
        action: "create".to_string(),
        entity: "trader".to_string(),
        parameters: json!({
            "name": name,
            "fundamental_perspective": extract_perspective(message).unwrap_or_else(|| "Cautious fundamental perspective.".to_string()),
            "freedom_level": freedom_level,
            "default_paper_account_id": null,
            "data_source_names": data_source_names
        }),
        confidence: 0.84,
        requires_confirmation: freedom_level == "senior_trader",
    }
}

fn parse_create_data_source(message: &str) -> ChatCommandIntent {
    let lower = message.to_ascii_lowercase();
    let source_type = if lower.contains("python") || lower.contains("script") {
        "python_script"
    } else if lower.contains("web page") || lower.contains("webpage") {
        "web_page"
    } else if lower.contains("manual") {
        "manual_note"
    } else if lower.contains("placeholder") {
        "placeholder_api"
    } else {
        "rss"
    };
    let name = extract_called(message)
        .or_else(|| extract_after_for(message))
        .unwrap_or_else(|| "New Data Source".to_string());
    let url = extract_url(message);
    ChatCommandIntent {
        action: "create".to_string(),
        entity: "data_source".to_string(),
        parameters: json!({
            "name": name,
            "source_type": source_type,
            "url": url,
            "enabled": true,
            "poll_interval_seconds": 30
        }),
        confidence: 0.86,
        requires_confirmation: false,
    }
}

fn parse_update_trader(message: &str) -> ChatCommandIntent {
    let lower = message.to_ascii_lowercase();
    let name = if lower.starts_with("rename trader ") {
        message
            .trim_start_matches_word("Rename trader")
            .split(" to ")
            .next()
            .unwrap_or("")
            .trim()
            .to_string()
    } else if lower.starts_with("update trader ") {
        message
            .trim_start_matches_word("Update trader")
            .split(" freedom level")
            .next()
            .unwrap_or("")
            .split(" perspective")
            .next()
            .unwrap_or("")
            .split(" to ")
            .next()
            .unwrap_or("")
            .trim()
            .to_string()
    } else {
        message
            .trim_start_matches_word("Change")
            .split(" freedom level")
            .next()
            .unwrap_or("")
            .split(" perspective")
            .next()
            .unwrap_or("")
            .trim()
            .to_string()
    };
    let new_name = if lower.starts_with("rename trader ") {
        message.split(" to ").nth(1).map(clean_name)
    } else {
        None
    };
    let freedom_level = if lower.contains("senior") {
        Some("senior_trader")
    } else if lower.contains("junior") {
        Some("junior_trader")
    } else if lower.contains("analyst") {
        Some("analyst")
    } else {
        None
    };
    let perspective = message
        .split("perspective to")
        .nth(1)
        .map(clean_name)
        .filter(|value| !value.is_empty());
    ChatCommandIntent {
        action: "update".to_string(),
        entity: "trader".to_string(),
        parameters: json!({
            "name": name,
            "new_name": new_name,
            "freedom_level": freedom_level,
            "fundamental_perspective": perspective
        }),
        confidence: 0.78,
        requires_confirmation: freedom_level == Some("senior_trader"),
    }
}

fn parse_update_data_source(message: &str) -> ChatCommandIntent {
    let lower = message.to_ascii_lowercase();
    let name = message
        .trim_start_matches_word("Update data source")
        .trim_start_matches_word("Change data source")
        .split(" url")
        .next()
        .unwrap_or("")
        .split(" URL")
        .next()
        .unwrap_or("")
        .split(" to ")
        .next()
        .unwrap_or("")
        .trim()
        .to_string();
    let source_type = if lower.contains(" to python") || lower.contains(" to script") {
        Some("python_script")
    } else if lower.contains(" to rss") {
        Some("rss")
    } else if lower.contains(" to web page") || lower.contains(" to webpage") {
        Some("web_page")
    } else if lower.contains(" to manual") {
        Some("manual_note")
    } else if lower.contains(" to placeholder") {
        Some("placeholder_api")
    } else {
        None
    };
    ChatCommandIntent {
        action: "update".to_string(),
        entity: "data_source".to_string(),
        parameters: json!({
            "name": name,
            "url": extract_url(message),
            "source_type": source_type
        }),
        confidence: 0.76,
        requires_confirmation: false,
    }
}

fn parse_assignment(message: &str) -> ChatCommandIntent {
    let without_assign = message.trim_start_matches_word("Assign").trim();
    let mut parts = without_assign.splitn(2, " to ");
    let sources = parts.next().unwrap_or("");
    let trader = parts.next().unwrap_or("").trim().trim_end_matches('.');
    ChatCommandIntent {
        action: "assign".to_string(),
        entity: "trader_data_source_assignment".to_string(),
        parameters: json!({
            "trader_name": trader,
            "data_source_names": sources.split(" and ").flat_map(|part| part.split(',')).map(|value| value.trim()).filter(|value| !value.is_empty()).collect::<Vec<_>>()
        }),
        confidence: 0.86,
        requires_confirmation: false,
    }
}

fn parse_unassignment(message: &str) -> ChatCommandIntent {
    let without = message.trim_start_matches_word("Unassign").trim();
    let mut parts = without.splitn(2, " from ");
    let sources = parts.next().unwrap_or("");
    let trader = parts.next().unwrap_or("").trim().trim_end_matches('.');
    ChatCommandIntent {
        action: "unassign".to_string(),
        entity: "trader_data_source_assignment".to_string(),
        parameters: json!({
            "trader_name": trader,
            "data_source_names": sources.split(" and ").flat_map(|part| part.split(',')).map(|value| value.trim()).filter(|value| !value.is_empty()).collect::<Vec<_>>()
        }),
        confidence: 0.84,
        requires_confirmation: false,
    }
}

fn named_intent(action: &str, entity: &str, name: &str) -> ChatCommandIntent {
    ChatCommandIntent {
        action: action.to_string(),
        entity: entity.to_string(),
        parameters: json!({ "name": name.trim().trim_end_matches('.') }),
        confidence: 0.84,
        requires_confirmation: false,
    }
}

async fn create_trader(
    database: &Database,
    intent: ChatCommandIntent,
) -> Result<ChatCommandResponse, String> {
    let p = &intent.parameters;
    let name = required_string(p, "name")?;
    let freedom_level = p
        .get("freedom_level")
        .and_then(Value::as_str)
        .unwrap_or("analyst");
    let existing = list_traders_by_name(database, &name).await?;
    if existing.len() == 1 {
        let trader = existing.into_iter().next().expect("one trader");
        let assigned_count = assign_sources_from_parameters(database, &trader.id, p).await?;
        let source_note = if assigned_count > 0 {
            format!("\nAssigned {assigned_count} data source(s).")
        } else {
            String::new()
        };
        return Ok(response(
            format!(
                "Trader {} already exists, so I did not create another one.{}",
                trader.name, source_note
            ),
            "trader_exists",
            Some(trader.id),
            intent,
        ));
    }
    if existing.len() > 1 {
        return Ok(response(
            format!(
                "I found {} traders named {} and did not create another duplicate. Please rename or delete duplicates before using chat creation for that name.",
                existing.len(),
                clean_entity_name(&name)
            ),
            "trader_duplicate_blocked",
            None,
            intent,
        ));
    }
    let key = env::var("CHAT_DEFAULT_OPENAI_API_KEY")
        .or_else(|_| env::var("OPENAI_API_KEY"))
        .unwrap_or_else(|_| "missing-key-add-in-trader-form".to_string());
    let trader = traders::create_trader(
        database,
        CreateTraderRequest {
            name: name.clone(),
            fundamental_perspective: p
                .get("fundamental_perspective")
                .and_then(Value::as_str)
                .unwrap_or("Cautious fundamental perspective.")
                .to_string(),
            freedom_level: freedom_level.to_string(),
            default_paper_account_id: None,
            openai_api_key: key,
            info_sources: vec![],
        },
    )
    .await
    .map_err(|err| err.message)?;
    let assigned_count = assign_sources_from_parameters(database, &trader.id, p).await?;
    let key_note =
        if env::var("CHAT_DEFAULT_OPENAI_API_KEY").is_ok() || env::var("OPENAI_API_KEY").is_ok() {
            ""
        } else {
            "\nAdd an OpenAI API key in the Trader form before starting engine-backed evaluations."
        };
    let source_note = if assigned_count > 0 {
        format!("\nAssigned {assigned_count} data source(s).")
    } else {
        String::new()
    };
    Ok(response(
        format!(
            "Created Trader\nName: {}\nFreedom Level: {}\nStatus: {}{}{}",
            trader.name, trader.freedom_level, trader.status, source_note, key_note
        ),
        "trader_created",
        Some(trader.id),
        intent,
    ))
}

async fn list_traders(
    database: &Database,
    intent: ChatCommandIntent,
) -> Result<ChatCommandResponse, String> {
    let status = intent
        .parameters
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("");
    let traders = traders::list_traders(database)
        .await
        .map_err(|err| err.message)?
        .into_iter()
        .filter(|trader| status.is_empty() || trader.status == status)
        .collect::<Vec<_>>();
    let reply = if traders.is_empty() {
        "No matching traders found.".to_string()
    } else {
        traders
            .iter()
            .map(|trader| {
                format!(
                    "{} - {} - {}",
                    trader.name, trader.freedom_level, trader.status
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    Ok(response(reply, "traders_listed", None, intent))
}

async fn assign_sources_from_parameters(
    database: &Database,
    trader_id: &str,
    parameters: &Value,
) -> Result<usize, String> {
    let names = parameters
        .get("data_source_names")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if names.is_empty() {
        return Ok(0);
    }

    let mut source_ids = database
        .list_trader_data_sources(trader_id)
        .await
        .map_err(|err| err.to_string())?
        .into_iter()
        .map(|source| source.id)
        .collect::<Vec<_>>();
    let mut assigned = 0;
    for value in names {
        let name = value.as_str().unwrap_or("");
        let source = find_data_source_by_name(database, name).await?;
        if !source_ids.contains(&source.id) {
            source_ids.push(source.id);
            assigned += 1;
        }
    }
    data_sources::replace_trader_sources(
        database,
        trader_id,
        UpdateTraderDataSourcesRequest {
            data_source_ids: source_ids,
        },
    )
    .await
    .map_err(|err| err.message)?;
    Ok(assigned)
}

async fn set_trader_status(
    database: &Database,
    intent: ChatCommandIntent,
    status: &str,
) -> Result<ChatCommandResponse, String> {
    let name = required_string(&intent.parameters, "name")?;
    let trader = find_trader_by_name(database, &name).await?;
    if status == "running"
        && trader.freedom_level == "senior_trader"
        && !intent.requires_confirmation
    {
        let mut next = intent.clone();
        next.requires_confirmation = true;
        return Ok(ChatCommandResponse {
            reply: confirmation_message(&next),
            actions: vec![],
            handled: true,
            requires_confirmation: true,
            confirmation_token: Some(token_for_intent(&next)),
            intent: Some(next),
        });
    }
    let updated = traders::set_status(database, &trader.id, status)
        .await
        .map_err(|err| err.message)?;
    Ok(response(
        format!("{} is now {}.", updated.name, updated.status),
        "trader_status_updated",
        Some(updated.id),
        intent,
    ))
}

async fn delete_trader(
    database: &Database,
    intent: ChatCommandIntent,
) -> Result<ChatCommandResponse, String> {
    let name = required_string(&intent.parameters, "name")?;
    let trader = find_trader_by_name(database, &name).await?;
    traders::delete_trader(database, &trader.id)
        .await
        .map_err(|err| err.message)?;
    Ok(response(
        format!(
            "Deleted trader {}. It has been stopped and removed from active use.",
            trader.name
        ),
        "trader_deleted",
        Some(trader.id),
        intent,
    ))
}

async fn update_trader(
    database: &Database,
    intent: ChatCommandIntent,
) -> Result<ChatCommandResponse, String> {
    let name = required_string(&intent.parameters, "name")?;
    let trader = find_trader_by_name(database, &name).await?;
    let freedom_level = intent
        .parameters
        .get("freedom_level")
        .and_then(Value::as_str)
        .map(str::to_string);
    let updated = traders::update_trader(
        database,
        &trader.id,
        UpdateTraderRequest {
            name: intent
                .parameters
                .get("new_name")
                .and_then(Value::as_str)
                .map(str::to_string),
            fundamental_perspective: intent
                .parameters
                .get("fundamental_perspective")
                .and_then(Value::as_str)
                .map(str::to_string),
            freedom_level,
            default_paper_account_id: None,
            openai_api_key: None,
            info_sources: None,
        },
    )
    .await
    .map_err(|err| err.message)?;
    Ok(response(
        format!(
            "Updated trader {}.\nFreedom Level: {}\nStatus: {}",
            updated.name, updated.freedom_level, updated.status
        ),
        "trader_updated",
        Some(updated.id),
        intent,
    ))
}

async fn show_trader_status(
    database: &Database,
    intent: ChatCommandIntent,
) -> Result<ChatCommandResponse, String> {
    let trader =
        find_trader_by_name(database, &required_string(&intent.parameters, "name")?).await?;
    let detail = traders::get_trader_detail(database, &trader.id)
        .await
        .map_err(|err| err.message)?;
    let recent = detail
        .recent_events
        .iter()
        .take(5)
        .map(|event| format!("{} - {}", event.event_type, event.message))
        .collect::<Vec<_>>()
        .join("\n");
    Ok(response(
        format!(
            "{} is {}.\nFreedom: {}\nCurrent task: {}{}{}",
            detail.trader.name,
            detail.trader.status,
            detail.trader.freedom_level,
            detail
                .runtime_state
                .as_ref()
                .and_then(|state| state.current_task.as_deref())
                .unwrap_or("none"),
            if recent.is_empty() {
                ""
            } else {
                "\nRecent events:\n"
            },
            recent
        ),
        "trader_status_shown",
        Some(detail.trader.id),
        intent,
    ))
}

async fn create_data_source(
    database: &Database,
    intent: ChatCommandIntent,
) -> Result<ChatCommandResponse, String> {
    let p = &intent.parameters;
    let source = data_sources::create(
        database,
        CreateDataSourceRequest {
            name: required_string(p, "name")?,
            source_type: required_string(p, "source_type")?,
            url: p.get("url").and_then(Value::as_str).map(str::to_string),
            config_json: p
                .get("config_json")
                .and_then(Value::as_str)
                .map(str::to_string),
            enabled: p.get("enabled").and_then(Value::as_bool).unwrap_or(true),
            poll_interval_seconds: p.get("poll_interval_seconds").and_then(Value::as_i64),
        },
    )
    .await
    .map_err(|err| err.message)?;
    Ok(response(
        format!(
            "Created Data Source\nName: {}\nType: {}\nPolling: every {} seconds",
            source.name, source.source_type, source.poll_interval_seconds
        ),
        "data_source_created",
        Some(source.id),
        intent,
    ))
}

async fn list_data_sources(
    database: &Database,
    intent: ChatCommandIntent,
) -> Result<ChatCommandResponse, String> {
    let failed_only = intent
        .parameters
        .get("failed_only")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let sources = data_sources::list(database)
        .await
        .map_err(|err| err.message)?
        .into_iter()
        .filter(|source| !failed_only || source.last_error.is_some())
        .collect::<Vec<_>>();
    let reply = if sources.is_empty() {
        "No matching data sources found.".to_string()
    } else {
        sources
            .iter()
            .map(|source| {
                format!(
                    "{} - {} - {} - checked {}",
                    source.name,
                    source.source_type,
                    if source.enabled {
                        "enabled"
                    } else {
                        "disabled"
                    },
                    source.last_checked_at.as_deref().unwrap_or("never")
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    Ok(response(reply, "data_sources_listed", None, intent))
}

async fn update_data_source(
    database: &Database,
    intent: ChatCommandIntent,
) -> Result<ChatCommandResponse, String> {
    let name = required_string(&intent.parameters, "name")?;
    let source = find_data_source_by_name(database, &name).await?;
    let updated = data_sources::update(
        database,
        &source.id,
        UpdateDataSourceRequest {
            name: None,
            source_type: intent
                .parameters
                .get("source_type")
                .and_then(Value::as_str)
                .map(str::to_string),
            url: intent
                .parameters
                .get("url")
                .and_then(Value::as_str)
                .map(str::to_string),
            config_json: None,
            enabled: None,
            poll_interval_seconds: None,
        },
    )
    .await
    .map_err(|err| err.message)?;
    Ok(response(
        format!(
            "Updated data source {}.\nType: {}\nURL: {}",
            updated.name,
            updated.source_type,
            updated.url.as_deref().unwrap_or("none")
        ),
        "data_source_updated",
        Some(updated.id),
        intent,
    ))
}

async fn delete_data_source(
    database: &Database,
    intent: ChatCommandIntent,
) -> Result<ChatCommandResponse, String> {
    let name = required_string(&intent.parameters, "name")?;
    let source = find_data_source_by_name(database, &name).await?;
    data_sources::delete(database, &source.id)
        .await
        .map_err(|err| err.message)?;
    Ok(response(
        format!(
            "Disabled data source {}. Existing items and events were preserved.",
            source.name
        ),
        "data_source_disabled",
        Some(source.id),
        intent,
    ))
}

async fn show_data_source(
    database: &Database,
    intent: ChatCommandIntent,
) -> Result<ChatCommandResponse, String> {
    let source =
        find_data_source_by_name(database, &required_string(&intent.parameters, "name")?).await?;
    let items = data_sources::items(database, &source.id)
        .await
        .map_err(|err| err.message)?
        .items;
    let item_lines = items
        .iter()
        .take(5)
        .map(|item| format!("{} - {}", item.title, item.discovered_at))
        .collect::<Vec<_>>()
        .join("\n");
    Ok(response(
        format!(
            "{} is {}. Last checked: {}. Last error: {}{}{}",
            source.name,
            if source.enabled {
                "enabled"
            } else {
                "disabled"
            },
            source.last_checked_at.as_deref().unwrap_or("never"),
            source.last_error.as_deref().unwrap_or("none"),
            if item_lines.is_empty() {
                ""
            } else {
                "\nRecent items:\n"
            },
            item_lines
        ),
        "data_source_status_shown",
        Some(source.id),
        intent,
    ))
}

async fn assign_sources(
    database: &Database,
    intent: ChatCommandIntent,
) -> Result<ChatCommandResponse, String> {
    let trader = find_trader_by_name(
        database,
        &required_string(&intent.parameters, "trader_name")?,
    )
    .await?;
    let names = intent
        .parameters
        .get("data_source_names")
        .and_then(Value::as_array)
        .ok_or_else(|| "data_source_names is required".to_string())?;
    let mut source_ids = database
        .list_trader_data_sources(&trader.id)
        .await
        .map_err(|err| err.to_string())?
        .into_iter()
        .map(|source| source.id)
        .collect::<Vec<_>>();
    for value in names {
        let name = value.as_str().unwrap_or("");
        let source = find_data_source_by_name(database, name).await?;
        if !source_ids.contains(&source.id) {
            source_ids.push(source.id);
        }
    }
    data_sources::replace_trader_sources(
        database,
        &trader.id,
        UpdateTraderDataSourcesRequest {
            data_source_ids: source_ids,
        },
    )
    .await
    .map_err(|err| err.message)?;
    Ok(response(
        format!(
            "Assigned {} data source(s) to {}.",
            names.len(),
            trader.name
        ),
        "trader_data_sources_assigned",
        Some(trader.id),
        intent,
    ))
}

async fn unassign_sources(
    database: &Database,
    intent: ChatCommandIntent,
) -> Result<ChatCommandResponse, String> {
    let trader = find_trader_by_name(
        database,
        &required_string(&intent.parameters, "trader_name")?,
    )
    .await?;
    let names = intent
        .parameters
        .get("data_source_names")
        .and_then(Value::as_array)
        .ok_or_else(|| "data_source_names is required".to_string())?;
    let mut source_ids = database
        .list_trader_data_sources(&trader.id)
        .await
        .map_err(|err| err.to_string())?
        .into_iter()
        .map(|source| source.id)
        .collect::<Vec<_>>();
    for value in names {
        let name = value.as_str().unwrap_or("");
        let source = find_data_source_by_name(database, name).await?;
        source_ids.retain(|id| id != &source.id);
    }
    data_sources::replace_trader_sources(
        database,
        &trader.id,
        UpdateTraderDataSourcesRequest {
            data_source_ids: source_ids,
        },
    )
    .await
    .map_err(|err| err.message)?;
    Ok(response(
        format!(
            "Unassigned {} data source(s) from {}.",
            names.len(),
            trader.name
        ),
        "trader_data_sources_unassigned",
        Some(trader.id),
        intent,
    ))
}

async fn list_trader_sources(
    database: &Database,
    intent: ChatCommandIntent,
) -> Result<ChatCommandResponse, String> {
    let trader = find_trader_by_name(
        database,
        &required_string(&intent.parameters, "trader_name")?,
    )
    .await?;
    let sources = data_sources::trader_sources(database, &trader.id)
        .await
        .map_err(|err| err.message)?
        .data_sources;
    let reply = if sources.is_empty() {
        format!("{} has no assigned data sources.", trader.name)
    } else {
        format!(
            "{} uses:\n{}",
            trader.name,
            sources
                .iter()
                .map(|source| format!("{} - {}", source.name, source.source_type))
                .collect::<Vec<_>>()
                .join("\n")
        )
    };
    Ok(response(
        reply,
        "trader_data_sources_listed",
        Some(trader.id),
        intent,
    ))
}

async fn find_trader_by_name(
    database: &Database,
    name: &str,
) -> Result<models::trader::Trader, String> {
    let normalized_name = clean_entity_name(name);
    let matches = list_traders_by_name(database, &normalized_name).await?;
    match matches.len() {
        0 => Err(format!("No trader named {normalized_name} was found.")),
        1 => Ok(matches.into_iter().next().expect("one match")),
        _ => Err(format!(
            "Multiple traders named {normalized_name} were found. Please clarify."
        )),
    }
}

async fn list_traders_by_name(
    database: &Database,
    name: &str,
) -> Result<Vec<models::trader::Trader>, String> {
    let normalized_name = clean_entity_name(name);
    traders::list_traders(database)
        .await
        .map_err(|err| err.message)
        .map(|traders| {
            traders
                .into_iter()
                .filter(|trader| trader.name.eq_ignore_ascii_case(normalized_name.trim()))
                .collect::<Vec<_>>()
        })
}

async fn find_data_source_by_name(
    database: &Database,
    name: &str,
) -> Result<models::data_sources::DataSource, String> {
    let normalized_name = clean_entity_name(name);
    let matches = data_sources::list(database)
        .await
        .map_err(|err| err.message)?
        .into_iter()
        .filter(|source| source.name.eq_ignore_ascii_case(normalized_name.trim()))
        .collect::<Vec<_>>();
    match matches.len() {
        0 => Err(format!("No data source named {normalized_name} was found.")),
        1 => Ok(matches.into_iter().next().expect("one match")),
        _ => Err(format!(
            "Multiple data sources named {normalized_name} were found. Please clarify."
        )),
    }
}

fn response(
    reply: String,
    action_type: &str,
    entity_id: Option<String>,
    intent: ChatCommandIntent,
) -> ChatCommandResponse {
    ChatCommandResponse {
        reply,
        actions: vec![ChatCommandAction {
            r#type: action_type.to_string(),
            entity_id,
            message: None,
        }],
        handled: true,
        requires_confirmation: false,
        confirmation_token: None,
        intent: Some(intent),
    }
}

fn required_string(parameters: &Value, key: &str) -> Result<String, String> {
    parameters
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .ok_or_else(|| format!("{key} is required"))
}

fn is_trader_create(lower: &str) -> bool {
    (lower.contains("trader named ")
        || lower.contains("trader called ")
        || lower.contains("analyst trader")
        || lower.contains("junior trader")
        || lower.contains("senior trader"))
        && (lower.contains("create") || lower.contains("add") || lower.contains("make"))
}

fn is_trader_update(lower: &str) -> bool {
    (lower.starts_with("update trader ")
        || lower.starts_with("rename trader ")
        || lower.starts_with("change "))
        && (lower.contains("freedom level")
            || lower.contains(" perspective")
            || lower.starts_with("rename trader "))
}

fn is_data_source_update(lower: &str) -> bool {
    (lower.starts_with("update data source ") || lower.starts_with("change data source "))
        && (lower.contains(" url")
            || lower.contains(" source type")
            || lower.contains(" to rss")
            || lower.contains(" to web page")
            || lower.contains(" to webpage")
            || lower.contains(" to manual")
            || lower.contains(" to placeholder"))
}

fn is_data_source_create(lower: &str) -> bool {
    lower.contains("data source")
        || lower.contains("rss data source")
        || lower.contains("web page source")
        || lower.contains("rss source")
}

fn extract_url(message: &str) -> Option<String> {
    message
        .split_whitespace()
        .find(|part| part.starts_with("http://") || part.starts_with("https://"))
        .map(|part| part.trim_end_matches('.').to_string())
}

fn clean_name(value: &str) -> String {
    value.trim().trim_end_matches('.').to_string()
}

fn extract_called(message: &str) -> Option<String> {
    message
        .split(" called ")
        .nth(1)
        .map(|value| value.split(" using ").next().unwrap_or(value))
        .map(|value| value.split(" that ").next().unwrap_or(value))
        .map(|value| value.split(" with ").next().unwrap_or(value))
        .map(|value| value.split(" is ").next().unwrap_or(value))
        .map(|value| value.trim().trim_end_matches('.').to_string())
        .filter(|value| !value.is_empty())
}

fn extract_after_named(message: &str) -> Option<String> {
    message
        .split(" named ")
        .nth(1)
        .map(|value| {
            value
                .split(" with ")
                .next()
                .unwrap_or(value)
                .split(" focused ")
                .next()
                .unwrap_or(value)
        })
        .map(|value| value.trim().trim_end_matches('.').to_string())
        .filter(|value| !value.is_empty())
}

fn extract_after_for(message: &str) -> Option<String> {
    message
        .split(" for ")
        .nth(1)
        .map(|value| value.trim().trim_end_matches('.').to_string())
        .filter(|value| !value.is_empty())
}

fn extract_perspective(message: &str) -> Option<String> {
    let lower = message.to_ascii_lowercase();
    if lower.contains("goal is") || lower.contains("goal ") {
        return message
            .split("goal is")
            .nth(1)
            .or_else(|| message.split("goal").nth(1))
            .map(|value| value.trim().trim_start_matches("is").trim())
            .map(|value| value.trim().trim_end_matches('.').to_string())
            .filter(|value| !value.is_empty());
    }
    if lower.contains("its goal is") {
        return message
            .split("Its goal is")
            .nth(1)
            .or_else(|| message.split("its goal is").nth(1))
            .map(|value| value.trim().trim_end_matches('.').to_string())
            .filter(|value| !value.is_empty());
    }
    if lower.contains("cautious") && lower.contains("macro") {
        return Some("Cautious macro-focused fundamental perspective.".to_string());
    }
    message
        .split("focused on")
        .nth(1)
        .map(|value| value.trim().trim_end_matches('.').to_string())
}

fn extract_trader_source_names(message: &str) -> Vec<String> {
    let Some(after_using) = message.split(" using ").nth(1) else {
        return vec![];
    };
    let source_text = after_using
        .split(".")
        .next()
        .unwrap_or(after_using)
        .trim()
        .trim_end_matches(" data sources")
        .trim_end_matches(" data source")
        .trim_end_matches(" sources")
        .trim_end_matches(" source")
        .trim();
    source_text
        .split(" and ")
        .flat_map(|part| part.split(','))
        .map(clean_entity_name)
        .filter(|value| !value.is_empty())
        .collect()
}

fn clean_entity_name(value: &str) -> String {
    clean_name(value)
        .trim_start_matches("the ")
        .trim_start_matches("The ")
        .trim_start_matches("a ")
        .trim_start_matches("A ")
        .trim()
        .to_string()
}

fn confirmation_message(intent: &ChatCommandIntent) -> String {
    match (intent.entity.as_str(), intent.action.as_str()) {
        ("trader", "delete") => format!(
            "Confirm deletion of trader {}? This will stop the trader and remove it from active use.",
            intent
                .parameters
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or("")
        ),
        ("trader", "start") => {
            "Confirm starting this senior trader? It can submit paper orders through risk controls."
                .to_string()
        }
        ("data_source", "delete") => format!(
            "Confirm disabling data source {}? Existing items/events will be preserved.",
            intent
                .parameters
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or("")
        ),
        _ => "Please confirm this command.".to_string(),
    }
}

fn token_for_intent(intent: &ChatCommandIntent) -> String {
    let payload = serde_json::to_string(intent).unwrap_or_default();
    let mut hasher = DefaultHasher::new();
    payload.hash(&mut hasher);
    format!("{}:{}", hasher.finish(), payload)
}

fn intent_from_token(token: &str) -> Option<ChatCommandIntent> {
    let payload = token.split_once(':')?.1;
    serde_json::from_str(payload).ok()
}

trait TrimStartWord {
    fn trim_start_matches_word<'a>(&'a self, word: &str) -> &'a str;
}

impl TrimStartWord for str {
    fn trim_start_matches_word<'a>(&'a self, word: &str) -> &'a str {
        self.strip_prefix(word)
            .or_else(|| self.strip_prefix(&word.to_ascii_lowercase()))
            .unwrap_or(self)
    }
}
