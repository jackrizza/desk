# API Notes

The backend is served by the `openapi` crate and exposes routes under `/api`.

## Utility

### `GET /api/hello`

Simple health/demo endpoint.

## Market Data

### `GET /api/stock_data`

Query params:

- `symbol`
- `range`
- `interval`
- `prepost`

Returns raw stock bars for charting.

### `GET /api/indicators`

Query params:

- `symbol`
- `range`
- `interval`
- `prepost`
- `indicators` as a comma-separated list

Returns indicator results plus unsupported indicators.

## Paper Trading

Paper trading is simulated only. No live broker integration exists, and market orders fill
immediately using the latest available cached market price.

### `POST /api/paper/accounts`

Creates a paper trading account.

Body:

- `name`
- `starting_cash`

### `GET /api/paper/accounts`

Returns all paper trading accounts ordered by newest first.

### `GET /api/paper/accounts/:account_id`

Returns a single paper trading account.

### `GET /api/paper/accounts/:account_id/summary`

Returns:

- `account`
- `positions`
- `open_orders`
- `recent_fills`
- `equity_estimate`
- `total_cost_basis`
- `total_market_value`
- `total_unrealized_gain`
- `total_unrealized_gain_percent`

Summary `positions` include display-only valuation fields:

- `current_price`
- `market_value`
- `cost_basis`
- `unrealized_gain`
- `unrealized_gain_percent`

Paper summary valuation uses the latest close from the cached `1d` / `1m` stock
dataset. If that lookup fails, the summary falls back to the position average
price and reports zero unrealized gain/loss for that position.

### `POST /api/paper/orders`

Creates and immediately executes a v1 market paper order.

Body:

- `account_id`
- `symbol`
- `side` as `buy` or `sell`
- `order_type` as `market`
- `quantity`
- `requested_price` optional
- `source` optional, such as `manual` or `engine`

For `source = "engine"` orders, `openapi` also revalidates persisted strategy paper-trading
settings and risk controls before accepting the order. This includes the kill switch, backend
trading-enabled flag, paper account match, symbol allow/block lists, and conservative per-trade
limits.

### `GET /api/paper/accounts/:account_id/orders`
### `GET /api/paper/accounts/:account_id/fills`
### `GET /api/paper/accounts/:account_id/positions`
### `GET /api/paper/accounts/:account_id/events`

Returns historical paper trading state for the account.

### `POST /api/paper/orders/:order_id/cancel`

Cancels a pending paper order. In v1, most market orders are already filled immediately, so this
normally returns an error for filled orders.

## Strategy Trading Config

Strategy live trading settings are persisted in the backend so the engine can continue evaluating
paper-enabled strategies after the browser closes.

### `GET /api/strategies/:strategy_id/trading-config`

Returns the persisted trading config for the strategy. If no config exists yet, the API returns the
default disabled config with `trading_mode = "off"`.

### `PUT /api/strategies/:strategy_id/trading-config`

Persists strategy trading settings.

Body:

- `trading_mode` as `off`, `paper`, or `real`
- `paper_account_id` optional, but required for `paper`
- `is_enabled`

Rules:

- `off` always disables trading and clears the paper account
- `paper` requires a valid active paper account
- `real` is rejected in v1

### `GET /api/engine/config/strategies`

Returns only engine-runnable strategy configs. In v1 this means enabled `paper` configs only, and
only when the persisted risk config has trading enabled and the kill switch is off.

Each returned strategy now includes:

- `strategy_definition`
- `risk`
- `risk_config`

## Strategy Risk Config

### `GET /api/strategies/:strategy_id/risk-config`

Returns the persisted risk config for the strategy, or safe defaults if no row exists yet.

### `PUT /api/strategies/:strategy_id/risk-config`

Updates risk controls for the strategy.

Body:

- `max_dollars_per_trade`
- `max_quantity_per_trade`
- `max_position_value_per_symbol`
- `max_total_exposure`
- `max_open_positions`
- `max_daily_trades`
- `max_daily_loss`
- `cooldown_seconds`
- `allowlist_symbols`
- `blocklist_symbols`
- `is_trading_enabled`
- `kill_switch_enabled`

Rules:

- numeric limits must be positive when provided
- count limits must be `>= 0`
- `cooldown_seconds` must be `>= 0`
- allowlist and blocklist symbols are normalized uppercase
- the same symbol cannot appear in both allowlist and blocklist

### `POST /api/strategies/:strategy_id/kill-switch`

Immediately sets `kill_switch_enabled = true` and `is_trading_enabled = false`.

### `POST /api/strategies/:strategy_id/resume-trading`

Clears the kill switch and re-enables backend risk-config trading for the strategy.

## Strategy Execution State

### `GET /api/strategies/:strategy_id/runtime-state`

Returns persisted engine runtime state rows for the strategy.

### `GET /api/strategies/:strategy_id/signals`

Returns recent persisted strategy signals, including:

- `status`
- `risk_decision`
- `risk_reason`
- `order_id`

### `POST /api/engine/strategy-signals`

Creates a persisted signal from the engine.

### `POST /api/engine/strategy-signals/:signal_id`

Updates a persisted signal status after risk evaluation or paper order submission.

### `POST /api/engine/strategy-runtime-state`

Upserts runtime state rows from the engine.

## Projects

### `GET /api/projects`

Returns all projects.

### `POST /api/projects`

Creates a project.

Current project fields include:

- `id`
- `name`
- `description`
- `strategy`
- `strategy_json`
- `strategy_status`
- `created_at`
- `updated_at`
- `symbols`
- `interval`
- `range`
- `prepost`

### `GET /api/projects/:project_id`

Returns a single project.

### `PUT /api/projects/:project_id`

Updates a project, including its saved strategy.

### `DELETE /api/projects/:project_id`

Deletes a project.

## Portfolios

The frontend Portfolio tab now supports selecting:

- `Manual Portfolio` for the existing manual-entry workflow
- paper trading accounts backed by `/api/paper/accounts`
- future live accounts through the same selector shape

Live account selections are currently display-only placeholders. No live broker integration,
credentials, or real trade execution are implemented.

### `GET /api/portfolios`
### `POST /api/portfolios`
### `GET /api/portfolios/:portfolio_id`
### `PUT /api/portfolios/:portfolio_id`
### `DELETE /api/portfolios/:portfolio_id`

Portfolio payloads include:

- `id`
- `name`
- `description`
- `created_at`
- `updated_at`
- `positions`

## Positions

### `GET /api/portfolios/:portfolio_id/positions`
### `POST /api/portfolios/:portfolio_id/positions`
### `GET /api/portfolios/:portfolio_id/positions/:symbol/:position_opened_at`
### `PUT /api/portfolios/:portfolio_id/positions/:symbol/:position_opened_at`
### `DELETE /api/portfolios/:portfolio_id/positions/:symbol/:position_opened_at`

Positions are identified by:

- `portfolio_id`
- `symbol`
- `position_opened_at`

## Frontend Account Selection

The Portfolio tab persists the selected account locally under:

- `portfolio:selectedAccount`

Paper account views use `/api/paper/accounts/:account_id/summary` to render:

- account name
- cash balance
- estimated equity
- starting cash
- return estimate
- unrealized gain/loss dollars and percentage
- positions
- recent fills

## Source of Truth

For the exact schema and route behavior, use:

- [main.rs](C:\Users\jack\OneDrive\Documents\Code\Rust\desk\apps\openapi\src\main.rs)
- [lib.rs](C:\Users\jack\OneDrive\Documents\Code\Rust\desk\crates\database\src\lib.rs)
- [api.ts](C:\Users\jack\OneDrive\Documents\Code\Rust\desk\frontend\app\lib\api.ts)
# Trader API

Trader endpoints live under `/api`. Trader OpenAI API keys are accepted only on create/update and are never returned by public Trader endpoints.

Public UI endpoints:
- `POST /traders`, `GET /traders`, `GET /traders/:trader_id`, `PUT /traders/:trader_id`, `DELETE /traders/:trader_id`
- `POST /traders/:trader_id/start`, `/stop`, `/pause`
- `GET /traders/:trader_id/events`, `/runtime-state`, `/trade-proposals`
- `POST /traders/:trader_id/trade-proposals/:proposal_id/approve`
- `POST /traders/:trader_id/trade-proposals/:proposal_id/reject`

Internal engine endpoints:
- `GET /engine/config/traders`
- `POST /engine/traders/:trader_id/runtime-state`
- `POST /engine/traders/:trader_id/events`
- `POST /engine/traders/:trader_id/trade-proposals`

`GET /engine/config/traders` returns active running traders and currently includes the stored OpenAI key for local engine execution. This is for v1 local Docker engine/openapi communication only and must be protected before public deployment.

# Data Source API

The frontend manages source configuration through OpenAPI only:
- `POST /data-sources`
- `GET /data-sources`
- `GET /data-sources/:source_id`
- `PUT /data-sources/:source_id`
- `DELETE /data-sources/:source_id`
- `GET /data-sources/:source_id/items`
- `GET /data-sources/:source_id/events`
- `GET /traders/:trader_id/data-sources`
- `PUT /traders/:trader_id/data-sources`

Supported v1 source types are `rss`, `web_page`, `manual_note`, and `placeholder_api`. RSS and web page sources require an `http://` or `https://` URL. Deletes are soft deletes that disable the source and preserve items/events.

# Chat Commands

`POST /chat/commands` parses and executes chat-driven app commands for Traders, Data Sources, and Trader/Data Source assignments.

Request fields:
- `message`: user chat text.
- `context`: optional UI context such as active page or selected trader.
- `confirmation_token` and `confirmed`: used when replying to a confirmation prompt.

Responses include `handled`, `reply`, `actions`, and optional confirmation fields. Destructive or sensitive actions return `requires_confirmation = true` and must be confirmed before execution.

Supported v1 actions include creating/updating/listing/starting/stopping/pausing/deleting Traders, creating/updating/listing/disabling Data Sources, showing status/items, and assigning/unassigning Data Sources to Traders. Chat commands never expose saved API keys.

Parsing first uses deterministic backend command patterns. If `CHAT_COMMAND_OPENAI_API_KEY` or `OPENAI_API_KEY` is configured, unrecognized messages can be parsed by OpenAI using `CHAT_COMMAND_MODEL` or the default `gpt-5.2`.
