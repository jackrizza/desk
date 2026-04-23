use anyhow::{Context, Result};
use reqwest::Client;

use crate::types::{
    ActiveSymbolsResponse, CreateStrategySignalRequest, EngineEventRequest, EngineHealthResponse,
    EngineHeartbeatRequest, EngineStrategyConfigResponse, PaperAccountSummaryResponse,
    PaperOrderExecutionResponse, StrategyRuntimeState, StrategyRuntimeStateListResponse,
    StrategySignal, StrategySignalListResponse, SubmitPaperOrderRequest,
    UpdateStrategySignalStatusRequest, UpsertStrategyRuntimeStateRequest,
};

#[derive(Clone)]
pub struct OpenApiClient {
    http: Client,
    base_url: String,
}

impl OpenApiClient {
    pub fn new(base_url: String) -> Self {
        Self {
            http: Client::new(),
            base_url,
        }
    }

    pub async fn health_check(&self) -> Result<EngineHealthResponse> {
        self.http
            .get(format!("{}/health/live", self.base_url))
            .send()
            .await
            .context("failed to call openapi health endpoint")?
            .error_for_status()
            .context("openapi health endpoint returned error status")?
            .json::<EngineHealthResponse>()
            .await
            .context("failed to deserialize openapi health response")
    }

    pub async fn fetch_active_symbols(&self) -> Result<Vec<String>> {
        let response = self
            .http
            .get(format!("{}/engine/config/symbols", self.base_url))
            .send()
            .await
            .context("failed to fetch active symbols from openapi")?
            .error_for_status()
            .context("openapi active symbols endpoint returned error status")?
            .json::<ActiveSymbolsResponse>()
            .await
            .context("failed to deserialize active symbols response")?;

        Ok(response.symbols)
    }

    pub async fn fetch_engine_strategy_configs(
        &self,
    ) -> Result<Vec<crate::types::EngineRunnableStrategy>> {
        let response = self
            .http
            .get(format!("{}/engine/config/strategies", self.base_url))
            .send()
            .await
            .context("failed to fetch engine strategy configs from openapi")?
            .error_for_status()
            .context("openapi engine strategy configs endpoint returned error status")?
            .json::<EngineStrategyConfigResponse>()
            .await
            .context("failed to deserialize engine strategy configs response")?;

        Ok(response.strategies)
    }

    pub async fn get_paper_account_summary(
        &self,
        account_id: &str,
    ) -> Result<PaperAccountSummaryResponse> {
        self.http
            .get(format!(
                "{}/paper/accounts/{}/summary",
                self.base_url,
                urlencoding::encode(account_id)
            ))
            .send()
            .await
            .context("failed to fetch paper account summary from openapi")?
            .error_for_status()
            .context("openapi paper account summary endpoint returned error status")?
            .json::<PaperAccountSummaryResponse>()
            .await
            .context("failed to deserialize paper account summary response")
    }

    pub async fn submit_paper_order(
        &self,
        request: &SubmitPaperOrderRequest,
    ) -> Result<PaperOrderExecutionResponse> {
        self.http
            .post(format!("{}/paper/orders", self.base_url))
            .json(request)
            .send()
            .await
            .context("failed to submit paper order to openapi")?
            .error_for_status()
            .context("openapi paper orders endpoint returned error status")?
            .json::<PaperOrderExecutionResponse>()
            .await
            .context("failed to deserialize paper order execution response")
    }

    pub async fn create_strategy_signal(
        &self,
        request: &CreateStrategySignalRequest,
    ) -> Result<crate::types::StrategySignal> {
        self.http
            .post(format!("{}/engine/strategy-signals", self.base_url))
            .json(request)
            .send()
            .await
            .context("failed to persist strategy signal to openapi")?
            .error_for_status()
            .context("openapi strategy signal endpoint returned error status")?
            .json::<crate::types::StrategySignal>()
            .await
            .context("failed to deserialize strategy signal response")
    }

    pub async fn update_strategy_signal(
        &self,
        signal_id: &str,
        request: &UpdateStrategySignalStatusRequest,
    ) -> Result<()> {
        self.http
            .post(format!(
                "{}/engine/strategy-signals/{}",
                self.base_url,
                urlencoding::encode(signal_id)
            ))
            .json(request)
            .send()
            .await
            .context("failed to update strategy signal in openapi")?
            .error_for_status()
            .context("openapi strategy signal update endpoint returned error status")?;

        Ok(())
    }

    pub async fn fetch_strategy_runtime_state(
        &self,
        strategy_id: &str,
    ) -> Result<Vec<StrategyRuntimeState>> {
        let response = self
            .http
            .get(format!(
                "{}/strategies/{}/runtime-state",
                self.base_url,
                urlencoding::encode(strategy_id)
            ))
            .send()
            .await
            .context("failed to fetch strategy runtime state from openapi")?
            .error_for_status()
            .context("openapi strategy runtime state endpoint returned error status")?
            .json::<StrategyRuntimeStateListResponse>()
            .await
            .context("failed to deserialize strategy runtime state response")?;

        Ok(response.states)
    }

    pub async fn fetch_strategy_signals(&self, strategy_id: &str) -> Result<Vec<StrategySignal>> {
        let response = self
            .http
            .get(format!(
                "{}/strategies/{}/signals",
                self.base_url,
                urlencoding::encode(strategy_id)
            ))
            .send()
            .await
            .context("failed to fetch strategy signals from openapi")?
            .error_for_status()
            .context("openapi strategy signals endpoint returned error status")?
            .json::<StrategySignalListResponse>()
            .await
            .context("failed to deserialize strategy signals response")?;

        Ok(response.signals)
    }

    pub async fn upsert_strategy_runtime_state(
        &self,
        request: &UpsertStrategyRuntimeStateRequest,
    ) -> Result<StrategyRuntimeState> {
        self.http
            .post(format!("{}/engine/strategy-runtime-state", self.base_url))
            .json(request)
            .send()
            .await
            .context("failed to persist strategy runtime state to openapi")?
            .error_for_status()
            .context("openapi strategy runtime state endpoint returned error status")?
            .json::<StrategyRuntimeState>()
            .await
            .context("failed to deserialize strategy runtime state response")
    }

    pub async fn send_heartbeat(&self, request: &EngineHeartbeatRequest) -> Result<()> {
        self.http
            .post(format!("{}/engine/heartbeat", self.base_url))
            .json(request)
            .send()
            .await
            .context("failed to send engine heartbeat")?
            .error_for_status()
            .context("openapi heartbeat endpoint returned error status")?;

        Ok(())
    }

    pub async fn report_engine_event(&self, request: &EngineEventRequest) -> Result<()> {
        self.http
            .post(format!("{}/engine/events", self.base_url))
            .json(request)
            .send()
            .await
            .context("failed to report engine event")?
            .error_for_status()
            .context("openapi engine events endpoint returned error status")?;

        Ok(())
    }
}
