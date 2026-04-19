use poem_openapi::Object;
use serde::{Deserialize, Serialize};

#[derive(Object, Serialize, Deserialize, Clone, Default)]
pub struct RawStockData {
    pub symbol: String,
    pub last_refreshed: String,
    pub interval: String,
    pub range: String,
    pub stock_data: Vec<RawStockDataEntry>,
}

impl std::fmt::Debug for RawStockData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "RawStockData {{ symbol: {}, last_refreshed: {}, stock_data: [.{}] }}",
            self.symbol,
            self.last_refreshed,
            self.stock_data.len()
        )
    }
}

#[derive(Object, Serialize, Deserialize, Debug, Clone, Default)]
pub struct RawStockDataEntry {
    pub date: String,
    pub open: String,
    pub high: String,
    pub low: String,
    pub close: String,
    pub volume: String,
}

#[derive(Object, Serialize, Deserialize, Debug, Clone, Default)]
pub struct IndicatorPoint {
    pub date: String,
    pub value: f64,
}

#[derive(Object, Serialize, Deserialize, Debug, Clone, Default)]
pub struct IndicatorLine {
    pub key: String,
    pub label: String,
    pub points: Vec<IndicatorPoint>,
}

#[derive(Object, Serialize, Deserialize, Debug, Clone, Default)]
pub struct IndicatorResult {
    pub key: String,
    pub display_name: String,
    pub overlay: bool,
    pub lines: Vec<IndicatorLine>,
}

#[derive(Object, Serialize, Deserialize, Debug, Clone, Default)]
pub struct StockIndicatorsResponse {
    pub symbol: String,
    pub last_refreshed: String,
    pub interval: String,
    pub range: String,
    pub indicators: Vec<IndicatorResult>,
    pub unsupported: Vec<String>,
}
