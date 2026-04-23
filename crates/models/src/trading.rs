use poem_openapi::Object;
use serde::{Deserialize, Serialize};

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct StrategyCondition {
    pub indicator: String,
    pub period: Option<i32>,
    pub operator: String,
    pub value: Option<f64>,
    pub compare_indicator: Option<String>,
    pub compare_period: Option<i32>,
}

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct StrategyConditionGroup {
    pub all: Option<Vec<StrategyCondition>>,
    pub any: Option<Vec<StrategyCondition>>,
}

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct StrategyPositionSize {
    pub r#type: String,
    pub quantity: Option<f64>,
    pub percent: Option<f64>,
}

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct StrategyRisk {
    pub position_size: StrategyPositionSize,
    pub max_position_per_symbol: Option<i32>,
    pub cooldown_seconds: Option<i64>,
}

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct StrategyDefinition {
    pub version: String,
    pub entry: StrategyConditionGroup,
    pub exit: StrategyConditionGroup,
    pub risk: StrategyRisk,
}

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct StrategyTradingConfig {
    pub strategy_id: String,
    pub trading_mode: String,
    pub paper_account_id: Option<String>,
    pub is_enabled: bool,
    pub last_started_at: Option<String>,
    pub last_stopped_at: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct StrategyRiskConfig {
    pub strategy_id: String,
    pub max_dollars_per_trade: Option<f64>,
    pub max_quantity_per_trade: Option<f64>,
    pub max_position_value_per_symbol: Option<f64>,
    pub max_total_exposure: Option<f64>,
    pub max_open_positions: Option<i64>,
    pub max_daily_trades: Option<i64>,
    pub max_daily_loss: Option<f64>,
    pub cooldown_seconds: i64,
    pub allowlist_symbols: Option<Vec<String>>,
    pub blocklist_symbols: Option<Vec<String>>,
    pub is_trading_enabled: bool,
    pub kill_switch_enabled: bool,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct UpdateStrategyTradingConfigRequest {
    pub trading_mode: String,
    pub paper_account_id: Option<String>,
    pub is_enabled: bool,
}

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct UpdateStrategyRiskConfigRequest {
    pub max_dollars_per_trade: Option<f64>,
    pub max_quantity_per_trade: Option<f64>,
    pub max_position_value_per_symbol: Option<f64>,
    pub max_total_exposure: Option<f64>,
    pub max_open_positions: Option<i64>,
    pub max_daily_trades: Option<i64>,
    pub max_daily_loss: Option<f64>,
    pub cooldown_seconds: i64,
    pub allowlist_symbols: Option<Vec<String>>,
    pub blocklist_symbols: Option<Vec<String>>,
    pub is_trading_enabled: bool,
    pub kill_switch_enabled: bool,
}

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct StrategyRuntimeState {
    pub id: String,
    pub strategy_id: String,
    pub paper_account_id: String,
    pub symbol: String,
    pub last_evaluated_at: Option<String>,
    pub last_signal: Option<String>,
    pub last_signal_at: Option<String>,
    pub last_order_id: Option<String>,
    pub position_state: String,
    pub cooldown_until: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct UpsertStrategyRuntimeStateRequest {
    pub strategy_id: String,
    pub paper_account_id: String,
    pub symbol: String,
    pub last_evaluated_at: Option<String>,
    pub last_signal: Option<String>,
    pub last_signal_at: Option<String>,
    pub last_order_id: Option<String>,
    pub position_state: String,
    pub cooldown_until: Option<String>,
}

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct StrategySignal {
    pub id: String,
    pub strategy_id: String,
    pub paper_account_id: String,
    pub symbol: String,
    pub signal_type: String,
    pub confidence: Option<f64>,
    pub reason: String,
    pub market_price: Option<f64>,
    pub source: String,
    pub status: String,
    pub risk_decision: Option<String>,
    pub risk_reason: Option<String>,
    pub order_id: Option<String>,
    pub created_at: String,
}

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct CreateStrategySignalRequest {
    pub strategy_id: String,
    pub paper_account_id: String,
    pub symbol: String,
    pub signal_type: String,
    pub confidence: Option<f64>,
    pub reason: String,
    pub market_price: Option<f64>,
    pub source: Option<String>,
    pub status: Option<String>,
    pub risk_decision: Option<String>,
    pub risk_reason: Option<String>,
    pub order_id: Option<String>,
}

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct UpdateStrategySignalStatusRequest {
    pub status: String,
    pub risk_decision: Option<String>,
    pub risk_reason: Option<String>,
    pub order_id: Option<String>,
}

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct StrategyRuntimeStateListResponse {
    pub states: Vec<StrategyRuntimeState>,
}

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct StrategySignalListResponse {
    pub signals: Vec<StrategySignal>,
}

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct EngineRunnableStrategy {
    pub strategy_id: String,
    pub name: String,
    pub trading_mode: String,
    pub paper_account_id: String,
    pub symbol_universe: Vec<String>,
    pub timeframe: String,
    pub strategy_definition: StrategyDefinition,
    pub risk: StrategyRisk,
    pub risk_config: StrategyRiskConfig,
}

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct EngineStrategyConfigResponse {
    pub strategies: Vec<EngineRunnableStrategy>,
}
