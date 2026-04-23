# Desk

Desk is a portfolio and trading workspace built with a Rust backend and a React frontend.

Today the app supports:

- manual portfolio operations
- project-based strategy building
- market charting with indicator overlays
- prototype backtesting per project symbol
- project strategy persistence
- an in-app chat workflow for shaping and refining trading strategies

## Repository Layout

```text
desk/
|- crates/
|  |- cache/        # stock data caching
|  |- database/     # PostgreSQL persistence
|  |- models/       # shared API and database models
|  |- stock_data/   # market data fetch and indicator calculations
|- apps/
|  |- openapi/      # Poem OpenAPI server
|  |- test_bench/   # local experimentation binary
|- frontend/        # React Router + Vite frontend
|- docs/            # project documentation
```

## Stack

- Rust workspace
- Poem + Poem OpenAPI
- PostgreSQL
- React 19
- React Router 7
- Vite
- Tailwind CSS
- MUI X Charts

## Current Features

### Portfolio workspace

- create portfolios and positions
- monitor aggregate NAV and gain
- manage manual position lifecycle
- scope portfolio views across one or more portfolios

### Market workspace

- dedicated market route per symbol
- candlestick + volume chart
- interval and range controls
- technical indicator overlays
- theme-aware chart styling

### Projects workspace

- create symbol-based strategy projects
- build strategies through chat
- save strategy drafts to the backend
- backtest project symbols with trade blocks and performance cards
- refine strategy prompts from backtest results

## Local Development

### Prerequisites

- Rust toolchain
- Node.js 20+
- npm
- Docker Desktop or Docker Engine for the containerized stack

### Start the full stack with Docker

From the repository root:

```bash
Copy-Item .env.example .env
docker compose up --build
```

The containerized stack starts:

- frontend: `http://localhost:5173`
- API + Swagger UI: `http://localhost:3000`
- PostgreSQL: `localhost:5432`

The API allows browser requests from the frontend origin configured by `CORS_ALLOW_ORIGIN`.

### Start the backend

From the repository root:

```bash
$env:DATABASE_URL="postgres://desk:desk@localhost:5432/desk"
cargo run -p openapi
```

The API server starts on:

- `http://localhost:3000`
- Swagger UI is served from the same server root
- API routes are under `http://localhost:3000/api`

### Start the frontend

From `frontend/`:

```bash
npm install
npm run dev
```

The frontend dev server proxies `/api` calls to `http://localhost:3000`.

## Build and Validation

### Rust

```bash
cargo fmt
cargo check
```

### Frontend

```bash
cd frontend
npm run typecheck
npm run build
```

## Data and Local State

- PostgreSQL data is stored in the `postgres_data` Docker volume when using Compose
- cached market data is stored in `cache_data/` locally or the `openapi_cache` Docker volume in Compose
- OpenAI API keys are currently stored in browser local storage from the Settings modal

Important:

- `cache_data/` is ignored from git
- browser-stored API keys are for local development only and should not be used as the long-term production security model

## API Overview

The backend currently exposes routes for:

- `hello`
- `stock_data`
- `indicators`
- `projects`
- `portfolios`
- `positions`

Projects persist:

- identity and metadata
- symbol universe
- interval/range/prepost settings
- saved strategy text

More detail lives in [docs/API.md](C:\Users\jack\OneDrive\Documents\Code\Rust\desk\docs\API.md).

## Documentation

- [Project overview](C:\Users\jack\OneDrive\Documents\Code\Rust\desk\docs\OVERVIEW.md)
- [Architecture](C:\Users\jack\OneDrive\Documents\Code\Rust\desk\docs\ARCHITECTURE.md)
- [API notes](C:\Users\jack\OneDrive\Documents\Code\Rust\desk\docs\API.md)
- [Development guide](C:\Users\jack\OneDrive\Documents\Code\Rust\desk\docs\DEVELOPMENT.md)

## Near-Term Roadmap

- move OpenAI calls behind the Rust backend
- replace prototype backtesting with structured executable strategy rules
- expand live trading workflow beyond placeholders
- improve market-route code splitting for smaller frontend bundles
