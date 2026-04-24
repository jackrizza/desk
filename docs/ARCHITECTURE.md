# Architecture

## High-Level Shape

Desk is split into a Rust backend and a React frontend.

```text
Frontend (React Router + Vite)
    |
    | HTTP / JSON
    v
OpenAPI server (Poem)
    |
    +-- Database crate (PostgreSQL)
    +-- Cache crate
    +-- Stock data crate
    +-- Shared models crate
    |
Background engine (Rust worker)
    |
    +-- HTTP coordination through OpenAPI
    +-- Cache crate
    +-- Stock data crate
    +-- Shared models crate
```

## Rust Workspace

### `crates/models`

Shared data types used across the backend, including:

- `Project`
- `Portfolio`
- `Position`
- raw stock and indicator response types

### `crates/database`

Owns PostgreSQL persistence for:

- projects
- project symbols
- portfolios
- positions
- paper accounts, orders, fills, positions, and audit events
- strategy trading configuration that persists mode and selected paper account for the engine
- strategy risk configuration, runtime state, and signal history for engine-driven paper trading

Projects now persist both a human-readable `strategy` field and a structured `strategy_json`
definition that the engine can execute without the frontend or AI remaining active.

### `crates/cache`

Caches market data locally to reduce repeat fetch cost and improve responsiveness.

### `crates/stock_data`

Responsible for:

- stock data retrieval
- indicator calculation

### `apps/openapi`

The API entrypoint. It exposes the application routes and serves Swagger UI.

Paper trading also lives here:

- paper accounts, orders, fills, positions, and events are stored in PostgreSQL
- v1 paper market orders fill immediately using the latest available market price
- the engine submits simulated orders to `POST /api/paper/orders`
- strategy trading mode and paper account selection are persisted through `/api/strategies/:strategy_id/trading-config`
- strategy risk controls are persisted through `/api/strategies/:strategy_id/risk-config`
- server-side paper order validation re-checks engine orders against persisted risk settings
- `GET /api/engine/config/strategies` exposes only engine-runnable paper configs
- there is no live broker integration or real order execution path yet

### `apps/engine`

The background worker entrypoint. It runs as a separate container, polls `openapi` for active
symbols and persisted strategy trading configs, evaluates those symbols using shared crates, and
reports heartbeat and engine events back to `openapi`.

For v1:

- the engine loads persisted structured strategy definitions and risk configs from `openapi`
- it evaluates enabled paper strategies against cached market data
- it persists signals before and after risk/order decisions
- it runs a conservative risk guard before any paper order submission
- risk-blocked signals are stored with explicit reasons
- market paper orders remain immediate-fill only
- real trading remains disabled

Future work for the engine includes:

- persisting heartbeat and event records
- deeper strategy and condition evaluation
- broker integration
- order intent persistence and orchestration

## Frontend

### Routing

The main frontend route groups are:

- `/` for portfolio operations
- `/market` and `/market/:symbol`
- `/projects` and `/projects/:projectId`

### Shared UI

Shared components include:

- top bar
- left sidebar
- chat panel

### Data Access

The frontend uses `frontend/app/lib/api.ts` as its shared API client.

### Chat

The chat layer:

- keeps per-page chat history
- can call OpenAI through the browser when a key is present
- uses page-aware instructions for market, portfolio, and project contexts
- sends management commands to `openapi` before falling back to normal chat behavior

## Persistence Model

Current persistence is split like this:

- PostgreSQL: projects, portfolios, positions, project strategy, strategy runtime state, strategy signals, strategy trading config, strategy risk config, paper trading state
- browser local storage: chat history, theme state, chat rail state, OpenAI API key
- local cache directory: market data cache

## Service Boundary

`openapi` remains the coordination boundary for the system:

- frontend and external clients talk to `openapi`
- `engine` fetches database-backed active symbols from `openapi`
- `engine` sends heartbeat and event callbacks to `openapi`
- `openapi` persists active symbols, heartbeat history, and engine event history for monitoring and audit trails
- strategy-driven paper trading continues after the browser closes because execution config, risk config, runtime state, and signals all live behind `openapi`
- future order intent and live-trading logic can build on those persisted histories

# Chat Command Architecture

Chat-driven management for Traders and Data Sources is executed through `openapi`, not through
frontend-only parsing. The frontend sends candidate management messages to `POST /api/chat/commands`;
if the backend marks the message as handled, the chat renders the backend reply and refreshes the
relevant Trader/Data Source state. If the backend does not recognize a command, the existing normal
chat path continues.

V1 command parsing uses deterministic backend patterns first, then an optional backend OpenAI parser
when `CHAT_COMMAND_OPENAI_API_KEY` or `OPENAI_API_KEY` is configured. It is scoped to supported
management phrases for:
- Traders: create, update, list, start, stop, pause, delete, and status.
- Data Sources: create, update, list, disable/delete, and recent item/status checks.
- Trader/Data Source relationships: assign, unassign, and list assigned sources.

Sensitive commands use a confirmation token returned by `openapi`; the frontend stores only that
pending token in chat state and sends it back when the user confirms. Deleting Traders or Data
Sources, promoting a Trader to `senior_trader`, and starting a senior Trader require confirmation.

Trader API keys are not requested in general chat. Chat-created Traders use a backend default key
from `CHAT_DEFAULT_OPENAI_API_KEY` or `OPENAI_API_KEY` when present; otherwise the user is told to
add the key through the write-only Trader form. Saved keys are never returned through chat responses.
# Trader Architecture

Trader is an engine-resident autonomous paper-trading feature. The browser creates and manages Trader records, then the engine continues polling `/engine/config/traders` and evaluating running Traders even after the web app closes.

Freedom levels are enforced in the engine:
- `analyst` records recommendations/events only.
- `junior_trader` creates pending trade proposals that require UI approval.
- `senior_trader` may submit paper orders only, and never live broker orders.

Trader API keys are handed to OpenAPI only during create/update and stored server-side in `trader_secrets`. Public Trader endpoints do not return secrets. V1 stores the secret through a database-backed abstraction with a TODO for encryption or Key Vault/secret manager integration.

Junior review flow: the engine creates `trader_trade_proposals`; the UI approve action submits a paper order with `source = trader`, `trader_id`, and `proposal_id`, then marks the proposal executed. Reject marks the proposal rejected.

Senior execution is paper-only. Trader-specific risk configuration is a future extension; v1 keeps conservative behavior and relies on selected paper account checks such as cash and position validation.

# Scrapper

`scrapper` is a Dockerized Rust background service for trader information gathering. It runs independently from the browser, polls enabled data sources every 30 seconds by default, and stores discovered items in a dedicated `scrapper` Postgres schema.

OpenAPI owns the REST surface for data source CRUD and trader-source assignment. The frontend never talks directly to `scrapper`; it writes source configuration through OpenAPI, and the `scrapper` worker reads the same schema.

V1 source types:
- `rss`: fetches RSS-like feeds and stores new items by guid/link/hash.
- `web_page`: fetches a page and stores a new item when the body hash changes.
- `manual_note`: saved configuration/content only; no external polling.
- `placeholder_api`: saved config only for future external APIs.

Trader engine config includes assigned data source metadata so trader prompts can include source context. Recent item consumption can be expanded through OpenAPI without requiring the web app to remain open.
