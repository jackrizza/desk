use cache::Cache;
use database::{Database, ExecutePaperMarketOrderParams};
use models::paper::{
    CreatePaperAccountRequest, CreatePaperOrderRequest, PaperAccount, PaperAccountEvent,
    PaperAccountSummaryResponse, PaperFill, PaperOrder, PaperOrderExecutionResponse, PaperPosition,
    PaperPositionSummary,
};
use tracing::{info, warn};

use crate::strategy_risk;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaperErrorKind {
    BadRequest,
    NotFound,
    Conflict,
    Internal,
}

#[derive(Debug)]
pub struct PaperApiError {
    pub kind: PaperErrorKind,
    pub message: String,
}

impl PaperApiError {
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self {
            kind: PaperErrorKind::BadRequest,
            message: message.into(),
        }
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self {
            kind: PaperErrorKind::NotFound,
            message: message.into(),
        }
    }

    pub fn conflict(message: impl Into<String>) -> Self {
        Self {
            kind: PaperErrorKind::Conflict,
            message: message.into(),
        }
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self {
            kind: PaperErrorKind::Internal,
            message: message.into(),
        }
    }
}

impl std::fmt::Display for PaperApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for PaperApiError {}

pub async fn create_account(
    database: &Database,
    request: CreatePaperAccountRequest,
) -> Result<PaperAccount, PaperApiError> {
    validate_create_account_request(&request)?;
    info!(
        name = %request.name,
        starting_cash = request.starting_cash,
        "creating paper trading account"
    );

    database
        .create_paper_account(request.name.trim(), request.starting_cash)
        .await
        .map_err(|err| PaperApiError::internal(format!("failed to create paper account: {err}")))
}

pub async fn list_accounts(database: &Database) -> Result<Vec<PaperAccount>, PaperApiError> {
    database
        .list_paper_accounts()
        .await
        .map_err(|err| PaperApiError::internal(format!("failed to list paper accounts: {err}")))
}

pub async fn get_account(
    database: &Database,
    account_id: &str,
) -> Result<PaperAccount, PaperApiError> {
    database
        .get_paper_account(account_id)
        .await
        .map_err(|err| PaperApiError::internal(format!("failed to load paper account: {err}")))?
        .ok_or_else(|| PaperApiError::not_found("paper account not found"))
}

pub async fn get_account_summary(
    database: &Database,
    cache: &Cache,
    account_id: &str,
) -> Result<PaperAccountSummaryResponse, PaperApiError> {
    let summary_parts = database
        .get_paper_account_summary_parts(account_id)
        .await
        .map_err(|err| {
            PaperApiError::internal(format!("failed to load paper account summary: {err}"))
        })?
        .ok_or_else(|| PaperApiError::not_found("paper account not found"))?;

    let mut equity_estimate = summary_parts.account.cash_balance;
    let mut total_cost_basis = 0.0;
    let mut total_market_value = 0.0;
    let mut positions = Vec::with_capacity(summary_parts.positions.len());

    for position in &summary_parts.positions {
        let (market_price, current_price) =
            match get_latest_paper_price(cache, &position.symbol).await {
                Ok(price) => (price, Some(price)),
                Err(err) => {
                    warn!(
                        symbol = %position.symbol,
                        error = %err,
                        "latest price lookup failed for summary; falling back to average price"
                    );
                    (position.average_price, None)
                }
            };

        let cost_basis = position.quantity * position.average_price;
        let market_value = position.quantity * market_price;
        let unrealized_gain = market_value - cost_basis;
        let unrealized_gain_percent = if cost_basis.abs() > f64::EPSILON {
            (unrealized_gain / cost_basis) * 100.0
        } else {
            0.0
        };

        total_cost_basis += cost_basis;
        total_market_value += market_value;
        equity_estimate += market_value;
        positions.push(PaperPositionSummary {
            id: position.id.clone(),
            account_id: position.account_id.clone(),
            symbol: position.symbol.clone(),
            quantity: position.quantity,
            average_price: position.average_price,
            current_price,
            market_value: Some(market_value),
            cost_basis,
            unrealized_gain,
            unrealized_gain_percent,
            realized_pnl: position.realized_pnl,
            created_at: position.created_at.clone(),
            updated_at: position.updated_at.clone(),
        });
    }

    let total_unrealized_gain = total_market_value - total_cost_basis;
    let total_unrealized_gain_percent = if total_cost_basis.abs() > f64::EPSILON {
        (total_unrealized_gain / total_cost_basis) * 100.0
    } else {
        0.0
    };

    Ok(PaperAccountSummaryResponse {
        account: summary_parts.account,
        positions,
        open_orders: summary_parts.open_orders,
        recent_fills: summary_parts.recent_fills,
        equity_estimate,
        total_cost_basis,
        total_market_value,
        total_unrealized_gain,
        total_unrealized_gain_percent,
    })
}

pub async fn execute_paper_market_order(
    database: &Database,
    cache: &Cache,
    request: CreatePaperOrderRequest,
) -> Result<PaperOrderExecutionResponse, PaperApiError> {
    // TODO: Extend this flow with commissions, slippage, partial fills, limit orders,
    // and short selling once the paper trading model grows beyond immediate-fill market orders.
    let normalized = validate_create_order_request(request)?;

    info!(
        account_id = %normalized.account_id,
        symbol = %normalized.symbol,
        side = %normalized.side,
        quantity = normalized.quantity,
        source = %normalized.source,
        "paper order received"
    );

    let account = get_account(database, &normalized.account_id).await?;
    if !account.is_active {
        info!(account_id = %account.id, "paper order rejected because account is inactive");
        return Err(PaperApiError::conflict("paper account is inactive"));
    }

    let fill_price = get_latest_paper_price(cache, &normalized.symbol).await?;
    info!(
        symbol = %normalized.symbol,
        fill_price = fill_price,
        "latest paper price resolved"
    );

    if normalized.source == "engine" {
        let Some(strategy_id) = normalized.strategy_id.as_deref() else {
            return Err(PaperApiError::bad_request(
                "engine paper orders must include strategy_id",
            ));
        };

        strategy_risk::validate_engine_order(
            database,
            strategy_id,
            &normalized.account_id,
            &normalized.symbol,
            normalized.quantity,
            fill_price,
        )
        .await
        .map_err(|error| match error.kind {
            strategy_risk::StrategyRiskErrorKind::BadRequest => {
                PaperApiError::bad_request(error.message)
            }
            strategy_risk::StrategyRiskErrorKind::NotFound => {
                PaperApiError::not_found(error.message)
            }
            strategy_risk::StrategyRiskErrorKind::Conflict => {
                PaperApiError::conflict(error.message)
            }
            strategy_risk::StrategyRiskErrorKind::Internal => {
                PaperApiError::internal(error.message)
            }
        })?;
    }

    if normalized.side == "buy" {
        let estimated_notional = normalized.quantity * fill_price;
        if estimated_notional > account.cash_balance {
            info!(
                account_id = %account.id,
                cash_balance = account.cash_balance,
                estimated_notional = estimated_notional,
                "paper order rejected for insufficient cash"
            );
            return Err(PaperApiError::conflict("insufficient cash"));
        }
    } else {
        let existing_position = database
            .get_paper_position(&normalized.account_id, &normalized.symbol)
            .await
            .map_err(|err| {
                PaperApiError::internal(format!("failed to load paper position: {err}"))
            })?;

        let Some(position) = existing_position else {
            info!(
                account_id = %normalized.account_id,
                symbol = %normalized.symbol,
                "paper order rejected because no position exists"
            );
            return Err(PaperApiError::conflict("insufficient position quantity"));
        };

        if position.quantity < normalized.quantity {
            info!(
                account_id = %normalized.account_id,
                symbol = %normalized.symbol,
                held_quantity = position.quantity,
                requested_quantity = normalized.quantity,
                "paper order rejected because sell quantity exceeds holdings"
            );
            return Err(PaperApiError::conflict("insufficient position quantity"));
        }
    }

    let result = database
        .execute_paper_market_order(ExecutePaperMarketOrderParams {
            account_id: normalized.account_id.clone(),
            symbol: normalized.symbol.clone(),
            side: normalized.side.clone(),
            order_type: normalized.order_type.clone(),
            quantity: normalized.quantity,
            requested_price: normalized.requested_price,
            source: normalized.source.clone(),
            trader_id: normalized.trader_id.clone(),
            strategy_id: normalized.strategy_id.clone(),
            signal_id: normalized.signal_id.clone(),
            proposal_id: normalized.proposal_id.clone(),
            fill_price,
        })
        .await
        .map_err(map_database_execution_error)?;

    info!(
        order_id = %result.order.id,
        account_id = %result.account.id,
        symbol = %result.order.symbol,
        side = %result.order.side,
        price = fill_price,
        quantity = result.fill.quantity,
        cash_balance = result.account.cash_balance,
        "paper order filled"
    );

    Ok(PaperOrderExecutionResponse {
        order: result.order,
        fill: Some(result.fill),
        position: Some(result.position),
        cash_balance: result.account.cash_balance,
    })
}

pub async fn list_orders(
    database: &Database,
    account_id: &str,
) -> Result<Vec<PaperOrder>, PaperApiError> {
    ensure_account_exists(database, account_id).await?;
    database
        .list_paper_orders(account_id)
        .await
        .map_err(|err| PaperApiError::internal(format!("failed to list paper orders: {err}")))
}

pub async fn list_fills(
    database: &Database,
    account_id: &str,
) -> Result<Vec<PaperFill>, PaperApiError> {
    ensure_account_exists(database, account_id).await?;
    database
        .list_paper_fills(account_id)
        .await
        .map_err(|err| PaperApiError::internal(format!("failed to list paper fills: {err}")))
}

pub async fn list_positions(
    database: &Database,
    account_id: &str,
) -> Result<Vec<PaperPosition>, PaperApiError> {
    ensure_account_exists(database, account_id).await?;
    database
        .list_paper_positions(account_id)
        .await
        .map_err(|err| PaperApiError::internal(format!("failed to list paper positions: {err}")))
}

pub async fn list_events(
    database: &Database,
    account_id: &str,
) -> Result<Vec<PaperAccountEvent>, PaperApiError> {
    ensure_account_exists(database, account_id).await?;
    database.list_paper_events(account_id).await.map_err(|err| {
        PaperApiError::internal(format!("failed to list paper account events: {err}"))
    })
}

pub async fn cancel_order(
    database: &Database,
    order_id: &str,
) -> Result<PaperOrder, PaperApiError> {
    info!(order_id = %order_id, "paper order cancellation requested");

    let Some(order) = database
        .cancel_paper_order(order_id)
        .await
        .map_err(|err| PaperApiError::internal(format!("failed to cancel paper order: {err}")))?
    else {
        return Err(PaperApiError::not_found("paper order not found"));
    };

    if order.status != "cancelled" {
        info!(
            order_id = %order.id,
            status = %order.status,
            "paper order cancellation rejected"
        );
        return Err(PaperApiError::conflict("paper order cannot be cancelled"));
    }

    info!(order_id = %order.id, "paper order cancelled");
    Ok(order)
}

async fn ensure_account_exists(database: &Database, account_id: &str) -> Result<(), PaperApiError> {
    let _ = get_account(database, account_id).await?;
    Ok(())
}

async fn get_latest_paper_price(cache: &Cache, symbol: &str) -> Result<f64, PaperApiError> {
    let key = paper_price_cache_key(symbol);
    let data = cache.check_cache(&key).await.map_err(|err| {
        PaperApiError::bad_request(format!("price lookup failure for {symbol}: {err}"))
    })?;

    let Some(entry) = data.stock_data.last() else {
        return Err(PaperApiError::bad_request(format!(
            "price lookup failure for {symbol}: no market data returned"
        )));
    };

    let price = entry.close.parse::<f64>().map_err(|err| {
        PaperApiError::bad_request(format!("price lookup failure for {symbol}: {err}"))
    })?;

    if !price.is_finite() || price <= 0.0 {
        return Err(PaperApiError::bad_request(format!(
            "price lookup failure for {symbol}: invalid latest price"
        )));
    }

    Ok(price)
}

fn paper_price_cache_key(symbol: &str) -> String {
    format!("{symbol}_1d_1m_false")
}

fn validate_create_account_request(
    request: &CreatePaperAccountRequest,
) -> Result<(), PaperApiError> {
    if request.name.trim().is_empty() {
        return Err(PaperApiError::bad_request("account name must be non-empty"));
    }

    if request.starting_cash <= 0.0 {
        return Err(PaperApiError::bad_request(
            "starting_cash must be greater than 0",
        ));
    }

    Ok(())
}

fn validate_create_order_request(
    request: CreatePaperOrderRequest,
) -> Result<NormalizedOrderRequest, PaperApiError> {
    if request.account_id.trim().is_empty() {
        return Err(PaperApiError::bad_request("account_id must be non-empty"));
    }

    let symbol = normalize_symbol(&request.symbol)?;
    let side = request.side.trim().to_ascii_lowercase();
    if side != "buy" && side != "sell" {
        return Err(PaperApiError::bad_request(
            "invalid side: must be buy or sell",
        ));
    }

    let order_type = request.order_type.trim().to_ascii_lowercase();
    if order_type != "market" {
        return Err(PaperApiError::bad_request(
            "invalid order type: only market is supported",
        ));
    }

    if request.quantity <= 0.0 {
        return Err(PaperApiError::bad_request(
            "quantity must be greater than 0",
        ));
    }

    Ok(NormalizedOrderRequest {
        account_id: request.account_id.trim().to_string(),
        symbol,
        side,
        order_type,
        quantity: request.quantity,
        requested_price: request.requested_price,
        source: match request.source {
            Some(source) if !source.trim().is_empty() => source.trim().to_string(),
            _ => "manual".to_string(),
        },
        trader_id: request
            .trader_id
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty()),
        strategy_id: request
            .strategy_id
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty()),
        signal_id: request
            .signal_id
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty()),
        proposal_id: request
            .proposal_id
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty()),
    })
}

fn normalize_symbol(symbol: &str) -> Result<String, PaperApiError> {
    let normalized = symbol.trim().to_ascii_uppercase();
    if normalized.is_empty() {
        return Err(PaperApiError::bad_request("symbol must be non-empty"));
    }

    Ok(normalized)
}

fn map_database_execution_error(err: sqlx::Error) -> PaperApiError {
    match err {
        sqlx::Error::RowNotFound => PaperApiError::not_found("paper account not found"),
        sqlx::Error::Protocol(message) if message.contains("insufficient cash") => {
            PaperApiError::conflict("insufficient cash")
        }
        sqlx::Error::Protocol(message) if message.contains("insufficient position quantity") => {
            PaperApiError::conflict("insufficient position quantity")
        }
        sqlx::Error::Protocol(message) if message.contains("inactive") => {
            PaperApiError::conflict("paper account is inactive")
        }
        other => PaperApiError::internal(format!("failed to execute paper order: {other}")),
    }
}

struct NormalizedOrderRequest {
    account_id: String,
    symbol: String,
    side: String,
    order_type: String,
    quantity: f64,
    requested_price: Option<f64>,
    source: String,
    trader_id: Option<String>,
    strategy_id: Option<String>,
    signal_id: Option<String>,
    proposal_id: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::{normalize_symbol, validate_create_account_request, validate_create_order_request};
    use models::paper::{CreatePaperAccountRequest, CreatePaperOrderRequest};

    #[test]
    fn normalizes_symbol_to_uppercase() {
        assert_eq!(normalize_symbol(" aapl ").unwrap(), "AAPL");
    }

    #[test]
    fn rejects_invalid_account_cash() {
        let err = validate_create_account_request(&CreatePaperAccountRequest {
            name: "Test".to_string(),
            starting_cash: 0.0,
        })
        .unwrap_err();

        assert_eq!(err.message, "starting_cash must be greater than 0");
    }

    #[test]
    fn rejects_non_market_order_types() {
        let err = validate_create_order_request(CreatePaperOrderRequest {
            account_id: "acct".to_string(),
            symbol: "AAPL".to_string(),
            side: "buy".to_string(),
            order_type: "limit".to_string(),
            quantity: 1.0,
            requested_price: Some(150.0),
            source: None,
            trader_id: None,
            strategy_id: None,
            signal_id: None,
            proposal_id: None,
        })
        .unwrap_err();

        assert_eq!(err.message, "invalid order type: only market is supported");
    }
}
