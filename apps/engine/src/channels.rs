use anyhow::Result;
use chrono::{DateTime, Utc};
use serde_json::Value;
use tracing::{info, warn};

use crate::{
    config::EngineConfig,
    trader_client::TraderClient,
    trader_types::{
        ChannelMessage, CreateChannelMessageRequest, EngineChannelContext, EngineRunnableTrader,
        OpenAiMessage, OpenAiTextChatRequest, TraderAiDecision,
    },
};

const TRADER_COOLDOWN_SECONDS: i64 = 300;
const MD_COOLDOWN_SECONDS: i64 = 180;

#[derive(Clone, Debug, Default)]
pub struct TraderChannelInsight {
    pub summary: String,
    pub latest_question_answered: bool,
}

pub async fn inspect_trader_channels(
    config: &EngineConfig,
    client: &TraderClient,
    trader: &EngineRunnableTrader,
) -> Option<TraderChannelInsight> {
    if !config.channels_enabled {
        return None;
    }
    let context = match client.fetch_channel_context().await {
        Ok(context) => context,
        Err(err) => {
            warn!(trader_id = %trader.id, error = %err, "failed to inspect trader channel context");
            return None;
        }
    };
    Some(build_trader_channel_insight(trader, &context))
}

pub async fn maybe_post_trader_message(
    config: &EngineConfig,
    client: &TraderClient,
    trader: &EngineRunnableTrader,
    decision: &TraderAiDecision,
) -> Result<()> {
    if !config.channels_enabled {
        return Ok(());
    }
    let Some(reason) = trader_message_reason(decision) else {
        return Ok(());
    };
    let context = client.fetch_channel_context().await?;
    let channel_name = choose_trader_channel(&reason, decision);
    if !cooldown_elapsed(
        &context,
        channel_name,
        "trader",
        Some(&trader.id),
        TRADER_COOLDOWN_SECONDS.max(config.channel_check_interval_seconds as i64),
    ) {
        return Ok(());
    }

    let content = build_trader_message(trader, decision, &reason, &context);
    client
        .post_channel_message(
            channel_name,
            &CreateChannelMessageRequest {
                author_type: "trader".to_string(),
                author_id: Some(trader.id.clone()),
                author_name: Some(trader.name.clone()),
                role: Some("question".to_string()),
                content_markdown: content,
                metadata_json: serde_json::to_string(&serde_json::json!({
                    "conversation_kind": "trader_question",
                    "reason": reason,
                    "confidence": decision.confidence,
                    "engine_name": config.engine_name,
                }))
                .ok(),
            },
        )
        .await?;
    info!(trader_id = %trader.id, channel_name, "posted trader channel message");
    Ok(())
}

pub async fn maybe_post_trader_answer_followup(
    config: &EngineConfig,
    client: &TraderClient,
    trader: &EngineRunnableTrader,
) -> Result<()> {
    if !config.channels_enabled {
        return Ok(());
    }

    let context = client.fetch_channel_context().await?;
    let Some((question, answer)) =
        latest_answered_question_needing_trader_followup(trader, &context)
    else {
        return Ok(());
    };
    let Some(channel_name) = channel_name_for_id(&context, &question.channel_id) else {
        return Ok(());
    };

    let content =
        build_trader_answer_followup_message(client, trader, &context, question, answer).await;
    client
        .post_channel_message(
            channel_name,
            &CreateChannelMessageRequest {
                author_type: "trader".to_string(),
                author_id: Some(trader.id.clone()),
                author_name: Some(trader.name.clone()),
                role: Some("answer".to_string()),
                content_markdown: content,
                metadata_json: serde_json::to_string(&serde_json::json!({
                    "conversation_kind": "trader_followup",
                    "question_message_id": question.id,
                    "answers_message_id": answer.id,
                    "reply_to_message_id": answer.id,
                    "engine_name": config.engine_name,
                }))
                .ok(),
            },
        )
        .await?;
    info!(
        trader_id = %trader.id,
        channel_name,
        question_message_id = %question.id,
        answer_message_id = %answer.id,
        "posted trader channel follow-up"
    );
    Ok(())
}

pub async fn maybe_post_md_message(config: &EngineConfig, client: &TraderClient) -> Result<()> {
    if !config.channels_enabled || !config.md_enabled {
        return Ok(());
    }
    let context = client.fetch_channel_context().await?;
    let Some(trigger) = latest_unanswered_md_trigger(&context) else {
        return Ok(());
    };
    let Some(channel_name) = channel_name_for_id(&context, &trigger.channel_id) else {
        return Ok(());
    };

    let direct_mention = has_md_mention(&trigger.content_markdown, &context.md_profile.name);
    if !direct_mention
        && !cooldown_elapsed(
            &context,
            channel_name,
            "md",
            Some("default"),
            MD_COOLDOWN_SECONDS.max(config.channel_check_interval_seconds as i64),
        )
    {
        return Ok(());
    }

    let message = build_md_message(client, &context, trigger).await;
    client
        .post_channel_message(
            channel_name,
            &CreateChannelMessageRequest {
                author_type: "md".to_string(),
                author_id: Some("default".to_string()),
                author_name: Some(context.md_profile.name.clone()),
                role: Some("review".to_string()),
                content_markdown: message,
                metadata_json: serde_json::to_string(&serde_json::json!({
                    "conversation_kind": "md_review",
                    "engine_name": config.engine_name,
                    "reason": "coordination_monitor",
                    "trigger_message_id": trigger.id,
                    "trigger_channel_id": trigger.channel_id,
                    "reply_to_message_id": trigger.id,
                    "answers_message_id": trigger.id,
                }))
                .ok(),
            },
        )
        .await?;
    info!(channel_name, trigger_message_id = %trigger.id, "posted MD channel message");
    Ok(())
}

pub async fn post_no_symbols_message(
    config: &EngineConfig,
    client: &TraderClient,
    trader: &EngineRunnableTrader,
) {
    if !config.channels_enabled {
        return;
    }
    let result = async {
        let context = client.fetch_channel_context().await?;
        if !cooldown_elapsed(
            &context,
            "general",
            "trader",
            Some(&trader.id),
            TRADER_COOLDOWN_SECONDS.max(config.channel_check_interval_seconds as i64),
        ) {
            return Ok(());
        }
        client
            .post_channel_message(
                "general",
                &CreateChannelMessageRequest {
                    author_type: "trader".to_string(),
                    author_id: Some(trader.id.clone()),
                    author_name: Some(trader.name.clone()),
                    role: Some("alert".to_string()),
                    content_markdown: format!(
                        "I do not have active symbols to evaluate. I need a watchlist or data-source assignment before I can contribute useful analysis."
                    ),
                    metadata_json: serde_json::to_string(&serde_json::json!({
                        "reason": "insufficient_information",
                        "engine_name": config.engine_name,
                    }))
                    .ok(),
                },
            )
            .await?;
        Ok::<(), anyhow::Error>(())
    }
    .await;
    if let Err(err) = result {
        warn!(trader_id = %trader.id, error = %err, "failed to post no-symbols channel message");
    }
}

fn build_trader_channel_insight(
    trader: &EngineRunnableTrader,
    context: &EngineChannelContext,
) -> TraderChannelInsight {
    let latest_question = context
        .recent_messages
        .iter()
        .filter(|message| {
            message.author_type == "trader"
                && message.author_id.as_deref() == Some(trader.id.as_str())
                && message.role == "question"
        })
        .max_by_key(|message| &message.created_at);

    let latest_answer = latest_question.and_then(|question| {
        context
            .recent_messages
            .iter()
            .filter(|message| is_answer_to_trader_question(message, trader, question))
            .max_by_key(|message| &message.created_at)
    });

    let mut lines = Vec::new();
    if let Some(question) = latest_question {
        lines.push(format!(
            "Latest own channel question at {} in {}: {}",
            question.created_at,
            channel_label(context, &question.channel_id),
            compact_markdown(&question.content_markdown)
        ));
        if let Some(answer) = latest_answer {
            lines.push(format!(
                "That question has a later {} response from {} at {}: {}",
                answer.role,
                answer.author_name,
                answer.created_at,
                compact_markdown(&answer.content_markdown)
            ));
        } else {
            lines.push(
                "That latest question has not received a later answer or review yet.".to_string(),
            );
        }
    }

    let relevant = context
        .recent_messages
        .iter()
        .rev()
        .filter(|message| is_relevant_channel_context(message, trader))
        .take(8)
        .collect::<Vec<_>>();
    if !relevant.is_empty() {
        lines.push("Recent useful channel information:".to_string());
        for message in relevant.into_iter().rev() {
            lines.push(format!(
                "- [{}] {} {}: {}",
                channel_label(context, &message.channel_id),
                message.author_name,
                message.role,
                compact_markdown(&message.content_markdown)
            ));
        }
    }

    TraderChannelInsight {
        summary: if lines.is_empty() {
            "No relevant channel context found for this trader.".to_string()
        } else {
            lines.join("\n")
        },
        latest_question_answered: latest_answer.is_some(),
    }
}

fn latest_answered_question_needing_trader_followup<'a>(
    trader: &EngineRunnableTrader,
    context: &'a EngineChannelContext,
) -> Option<(&'a ChannelMessage, &'a ChannelMessage)> {
    context
        .recent_messages
        .iter()
        .filter(|message| {
            message.author_type == "trader"
                && message.author_id.as_deref() == Some(trader.id.as_str())
                && message.role == "question"
        })
        .filter_map(|question| {
            let answer = latest_answer_to_trader_question(context, trader, question)?;
            if trader_has_followed_up_to_question(context, trader, question) {
                return None;
            }
            Some((question, answer))
        })
        .max_by_key(|(question, answer)| (&question.created_at, &answer.created_at))
}

fn latest_answer_to_trader_question<'a>(
    context: &'a EngineChannelContext,
    trader: &EngineRunnableTrader,
    question: &ChannelMessage,
) -> Option<&'a ChannelMessage> {
    context
        .recent_messages
        .iter()
        .filter(|message| is_answer_to_trader_question(message, trader, question))
        .max_by_key(|message| &message.created_at)
}

fn trader_has_followed_up_to_question(
    context: &EngineChannelContext,
    trader: &EngineRunnableTrader,
    question: &ChannelMessage,
) -> bool {
    context.recent_messages.iter().any(|message| {
        message.channel_id == question.channel_id
            && message.created_at > question.created_at
            && message.author_type == "trader"
            && message.author_id.as_deref() == Some(trader.id.as_str())
            && metadata_string(message, "conversation_kind").as_deref() == Some("trader_followup")
            && metadata_string(message, "question_message_id").as_deref()
                == Some(question.id.as_str())
    })
}

fn is_answer_to_trader_question(
    message: &ChannelMessage,
    trader: &EngineRunnableTrader,
    question: &ChannelMessage,
) -> bool {
    if message.channel_id != question.channel_id || message.created_at <= question.created_at {
        return false;
    }
    if message.author_type == "trader" && message.author_id.as_deref() == Some(trader.id.as_str()) {
        return false;
    }
    if metadata_points_to(message, question) {
        return true;
    }

    let content = message.content_markdown.to_lowercase();
    let has_metadata = message.metadata_json.is_some();
    if message.author_type == "md" {
        return !has_metadata;
    }
    if matches!(message.role.as_str(), "answer" | "review") && !has_metadata {
        return true;
    }
    message.author_type == "user"
        && (content.contains(&trader.name.to_lowercase())
            || content.contains(&format!("@{}", trader.name.to_lowercase()))
            || content.contains(&format!("@{}", trader.id.to_lowercase())))
}

fn is_relevant_channel_context(message: &ChannelMessage, trader: &EngineRunnableTrader) -> bool {
    let content = message.content_markdown.to_lowercase();
    message.author_type == "user"
        || message.author_type == "md"
        || matches!(
            message.role.as_str(),
            "answer" | "review" | "alert" | "question"
        )
        || content.contains(&trader.name.to_lowercase())
        || trader.tracked_symbols.iter().any(|symbol| {
            !symbol.symbol.is_empty() && content.contains(&symbol.symbol.to_lowercase())
        })
}

fn channel_label<'a>(context: &'a EngineChannelContext, channel_id: &str) -> &'a str {
    context
        .channels
        .iter()
        .find(|channel| channel.id == channel_id)
        .map(|channel| channel.display_name.as_str())
        .unwrap_or("#unknown")
}

fn compact_markdown(value: &str) -> String {
    let compact = value.split_whitespace().collect::<Vec<_>>().join(" ");
    const MAX_LEN: usize = 360;
    if compact.len() > MAX_LEN {
        format!("{}...", &compact[..MAX_LEN])
    } else {
        compact
    }
}

async fn build_trader_answer_followup_message(
    client: &TraderClient,
    trader: &EngineRunnableTrader,
    context: &EngineChannelContext,
    question: &ChannelMessage,
    answer: &ChannelMessage,
) -> String {
    match client
        .ask_openai_for_text_message(
            &trader.openai_api_key,
            &build_trader_followup_openai_request(trader, context, question, answer),
            "trader channel follow-up",
        )
        .await
    {
        Ok(message) => message,
        Err(err) => {
            warn!(
                trader_id = %trader.id,
                error = %err,
                "failed to generate trader channel follow-up with OpenAI; using fallback"
            );
            build_fallback_trader_followup(answer)
        }
    }
}

fn build_trader_followup_openai_request(
    trader: &EngineRunnableTrader,
    context: &EngineChannelContext,
    question: &ChannelMessage,
    answer: &ChannelMessage,
) -> OpenAiTextChatRequest {
    let recent_messages = context
        .recent_messages
        .iter()
        .filter(|message| message.channel_id == question.channel_id)
        .rev()
        .take(12)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .map(|message| {
            format!(
                "{} {}: {}",
                message.author_name,
                message.role,
                compact_markdown(&message.content_markdown)
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let user_profile =
        serde_json::to_string(&context.user_investor_profile).unwrap_or_else(|_| "{}".to_string());
    let prompt = format!(
        r#"You are an AI trader participating in a shared channel.

Trader name: {name}
Persona: {persona}
Tone: {tone}
Communication style: {communication_style}
Perspective: {perspective}

You asked this question:
{question}

You received this later answer or review from {answer_author}:
{answer}

Recent channel context:
{recent_messages}

User investor profile context:
{user_profile}

Write one concise markdown follow-up as the trader. If the answer resolves the question, acknowledge it and state what you will use from it. If there is still an important gap, ask one concrete follow-up question. Do not propose or execute trades from channel chat. Do not return JSON."#,
        name = trader.name,
        persona = trader.persona.as_deref().unwrap_or("unset"),
        tone = trader
            .tone
            .as_deref()
            .unwrap_or("professional, concise, analytical"),
        communication_style = trader
            .communication_style
            .as_deref()
            .unwrap_or("explains uncertainty and asks for help when needed"),
        perspective = trader.fundamental_perspective,
        question = question.content_markdown,
        answer_author = answer.author_name,
        answer = answer.content_markdown,
        recent_messages = recent_messages,
        user_profile = user_profile,
    );

    OpenAiTextChatRequest {
        model: "gpt-4o-mini".to_string(),
        messages: vec![
            OpenAiMessage {
                role: "system".to_string(),
                content: "You write only the trader's markdown follow-up. Do not return JSON. Do not trade.".to_string(),
            },
            OpenAiMessage {
                role: "user".to_string(),
                content: prompt,
            },
        ],
    }
}

fn build_fallback_trader_followup(answer: &ChannelMessage) -> String {
    if answer.content_markdown.contains('?') {
        format!(
            "Thanks, {author}. I still need to resolve the open clarification before treating this as proposal-ready.",
            author = answer.author_name
        )
    } else {
        format!(
            "Thanks, {author}. This gives me what I needed: I will incorporate the review into my next evaluation and keep the discussion separate from any trade execution.",
            author = answer.author_name
        )
    }
}

fn trader_message_reason(decision: &TraderAiDecision) -> Option<String> {
    if decision
        .confidence
        .map(|value| value < 0.45)
        .unwrap_or(false)
    {
        return Some("low_confidence".to_string());
    }
    let reason = decision.reason.to_lowercase();
    for marker in [
        "insufficient",
        "incomplete",
        "uncertain",
        "contradiction",
        "blocked",
        "risk",
        "invalid",
    ] {
        if reason.contains(marker) {
            return Some(marker.to_string());
        }
    }
    None
}

fn choose_trader_channel(reason: &str, decision: &TraderAiDecision) -> &'static str {
    let text = decision.reason.to_lowercase();
    if reason == "insufficient" || reason == "incomplete" || text.contains("data") {
        return "data_analysis";
    }
    if text.contains("risk") || text.contains("proposal") || text.contains("trade") {
        return "trading";
    }
    "general"
}

fn build_trader_message(
    trader: &EngineRunnableTrader,
    decision: &TraderAiDecision,
    reason: &str,
    context: &EngineChannelContext,
) -> String {
    let tone = trader
        .tone
        .as_deref()
        .unwrap_or("professional, concise, analytical");
    let profile_hint = context
        .user_investor_profile
        .risk_tolerance
        .as_deref()
        .map(|risk| format!(" User risk tolerance context: {risk}."))
        .unwrap_or_default();
    format!(
        "I need review on my latest evaluation.\n\n- **Reason:** {reason}\n- **Action:** {action}\n- **Symbol:** {symbol}\n- **Confidence:** {confidence}\n- **Tone:** {tone}\n\n{decision_reason}{profile_hint}\n\n@MD or another trader should challenge weak assumptions before this becomes a proposal.",
        action = decision.action,
        symbol = decision.symbol.as_deref().unwrap_or("n/a"),
        confidence = decision
            .confidence
            .map(|value| format!("{value:.2}"))
            .unwrap_or_else(|| "unknown".to_string()),
        decision_reason = decision.reason,
    )
}

fn latest_unanswered_md_trigger(context: &EngineChannelContext) -> Option<&ChannelMessage> {
    context
        .recent_messages
        .iter()
        .filter(|message| {
            is_md_trigger(
                message.author_type.as_str(),
                &message.role,
                &message.content_markdown,
                &context.md_profile.name,
            )
        })
        .filter(|trigger| !md_has_answered_trigger(context, trigger))
        .max_by_key(|message| &message.created_at)
}

fn md_has_answered_trigger(context: &EngineChannelContext, trigger: &ChannelMessage) -> bool {
    context.recent_messages.iter().any(|message| {
        message.author_type == "md"
            && message.channel_id == trigger.channel_id
            && message.created_at > trigger.created_at
            && (metadata_points_to(message, trigger) || message.metadata_json.is_none())
    })
}

async fn build_md_message(
    client: &TraderClient,
    context: &EngineChannelContext,
    trigger: &ChannelMessage,
) -> String {
    if let Some(api_key) = context
        .md_openai_api_key
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        match client
            .ask_openai_for_md_message(api_key, &build_md_openai_request(context, trigger))
            .await
        {
            Ok(message) => return ensure_md_trader_mention(trigger, message),
            Err(err) => {
                warn!(error = %err, "failed to generate MD message with OpenAI; using fallback")
            }
        }
    }

    build_fallback_md_message(context, trigger)
}

fn build_md_openai_request(
    context: &EngineChannelContext,
    trigger: &ChannelMessage,
) -> OpenAiTextChatRequest {
    let recent_messages = context
        .recent_messages
        .iter()
        .rev()
        .take(16)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .map(|message| {
            format!(
                "[{}] {} {}: {}",
                channel_label(context, &message.channel_id),
                message.author_name,
                message.role,
                compact_markdown(&message.content_markdown)
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let trader_personas = context
        .trader_personas
        .iter()
        .map(|persona| {
            format!(
                "{}: persona={}; tone={}; communication_style={}",
                persona.trader_id,
                persona.persona.as_deref().unwrap_or("unset"),
                persona.tone.as_deref().unwrap_or("unset"),
                persona.communication_style.as_deref().unwrap_or("unset")
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let user_profile =
        serde_json::to_string(&context.user_investor_profile).unwrap_or_else(|_| "{}".to_string());
    let prompt = format!(
        r#"You are the MD in a paper-trading coordination app.

Persona: {persona}
Tone: {tone}
Communication style: {communication_style}

Trigger message:
[{trigger_channel}] {trigger_author} {trigger_role}: {trigger_content}

Recent channel messages:
{recent_messages}

Trader personas:
{trader_personas}

User investor profile context:
{user_profile}

Write one concise markdown response as the MD. If the trigger is from a trader, begin the response with @{trigger_handle} so the trader is explicitly tagged. Monitor, question, summarize, reduce drift, and challenge weak assumptions. If the user or trader mentions @MD, answer that request directly. Never place trades, submit orders, approve trades, or imply channel chat can bypass risk controls."#,
        persona = context.md_profile.persona,
        tone = context.md_profile.tone,
        communication_style = context.md_profile.communication_style,
        trigger_channel = channel_label(context, &trigger.channel_id),
        trigger_author = trigger.author_name,
        trigger_handle = channel_handle(&trigger.author_name),
        trigger_role = trigger.role,
        trigger_content = trigger.content_markdown,
        recent_messages = recent_messages,
        trader_personas = if trader_personas.is_empty() {
            "none"
        } else {
            &trader_personas
        },
        user_profile = user_profile,
    );

    OpenAiTextChatRequest {
        model: "gpt-4o-mini".to_string(),
        messages: vec![
            OpenAiMessage {
                role: "system".to_string(),
                content:
                    "You write only the MD's markdown response. Do not return JSON. Do not trade."
                        .to_string(),
            },
            OpenAiMessage {
                role: "user".to_string(),
                content: prompt,
            },
        ],
    }
}

fn build_fallback_md_message(context: &EngineChannelContext, trigger: &ChannelMessage) -> String {
    let latest = format!(
        "{} wrote: {}",
        trigger.author_name, trigger.content_markdown
    );
    let risk_context = context
        .user_investor_profile
        .risk_tolerance
        .as_deref()
        .map(|risk| format!(" Keep the user's risk tolerance in view: {risk}."))
        .unwrap_or_default();
    let mention = md_trader_mention_prefix(trigger);

    format!(
        "{mention}Review checkpoint: {latest}\n\nBefore any proposal changes, clarify the data gap, the confidence driver, and whether the active plan is actually invalidated.{risk_context}\n\nReminder: channel discussion is coordination only and does not execute trades."
    )
}

fn ensure_md_trader_mention(trigger: &ChannelMessage, message: String) -> String {
    let mention = md_trader_mention_prefix(trigger);
    if mention.is_empty() {
        return message;
    }
    let trimmed = message.trim_start();
    if trimmed
        .to_lowercase()
        .starts_with(&mention.trim().to_lowercase())
    {
        message
    } else {
        format!("{mention}{trimmed}")
    }
}

fn md_trader_mention_prefix(trigger: &ChannelMessage) -> String {
    if trigger.author_type == "trader" {
        format!("@{} ", channel_handle(&trigger.author_name))
    } else {
        String::new()
    }
}

fn channel_handle(name: &str) -> String {
    let handle = name
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-'))
        .collect::<String>();
    if handle.is_empty() {
        "trader".to_string()
    } else {
        handle
    }
}

fn is_md_trigger(author_type: &str, role: &str, content_markdown: &str, md_name: &str) -> bool {
    let content = content_markdown.to_lowercase();
    match author_type {
        "trader" => {
            role == "question"
                || has_md_mention(content_markdown, md_name)
                || content.contains("not confident")
                || content.contains("need review")
        }
        "user" => has_md_mention(content_markdown, md_name),
        _ => false,
    }
}

fn has_md_mention(content_markdown: &str, md_name: &str) -> bool {
    contains_mention(content_markdown, "md")
        || contains_mention(content_markdown, &channel_handle(md_name))
        || md_name
            .split_whitespace()
            .next()
            .map(|first_name| contains_mention(content_markdown, &channel_handle(first_name)))
            .unwrap_or(false)
}

fn contains_mention(content_markdown: &str, handle: &str) -> bool {
    let handle = handle.trim().trim_start_matches('@').to_ascii_lowercase();
    if handle.is_empty() {
        return false;
    }
    let content = content_markdown.to_ascii_lowercase();
    let needle = format!("@{handle}");
    let mut search_from = 0;
    while let Some(offset) = content[search_from..].find(&needle) {
        let start = search_from + offset;
        let end = start + needle.len();
        let next = content[end..].chars().next();
        if next
            .map(|ch| !ch.is_ascii_alphanumeric() && ch != '_' && ch != '-')
            .unwrap_or(true)
        {
            return true;
        }
        search_from = end;
    }
    false
}

fn metadata_points_to(message: &ChannelMessage, target: &ChannelMessage) -> bool {
    metadata_string(message, "reply_to_message_id").as_deref() == Some(target.id.as_str())
        || metadata_string(message, "answers_message_id").as_deref() == Some(target.id.as_str())
        || metadata_string(message, "trigger_message_id").as_deref() == Some(target.id.as_str())
}

fn metadata_string(message: &ChannelMessage, key: &str) -> Option<String> {
    message
        .metadata_json
        .as_deref()
        .and_then(|raw| serde_json::from_str::<Value>(raw).ok())
        .and_then(|value| {
            value
                .get(key)
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)
        })
}

fn channel_name_for_id<'a>(context: &'a EngineChannelContext, channel_id: &str) -> Option<&'a str> {
    context
        .channels
        .iter()
        .find(|channel| channel.id == channel_id)
        .map(|channel| channel.name.as_str())
}

fn cooldown_elapsed(
    context: &EngineChannelContext,
    channel_name: &str,
    author_type: &str,
    author_id: Option<&str>,
    cooldown_seconds: i64,
) -> bool {
    let Some(channel) = context
        .channels
        .iter()
        .find(|channel| channel.name == channel_name)
    else {
        return false;
    };
    let latest = context
        .recent_messages
        .iter()
        .filter(|message| {
            message.channel_id == channel.id
                && message.author_type == author_type
                && message.author_id.as_deref() == author_id
        })
        .filter_map(|message| DateTime::parse_from_rfc3339(&message.created_at).ok())
        .max();
    match latest {
        Some(last) => {
            Utc::now()
                .signed_duration_since(last.with_timezone(&Utc))
                .num_seconds()
                >= cooldown_seconds
        }
        None => true,
    }
}
