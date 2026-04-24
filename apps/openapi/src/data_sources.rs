use database::Database;
use models::data_sources::{
    CreateDataSourceRequest, DataSource, DataSourceEventsResponse, DataSourceItemsResponse,
    TraderDataSourcesResponse, UpdateDataSourceRequest, UpdateTraderDataSourcesRequest,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataSourceErrorKind {
    BadRequest,
    NotFound,
    Internal,
}

#[derive(Debug)]
pub struct DataSourceApiError {
    pub kind: DataSourceErrorKind,
    pub message: String,
}

impl DataSourceApiError {
    fn bad_request(message: impl Into<String>) -> Self {
        Self {
            kind: DataSourceErrorKind::BadRequest,
            message: message.into(),
        }
    }

    fn not_found(message: impl Into<String>) -> Self {
        Self {
            kind: DataSourceErrorKind::NotFound,
            message: message.into(),
        }
    }

    fn internal(message: impl Into<String>) -> Self {
        Self {
            kind: DataSourceErrorKind::Internal,
            message: message.into(),
        }
    }
}

pub async fn create(
    database: &Database,
    request: CreateDataSourceRequest,
) -> Result<DataSource, DataSourceApiError> {
    validate_create(&request)?;
    database
        .create_data_source(&request)
        .await
        .map_err(|err| DataSourceApiError::internal(format!("failed to create data source: {err}")))
}

pub async fn list(database: &Database) -> Result<Vec<DataSource>, DataSourceApiError> {
    database
        .list_data_sources()
        .await
        .map_err(|err| DataSourceApiError::internal(format!("failed to list data sources: {err}")))
}

pub async fn get(database: &Database, source_id: &str) -> Result<DataSource, DataSourceApiError> {
    database
        .get_data_source(source_id)
        .await
        .map_err(|err| DataSourceApiError::internal(format!("failed to load data source: {err}")))?
        .ok_or_else(|| DataSourceApiError::not_found("data source not found"))
}

pub async fn update(
    database: &Database,
    source_id: &str,
    request: UpdateDataSourceRequest,
) -> Result<DataSource, DataSourceApiError> {
    validate_update(&request)?;
    database
        .update_data_source(source_id, &request)
        .await
        .map_err(|err| {
            DataSourceApiError::internal(format!("failed to update data source: {err}"))
        })?
        .ok_or_else(|| DataSourceApiError::not_found("data source not found"))
}

pub async fn delete(database: &Database, source_id: &str) -> Result<(), DataSourceApiError> {
    match database.disable_data_source(source_id).await {
        Ok(true) => Ok(()),
        Ok(false) => Err(DataSourceApiError::not_found("data source not found")),
        Err(err) => Err(DataSourceApiError::internal(format!(
            "failed to disable data source: {err}"
        ))),
    }
}

pub async fn items(
    database: &Database,
    source_id: &str,
) -> Result<DataSourceItemsResponse, DataSourceApiError> {
    let _ = get(database, source_id).await?;
    Ok(DataSourceItemsResponse {
        items: database
            .list_data_source_items(source_id, 100)
            .await
            .map_err(|err| DataSourceApiError::internal(format!("failed to list items: {err}")))?,
    })
}

pub async fn events(
    database: &Database,
    source_id: &str,
) -> Result<DataSourceEventsResponse, DataSourceApiError> {
    let _ = get(database, source_id).await?;
    Ok(DataSourceEventsResponse {
        events: database
            .list_data_source_events(source_id, 100)
            .await
            .map_err(|err| DataSourceApiError::internal(format!("failed to list events: {err}")))?,
    })
}

pub async fn trader_sources(
    database: &Database,
    trader_id: &str,
) -> Result<TraderDataSourcesResponse, DataSourceApiError> {
    Ok(TraderDataSourcesResponse {
        data_sources: database
            .list_trader_data_sources(trader_id)
            .await
            .map_err(|err| {
                DataSourceApiError::internal(format!("failed to list trader data sources: {err}"))
            })?,
    })
}

pub async fn replace_trader_sources(
    database: &Database,
    trader_id: &str,
    request: UpdateTraderDataSourcesRequest,
) -> Result<TraderDataSourcesResponse, DataSourceApiError> {
    database
        .replace_trader_data_sources(trader_id, &request.data_source_ids)
        .await
        .map_err(|err| {
            DataSourceApiError::internal(format!("failed to replace trader data sources: {err}"))
        })?;
    trader_sources(database, trader_id).await
}

fn validate_create(request: &CreateDataSourceRequest) -> Result<(), DataSourceApiError> {
    if request.name.trim().is_empty() {
        return Err(DataSourceApiError::bad_request("name must be non-empty"));
    }
    validate_source_type(&request.source_type)?;
    validate_poll_interval(request.poll_interval_seconds.unwrap_or(30))?;
    validate_url(&request.source_type, request.url.as_deref())
}

fn validate_update(request: &UpdateDataSourceRequest) -> Result<(), DataSourceApiError> {
    if request
        .name
        .as_deref()
        .map(str::trim)
        .is_some_and(str::is_empty)
    {
        return Err(DataSourceApiError::bad_request("name must be non-empty"));
    }
    if let Some(source_type) = &request.source_type {
        validate_source_type(source_type)?;
        validate_url(source_type, request.url.as_deref())?;
    }
    if let Some(interval) = request.poll_interval_seconds {
        validate_poll_interval(interval)?;
    }
    Ok(())
}

fn validate_source_type(source_type: &str) -> Result<(), DataSourceApiError> {
    match source_type.trim() {
        "rss" | "web_page" | "manual_note" | "placeholder_api" => Ok(()),
        _ => Err(DataSourceApiError::bad_request("invalid source_type")),
    }
}

fn validate_poll_interval(interval: i64) -> Result<(), DataSourceApiError> {
    if interval < 30 {
        return Err(DataSourceApiError::bad_request(
            "poll_interval_seconds must be at least 30",
        ));
    }
    Ok(())
}

fn validate_url(source_type: &str, url: Option<&str>) -> Result<(), DataSourceApiError> {
    if source_type == "rss" || source_type == "web_page" {
        let Some(url) = url.map(str::trim).filter(|value| !value.is_empty()) else {
            return Err(DataSourceApiError::bad_request("url is required"));
        };
        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Err(DataSourceApiError::bad_request(
                "url must start with http:// or https://",
            ));
        }
    }
    Ok(())
}
