pub use models::engine::{
    ActiveSymbolsResponse, EngineEventRequest, EngineHealthResponse, EngineHeartbeatRequest,
};
pub use models::paper::{PaperAccountSummaryResponse, PaperOrderExecutionResponse};
pub use models::trading::{
    CreateStrategySignalRequest, EngineRunnableStrategy, EngineStrategyConfigResponse,
    StrategyCondition, StrategyConditionGroup, StrategyDefinition, StrategyRuntimeState,
    StrategyRuntimeStateListResponse, StrategySignal, StrategySignalListResponse,
    UpdateStrategySignalStatusRequest, UpsertStrategyRuntimeStateRequest,
};
use serde::Serialize;

#[derive(Serialize, Clone, Debug)]
pub struct SubmitPaperOrderRequest {
    pub account_id: String,
    pub symbol: String,
    pub side: String,
    pub order_type: String,
    pub quantity: f64,
    pub requested_price: Option<f64>,
    pub source: Option<String>,
    pub strategy_id: Option<String>,
    pub signal_id: Option<String>,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct ProposedPaperOrder {
    pub strategy_id: String,
    pub signal_id: String,
    pub account_id: String,
    pub symbol: String,
    pub side: String,
    pub order_type: String,
    pub quantity: f64,
    pub estimated_price: f64,
    pub estimated_notional: f64,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct RiskDecision {
    pub allowed: bool,
    pub reason: String,
    pub adjusted_quantity: Option<f64>,
}
