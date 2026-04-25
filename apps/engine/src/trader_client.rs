use anyhow::{Context, Result};
use reqwest::Client;

use crate::trader_types::{
    ChannelMessage, CreateChannelMessageRequest, CreateTraderEventRequest,
    CreateTraderPortfolioProposalRequest, CreateTraderTradeProposalRequest, EngineChannelContext,
    EngineRunnableTrader, EngineTraderConfigResponse, OpenAiChatRequest, OpenAiChatResponse,
    OpenAiTextChatRequest, TraderAiDecision, TraderAiPortfolioProposal,
    TraderPortfolioProposalDetail, TraderRuntimeState, TraderTradeProposal,
    UpsertTraderRuntimeStateRequest,
};
use crate::types::{PaperOrderExecutionResponse, SubmitPaperOrderRequest};
use serde::Serialize;

#[derive(Clone)]
pub struct TraderClient {
    http: Client,
    openapi_base_url: String,
}

#[derive(Serialize)]
struct ReviewPortfolioProposalRequest<'a> {
    status: &'a str,
    review_note: Option<&'a str>,
}

impl TraderClient {
    pub fn new(openapi_base_url: String) -> Self {
        Self {
            http: Client::new(),
            openapi_base_url,
        }
    }

    pub async fn fetch_running_traders(&self) -> Result<Vec<EngineRunnableTrader>> {
        let response = self
            .http
            .get(format!("{}/engine/config/traders", self.openapi_base_url))
            .send()
            .await
            .context("failed to fetch trader config from openapi")?
            .error_for_status()
            .context("openapi trader config endpoint returned error status")?
            .json::<EngineTraderConfigResponse>()
            .await
            .context("failed to deserialize trader config")?;

        Ok(response.traders)
    }

    pub async fn fetch_channel_context(&self) -> Result<EngineChannelContext> {
        self.http
            .get(format!("{}/engine/channel-context", self.openapi_base_url))
            .send()
            .await
            .context("failed to fetch channel context")?
            .error_for_status()
            .context("openapi channel context endpoint returned error status")?
            .json::<EngineChannelContext>()
            .await
            .context("failed to deserialize channel context")
    }

    pub async fn post_channel_message(
        &self,
        channel_name: &str,
        request: &CreateChannelMessageRequest,
    ) -> Result<ChannelMessage> {
        self.http
            .post(format!(
                "{}/engine/channels/{}/messages",
                self.openapi_base_url,
                urlencoding::encode(channel_name)
            ))
            .json(request)
            .send()
            .await
            .context("failed to post channel message")?
            .error_for_status()
            .context("openapi channel message endpoint returned error status")?
            .json::<ChannelMessage>()
            .await
            .context("failed to deserialize channel message")
    }

    pub async fn upsert_runtime_state(
        &self,
        trader_id: &str,
        request: &UpsertTraderRuntimeStateRequest,
    ) -> Result<TraderRuntimeState> {
        self.http
            .post(format!(
                "{}/engine/traders/{}/runtime-state",
                self.openapi_base_url,
                urlencoding::encode(trader_id)
            ))
            .json(request)
            .send()
            .await
            .context("failed to upsert trader runtime state")?
            .error_for_status()
            .context("openapi trader runtime endpoint returned error status")?
            .json::<TraderRuntimeState>()
            .await
            .context("failed to deserialize trader runtime state")
    }

    pub async fn create_event(
        &self,
        trader_id: &str,
        request: &CreateTraderEventRequest,
    ) -> Result<()> {
        self.http
            .post(format!(
                "{}/engine/traders/{}/events",
                self.openapi_base_url,
                urlencoding::encode(trader_id)
            ))
            .json(request)
            .send()
            .await
            .context("failed to create trader event")?
            .error_for_status()
            .context("openapi trader event endpoint returned error status")?;

        Ok(())
    }

    pub async fn create_trade_proposal(
        &self,
        trader_id: &str,
        request: &CreateTraderTradeProposalRequest,
    ) -> Result<TraderTradeProposal> {
        self.http
            .post(format!(
                "{}/engine/traders/{}/trade-proposals",
                self.openapi_base_url,
                urlencoding::encode(trader_id)
            ))
            .json(request)
            .send()
            .await
            .context("failed to create trader trade proposal")?
            .error_for_status()
            .context("openapi trader proposal endpoint returned error status")?
            .json::<TraderTradeProposal>()
            .await
            .context("failed to deserialize trader proposal")
    }

    pub async fn create_portfolio_proposal(
        &self,
        trader_id: &str,
        request: &CreateTraderPortfolioProposalRequest,
    ) -> Result<TraderPortfolioProposalDetail> {
        self.http
            .post(format!(
                "{}/engine/traders/{}/proposals",
                self.openapi_base_url,
                urlencoding::encode(trader_id)
            ))
            .json(request)
            .send()
            .await
            .context("failed to create trader portfolio proposal")?
            .error_for_status()
            .context("openapi trader portfolio proposal endpoint returned error status")?
            .json::<TraderPortfolioProposalDetail>()
            .await
            .context("failed to deserialize trader portfolio proposal")
    }

    pub async fn fetch_active_portfolio_proposal(
        &self,
        trader_id: &str,
    ) -> Result<Option<TraderPortfolioProposalDetail>> {
        let response = self
            .http
            .get(format!(
                "{}/traders/{}/proposals/active",
                self.openapi_base_url,
                urlencoding::encode(trader_id)
            ))
            .send()
            .await
            .context("failed to fetch active trader portfolio proposal")?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }
        let proposal = response
            .error_for_status()
            .context("openapi active portfolio proposal endpoint returned error status")?
            .json::<TraderPortfolioProposalDetail>()
            .await
            .context("failed to deserialize active trader portfolio proposal")?;
        Ok(Some(proposal))
    }

    pub async fn review_portfolio_proposal(
        &self,
        trader_id: &str,
        proposal_id: &str,
        status: &str,
        review_note: Option<&str>,
    ) -> Result<TraderPortfolioProposalDetail> {
        self.http
            .post(format!(
                "{}/traders/{}/proposals/{}/review",
                self.openapi_base_url,
                urlencoding::encode(trader_id),
                urlencoding::encode(proposal_id)
            ))
            .json(&ReviewPortfolioProposalRequest {
                status,
                review_note,
            })
            .send()
            .await
            .context("failed to review trader portfolio proposal")?
            .error_for_status()
            .context("openapi trader proposal review endpoint returned error status")?
            .json::<TraderPortfolioProposalDetail>()
            .await
            .context("failed to deserialize reviewed trader portfolio proposal")
    }

    pub async fn submit_paper_order(
        &self,
        request: &SubmitPaperOrderRequest,
    ) -> Result<PaperOrderExecutionResponse> {
        self.http
            .post(format!("{}/paper/orders", self.openapi_base_url))
            .json(request)
            .send()
            .await
            .context("failed to submit trader paper order")?
            .error_for_status()
            .context("openapi paper order endpoint returned error status")?
            .json::<PaperOrderExecutionResponse>()
            .await
            .context("failed to deserialize paper order response")
    }

    pub async fn ask_openai(
        &self,
        api_key: &str,
        request: &OpenAiChatRequest,
    ) -> Result<TraderAiDecision> {
        let response = self
            .http
            .post("https://api.openai.com/v1/chat/completions")
            .bearer_auth(api_key)
            .json(request)
            .send()
            .await
            .context("failed to call OpenAI for trader decision")?
            .error_for_status()
            .context("OpenAI trader decision request returned error status")?
            .json::<OpenAiChatResponse>()
            .await
            .context("failed to deserialize OpenAI response")?;

        let content = response
            .choices
            .first()
            .map(|choice| choice.message.content.as_str())
            .unwrap_or("{}");
        serde_json::from_str(content).context("failed to parse trader decision JSON")
    }

    pub async fn ask_openai_for_portfolio_proposal(
        &self,
        api_key: &str,
        request: &OpenAiChatRequest,
    ) -> Result<TraderAiPortfolioProposal> {
        let response = self
            .http
            .post("https://api.openai.com/v1/chat/completions")
            .bearer_auth(api_key)
            .json(request)
            .send()
            .await
            .context("failed to call OpenAI for trader portfolio proposal")?
            .error_for_status()
            .context("OpenAI trader portfolio proposal request returned error status")?
            .json::<OpenAiChatResponse>()
            .await
            .context("failed to deserialize OpenAI response")?;

        let content = response
            .choices
            .first()
            .map(|choice| choice.message.content.as_str())
            .unwrap_or("{}");
        serde_json::from_str(content).context("failed to parse trader portfolio proposal JSON")
    }

    pub async fn ask_openai_for_md_message(
        &self,
        api_key: &str,
        request: &OpenAiTextChatRequest,
    ) -> Result<String> {
        self.ask_openai_for_text_message(api_key, request, "MD message")
            .await
    }

    pub async fn ask_openai_for_text_message(
        &self,
        api_key: &str,
        request: &OpenAiTextChatRequest,
        purpose: &str,
    ) -> Result<String> {
        let response = self
            .http
            .post("https://api.openai.com/v1/chat/completions")
            .bearer_auth(api_key)
            .json(request)
            .send()
            .await
            .with_context(|| format!("failed to call OpenAI for {purpose}"))?
            .error_for_status()
            .with_context(|| format!("OpenAI {purpose} request returned error status"))?
            .json::<OpenAiChatResponse>()
            .await
            .with_context(|| format!("failed to deserialize OpenAI {purpose} response"))?;

        Ok(response
            .choices
            .first()
            .map(|choice| choice.message.content.trim().to_string())
            .filter(|content| !content.is_empty())
            .unwrap_or_else(|| {
                "I need more context before I can give a useful response.".to_string()
            }))
    }
}
