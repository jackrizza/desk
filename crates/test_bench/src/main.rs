use env_logger;
use log::{error, info};

#[tokio::main]
async fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .init();

    let mut cache: cache::Cache = cache::Cache::new("cache.json".to_string());

    let symbol = "AAPL".to_string();
    let range = "1mo".to_string();
    let interval = "1d".to_string();
    let prepost = false;

    let key = format!("{}_{}_{}_{}", symbol, range, interval, prepost);
    let stock_data = cache.check_cache(&key).await;

    match stock_data {
        Ok(data) => info!("{:?}", data),
        Err(e) => error!("Error fetching stock data: {}", e),
    }
}
