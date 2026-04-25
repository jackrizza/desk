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

### Trader memories

Traders persist memories in `trader_memories`. Memories are short, actionable notes
created manually from the Trader UI or by the engine after an answered channel question.
Archived memories remain visible in the UI but are ignored by the engine repeat guard.

Useful checks:

```bash
curl http://localhost:3000/api/traders/TRADER_ID/memories

curl -X POST http://localhost:3000/api/traders/TRADER_ID/memories/search \
  -H "Content-Type: application/json" \
  -d '{"query":"sector concentration risk","limit":5}'
```

The v1 search is simple text search over topic and summary.

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

Desk fallback chat can call OpenAI directly only with a key kept in browser session storage. Backend
agents such as MD, Trader chat, chat commands, and Data Scientist chat use backend configuration
(`CHAT_DEFAULT_OPENAI_API_KEY`, feature-specific keys, or `OPENAI_API_KEY`) and do not read frontend
keys. Do not store API keys in frontend local storage.

### Chat command workflow

Management commands are sent to `POST /api/chat/commands` before normal chat fallback. Useful local
smoke tests:

```text
Create an analyst trader named Macro Scout focused on cautious macro trading.
Add an RSS data source called Fed News using https://www.federalreserve.gov/feeds/press_all.xml
Assign Fed News to Macro Scout.
Start Macro Scout.
Show all running traders.
Delete Macro Scout.
yes
```

The backend returns `handled = false` for ordinary explanatory chat so existing chat behavior still
works. It uses deterministic command patterns first, then the optional backend OpenAI parser when
`CHAT_COMMAND_OPENAI_API_KEY` or `OPENAI_API_KEY` is configured. Set `CHAT_COMMAND_MODEL` to override
the default parser model. For destructive or sensitive commands, it returns a confirmation token;
reply with `yes` to execute or `no` to cancel.

Do not paste Trader API keys into general chat. Chat-created Traders use `CHAT_DEFAULT_OPENAI_API_KEY`
or `OPENAI_API_KEY` on the backend when available, otherwise the write-only Trader form should be used
to add or replace the saved key.

### Chat target workflow

The right-side chat target selector supports `Desk`, `MD`, `Data Scientist`, and `Trader: <name>`.
Desk messages keep using the app-level command flow. Trader messages call:

```bash
curl -X POST http://localhost:3000/api/traders/TRADER_ID/chat \
  -H "Content-Type: application/json" \
  -d '{"message":"What is your goal?","conversation":[]}'
```

Trader chat uses the Trader's saved key if available, then backend defaults
(`CHAT_DEFAULT_OPENAI_API_KEY` or `OPENAI_API_KEY`). It is intentionally explanatory only: asking a
Trader to place an order should produce a refusal/explanation of the engine and risk workflow, not a
paper order.

Manual checks:
- Select `Desk` and run `show all traders`.
- Select `MD` and ask `Are any traders drifting from their goals?`; confirm it answers from a
  monitoring perspective and does not trade.
- Select `Data Scientist` and send `Create a python data source from https://example.com`; confirm a
  Python Script source is created, the script is saved, the build endpoint is called, and the Data
  Sources UI refreshes.
- Select a Trader and ask `What is your goal?`.
- Ask the Trader to create a proposal, provide symbols/risk details if it asks, and confirm the
  Portfolio Proposals panel refreshes with a new `proposed` plan.
- Ask why it made its latest proposal and confirm it references known events/proposals when present.
- Ask it to place a trade and confirm it does not mutate app state.

Data Scientist URL-to-source generation uses backend OpenAI when available. Set
`DATA_SCIENTIST_OPENAI_WEB_SEARCH=true` to let the backend try OpenAI web search for URL inspection.
Without model or web-search support, it creates a generic standard-library collector and reports that
the script may need inspection/testing.

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
# Trader Development Notes

Run the normal validation flow after Trader changes:

```bash
cargo fmt
cargo check --workspace
cd frontend
npm run build
docker compose up --build
```

Manual checks:
- Create a Trader with a write-only OpenAI API key.
- Confirm list/detail responses do not include the key.
- Start, pause, stop, edit, and soft-delete the Trader.
- Close the browser and confirm engine logs/runtime state continue updating for running Traders.
- Confirm analysts only create events, junior Traders create pending proposals, and senior Traders only attempt paper orders with a selected paper account.
- Add a tracked symbol from the Trader detail page and confirm it persists after refresh.
- Use AI Suggest in the Tracked Symbols panel and confirm suggestions appear as `candidate` symbols with `source = ai`.
- Activate one symbol and reject/archive another.
- Confirm `/api/engine/config/traders` includes only `active`/`watching` tracked symbols for running Traders.
- Ask Trader chat what symbols it is tracking and confirm the answer references the symbol universe.
- Set `TRADER_PROPOSAL_INTERVAL_SECONDS=60` or lower for local proposal testing.
- Start a Trader and confirm `/api/traders/:trader_id/proposals/latest` returns a generated portfolio proposal.
- Confirm `/api/traders/:trader_id/proposals/active` returns 404 before acceptance and the active plan after acceptance.
- Confirm the Trader UI shows latest proposal, proposal actions, and history.
- Accept a proposal and confirm it becomes `status = accepted` and `plan_state = active`.
- Wait for the next proposal interval and confirm no replacement is created when no material change is detected; the engine should emit `active_plan_held`.
- Reject a proposed replacement and confirm the active plan remains active.
- Ask Trader chat about its current proposal and confirm it references the latest proposal.

V1 security limitation: `trader_secrets` is database-backed and not encrypted yet. Add encryption or a dedicated secret manager before using beyond local development.

## Scrapper

`apps/scrapper` is part of the Rust workspace and starts from Docker Compose as `desk-scrapper`. It uses `SCRAPPER_DATABASE_URL` and creates/uses the `scrapper` schema in the existing Postgres database.

Manual validation:
- Create an RSS or web page data source from `/data-sources`.
- Wait at least 30 seconds.
- Confirm `scrapper` logs show polling and OpenAPI shows updated `last_checked_at`.
- Open a Trader and assign data sources from the Data Sources section.
- Refresh the browser and confirm assignments persist.
- Confirm `/api/engine/config/traders` includes assigned data source metadata.

Python Script source validation:
- Create a Data Source with type `Python Script`.
- Confirm the Monaco editor appears on the right and the starter script loads.
- Edit the script and wait for the save state to become `Saved`.
- Click `Build` and confirm the console reports `Build successful` and `collect(context) function found`.
- Add a syntax error, build again, and confirm the console shows the Python error.
- Restore a valid script and wait for the scrapper poll interval.
- Confirm scrapper logs show Python execution and OpenAPI shows returned items/events.
- Confirm a runtime exception records `python_script_failed` without crashing the worker.

Runtime knobs:

```bash
SCRAPPER_PYTHON_VENV_PATH=/app/.venv
SCRAPPER_PYTHON_MAX_ITEMS=100
SCRAPPER_PYTHON_TIMEOUT_SECONDS=20
```

Known v1 limitation: the scrapper creates/reuses the shared venv, but there is no package
installation UI or dependency manifest yet. Keep scripts self-contained or limited to modules
available in the runtime image.
## Channels Development Notes

Channels are enabled by default in the engine:

```bash
ENGINE_CHANNELS_ENABLED=true
ENGINE_MD_ENABLED=true
ENGINE_CHANNEL_CHECK_INTERVAL_SECONDS=60
```

For local validation, start the app, open `/channels`, confirm `#general`, `#data_analysis`, and `#trading` exist, post markdown, refresh, and confirm the message persists. Then update the investor profile in Settings and a trader's Persona & Tone section from the Traders page. Engine posting uses persisted message timestamps for anti-spam cooldowns, so restarts do not immediately erase the effective channel cooldown.
