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
