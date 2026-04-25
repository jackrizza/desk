use std::{env, time::Duration};

use database::Database;
use models::{
    channels::{
        AgentChatMessage, DataScientistChatAction, DataScientistChatRequest,
        DataScientistChatResponse, MdChatRequest, MdChatResponse,
    },
    data_sources::{
        BuildDataSourceScriptRequest, CreateDataSourceRequest, UpdateDataSourceScriptRequest,
    },
};
use serde_json::{Value, json};

use crate::data_sources;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentChatErrorKind {
    BadRequest,
    Internal,
}

#[derive(Debug)]
pub struct AgentChatError {
    pub kind: AgentChatErrorKind,
    pub message: String,
}

impl AgentChatError {
    fn bad_request(message: impl Into<String>) -> Self {
        Self {
            kind: AgentChatErrorKind::BadRequest,
            message: message.into(),
        }
    }

    fn internal(message: impl Into<String>) -> Self {
        Self {
            kind: AgentChatErrorKind::Internal,
            message: message.into(),
        }
    }
}

pub async fn md_chat(
    database: &Database,
    request: MdChatRequest,
) -> Result<MdChatResponse, AgentChatError> {
    let message = request.message.trim();
    if message.is_empty() {
        return Err(AgentChatError::bad_request("message must be non-empty"));
    }

    let channel_context = database.engine_channel_context().await.map_err(|err| {
        AgentChatError::internal(format!("failed to load channel context: {err}"))
    })?;
    let traders = database
        .list_traders()
        .await
        .map_err(|err| AgentChatError::internal(format!("failed to load traders: {err}")))?;
    let engine_events = database
        .list_recent_engine_events(20)
        .await
        .unwrap_or_default();

    let mut trader_event_lines = Vec::new();
    let mut proposal_lines = Vec::new();
    let mut referenced_traders = Vec::new();
    let mut referenced_events = Vec::new();
    for trader in traders.iter().take(20) {
        if trader.status == "running" {
            referenced_traders.push(trader.id.clone());
        }
        if let Ok(events) = database.list_trader_events(&trader.id, 5).await {
            for event in events {
                if event.event_type.to_lowercase().contains("risk")
                    || event.message.to_lowercase().contains("risk")
                    || trader_event_lines.len() < 20
                {
                    referenced_events.push(event.id.clone());
                    trader_event_lines.push(format!(
                        "{} {} {}: {}",
                        event.created_at, trader.name, event.event_type, event.message
                    ));
                }
            }
        }
        if let Ok(proposals) = database.list_trader_trade_proposals(&trader.id).await {
            for proposal in proposals.into_iter().take(5) {
                proposal_lines.push(format!(
                    "{} {} {} {} status={} confidence={:?}: {}",
                    trader.name,
                    proposal.side,
                    proposal.quantity,
                    proposal.symbol,
                    proposal.status,
                    proposal.confidence,
                    proposal.reason
                ));
            }
        }
    }

    let sources = database.list_data_sources().await.unwrap_or_default();
    let source_errors = sources
        .iter()
        .filter_map(|source| {
            source
                .last_error
                .as_ref()
                .map(|err| format!("{} [{}]: {}", source.name, source.source_type, err))
        })
        .take(20)
        .collect::<Vec<_>>();

    let system_prompt = format!(
        r#"You are {name}, the managing director AI inside Desk.

Persona:
{persona}

Tone:
{tone}

Communication style:
{style}

Investor profile:
{investor}

Rules:
- Monitor traders, reduce drift, ask clarifying questions, identify missing information, and summarize disagreement.
- Never place trades, submit paper orders, approve trades, or claim execution.
- Never claim a trade happened unless it appears in provided events/orders.
- If asked to trade, refuse briefly and redirect to review/monitoring.
- Base answers on the context below. If context is thin, say what is missing.

Running traders:
{running_traders}

Recent channel messages:
{channel_messages}

Recent trader events and risk blocks:
{trader_events}

Recent trader proposals:
{proposals}

Recent data source errors:
{source_errors}

Recent engine events:
{engine_events}
"#,
        name = channel_context.md_profile.name,
        persona = channel_context.md_profile.persona,
        tone = channel_context.md_profile.tone,
        style = channel_context.md_profile.communication_style,
        investor = format_investor(&channel_context.user_investor_profile),
        running_traders = traders
            .iter()
            .filter(|trader| trader.status == "running")
            .map(|trader| format!("{} ({})", trader.name, trader.freedom_level))
            .collect::<Vec<_>>()
            .join("\n")
            .if_empty("No traders are currently running."),
        channel_messages = channel_context
            .recent_messages
            .iter()
            .rev()
            .take(50)
            .map(|message| {
                format!(
                    "{} {}: {}",
                    message.created_at, message.author_name, message.content_markdown
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
            .if_empty("No recent channel messages."),
        trader_events = trader_event_lines
            .join("\n")
            .if_empty("No recent trader events."),
        proposals = proposal_lines.join("\n").if_empty("No recent proposals."),
        source_errors = source_errors
            .join("\n")
            .if_empty("No recent source errors."),
        engine_events = engine_events
            .iter()
            .map(|event| {
                format!(
                    "{} {} {} {:?}: {}",
                    event.created_at,
                    event.engine_name,
                    event.event_type,
                    event.symbol,
                    event.message
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
            .if_empty("No recent engine events."),
    );

    let reply = match select_openai_key() {
        Some(key) => {
            call_openai_chat(
                &key,
                &system_prompt,
                request.conversation.unwrap_or_default(),
                message,
                "MD_CHAT_MODEL",
            )
            .await?
        }
        None => fallback_md_reply(message, &traders, &source_errors),
    };

    Ok(MdChatResponse {
        reply,
        referenced_channels: channel_context
            .channels
            .iter()
            .map(|channel| channel.id.clone())
            .collect(),
        referenced_traders,
        referenced_events,
    })
}

pub async fn data_scientist_chat(
    database: &Database,
    request: DataScientistChatRequest,
) -> Result<DataScientistChatResponse, AgentChatError> {
    let message = request.message.trim();
    if message.is_empty() {
        return Err(AgentChatError::bad_request("message must be non-empty"));
    }

    if let Some(url) = extract_url(message) {
        return create_python_source_from_url(database, request, &url).await;
    }

    let profile = database.get_data_scientist_profile().await.map_err(|err| {
        AgentChatError::internal(format!("failed to load Data Scientist profile: {err}"))
    })?;
    let sources = database.list_data_sources().await.unwrap_or_default();
    let source_lines = sources
        .iter()
        .take(50)
        .map(|source| {
            format!(
                "{} [{}] enabled={} url={}",
                source.name,
                source.source_type,
                source.enabled,
                source.url.as_deref().unwrap_or("none")
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
        .if_empty("No data sources exist yet.");

    let system_prompt = format!(
        r#"You are {name}, the Data Scientist AI inside Desk.

Persona:
{persona}

Tone:
{tone}

Communication style:
{style}

Rules:
- Help create, debug, and improve data sources.
- Do not trade, place orders, approve trades, or ask for secrets.
- Python Script data sources must define collect(context), use context["url"] or context.get("url"), return items, and avoid shell commands, file writes, environment access, and trading endpoints.

Available data source types: rss, web_page, manual_note, placeholder_api, python_script.

Existing data sources:
{source_lines}
"#,
        name = profile.name,
        persona = profile.persona,
        tone = profile.tone,
        style = profile.communication_style,
        source_lines = source_lines,
    );

    let reply = match select_data_scientist_openai_key(database).await {
        Some(key) => {
            call_openai_chat(
                &key,
                &system_prompt,
                request.conversation.unwrap_or_default(),
                message,
                "DATA_SCIENTIST_CHAT_MODEL",
            )
            .await?
        }
        None => "I can help design and debug data sources. Send me a URL and I can create a Python Script source using the backend data-source workflow.".to_string(),
    };

    Ok(DataScientistChatResponse {
        reply,
        actions: Vec::new(),
    })
}

async fn create_python_source_from_url(
    database: &Database,
    request: DataScientistChatRequest,
    url: &str,
) -> Result<DataScientistChatResponse, AgentChatError> {
    let user_message = request.message.trim().to_string();
    let conversation = request.conversation.unwrap_or_default();
    let profile = database.get_data_scientist_profile().await.map_err(|err| {
        AgentChatError::internal(format!("failed to load Data Scientist profile: {err}"))
    })?;
    let existing_sources = database.list_data_sources().await.unwrap_or_default();
    let source_context = existing_sources
        .iter()
        .take(40)
        .map(|source| {
            format!(
                "{} [{}] {}",
                source.name,
                source.source_type,
                source.url.as_deref().unwrap_or("")
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let generated = generate_source_plan(
        select_data_scientist_openai_key(database).await,
        &profile.name,
        &profile.persona,
        conversation,
        &user_message,
        url,
        &source_context,
    )
    .await?;

    let source = data_sources::create(
        database,
        CreateDataSourceRequest {
            name: generated.name.clone(),
            source_type: "python_script".to_string(),
            url: Some(url.to_string()),
            config_json: None,
            enabled: true,
            poll_interval_seconds: Some(30),
        },
    )
    .await
    .map_err(|err| AgentChatError::internal(err.message))?;

    let script = if script_has_forbidden_behavior(&generated.script) {
        fallback_python_script(url)
    } else {
        generated.script
    };

    data_sources::update_script(
        database,
        &source.id,
        UpdateDataSourceScriptRequest {
            script_text: script.clone(),
        },
    )
    .await
    .map_err(|err| AgentChatError::internal(err.message))?;

    let mut build = data_sources::build_script(
        database,
        &source.id,
        BuildDataSourceScriptRequest { script_text: None },
    )
    .await
    .map_err(|err| AgentChatError::internal(err.message))?;

    if !build.success {
        if let Some(key) = select_data_scientist_openai_key(database).await {
            if let Ok(repaired) = repair_script(&key, url, &script, &build.output).await {
                if !script_has_forbidden_behavior(&repaired) {
                    data_sources::update_script(
                        database,
                        &source.id,
                        UpdateDataSourceScriptRequest {
                            script_text: repaired,
                        },
                    )
                    .await
                    .map_err(|err| AgentChatError::internal(err.message))?;
                    build = data_sources::build_script(
                        database,
                        &source.id,
                        BuildDataSourceScriptRequest { script_text: None },
                    )
                    .await
                    .map_err(|err| AgentChatError::internal(err.message))?;
                }
            }
        }
    }

    let build_word = if build.success { "Success" } else { "Failed" };
    let reply = format!(
        "Created Python Script data source.\n\nName: {}\nSource id: {}\nURL: {}\nBuild: {}\n\nBuild output:\n{}\n\nNext steps: open Data Sources, inspect the generated script, and run the source after confirming the extraction matches the page.",
        source.name, source.id, url, build_word, build.output
    );

    Ok(DataScientistChatResponse {
        reply,
        actions: vec![DataScientistChatAction {
            r#type: "data_source_created".to_string(),
            entity_id: Some(source.id),
            name: Some(source.name),
            source_type: Some(source.source_type),
            url: source.url,
            build_status: Some(build.status),
            build_output: Some(build.output),
        }],
    })
}

struct GeneratedSource {
    name: String,
    script: String,
}

async fn generate_source_plan(
    api_key: Option<String>,
    agent_name: &str,
    persona: &str,
    conversation: Vec<AgentChatMessage>,
    user_message: &str,
    url: &str,
    existing_sources: &str,
) -> Result<GeneratedSource, AgentChatError> {
    let Some(key) = api_key else {
        return Ok(GeneratedSource {
            name: name_from_url(url),
            script: fallback_python_script(url),
        });
    };

    let conversation_text = conversation
        .into_iter()
        .take(20)
        .filter(|entry| entry.role == "user" || entry.role == "assistant")
        .map(|entry| format!("{}: {}", entry.role, entry.content))
        .collect::<Vec<_>>()
        .join("\n");
    let prompt = format!(
        r#"{agent_name} profile:
{persona}

User asked:
{user_message}

Target URL:
{url}

Existing sources:
{existing_sources}

Conversation:
{conversation_text}

Create a Desk python_script data source plan. If web search is enabled, inspect/reason about the URL. Return only JSON with this shape:
{{"name":"short descriptive source name","script":"python code"}}

Script requirements:
- Define def collect(context):
- Use context.get("url") or context["url"] for the URL.
- Use only Python standard library unless the page is impossible without optional dependencies.
- Prefer urllib.request, urllib.parse, html.parser, json, re, hashlib, datetime.
- Return {{"items":[{{"external_id":"...","title":"...","url":"...","content":"...","summary":"...","published_at":"..."}}]}}.
- Return at most 100 items.
- Handle network and parse errors gracefully by returning {{"items":[]}}.
- Do not place trades, call trading endpoints, run shell commands, read environment variables, write files, or require secrets.
"#
    );

    let text = call_openai_text(&key, &prompt, "DATA_SCIENTIST_CHAT_MODEL").await?;
    let parsed = parse_json_object(&text).and_then(|value| {
        let name = value.get("name")?.as_str()?.trim().to_string();
        let script = value.get("script")?.as_str()?.trim().to_string();
        if name.is_empty() || script.is_empty() {
            return None;
        }
        Some(GeneratedSource { name, script })
    });

    Ok(parsed.unwrap_or_else(|| GeneratedSource {
        name: name_from_url(url),
        script: fallback_python_script(url),
    }))
}

async fn repair_script(
    api_key: &str,
    url: &str,
    script: &str,
    build_output: &str,
) -> Result<String, AgentChatError> {
    let prompt = format!(
        r#"Repair this Desk python_script data source. Return only the full Python script.

URL: {url}

Build output:
{build_output}

Current script:
```python
{script}
```

Keep the same safety rules: define collect(context), use standard library, no shell commands, no file writes, no environment access, no secrets, no trading endpoints, at most 100 items."#
    );
    let text = call_openai_text(api_key, &prompt, "DATA_SCIENTIST_CHAT_MODEL").await?;
    Ok(strip_code_fence(&text))
}

async fn call_openai_chat(
    api_key: &str,
    system_prompt: &str,
    conversation: Vec<AgentChatMessage>,
    message: &str,
    model_env: &str,
) -> Result<String, AgentChatError> {
    let mut messages = vec![json!({ "role": "system", "content": system_prompt })];
    for entry in conversation.into_iter().take(20) {
        let role = match entry.role.as_str() {
            "assistant" => "assistant",
            "user" => "user",
            _ => continue,
        };
        if !entry.content.trim().is_empty() {
            messages.push(json!({ "role": role, "content": entry.content.trim() }));
        }
    }
    messages.push(json!({ "role": "user", "content": message }));

    let model = env::var(model_env)
        .or_else(|_| env::var("CHAT_COMMAND_MODEL"))
        .unwrap_or_else(|_| "gpt-5.2".to_string());
    let body = json!({
        "model": model,
        "messages": messages
    });
    let response = openai_client()?
        .post("https://api.openai.com/v1/chat/completions")
        .bearer_auth(api_key)
        .json(&body)
        .send()
        .await
        .map_err(|err| AgentChatError::internal(format!("OpenAI request failed: {err}")))?
        .error_for_status()
        .map_err(|err| AgentChatError::internal(format!("OpenAI request failed: {err}")))?
        .json::<Value>()
        .await
        .map_err(|err| AgentChatError::internal(format!("OpenAI response was invalid: {err}")))?;

    extract_chat_completion_text(&response)
        .ok_or_else(|| AgentChatError::internal("OpenAI returned an empty response"))
}

async fn call_openai_text(
    api_key: &str,
    prompt: &str,
    model_env: &str,
) -> Result<String, AgentChatError> {
    let model = env::var(model_env)
        .or_else(|_| env::var("CHAT_COMMAND_MODEL"))
        .unwrap_or_else(|_| "gpt-5.2".to_string());
    let web_search_enabled = env::var("DATA_SCIENTIST_OPENAI_WEB_SEARCH")
        .map(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes"))
        .unwrap_or(false);
    let client = openai_client()?;

    if web_search_enabled {
        let body = json!({
            "model": model,
            "tools": [{ "type": "web_search_preview" }],
            "input": prompt
        });
        let response = client
            .post("https://api.openai.com/v1/responses")
            .bearer_auth(api_key)
            .json(&body)
            .send()
            .await
            .map_err(|err| {
                AgentChatError::internal(format!("OpenAI web search request failed: {err}"))
            })?
            .error_for_status()
            .map_err(|err| {
                AgentChatError::internal(format!("OpenAI web search request failed: {err}"))
            })?
            .json::<Value>()
            .await
            .map_err(|err| {
                AgentChatError::internal(format!("OpenAI response was invalid: {err}"))
            })?;
        if let Some(text) = extract_responses_text(&response) {
            return Ok(text);
        }
    }

    let body = json!({
        "model": model,
        "messages": [
            { "role": "system", "content": "You create safe Desk data-source scripts and return exactly what the user requested." },
            { "role": "user", "content": prompt }
        ]
    });
    let response = client
        .post("https://api.openai.com/v1/chat/completions")
        .bearer_auth(api_key)
        .json(&body)
        .send()
        .await
        .map_err(|err| AgentChatError::internal(format!("OpenAI request failed: {err}")))?
        .error_for_status()
        .map_err(|err| AgentChatError::internal(format!("OpenAI request failed: {err}")))?
        .json::<Value>()
        .await
        .map_err(|err| AgentChatError::internal(format!("OpenAI response was invalid: {err}")))?;

    extract_chat_completion_text(&response)
        .ok_or_else(|| AgentChatError::internal("OpenAI returned an empty response"))
}

fn openai_client() -> Result<reqwest::Client, AgentChatError> {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(45))
        .build()
        .map_err(|err| AgentChatError::internal(format!("failed to build OpenAI client: {err}")))
}

fn select_openai_key() -> Option<String> {
    env::var("CHAT_DEFAULT_OPENAI_API_KEY")
        .ok()
        .or_else(|| env::var("OPENAI_API_KEY").ok())
        .map(|key| key.trim().to_string())
        .filter(|key| !key.is_empty())
}

async fn select_data_scientist_openai_key(database: &Database) -> Option<String> {
    database
        .get_data_scientist_openai_api_key()
        .await
        .ok()
        .flatten()
        .map(|key| key.trim().to_string())
        .filter(|key| !key.is_empty())
        .or_else(select_openai_key)
}

fn extract_chat_completion_text(value: &Value) -> Option<String> {
    value
        .get("choices")
        .and_then(Value::as_array)
        .and_then(|choices| choices.first())
        .and_then(|choice| choice.get("message"))
        .and_then(|message| message.get("content"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(str::to_string)
}

fn extract_responses_text(value: &Value) -> Option<String> {
    if let Some(text) = value.get("output_text").and_then(Value::as_str) {
        if !text.trim().is_empty() {
            return Some(text.trim().to_string());
        }
    }
    value
        .get("output")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .flat_map(|item| {
                    item.get("content")
                        .and_then(Value::as_array)
                        .cloned()
                        .unwrap_or_default()
                })
                .filter_map(|content| {
                    content
                        .get("text")
                        .and_then(Value::as_str)
                        .map(str::to_string)
                })
                .collect::<Vec<_>>()
                .join("\n")
        })
        .map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty())
}

fn parse_json_object(text: &str) -> Option<Value> {
    serde_json::from_str::<Value>(text).ok().or_else(|| {
        let start = text.find('{')?;
        let end = text.rfind('}')?;
        serde_json::from_str::<Value>(&text[start..=end]).ok()
    })
}

fn extract_url(message: &str) -> Option<String> {
    message.split_whitespace().find_map(|part| {
        let trimmed = part
            .trim_matches(|ch: char| matches!(ch, '"' | '\'' | '<' | '>' | ')' | '(' | ',' | '.'));
        if trimmed.starts_with("https://") || trimmed.starts_with("http://") {
            Some(trimmed.to_string())
        } else {
            None
        }
    })
}

fn name_from_url(url: &str) -> String {
    let without_scheme = url
        .trim_start_matches("https://")
        .trim_start_matches("http://");
    let host = without_scheme.split('/').next().unwrap_or("URL");
    let path = without_scheme
        .split('/')
        .skip(1)
        .find(|part| !part.trim().is_empty())
        .unwrap_or("feed");
    format!(
        "{} {}",
        title_word(host.split('.').next().unwrap_or(host)),
        title_word(path)
    )
}

fn title_word(value: &str) -> String {
    let cleaned = value
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { ' ' })
        .collect::<String>();
    let mut chars = cleaned.trim().chars();
    match chars.next() {
        Some(first) => format!("{}{}", first.to_ascii_uppercase(), chars.as_str()),
        None => "Source".to_string(),
    }
}

fn script_has_forbidden_behavior(script: &str) -> bool {
    let lower = script.to_lowercase();
    [
        "subprocess",
        "os.system",
        "os.environ",
        "import os",
        "import requests",
        "open(",
        "paper/orders",
        "/trades",
        "/orders",
        "createpaperorder",
    ]
    .iter()
    .any(|pattern| lower.contains(pattern))
}

fn strip_code_fence(text: &str) -> String {
    let trimmed = text.trim();
    if !trimmed.starts_with("```") {
        return trimmed.to_string();
    }
    trimmed
        .lines()
        .skip(1)
        .take_while(|line| !line.trim_start().starts_with("```"))
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

fn fallback_python_script(_url: &str) -> String {
    r#"from html.parser import HTMLParser
from urllib.parse import urljoin
from urllib.request import Request, urlopen
import hashlib
import re


class LinkTextParser(HTMLParser):
    def __init__(self, base_url):
        super().__init__()
        self.base_url = base_url
        self.items = []
        self._href = None
        self._text = []

    def handle_starttag(self, tag, attrs):
        if tag.lower() == "a":
            attrs_dict = dict(attrs)
            href = attrs_dict.get("href")
            if href:
                self._href = urljoin(self.base_url, href)
                self._text = []

    def handle_data(self, data):
        if self._href:
            self._text.append(data)

    def handle_endtag(self, tag):
        if tag.lower() == "a" and self._href:
            title = re.sub(r"\s+", " ", " ".join(self._text)).strip()
            if title and len(title) > 8:
                self.items.append({"title": title, "url": self._href})
            self._href = None
            self._text = []


def _external_id(url, title):
    return hashlib.sha256((url + "|" + title).encode("utf-8")).hexdigest()[:32]


def collect(context):
    url = context.get("url")
    if not url:
        return {"items": []}

    try:
        req = Request(url, headers={"User-Agent": "DeskScrapper/1.0"})
        with urlopen(req, timeout=15) as resp:
            html = resp.read().decode("utf-8", errors="replace")
    except Exception:
        return {"items": []}

    parser = LinkTextParser(url)
    try:
        parser.feed(html)
    except Exception:
        return {"items": []}

    seen = set()
    items = []
    for entry in parser.items:
        item_url = entry["url"]
        title = entry["title"]
        key = _external_id(item_url, title)
        if key in seen:
            continue
        seen.add(key)
        items.append({
            "external_id": key,
            "title": title,
            "url": item_url,
            "content": title,
            "summary": title,
            "published_at": None,
        })
        if len(items) >= 100:
            break

    return {"items": items}
"#
    .to_string()
}

fn fallback_md_reply(
    message: &str,
    traders: &[models::trader::Trader],
    source_errors: &[String],
) -> String {
    let running = traders
        .iter()
        .filter(|trader| trader.status == "running")
        .map(|trader| trader.name.clone())
        .collect::<Vec<_>>();
    let mut reply = format!(
        "I can monitor drift and coordination, but I cannot place trades. I see {} running trader(s): {}.",
        running.len(),
        if running.is_empty() {
            "none".to_string()
        } else {
            running.join(", ")
        }
    );
    if !source_errors.is_empty() {
        reply.push_str("\n\nRecent data source errors may be affecting trader context:\n");
        reply.push_str(&source_errors.join("\n"));
    }
    if message.to_lowercase().contains("drift") {
        reply.push_str("\n\nTo assess drift more rigorously, I would compare each trader's latest proposal and runtime task against its stated perspective, symbol universe, and recent source evidence.");
    }
    reply
}

fn format_investor(profile: &models::channels::UserInvestorProfile) -> String {
    [
        ("name", profile.name.as_deref()),
        ("about", profile.about.as_deref()),
        ("goals", profile.investment_goals.as_deref()),
        ("risk", profile.risk_tolerance.as_deref()),
        ("horizon", profile.time_horizon.as_deref()),
        ("restrictions", profile.restrictions.as_deref()),
        ("notes", profile.notes.as_deref()),
    ]
    .into_iter()
    .filter_map(|(label, value)| value.map(|value| format!("{label}: {value}")))
    .collect::<Vec<_>>()
    .join("\n")
    .if_empty("No investor profile details configured.")
}

trait IfEmpty {
    fn if_empty(self, fallback: &str) -> String;
}

impl IfEmpty for String {
    fn if_empty(self, fallback: &str) -> String {
        if self.trim().is_empty() {
            fallback.to_string()
        } else {
            self
        }
    }
}
