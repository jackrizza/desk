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

Projects now persist a `strategy` field directly on the project record.

### `crates/cache`

Caches market data locally to reduce repeat fetch cost and improve responsiveness.

### `crates/stock_data`

Responsible for:

- stock data retrieval
- indicator calculation

### `crates/openapi`

The API entrypoint. It exposes the application routes and serves Swagger UI.

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

- PostgreSQL: projects, portfolios, positions, project strategy
- browser local storage: chat history, theme state, chat rail state, OpenAI API key
- local cache directory: market data cache
