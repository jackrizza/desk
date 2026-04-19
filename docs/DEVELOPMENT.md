# Development Guide

## Running the App

### Backend

From the repo root:

```bash
cargo run -p openapi
```

### Frontend

From `frontend/`:

```bash
npm install
npm run dev
```

## Recommended Validation

Before pushing:

```bash
cargo fmt
cargo check
cd frontend
npm run typecheck
npm run build
```

## Local Files You Should Not Commit

- `db.sql`
- `cache_data/`
- `frontend/node_modules/`
- `frontend/build/`

## Current Development Notes

### Strategy workflow

- project chat builds a strategy response
- that response is refined into a concise strategy outline
- the outline can be saved onto the project
- backtest uses the current saved or active draft

### Backtesting

Current backtesting is a prototype simulation layer. It is useful for UX and iteration, but it is not yet a full execution engine for a structured strategy definition.

### OpenAI integration

The frontend currently calls OpenAI directly when an API key is saved in local storage. This is okay for local development, but production should move model access behind the backend.

## Good Next Infrastructure Tasks

- add an `.env.example` if backend configuration becomes environment-driven
- move OpenAI calls to the backend
- add automated tests around project strategy persistence and backtest preparation
- add database migration management if schema changes start growing beyond simple startup migration
