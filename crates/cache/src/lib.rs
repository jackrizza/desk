use std::sync::Arc;

use arc_swap::ArcSwap;
use chrono::{DateTime, Datelike, Duration, Timelike, Utc};
use log::info;
use models::raw::RawStockData;
use rustc_hash::FxHashMap;
use tokio::fs;

use stock_data::get_stock_data;

type AnyError = Box<dyn std::error::Error + Send + Sync>;

#[derive(Debug, Clone, Default)]
struct CacheState {
    data: FxHashMap<String, Arc<RawStockData>>,
}

pub struct Cache {
    root: String,
    state: ArcSwap<CacheState>,
}

impl Cache {
    pub fn new(root: String) -> Self {
        Self {
            root,
            state: ArcSwap::from_pointee(CacheState::default()),
        }
    }

    fn parse_key<'a>(
        &self,
        key: &'a str,
    ) -> Result<(&'a str, &'a str, &'a str, &'a str), AnyError> {
        let parts: Vec<&str> = key.split('_').collect();
        if parts.len() != 4 {
            return Err("Invalid key format".into());
        }

        Ok((parts[0], parts[1], parts[2], parts[3]))
    }

    fn shard_dir(&self, symbol: &str) -> std::path::PathBuf {
        let mut path = std::path::PathBuf::from(&self.root);
        path.push(symbol);
        path
    }

    fn shard_file_path(&self, key: &str) -> Result<std::path::PathBuf, AnyError> {
        let (symbol, range, interval, prepost) = self.parse_key(key)?;

        let mut path = self.shard_dir(symbol);
        path.push(format!("{}_{}_{}", range, interval, prepost));
        path.set_extension("json");
        Ok(path)
    }

    fn should_refresh(data: &RawStockData) -> Result<bool, AnyError> {
        let last_refreshed = DateTime::parse_from_rfc3339(&data.last_refreshed)
            .map(|dt| dt.with_timezone(&Utc))
            .map_err(|_| "Invalid date format in cache")?;

        let now = Utc::now();

        let same_day_after_hours = now.hour() >= 16
            && last_refreshed.day() == now.day()
            && last_refreshed.month() == now.month()
            && last_refreshed.year() == now.year();

        let fresh = now - last_refreshed < Duration::minutes(5);

        Ok(!(same_day_after_hours || fresh))
    }

    async fn load_entry_from_disk(&self, key: &str) -> Result<Option<RawStockData>, AnyError> {
        let path = self.shard_file_path(key)?;

        match fs::read_to_string(&path).await {
            Ok(json) => {
                let data: RawStockData = serde_json::from_str(&json)?;
                Ok(Some(data))
            }
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(err) => Err(err.into()),
        }
    }

    async fn save_entry_to_disk(&self, key: &str, value: &RawStockData) -> Result<(), AnyError> {
        let path = self.shard_file_path(key)?;

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }

        let json = serde_json::to_string(value)?;
        fs::write(path, json).await?;
        Ok(())
    }

    async fn fetch_data_from_provider(&self, key: &str) -> Result<RawStockData, AnyError> {
        let (symbol, range, interval, prepost) = self.parse_key(key)?;

        let data = get_stock_data(
            symbol.to_string(),
            range.to_string(),
            interval.to_string(),
            prepost == "true",
        )
        .await;

        match data {
            Ok(data) => Ok(data),
            Err(e) => Err(format!("Error fetching data from provider: {}", e).into()),
        }
    }

    fn upsert_memory(&self, key: &str, value: Arc<RawStockData>) {
        self.state.rcu(|current| {
            let mut next = (**current).clone();
            next.data.insert(key.to_string(), value.clone());
            Arc::new(next)
        });
    }

    pub async fn check_cache(&self, key: &str) -> Result<Arc<RawStockData>, AnyError> {
        info!("Checking cache for key '{}'", key);

        {
            let snapshot = self.state.load();
            if let Some(data) = snapshot.data.get(key) {
                if !Self::should_refresh(data.as_ref())? {
                    return Ok(data.clone());
                }
            }
        }

        if let Some(disk_data) = self.load_entry_from_disk(key).await? {
            if !Self::should_refresh(&disk_data)? {
                let disk_data = Arc::new(disk_data);
                self.upsert_memory(key, disk_data.clone());
                return Ok(disk_data);
            }
        }

        info!(
            "Cache miss or stale data for key '{}', fetching new data",
            key
        );
        let fresh = Arc::new(self.fetch_data_from_provider(key).await?);

        self.upsert_memory(key, fresh.clone());
        self.save_entry_to_disk(key, fresh.as_ref()).await?;

        Ok(fresh)
    }
}
