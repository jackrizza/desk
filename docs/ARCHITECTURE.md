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
- trader memories that summarize answered channel questions and reduce repeated trader prompts

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

The engine reads recent channel context and active trader memories before evaluating
or posting as a trader. When a trader question receives an MD/user/trader answer, the
engine summarizes that exchange into a short persisted memory. Before posting another
channel question, the engine compares the draft message with active memories and skips
repetitive already-answered requests. This coordination layer cannot execute trades
or bypass risk controls.

Memory behavior is controlled by:

- `ENGINE_TRADER_MEMORY_ENABLED` (default `true`)
- `TRADER_MEMORY_REPEAT_WINDOW_MINUTES` (default `60`)

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
- can call OpenAI through the browser for Desk fallback only when a session key is present
- uses page-aware instructions for market, portfolio, and project contexts
- sends management commands to `openapi` before falling back to normal chat behavior
- routes MD and Data Scientist chat to backend endpoints using backend OpenAI configuration

## Persistence Model

Current persistence is split like this:

- PostgreSQL: projects, portfolios, positions, project strategy, strategy runtime state, strategy signals, strategy trading config, strategy risk config, paper trading state
- browser local storage: chat history, theme state, chat rail state
- browser session storage: optional local Desk-chat OpenAI key for the current browser session only
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

The right-side chat can target Desk, MD, Data Scientist, or a specific Trader. Desk chat keeps the
existing app-level command path. Trader chat calls `POST /api/traders/:trader_id/chat` and is explanatory:
the backend assembles the Trader's saved perspective, freedom level, assigned data sources, recent
items, recent events, runtime state, proposals, and linked paper orders/fills, then asks the model
to answer as that Trader. Trader chat never receives or returns raw API keys in the browser and
cannot execute trades directly.

MD chat calls `POST /api/md-profile/chat`. It is a monitoring role that reads MD profile,
investor profile, recent channel context, running Traders, Trader events/proposals, data source
errors, and engine events. It can explain concerns, summarize disagreement, identify drift, and ask
clarifying questions. It cannot place trades, approve trades, or submit paper orders.

Data Scientist chat calls `POST /api/data-scientist-profile/chat`. It is a data-source role that
can reason about extraction strategies and, when a URL is supplied, create a `python_script` data
source through OpenAPI data-source functions, save the generated script, run the existing build
validator, and return a creation card. Optional OpenAI web search is controlled by backend
configuration. If unavailable, the backend falls back to a generic standard-library collector.

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

Chat history is separated by target: Desk histories remain page-oriented, MD and Data Scientist have
their own histories, and Trader chat histories use `trader:<trader_id>` storage keys. If a selected
Trader disappears, the frontend falls back to Desk.
# Trader Architecture

Trader is an engine-resident autonomous paper-trading feature. The browser creates and manages Trader records, then the engine continues polling `/engine/config/traders` and evaluating running Traders even after the web app closes.

Freedom levels are enforced in the engine:
- `analyst` records recommendations/events only.
- `junior_trader` creates pending trade proposals that require UI approval.
- `senior_trader` may submit paper orders only, and never live broker orders.

Trader API keys are handed to OpenAPI only during create/update and stored server-side in `trader_secrets`. Public Trader endpoints do not return secrets. V1 stores the secret through a database-backed abstraction with a TODO for encryption or Key Vault/secret manager integration.

Junior review flow: the engine creates `trader_trade_proposals`; the UI approve action submits a paper order with `source = trader`, `trader_id`, and `proposal_id`, then marks the proposal executed. Reject marks the proposal rejected.

Senior execution is paper-only. Trader-specific risk configuration is a future extension; v1 keeps conservative behavior and relies on selected paper account checks such as cash and position validation.

Trader symbol universes are stored in `trader_symbols`. They describe the securities a Trader is designed to watch, with metadata such as asset type, thesis, fit score, status, and source. The UI supports manual additions, AI-suggested candidates, activation, rejection, and archiving.

The engine receives `tracked_symbols` in `/engine/config/traders` and only evaluates symbols whose status is `active` or `watching`. If a running Trader has no evaluable tracked symbols, the engine records a `no_tracked_symbols` event and does not trade. Symbol tracking is candidate selection context only; it does not bypass freedom levels, review flow, paper trading, or risk controls.

Trader chat includes the symbol universe in its server-side prompt context so a Trader can explain what it is watching and why. Direct Trader chat can create or revise draft portfolio proposals when the user explicitly asks the Trader to do so. It persists a proposed plan through OpenAPI and refreshes the UI, but it does not mutate the watchlist, approve proposals, submit orders, or place trades.

Trader portfolio proposals are generated by the engine for running Traders on `TRADER_PROPOSAL_INTERVAL_SECONDS` (default 300 seconds). A proposal summarizes the Trader's desired posture and proposed actions using its perspective, tracked symbols, source context, paper account selection, and conservative risk metadata. Creating a new proposal marks older `proposed` proposals as `superseded`.

Portfolio proposals are review artifacts, not execution instructions. Analyst and junior Traders only create proposals/events. Senior Traders still execute paper orders only through the existing engine workflow and risk checks; accepting a portfolio proposal does not submit orders.

Accepted proposals become active plans. The engine checks the active plan on each proposal interval and emits `active_plan_held` when no material change is found. Replacement proposals are generated only after deterministic plan monitoring detects material change, such as active duration expiry or tracked-symbol mismatch. Future extensions can add richer price, data-source, risk-config, and paper-position checks in `apps/engine/src/plan_monitor.rs`.

Active plans carry market basis, invalidation conditions, change thresholds, expected duration, and action-level entry/exit/limit/stop prices. A proposed replacement starts as `proposed`; the existing active plan remains the plan of record until the replacement is accepted or the active plan is invalidated.

# Scrapper

`scrapper` is a Dockerized Rust background service for trader information gathering. It runs independently from the browser, polls enabled data sources every 30 seconds by default, and stores discovered items in a dedicated `scrapper` Postgres schema.

OpenAPI owns the REST surface for data source CRUD and trader-source assignment. The frontend never talks directly to `scrapper`; it writes source configuration through OpenAPI, and the `scrapper` worker reads the same schema.

V1 source types:
- `rss`: fetches RSS-like feeds and stores new items by guid/link/hash.
- `web_page`: fetches a page and stores a new item when the body hash changes.
- `manual_note`: saved configuration/content only; no external polling.
- `placeholder_api`: saved config only for future external APIs.
- `python_script`: script-backed collector stored through OpenAPI and executed by `scrapper`.

Python Script sources add a right-side Monaco editor in the Data Sources UI. The source must exist
before editing; after creation the frontend loads `/data-sources/:source_id/script`, autosaves
debounced edits through OpenAPI, and calls `/data-sources/:source_id/script/build` for syntax and
`collect(context)` validation. The frontend never calls `scrapper` directly.

During polling, `scrapper` fetches script content from
`/engine/data-sources/:source_id/script`, creates/reuses one shared Python virtual environment
(`SCRAPPER_PYTHON_VENV_PATH`, default `/app/.venv`), and executes `collect(context)` with PyO3.
The current v1 venv support creates and logs the environment path; package installation and
dependency management are intentionally not exposed yet.

Collector scripts receive a plain context dictionary with data source metadata and must return a
dict containing an `items` list. Each item requires `title`; `external_id`, `url`, `content`,
`summary`, and `published_at` are optional. Missing `external_id` values are hashed from item
content. Results are capped by `SCRAPPER_PYTHON_MAX_ITEMS` (default 100), large text fields are
truncated, Python exceptions are captured into `last_error`, and the worker records
`python_script_polled` or `python_script_failed` events. Scripts are collectors only; no trading
helpers or secrets are provided to script context.

Trader engine config includes assigned data source metadata so trader prompts can include source context. Recent item consumption can be expanded through OpenAPI without requiring the web app to remain open.
## Channels

Channels add a persisted coordination layer between the user, AI traders, and the MD. The database owns `channels`, `channel_messages`, trader persona fields, `md_profile`, and `user_investor_profile`. OpenAPI is the only REST surface for this feature; the engine posts through OpenAPI and never writes channel data directly.

The engine adds a communication decision after trader evaluation. It can post to `#data_analysis` for source/data uncertainty, `#trading` for proposal, risk, portfolio, or trade-plan topics, and `#general` for broad coordination. Channel discussion is advisory context only: no channel message can submit paper orders, approve proposals, or bypass risk guards.

The MD is an engine-side and chat-side monitoring role. It reads recent channel context, asks clarifying questions, challenges weak assumptions, summarizes disagreement, and reminds traders of risk and user-profile context. It does not place trades.

## Data Scientist

The Data Scientist profile is persisted in `data_scientist_profile`. Its chat endpoint is hosted by
OpenAPI so the browser never calls `scrapper` directly and never supplies backend agent API keys.
URL-to-Python-source creation is an OpenAPI orchestration over existing data source CRUD, script save,
and script build functions.

Generated collector scripts are safety-limited: `collect(context)`, no secrets, no environment
access, no shell commands, no file writes, no trading endpoints, at most 100 items, and graceful
network failure handling. Build validation checks syntax and the `collect(context)` contract; it does
not guarantee extraction quality, so users should inspect generated scripts in the Data Sources UI.
