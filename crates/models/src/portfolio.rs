use poem_openapi::Object;
use serde::{Deserialize, Serialize};

#[derive(Object, Serialize, Deserialize, Clone, Debug)]
pub struct Position {
    pub symbol: String,
    pub quantity: f64,
    pub average_price: f64,
    pub position_opened_at: String,
    pub position_closed_at: Option<String>,
    pub position_closed_price: Option<f64>,
}

#[derive(Object, Serialize, Deserialize, Clone, Debug)]
pub struct Portfolio {
    pub id: String,
    pub name: String,
    pub description: String,
    pub created_at: String,
    pub updated_at: String,
    pub positions: Vec<Position>,
}
