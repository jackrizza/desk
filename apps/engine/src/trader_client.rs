use anyhow::{Context, Result};
use reqwest::Client;

use crate::trader_types::{
    CreateTraderEventRequest, CreateTraderTradeProposalRequest, EngineRunnableTrader,
    EngineTraderConfigResponse, OpenAiChatRequest, OpenAiChatResponse, TraderAiDecision,
    TraderRuntimeState, TraderTradeProposal, UpsertTraderRuntimeStateRequest,
};
use crate::types::{PaperOrderExecutionResponse, SubmitPaperOrderRequest};

#[derive(Clone)]
pub struct TraderClient {
    http: Client,
    openapi_base_url: String,
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
}
