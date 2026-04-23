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
- that response is refined into a concise strategy outline plus a structured executable strategy definition
- the outline and structured definition can be saved onto the project
- backtest uses the current saved or active draft
- live paper trading uses the saved structured strategy definition, not frontend-local state

### Backtesting

Current backtesting is a prototype simulation layer. It is useful for UX and iteration, but it is not yet a full execution engine for a structured strategy definition.

### Paper trading examples

Create an account:

```bash
curl -X POST http://localhost:3000/api/paper/accounts \
  -H "Content-Type: application/json" \
  -d '{"name":"Demo Account","starting_cash":10000}'
```

Submit a buy market order:

```bash
curl -X POST http://localhost:3000/api/paper/orders \
  -H "Content-Type: application/json" \
  -d '{"account_id":"<account-id>","symbol":"AAPL","side":"buy","order_type":"market","quantity":1,"source":"manual"}'
```

View positions:

```bash
curl http://localhost:3000/api/paper/accounts/<account-id>/positions
```

View account summary:

```bash
curl http://localhost:3000/api/paper/accounts/<account-id>/summary
```

Persist strategy trading config:

```bash
curl http://localhost:3000/api/strategies/test-strategy/trading-config

curl -X PUT http://localhost:3000/api/strategies/test-strategy/trading-config \
  -H "Content-Type: application/json" \
  -d '{"trading_mode":"paper","paper_account_id":"ACCOUNT_ID","is_enabled":true}'

curl http://localhost:3000/api/engine/config/strategies
```

View and update strategy risk config:

```bash
curl http://localhost:3000/api/strategies/STRATEGY_ID/risk-config

curl -X PUT http://localhost:3000/api/strategies/STRATEGY_ID/risk-config \
  -H "Content-Type: application/json" \
  -d '{
    "max_dollars_per_trade": 500,
    "max_quantity_per_trade": 5,
    "max_position_value_per_symbol": 1000,
    "max_total_exposure": 5000,
    "max_open_positions": 3,
    "max_daily_trades": 5,
    "max_daily_loss": 250,
    "cooldown_seconds": 600,
    "allowlist_symbols": ["AAPL", "MSFT"],
    "blocklist_symbols": [],
    "is_trading_enabled": true,
    "kill_switch_enabled": false
  }'

curl -X POST http://localhost:3000/api/strategies/STRATEGY_ID/kill-switch

curl -X POST http://localhost:3000/api/strategies/STRATEGY_ID/resume-trading
```

### Full validation flow

```bash
docker compose up --build
```

Then validate:

1. Open the frontend.
2. Create or select a strategy, then save the strategy so `strategy_json` is present.
3. Backtest it.
4. Create or select a paper account.
5. Open `Strategies > Live`.
6. Set mode to `Paper`.
7. Select the paper account.
8. Review the Risk Controls panel and save any limits you want to test.
9. Click `Enable Strategy`.
10. Refresh the browser and confirm the saved mode, account, and risk controls still appear.
11. Close the browser.
12. Confirm engine logs still show strategy evaluation events.
13. Confirm engine heartbeats and events continue to persist through `openapi`.
14. Confirm recent signals and runtime state appear in the Live tab after reopening the browser.
15. Confirm blocked signals show explicit `risk_reason` values when limits are exceeded.
16. If a paper order is allowed, confirm the resulting order/fill/position rows link back to the strategy and signal.
17. Trigger the kill switch and confirm the engine stops submitting new paper orders for that strategy.

### OpenAI integration

The frontend currently calls OpenAI directly when an API key is saved in local storage. This is okay for local development, but production should move model access behind the backend.

## Risk Controls Notes

- Risk checks run in the engine before paper order submission.
- `openapi` revalidates engine-submitted paper orders against persisted strategy risk config.
- v1 daily loss checks are conservative and mostly rely on realized P/L persisted on paper positions.
- v1 risk logic prefers rejecting unsafe orders over automatically resizing them.

## Good Next Infrastructure Tasks

- add an `.env.example` if backend configuration becomes environment-driven
- move OpenAI calls to the backend
- add automated tests around project strategy persistence and backtest preparation
- add database migration management if schema changes start growing beyond simple startup migration
