# API Notes

The backend is served by the `openapi` crate and exposes routes under `/api`.

## Utility

### `GET /api/hello`

Simple health/demo endpoint.

## Trader Memories

Trader memories persist short, actionable lessons from answered channel questions,
reviews, user preferences, and decisions. They are app context only and never execute
trades.

Endpoints:

- `GET /api/traders/:trader_id/memories`
- `POST /api/traders/:trader_id/memories`
- `PUT /api/traders/:trader_id/memories/:memory_id`
- `DELETE /api/traders/:trader_id/memories/:memory_id`
- `POST /api/traders/:trader_id/memories/search`
- `POST /api/engine/traders/:trader_id/memories`
- `POST /api/engine/traders/:trader_id/memories/:memory_id/mark-used`

`GET` supports `status`, `memory_type`, and `topic` filters. `DELETE` soft-archives
the memory by setting `status = archived`. Search is v1 text matching over topic and
summary; there is no vector database.

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
- `GET /traders/:trader_id/proposals`
- `GET /traders/:trader_id/proposals/latest`
- `GET /traders/:trader_id/proposals/active`
- `GET /traders/:trader_id/proposals/:proposal_id`
- `POST /traders/:trader_id/proposals/:proposal_id/review`
- `POST /traders/:trader_id/proposals/:proposal_id/accept`
- `POST /traders/:trader_id/proposals/:proposal_id/reject`
- `GET /traders/:trader_id/symbols`
- `POST /traders/:trader_id/symbols`
- `PUT /traders/:trader_id/symbols/:symbol_id`
- `DELETE /traders/:trader_id/symbols/:symbol_id`
- `POST /traders/:trader_id/symbols/bulk`
- `POST /traders/:trader_id/symbols/suggest`
- `POST /traders/:trader_id/symbols/:symbol_id/archive`
- `POST /traders/:trader_id/symbols/:symbol_id/activate`
- `POST /traders/:trader_id/symbols/:symbol_id/reject`

Internal engine endpoints:
- `GET /engine/config/traders`
- `POST /engine/traders/:trader_id/runtime-state`
- `POST /engine/traders/:trader_id/events`
- `POST /engine/traders/:trader_id/trade-proposals`
- `POST /engine/traders/:trader_id/proposals`

`GET /engine/config/traders` returns active running traders and currently includes the stored OpenAI key for local engine execution. This is for v1 local Docker engine/openapi communication only and must be protected before public deployment.

## Trader Symbol Universe

Each Trader can maintain a persisted symbol universe/watchlist in `trader_symbols`. Symbols are normalized to uppercase and constrained by:
- `asset_type`: `stock`, `etf`, `index`, `crypto`, `other`
- `status`: `watching`, `candidate`, `active`, `rejected`, `archived`
- `source`: `manual`, `ai`, `import`, `engine`

`GET /traders/:trader_id/symbols` supports optional `status`, `asset_type`, and `source` query filters. Delete/archive keeps history by setting `status = archived`.

`POST /traders/:trader_id/symbols/suggest` asks OpenAI for up to 50 liquid stocks/ETFs that fit the Trader's perspective, assigned data sources, existing symbols, and optional focus. Suggestions are persisted as `source = ai` and `status = candidate`.

Engine trader config includes `tracked_symbols`; the engine evaluates only `active` and `watching` symbols. `rejected` and `archived` symbols are not evaluated. This feature does not directly execute trades.

## Trader Portfolio Proposals

Running Traders periodically create durable portfolio proposals in `trader_portfolio_proposals` with child rows in `trader_portfolio_proposal_actions`. The engine posts proposals through `POST /engine/traders/:trader_id/proposals`; new proposals supersede previous `proposed` proposals for that Trader.

Proposal statuses are `proposed`, `accepted`, `rejected`, `superseded`, `executed`, and `expired`. Accepted proposals become active plans with `plan_state = active`, `accepted_at`, optional `active_until`, expected duration, market basis, invalidation conditions, change thresholds, and replacement reason metadata. `GET /traders/:trader_id/proposals/active` returns the current accepted active plan.

Proposal actions use action types `buy`, `sell`, `hold`, `watch`, `activate_symbol`, `reject_symbol`, `reduce`, `increase`, and `no_action`. Actions can include entry, exit, limit, stop, enact-by, expected duration, and market price at creation fields. Risk decisions are attached as explanatory metadata. The engine proposal flow never submits orders.

Once a plan is active, the engine holds it until deterministic monitoring detects a material change such as plan expiry or symbol universe mismatch. Replacement proposals are created as `proposed`; the active plan remains active until the user accepts the replacement or the engine marks it invalidated/expired.

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

Supported v1 source types are `rss`, `web_page`, `manual_note`, `placeholder_api`, and `python_script`. RSS and web page sources require an `http://` or `https://` URL. Deletes are soft deletes that disable the source and preserve items/events.

Python script source endpoints:
- `GET /data-sources/:source_id/script`: returns the saved script, creating a starter script if needed.
- `PUT /data-sources/:source_id/script`: saves script text and updates its hash without executing it.
- `POST /data-sources/:source_id/script/build`: validates Python syntax and `collect(context)`, then persists build status/output.
- `GET /engine/data-sources/:source_id/script`: internal scrapper endpoint for enabled Python Script sources.

Script builds return:
- `success`
- `status` as `success` or `failed`
- `output`
- `script_hash`

Example collector:

```python
def collect(context):
    return {
        "items": [
            {
                "external_id": "hello-world",
                "title": "Hello from Python",
                "content": "This item was generated by a Python script.",
                "summary": "Python script test item.",
            }
        ]
    }
```

# Chat Commands

`POST /chat/commands` parses and executes chat-driven app commands for Traders, Data Sources, and Trader/Data Source assignments.

Request fields:
- `message`: user chat text.
- `context`: optional UI context such as active page or selected trader.
- `confirmation_token` and `confirmed`: used when replying to a confirmation prompt.

Responses include `handled`, `reply`, `actions`, and optional confirmation fields. Destructive or sensitive actions return `requires_confirmation = true` and must be confirmed before execution.

Supported v1 actions include creating/updating/listing/starting/stopping/pausing/deleting Traders, creating/updating/listing/disabling Data Sources, showing status/items, and assigning/unassigning Data Sources to Traders. Chat commands never expose saved API keys.

Parsing first uses deterministic backend command patterns. If `CHAT_COMMAND_OPENAI_API_KEY` or `OPENAI_API_KEY` is configured, unrecognized messages can be parsed by OpenAI using `CHAT_COMMAND_MODEL` or the default `gpt-5.2`.

## Trader Chat

`POST /traders/:trader_id/chat` sends an explanatory conversation message to one Trader.

Request:
- `message`: user text.
- `conversation`: optional recent chat messages with `role` and `content`.

Response:
- `reply`
- `trader_id`
- `trader_name`
- `referenced_events`
- `referenced_proposals`
- `referenced_orders`

Trader chat loads server-side context: Trader perspective, freedom level, status, tracked symbols,
latest portfolio proposal, assigned data sources and recent items, recent events, trade proposals,
runtime state, and linked paper orders/fills.
The endpoint uses the Trader's saved OpenAI key when available, then backend defaults. It never
returns API keys.

Trader chat can create or revise draft portfolio proposals when the user directly asks the Trader to
make/update a proposal. The backend converts the conversation into a structured
`trader_portfolio_proposals` row with proposed actions and returns a
`portfolio_proposal_created` action so the UI refreshes. Creating a chat proposal supersedes older
`proposed` proposals for that Trader, matching the existing proposal workflow.

Trader chat cannot place, approve, submit, or execute trades. Paper execution remains limited to the
engine/review/risk-controlled workflows.

## Chat Targets

The right-side chat can target:

- `Desk`: the main app assistant. Desk keeps app chat commands through `POST /chat/commands`.
- `MD`: managing director AI. It monitors traders, drift, disagreement, risk context, and missing information.
- `Data Scientist`: data-source AI. It creates and improves data sources, including URL-to-`python_script` sources.
- `Trader: <name>`: an individual Trader using the existing Trader chat endpoint.

If a selected Trader no longer exists, the frontend falls back to Desk. MD and Data Scientist chat
are advisory and cannot place, approve, submit, or execute trades.

## MD Chat

`POST /md-profile/chat`

Request:

- `message`
- `conversation`: optional `{ role, content }` chat history

Response:

- `reply`
- `referenced_channels`
- `referenced_traders`
- `referenced_events`

The backend prompt includes the MD profile, investor profile, recent channel messages, running
Traders, recent Trader events/proposals, recent data source errors, and recent engine events when
available. The endpoint uses backend OpenAI configuration only (`CHAT_DEFAULT_OPENAI_API_KEY` or
`OPENAI_API_KEY`) and never exposes keys to the frontend.

## Data Scientist Chat

Profile endpoints:

- `GET /data-scientist-profile`
- `PUT /data-scientist-profile`
- `POST /data-scientist-profile/chat`

`POST /data-scientist-profile/chat` accepts `message` and optional `conversation`. Responses include
`reply` and `actions`. A created source action has `type = "data_source_created"` plus the source id,
name, URL, source type, build status, and build output.

When the message contains a URL, the Data Scientist:

1. Plans a collector script with backend OpenAI when configured, optionally using OpenAI web search
   when `DATA_SCIENTIST_OPENAI_WEB_SEARCH=true`.
2. Falls back to a generic standard-library HTML link collector when model/web search support is
   unavailable.
3. Creates a `python_script` data source through OpenAPI data-source functions.
4. Saves the generated script through the existing script endpoint logic.
5. Calls the existing build validator and reports success or failure.
6. Attempts one model-based repair after a build failure when OpenAI is available.

Generated scripts must define `collect(context)`, use the source URL from `context`, return at most
100 items, avoid secrets, file writes, shell commands, environment variables, and trading endpoints.
The frontend never talks directly to `scrapper`; Python data sources remain created, edited, and
built through OpenAPI.

OpenAI web search is optional. If unavailable, generated scripts are generic and may need inspection
in the Data Sources editor before relying on extraction quality.
## Channels

Channels are persisted shared rooms for user, trader, MD, and system discussion. Initial system channels are `#general`, `#data_analysis`, and `#trading`. Channel messages are coordination context only; they do not execute trades and do not bypass existing proposal, paper trading, or risk-control workflows.

Endpoints:

- `GET /channels` lists channels in default order.
- `GET /channels/:channel_id/messages?limit=&before=&after=` returns persisted messages ascending by `created_at`.
- `POST /channels/:channel_id/messages` posts a user markdown message.
- `POST /engine/channels/:channel_name/messages` lets the engine persist trader, MD, or system markdown messages.
- `GET /engine/channel-context` returns channels, recent messages, MD profile, investor profile, and trader personas for engine prompting.
- `GET|PUT /traders/:trader_id/persona` reads or updates trader persona, tone, and communication style.
- `GET|PUT /md-profile` reads or updates the MD profile.
- `GET|PUT /data-scientist-profile` reads or updates the Data Scientist profile.
- `GET|PUT /settings/investor-profile` reads or updates investor profile context.

Engine channel posting is rate-limited by persisted recent messages: traders default to a five-minute cooldown per channel and the MD defaults to three minutes. Engine environment flags are `ENGINE_CHANNELS_ENABLED`, `ENGINE_MD_ENABLED`, and `ENGINE_CHANNEL_CHECK_INTERVAL_SECONDS`.
