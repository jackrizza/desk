use poem_openapi::Object;
use serde::{Deserialize, Serialize};

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct PaperAccount {
    pub id: String,
    pub name: String,
    pub starting_cash: f64,
    pub cash_balance: f64,
    pub currency: String,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct PaperPosition {
    pub id: String,
    pub account_id: String,
    pub symbol: String,
    pub quantity: f64,
    pub average_price: f64,
    pub realized_pnl: f64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct PaperOrder {
    pub id: String,
    pub account_id: String,
    pub symbol: String,
    pub side: String,
    pub order_type: String,
    pub quantity: f64,
    pub requested_price: Option<f64>,
    pub filled_quantity: f64,
    pub average_fill_price: Option<f64>,
    pub status: String,
    pub source: String,
    pub strategy_id: Option<String>,
    pub signal_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct PaperFill {
    pub id: String,
    pub account_id: String,
    pub order_id: String,
    pub symbol: String,
    pub side: String,
    pub quantity: f64,
    pub price: f64,
    pub notional: f64,
    pub created_at: String,
}

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct PaperAccountEvent {
    pub id: String,
    pub account_id: String,
    pub event_type: String,
    pub message: String,
    pub payload: Option<String>,
    pub created_at: String,
}

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct CreatePaperAccountRequest {
    pub name: String,
    pub starting_cash: f64,
}

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct CreatePaperOrderRequest {
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

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct PaperAccountSummaryResponse {
    pub account: PaperAccount,
    pub positions: Vec<PaperPosition>,
    pub open_orders: Vec<PaperOrder>,
    pub recent_fills: Vec<PaperFill>,
    pub equity_estimate: f64,
}

#[derive(Object, Serialize, Deserialize, Clone, Debug, Default)]
pub struct PaperOrderExecutionResponse {
    pub order: PaperOrder,
    pub fill: Option<PaperFill>,
    pub position: Option<PaperPosition>,
    pub cash_balance: f64,
}
