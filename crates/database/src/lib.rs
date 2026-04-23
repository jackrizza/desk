use chrono::Utc;
use models::{
    engine::{
        ActiveSymbol, EngineEvent, EngineEventRequest, EngineHeartbeat, EngineHeartbeatRequest,
    },
    paper::{PaperAccount, PaperAccountEvent, PaperFill, PaperOrder, PaperPosition},
    portfolio::{Portfolio, Position},
    projects::Project,
    trading::{
        CreateStrategySignalRequest, EngineRunnableStrategy, StrategyDefinition,
        StrategyRiskConfig, StrategyRuntimeState, StrategySignal, StrategyTradingConfig,
        UpsertStrategyRuntimeStateRequest,
    },
};
use serde_json::Value;
use sqlx::{PgPool, Postgres, Row, Transaction, postgres::PgPoolOptions};
use uuid::Uuid;

pub struct Database {
    pool: PgPool,
}

pub struct PaperAccountSummaryParts {
    pub account: PaperAccount,
    pub positions: Vec<PaperPosition>,
    pub open_orders: Vec<PaperOrder>,
    pub recent_fills: Vec<PaperFill>,
}

pub struct ExecutePaperMarketOrderParams {
    pub account_id: String,
    pub symbol: String,
    pub side: String,
    pub order_type: String,
    pub quantity: f64,
    pub requested_price: Option<f64>,
    pub source: String,
    pub strategy_id: Option<String>,
    pub signal_id: Option<String>,
    pub fill_price: f64,
}

pub struct ExecutePaperMarketOrderResult {
    pub account: PaperAccount,
    pub order: PaperOrder,
    pub fill: PaperFill,
    pub position: PaperPosition,
}

impl Database {
    pub async fn connect(database_url: &str) -> Result<Self, sqlx::Error> {
        let pool = PgPoolOptions::new()
            .max_connections(10)
            .connect(database_url)
            .await?;

        let db = Self { pool };
        db.init().await?;
        Ok(db)
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    async fn init(&self) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS projects (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT NOT NULL,
                strategy TEXT NOT NULL DEFAULT '',
                strategy_json TEXT NOT NULL DEFAULT '{}',
                strategy_status TEXT NOT NULL DEFAULT 'draft',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                interval TEXT NOT NULL,
                range TEXT NOT NULL,
                prepost BOOLEAN NOT NULL DEFAULT FALSE
            );
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            ALTER TABLE projects
            ADD COLUMN IF NOT EXISTS strategy TEXT NOT NULL DEFAULT ''
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            ALTER TABLE projects
            ADD COLUMN IF NOT EXISTS strategy_json TEXT NOT NULL DEFAULT '{}'
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            ALTER TABLE projects
            ADD COLUMN IF NOT EXISTS strategy_status TEXT NOT NULL DEFAULT 'draft'
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS project_symbols (
                project_id TEXT NOT NULL,
                symbol TEXT NOT NULL,
                ordinal BIGINT NOT NULL,
                PRIMARY KEY (project_id, ordinal),
                CONSTRAINT fk_project_symbols_project
                    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
            );
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS portfolios (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS positions (
                id BIGSERIAL PRIMARY KEY,
                portfolio_id TEXT NOT NULL,
                symbol TEXT NOT NULL,
                quantity DOUBLE PRECISION NOT NULL,
                average_price DOUBLE PRECISION NOT NULL,
                position_opened_at TEXT NOT NULL,
                position_closed_at TEXT NULL,
                position_closed_price DOUBLE PRECISION NULL,
                ordinal BIGINT NOT NULL,
                CONSTRAINT fk_positions_portfolio
                    FOREIGN KEY (portfolio_id) REFERENCES portfolios(id) ON DELETE CASCADE,
                CONSTRAINT positions_unique_identity
                    UNIQUE (portfolio_id, symbol, position_opened_at)
            );
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS active_symbols (
                id TEXT PRIMARY KEY,
                symbol TEXT NOT NULL UNIQUE,
                enabled BOOLEAN NOT NULL DEFAULT TRUE,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_active_symbols_enabled
            ON active_symbols(enabled);
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS engine_heartbeats (
                id TEXT PRIMARY KEY,
                engine_name TEXT NOT NULL,
                status TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                created_at TEXT NOT NULL
            );
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_engine_heartbeats_engine_name
            ON engine_heartbeats(engine_name);
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_engine_heartbeats_timestamp
            ON engine_heartbeats(timestamp);
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS engine_events (
                id TEXT PRIMARY KEY,
                engine_name TEXT NOT NULL,
                event_type TEXT NOT NULL,
                symbol TEXT NULL,
                message TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                created_at TEXT NOT NULL
            );
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_engine_events_engine_name
            ON engine_events(engine_name);
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_engine_events_event_type
            ON engine_events(event_type);
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_engine_events_symbol
            ON engine_events(symbol);
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_engine_events_timestamp
            ON engine_events(timestamp);
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS paper_accounts (
                id UUID PRIMARY KEY,
                name TEXT NOT NULL,
                starting_cash DOUBLE PRECISION NOT NULL,
                cash_balance DOUBLE PRECISION NOT NULL,
                currency TEXT NOT NULL DEFAULT 'USD',
                is_active BOOLEAN NOT NULL DEFAULT TRUE,
                created_at TIMESTAMPTZ NOT NULL,
                updated_at TIMESTAMPTZ NOT NULL
            );
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS paper_positions (
                id UUID PRIMARY KEY,
                account_id UUID NOT NULL REFERENCES paper_accounts(id) ON DELETE CASCADE,
                symbol TEXT NOT NULL,
                quantity DOUBLE PRECISION NOT NULL,
                average_price DOUBLE PRECISION NOT NULL,
                realized_pnl DOUBLE PRECISION NOT NULL DEFAULT 0,
                created_at TIMESTAMPTZ NOT NULL,
                updated_at TIMESTAMPTZ NOT NULL,
                UNIQUE(account_id, symbol)
            );
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS paper_orders (
                id UUID PRIMARY KEY,
                account_id UUID NOT NULL REFERENCES paper_accounts(id) ON DELETE CASCADE,
                symbol TEXT NOT NULL,
                side TEXT NOT NULL,
                order_type TEXT NOT NULL,
                quantity DOUBLE PRECISION NOT NULL,
                requested_price DOUBLE PRECISION NULL,
                filled_quantity DOUBLE PRECISION NOT NULL DEFAULT 0,
                average_fill_price DOUBLE PRECISION NULL,
                status TEXT NOT NULL,
                source TEXT NOT NULL DEFAULT 'manual',
                strategy_id TEXT NULL,
                signal_id UUID NULL,
                created_at TIMESTAMPTZ NOT NULL,
                updated_at TIMESTAMPTZ NOT NULL
            );
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            ALTER TABLE paper_orders
            ADD COLUMN IF NOT EXISTS strategy_id TEXT NULL
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            ALTER TABLE paper_orders
            ADD COLUMN IF NOT EXISTS signal_id UUID NULL
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS paper_fills (
                id UUID PRIMARY KEY,
                account_id UUID NOT NULL REFERENCES paper_accounts(id) ON DELETE CASCADE,
                order_id UUID NOT NULL REFERENCES paper_orders(id) ON DELETE CASCADE,
                symbol TEXT NOT NULL,
                side TEXT NOT NULL,
                quantity DOUBLE PRECISION NOT NULL,
                price DOUBLE PRECISION NOT NULL,
                notional DOUBLE PRECISION NOT NULL,
                created_at TIMESTAMPTZ NOT NULL
            );
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS paper_account_events (
                id UUID PRIMARY KEY,
                account_id UUID NOT NULL REFERENCES paper_accounts(id) ON DELETE CASCADE,
                event_type TEXT NOT NULL,
                message TEXT NOT NULL,
                payload JSONB NULL,
                created_at TIMESTAMPTZ NOT NULL
            );
            "#,
        )
        .execute(&self.pool)
        .await?;

        for statement in [
            "CREATE INDEX IF NOT EXISTS idx_paper_positions_account_id ON paper_positions(account_id);",
            "CREATE INDEX IF NOT EXISTS idx_paper_positions_symbol ON paper_positions(symbol);",
            "CREATE INDEX IF NOT EXISTS idx_paper_positions_created_at ON paper_positions(created_at DESC);",
            "CREATE INDEX IF NOT EXISTS idx_paper_orders_account_id ON paper_orders(account_id);",
            "CREATE INDEX IF NOT EXISTS idx_paper_orders_symbol ON paper_orders(symbol);",
            "CREATE INDEX IF NOT EXISTS idx_paper_orders_status ON paper_orders(status);",
            "CREATE INDEX IF NOT EXISTS idx_paper_orders_created_at ON paper_orders(created_at DESC);",
            "CREATE INDEX IF NOT EXISTS idx_paper_fills_account_id ON paper_fills(account_id);",
            "CREATE INDEX IF NOT EXISTS idx_paper_fills_symbol ON paper_fills(symbol);",
            "CREATE INDEX IF NOT EXISTS idx_paper_fills_created_at ON paper_fills(created_at DESC);",
            "CREATE INDEX IF NOT EXISTS idx_paper_account_events_account_id ON paper_account_events(account_id);",
            "CREATE INDEX IF NOT EXISTS idx_paper_account_events_created_at ON paper_account_events(created_at DESC);",
        ] {
            sqlx::query(statement).execute(&self.pool).await?;
        }

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS strategy_trading_config (
                strategy_id TEXT PRIMARY KEY,
                trading_mode TEXT NOT NULL DEFAULT 'off',
                paper_account_id UUID NULL REFERENCES paper_accounts(id) ON DELETE SET NULL,
                is_enabled BOOLEAN NOT NULL DEFAULT FALSE,
                last_started_at TIMESTAMPTZ NULL,
                last_stopped_at TIMESTAMPTZ NULL,
                updated_at TIMESTAMPTZ NOT NULL,
                created_at TIMESTAMPTZ NOT NULL
            );
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            ALTER TABLE strategy_trading_config
            ADD COLUMN IF NOT EXISTS last_started_at TIMESTAMPTZ NULL
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            ALTER TABLE strategy_trading_config
            ADD COLUMN IF NOT EXISTS last_stopped_at TIMESTAMPTZ NULL
            "#,
        )
        .execute(&self.pool)
        .await?;

        for statement in [
            "CREATE INDEX IF NOT EXISTS idx_strategy_trading_config_mode ON strategy_trading_config(trading_mode);",
            "CREATE INDEX IF NOT EXISTS idx_strategy_trading_config_enabled ON strategy_trading_config(is_enabled);",
            "CREATE INDEX IF NOT EXISTS idx_strategy_trading_config_paper_account_id ON strategy_trading_config(paper_account_id);",
            "CREATE INDEX IF NOT EXISTS idx_strategy_trading_config_updated_at ON strategy_trading_config(updated_at DESC);",
        ] {
            sqlx::query(statement).execute(&self.pool).await?;
        }

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS strategy_runtime_state (
                id UUID PRIMARY KEY,
                strategy_id TEXT NOT NULL,
                paper_account_id UUID NOT NULL REFERENCES paper_accounts(id) ON DELETE CASCADE,
                symbol TEXT NOT NULL,
                last_evaluated_at TIMESTAMPTZ NULL,
                last_signal TEXT NULL,
                last_signal_at TIMESTAMPTZ NULL,
                last_order_id UUID NULL,
                position_state TEXT NOT NULL DEFAULT 'flat',
                cooldown_until TIMESTAMPTZ NULL,
                created_at TIMESTAMPTZ NOT NULL,
                updated_at TIMESTAMPTZ NOT NULL,
                UNIQUE(strategy_id, paper_account_id, symbol)
            );
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS strategy_signals (
                id UUID PRIMARY KEY,
                strategy_id TEXT NOT NULL,
                paper_account_id UUID NOT NULL REFERENCES paper_accounts(id) ON DELETE CASCADE,
                symbol TEXT NOT NULL,
                signal_type TEXT NOT NULL,
                confidence DOUBLE PRECISION NULL,
                reason TEXT NOT NULL,
                market_price DOUBLE PRECISION NULL,
                source TEXT NOT NULL DEFAULT 'engine',
                status TEXT NOT NULL DEFAULT 'created',
                order_id UUID NULL,
                created_at TIMESTAMPTZ NOT NULL
            );
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            ALTER TABLE strategy_signals
            ADD COLUMN IF NOT EXISTS risk_decision TEXT NULL
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            ALTER TABLE strategy_signals
            ADD COLUMN IF NOT EXISTS risk_reason TEXT NULL
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS strategy_risk_config (
                strategy_id TEXT PRIMARY KEY,
                max_dollars_per_trade DOUBLE PRECISION NULL,
                max_quantity_per_trade DOUBLE PRECISION NULL,
                max_position_value_per_symbol DOUBLE PRECISION NULL,
                max_total_exposure DOUBLE PRECISION NULL,
                max_open_positions BIGINT NULL,
                max_daily_trades BIGINT NULL,
                max_daily_loss DOUBLE PRECISION NULL,
                cooldown_seconds BIGINT NOT NULL DEFAULT 0,
                allowlist_symbols JSONB NULL,
                blocklist_symbols JSONB NULL,
                is_trading_enabled BOOLEAN NOT NULL DEFAULT TRUE,
                kill_switch_enabled BOOLEAN NOT NULL DEFAULT FALSE,
                created_at TIMESTAMPTZ NOT NULL,
                updated_at TIMESTAMPTZ NOT NULL
            );
            "#,
        )
        .execute(&self.pool)
        .await?;

        for statement in [
            "CREATE INDEX IF NOT EXISTS idx_strategy_runtime_state_strategy_id ON strategy_runtime_state(strategy_id);",
            "CREATE INDEX IF NOT EXISTS idx_strategy_runtime_state_account_id ON strategy_runtime_state(paper_account_id);",
            "CREATE INDEX IF NOT EXISTS idx_strategy_runtime_state_symbol ON strategy_runtime_state(symbol);",
            "CREATE INDEX IF NOT EXISTS idx_strategy_runtime_state_updated_at ON strategy_runtime_state(updated_at DESC);",
            "CREATE INDEX IF NOT EXISTS idx_strategy_signals_strategy_id ON strategy_signals(strategy_id);",
            "CREATE INDEX IF NOT EXISTS idx_strategy_signals_account_id ON strategy_signals(paper_account_id);",
            "CREATE INDEX IF NOT EXISTS idx_strategy_signals_symbol ON strategy_signals(symbol);",
            "CREATE INDEX IF NOT EXISTS idx_strategy_signals_created_at ON strategy_signals(created_at DESC);",
            "CREATE INDEX IF NOT EXISTS idx_strategy_signals_status ON strategy_signals(status);",
            "CREATE INDEX IF NOT EXISTS idx_strategy_risk_config_updated_at ON strategy_risk_config(updated_at DESC);",
            "CREATE INDEX IF NOT EXISTS idx_strategy_risk_config_enabled ON strategy_risk_config(is_trading_enabled);",
            "CREATE INDEX IF NOT EXISTS idx_strategy_risk_config_kill_switch ON strategy_risk_config(kill_switch_enabled);",
        ] {
            sqlx::query(statement).execute(&self.pool).await?;
        }

        self.seed_active_symbols().await?;

        Ok(())
    }

    async fn seed_active_symbols(&self) -> Result<(), sqlx::Error> {
        for symbol in ["AAPL", "MSFT", "NVDA"] {
            let now = Self::now_string();

            sqlx::query(
                r#"
                INSERT INTO active_symbols (id, symbol, enabled, created_at, updated_at)
                VALUES ($1, $2, $3, $4, $5)
                ON CONFLICT (symbol) DO NOTHING
                "#,
            )
            .bind(Self::new_id())
            .bind(symbol)
            .bind(true)
            .bind(&now)
            .bind(&now)
            .execute(&self.pool)
            .await?;
        }

        Ok(())
    }

    fn new_id() -> String {
        Uuid::now_v7().to_string()
    }

    fn now_string() -> String {
        Utc::now().to_rfc3339()
    }

    fn row_to_paper_account(row: sqlx::postgres::PgRow) -> PaperAccount {
        PaperAccount {
            id: row.get::<String, _>("id"),
            name: row.get("name"),
            starting_cash: row.get("starting_cash"),
            cash_balance: row.get("cash_balance"),
            currency: row.get("currency"),
            is_active: row.get("is_active"),
            created_at: row.get::<String, _>("created_at"),
            updated_at: row.get::<String, _>("updated_at"),
        }
    }

    fn row_to_paper_position(row: sqlx::postgres::PgRow) -> PaperPosition {
        PaperPosition {
            id: row.get::<String, _>("id"),
            account_id: row.get::<String, _>("account_id"),
            symbol: row.get("symbol"),
            quantity: row.get("quantity"),
            average_price: row.get("average_price"),
            realized_pnl: row.get("realized_pnl"),
            created_at: row.get::<String, _>("created_at"),
            updated_at: row.get::<String, _>("updated_at"),
        }
    }

    fn row_to_paper_order(row: sqlx::postgres::PgRow) -> PaperOrder {
        PaperOrder {
            id: row.get::<String, _>("id"),
            account_id: row.get::<String, _>("account_id"),
            symbol: row.get("symbol"),
            side: row.get("side"),
            order_type: row.get("order_type"),
            quantity: row.get("quantity"),
            requested_price: row.get("requested_price"),
            filled_quantity: row.get("filled_quantity"),
            average_fill_price: row.get("average_fill_price"),
            status: row.get("status"),
            source: row.get("source"),
            strategy_id: row.get("strategy_id"),
            signal_id: row.get("signal_id"),
            created_at: row.get::<String, _>("created_at"),
            updated_at: row.get::<String, _>("updated_at"),
        }
    }

    fn row_to_paper_fill(row: sqlx::postgres::PgRow) -> PaperFill {
        PaperFill {
            id: row.get::<String, _>("id"),
            account_id: row.get::<String, _>("account_id"),
            order_id: row.get::<String, _>("order_id"),
            symbol: row.get("symbol"),
            side: row.get("side"),
            quantity: row.get("quantity"),
            price: row.get("price"),
            notional: row.get("notional"),
            created_at: row.get::<String, _>("created_at"),
        }
    }

    fn row_to_paper_account_event(row: sqlx::postgres::PgRow) -> PaperAccountEvent {
        let payload = row
            .try_get::<Option<String>, _>("payload")
            .ok()
            .flatten()
            .or_else(|| {
                row.try_get::<Option<Value>, _>("payload")
                    .ok()
                    .flatten()
                    .map(|value| value.to_string())
            });

        PaperAccountEvent {
            id: row.get::<String, _>("id"),
            account_id: row.get::<String, _>("account_id"),
            event_type: row.get("event_type"),
            message: row.get("message"),
            payload,
            created_at: row.get::<String, _>("created_at"),
        }
    }

    fn row_to_strategy_trading_config(row: sqlx::postgres::PgRow) -> StrategyTradingConfig {
        StrategyTradingConfig {
            strategy_id: row.get("strategy_id"),
            trading_mode: row.get("trading_mode"),
            paper_account_id: row.get("paper_account_id"),
            is_enabled: row.get("is_enabled"),
            last_started_at: row.get("last_started_at"),
            last_stopped_at: row.get("last_stopped_at"),
            created_at: Some(row.get::<String, _>("created_at")),
            updated_at: Some(row.get::<String, _>("updated_at")),
        }
    }

    fn row_to_strategy_runtime_state(row: sqlx::postgres::PgRow) -> StrategyRuntimeState {
        StrategyRuntimeState {
            id: row.get("id"),
            strategy_id: row.get("strategy_id"),
            paper_account_id: row.get("paper_account_id"),
            symbol: row.get("symbol"),
            last_evaluated_at: row.get("last_evaluated_at"),
            last_signal: row.get("last_signal"),
            last_signal_at: row.get("last_signal_at"),
            last_order_id: row.get("last_order_id"),
            position_state: row.get("position_state"),
            cooldown_until: row.get("cooldown_until"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        }
    }

    fn row_to_strategy_risk_config(row: sqlx::postgres::PgRow) -> StrategyRiskConfig {
        StrategyRiskConfig {
            strategy_id: row.get("strategy_id"),
            max_dollars_per_trade: row.get("max_dollars_per_trade"),
            max_quantity_per_trade: row.get("max_quantity_per_trade"),
            max_position_value_per_symbol: row.get("max_position_value_per_symbol"),
            max_total_exposure: row.get("max_total_exposure"),
            max_open_positions: row.get("max_open_positions"),
            max_daily_trades: row.get("max_daily_trades"),
            max_daily_loss: row.get("max_daily_loss"),
            cooldown_seconds: row.get("cooldown_seconds"),
            allowlist_symbols: row
                .try_get::<Option<Value>, _>("allowlist_symbols")
                .ok()
                .flatten()
                .and_then(Self::json_value_to_string_vec),
            blocklist_symbols: row
                .try_get::<Option<Value>, _>("blocklist_symbols")
                .ok()
                .flatten()
                .and_then(Self::json_value_to_string_vec),
            is_trading_enabled: row.get("is_trading_enabled"),
            kill_switch_enabled: row.get("kill_switch_enabled"),
            created_at: Some(row.get::<String, _>("created_at")),
            updated_at: Some(row.get::<String, _>("updated_at")),
        }
    }

    fn row_to_strategy_signal(row: sqlx::postgres::PgRow) -> StrategySignal {
        StrategySignal {
            id: row.get("id"),
            strategy_id: row.get("strategy_id"),
            paper_account_id: row.get("paper_account_id"),
            symbol: row.get("symbol"),
            signal_type: row.get("signal_type"),
            confidence: row.get("confidence"),
            reason: row.get("reason"),
            market_price: row.get("market_price"),
            source: row.get("source"),
            status: row.get("status"),
            risk_decision: row.try_get("risk_decision").ok(),
            risk_reason: row.try_get("risk_reason").ok(),
            order_id: row.get("order_id"),
            created_at: row.get("created_at"),
        }
    }

    fn parse_strategy_definition(strategy_json: &str) -> Option<StrategyDefinition> {
        serde_json::from_str(strategy_json).ok()
    }

    fn json_value_to_string_vec(value: Value) -> Option<Vec<String>> {
        match value {
            Value::Array(items) => Some(
                items
                    .into_iter()
                    .filter_map(|item| item.as_str().map(|text| text.to_string()))
                    .collect(),
            ),
            _ => None,
        }
    }

    fn string_vec_to_json_value(values: &Option<Vec<String>>) -> Option<Value> {
        values
            .as_ref()
            .map(|items| Value::Array(items.iter().cloned().map(Value::String).collect()))
    }

    pub fn default_strategy_risk_config(strategy_id: &str) -> StrategyRiskConfig {
        StrategyRiskConfig {
            strategy_id: strategy_id.to_string(),
            max_dollars_per_trade: Some(1000.0),
            max_quantity_per_trade: Some(10.0),
            max_position_value_per_symbol: Some(2500.0),
            max_total_exposure: Some(10000.0),
            max_open_positions: Some(5),
            max_daily_trades: Some(10),
            max_daily_loss: Some(500.0),
            cooldown_seconds: 300,
            allowlist_symbols: None,
            blocklist_symbols: None,
            is_trading_enabled: true,
            kill_switch_enabled: false,
            created_at: None,
            updated_at: None,
        }
    }

    async fn get_paper_account_for_update(
        tx: &mut Transaction<'_, Postgres>,
        account_id: &str,
    ) -> Result<Option<PaperAccount>, sqlx::Error> {
        let row = sqlx::query(
            r#"
            SELECT
                id::text AS id,
                name,
                starting_cash,
                cash_balance,
                currency,
                is_active,
                created_at::text AS created_at,
                updated_at::text AS updated_at
            FROM paper_accounts
            WHERE id = $1::uuid
            FOR UPDATE
            "#,
        )
        .bind(account_id)
        .fetch_optional(&mut **tx)
        .await?;

        Ok(row.map(Self::row_to_paper_account))
    }

    async fn get_paper_position_for_update(
        tx: &mut Transaction<'_, Postgres>,
        account_id: &str,
        symbol: &str,
    ) -> Result<Option<PaperPosition>, sqlx::Error> {
        let row = sqlx::query(
            r#"
            SELECT
                id::text AS id,
                account_id::text AS account_id,
                symbol,
                quantity,
                average_price,
                realized_pnl,
                created_at::text AS created_at,
                updated_at::text AS updated_at
            FROM paper_positions
            WHERE account_id = $1::uuid AND symbol = $2
            FOR UPDATE
            "#,
        )
        .bind(account_id)
        .bind(symbol)
        .fetch_optional(&mut **tx)
        .await?;

        Ok(row.map(Self::row_to_paper_position))
    }

    pub async fn create_paper_account(
        &self,
        name: &str,
        starting_cash: f64,
    ) -> Result<PaperAccount, sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        let now = Self::now_string();
        let account = PaperAccount {
            id: Self::new_id(),
            name: name.to_string(),
            starting_cash,
            cash_balance: starting_cash,
            currency: "USD".to_string(),
            is_active: true,
            created_at: now.clone(),
            updated_at: now.clone(),
        };

        sqlx::query(
            r#"
            INSERT INTO paper_accounts (
                id, name, starting_cash, cash_balance, currency, is_active, created_at, updated_at
            ) VALUES ($1::uuid, $2, $3, $4, $5, $6, $7::timestamptz, $8::timestamptz)
            "#,
        )
        .bind(&account.id)
        .bind(&account.name)
        .bind(account.starting_cash)
        .bind(account.cash_balance)
        .bind(&account.currency)
        .bind(account.is_active)
        .bind(&account.created_at)
        .bind(&account.updated_at)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            r#"
            INSERT INTO paper_account_events (
                id, account_id, event_type, message, payload, created_at
            ) VALUES ($1::uuid, $2::uuid, $3, $4, $5::jsonb, $6::timestamptz)
            "#,
        )
        .bind(Self::new_id())
        .bind(&account.id)
        .bind("account_created")
        .bind(format!(
            "Created paper account {} with starting cash {:.4}",
            account.name, account.starting_cash
        ))
        .bind(Some(
            serde_json::json!({
                "account_id": account.id,
                "starting_cash": account.starting_cash,
                "currency": account.currency,
            })
            .to_string(),
        ))
        .bind(&account.created_at)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(account)
    }

    pub async fn list_paper_accounts(&self) -> Result<Vec<PaperAccount>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT
                id::text AS id,
                name,
                starting_cash,
                cash_balance,
                currency,
                is_active,
                created_at::text AS created_at,
                updated_at::text AS updated_at
            FROM paper_accounts
            ORDER BY created_at DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(Self::row_to_paper_account).collect())
    }

    pub async fn get_paper_account(
        &self,
        account_id: &str,
    ) -> Result<Option<PaperAccount>, sqlx::Error> {
        let row = sqlx::query(
            r#"
            SELECT
                id::text AS id,
                name,
                starting_cash,
                cash_balance,
                currency,
                is_active,
                created_at::text AS created_at,
                updated_at::text AS updated_at
            FROM paper_accounts
            WHERE id = $1::uuid
            "#,
        )
        .bind(account_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Self::row_to_paper_account))
    }

    pub async fn get_paper_account_summary_parts(
        &self,
        account_id: &str,
    ) -> Result<Option<PaperAccountSummaryParts>, sqlx::Error> {
        let Some(account) = self.get_paper_account(account_id).await? else {
            return Ok(None);
        };

        let positions = self.list_paper_positions(account_id).await?;
        let open_orders = self.list_open_paper_orders(account_id).await?;
        let recent_fills = self.list_recent_paper_fills(account_id, 20).await?;

        Ok(Some(PaperAccountSummaryParts {
            account,
            positions,
            open_orders,
            recent_fills,
        }))
    }

    pub async fn execute_paper_market_order(
        &self,
        params: ExecutePaperMarketOrderParams,
    ) -> Result<ExecutePaperMarketOrderResult, sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        let now = Self::now_string();

        let mut account =
            match Self::get_paper_account_for_update(&mut tx, &params.account_id).await? {
                Some(account) => account,
                None => {
                    tx.rollback().await?;
                    return Err(sqlx::Error::RowNotFound);
                }
            };

        let existing_position =
            Self::get_paper_position_for_update(&mut tx, &params.account_id, &params.symbol)
                .await?;

        let notional = params.quantity * params.fill_price;

        if !account.is_active {
            tx.rollback().await?;
            return Err(sqlx::Error::Protocol("paper account is inactive".into()));
        }

        let (new_cash_balance, updated_position, order_status, event_message) = if params.side
            == "buy"
        {
            if account.cash_balance < notional {
                tx.rollback().await?;
                return Err(sqlx::Error::Protocol("insufficient cash".into()));
            }

            (
                account.cash_balance - notional,
                match existing_position {
                    Some(mut position) => {
                        let total_quantity = position.quantity + params.quantity;
                        let new_average_price = ((position.quantity * position.average_price)
                            + (params.quantity * params.fill_price))
                            / total_quantity;
                        position.quantity = total_quantity;
                        position.average_price = new_average_price;
                        position.updated_at = now.clone();
                        position
                    }
                    None => PaperPosition {
                        id: Self::new_id(),
                        account_id: params.account_id.clone(),
                        symbol: params.symbol.clone(),
                        quantity: params.quantity,
                        average_price: params.fill_price,
                        realized_pnl: 0.0,
                        created_at: now.clone(),
                        updated_at: now.clone(),
                    },
                },
                "filled".to_string(),
                format!(
                    "Filled buy market order for {} {} @ {:.4}",
                    params.quantity, params.symbol, params.fill_price
                ),
            )
        } else {
            let Some(mut position) = existing_position else {
                tx.rollback().await?;
                return Err(sqlx::Error::Protocol(
                    "insufficient position quantity".into(),
                ));
            };
            if position.quantity < params.quantity {
                tx.rollback().await?;
                return Err(sqlx::Error::Protocol(
                    "insufficient position quantity".into(),
                ));
            }
            position.quantity -= params.quantity;
            position.realized_pnl += (params.fill_price - position.average_price) * params.quantity;
            position.updated_at = now.clone();

            (
                account.cash_balance + notional,
                position,
                "filled".to_string(),
                format!(
                    "Filled sell market order for {} {} @ {:.4}",
                    params.quantity, params.symbol, params.fill_price
                ),
            )
        };

        let order = PaperOrder {
            id: Self::new_id(),
            account_id: params.account_id.clone(),
            symbol: params.symbol.clone(),
            side: params.side.clone(),
            order_type: params.order_type.clone(),
            quantity: params.quantity,
            requested_price: params.requested_price,
            filled_quantity: params.quantity,
            average_fill_price: Some(params.fill_price),
            status: order_status,
            source: params.source.clone(),
            strategy_id: params.strategy_id.clone(),
            signal_id: params.signal_id.clone(),
            created_at: now.clone(),
            updated_at: now.clone(),
        };

        sqlx::query(
            r#"
            INSERT INTO paper_orders (
                id, account_id, symbol, side, order_type, quantity, requested_price,
                filled_quantity, average_fill_price, status, source, strategy_id, signal_id,
                created_at, updated_at
            ) VALUES (
                $1::uuid, $2::uuid, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13::uuid,
                $14::timestamptz, $15::timestamptz
            )
            "#,
        )
        .bind(&order.id)
        .bind(&order.account_id)
        .bind(&order.symbol)
        .bind(&order.side)
        .bind(&order.order_type)
        .bind(order.quantity)
        .bind(order.requested_price)
        .bind(order.filled_quantity)
        .bind(order.average_fill_price)
        .bind(&order.status)
        .bind(&order.source)
        .bind(&order.strategy_id)
        .bind(&order.signal_id)
        .bind(&order.created_at)
        .bind(&order.updated_at)
        .execute(&mut *tx)
        .await?;

        let fill = PaperFill {
            id: Self::new_id(),
            account_id: params.account_id.clone(),
            order_id: order.id.clone(),
            symbol: params.symbol.clone(),
            side: params.side.clone(),
            quantity: params.quantity,
            price: params.fill_price,
            notional,
            created_at: now.clone(),
        };

        sqlx::query(
            r#"
            INSERT INTO paper_fills (
                id, account_id, order_id, symbol, side, quantity, price, notional, created_at
            ) VALUES ($1::uuid, $2::uuid, $3::uuid, $4, $5, $6, $7, $8, $9::timestamptz)
            "#,
        )
        .bind(&fill.id)
        .bind(&fill.account_id)
        .bind(&fill.order_id)
        .bind(&fill.symbol)
        .bind(&fill.side)
        .bind(fill.quantity)
        .bind(fill.price)
        .bind(fill.notional)
        .bind(&fill.created_at)
        .execute(&mut *tx)
        .await?;

        account.cash_balance = new_cash_balance;
        account.updated_at = now.clone();

        sqlx::query(
            r#"
            UPDATE paper_accounts
            SET cash_balance = $1, updated_at = $2::timestamptz
            WHERE id = $3::uuid
            "#,
        )
        .bind(account.cash_balance)
        .bind(&account.updated_at)
        .bind(&account.id)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            r#"
            INSERT INTO paper_positions (
                id, account_id, symbol, quantity, average_price, realized_pnl, created_at, updated_at
            ) VALUES (
                $1::uuid, $2::uuid, $3, $4, $5, $6, $7::timestamptz, $8::timestamptz
            )
            ON CONFLICT (account_id, symbol) DO UPDATE SET
                quantity = EXCLUDED.quantity,
                average_price = EXCLUDED.average_price,
                realized_pnl = EXCLUDED.realized_pnl,
                updated_at = EXCLUDED.updated_at
            "#,
        )
        .bind(&updated_position.id)
        .bind(&updated_position.account_id)
        .bind(&updated_position.symbol)
        .bind(updated_position.quantity)
        .bind(updated_position.average_price)
        .bind(updated_position.realized_pnl)
        .bind(&updated_position.created_at)
        .bind(&updated_position.updated_at)
        .execute(&mut *tx)
        .await?;

        for (event_type, message, payload) in [
            (
                "order_created",
                format!(
                    "Created {} market order for {} {}",
                    params.side, params.quantity, params.symbol
                ),
                Some(
                    serde_json::json!({
                        "order_id": order.id,
                        "symbol": params.symbol,
                        "side": params.side,
                        "quantity": params.quantity,
                        "source": params.source,
                    })
                    .to_string(),
                ),
            ),
            (
                "order_filled",
                event_message,
                Some(
                    serde_json::json!({
                        "order_id": order.id,
                        "fill_id": fill.id,
                        "price": fill.price,
                        "quantity": fill.quantity,
                        "notional": fill.notional,
                    })
                    .to_string(),
                ),
            ),
            (
                "position_updated",
                format!(
                    "Position for {} updated to {} shares at average {:.4}",
                    updated_position.symbol,
                    updated_position.quantity,
                    updated_position.average_price
                ),
                Some(
                    serde_json::json!({
                        "symbol": updated_position.symbol,
                        "quantity": updated_position.quantity,
                        "average_price": updated_position.average_price,
                        "realized_pnl": updated_position.realized_pnl,
                    })
                    .to_string(),
                ),
            ),
            (
                "cash_adjusted",
                format!("Cash balance updated to {:.4}", account.cash_balance),
                Some(
                    serde_json::json!({
                        "cash_balance": account.cash_balance,
                    })
                    .to_string(),
                ),
            ),
        ] {
            sqlx::query(
                r#"
                INSERT INTO paper_account_events (
                    id, account_id, event_type, message, payload, created_at
                ) VALUES ($1::uuid, $2::uuid, $3, $4, $5::jsonb, $6::timestamptz)
                "#,
            )
            .bind(Self::new_id())
            .bind(&account.id)
            .bind(event_type)
            .bind(message)
            .bind(payload)
            .bind(&now)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;

        Ok(ExecutePaperMarketOrderResult {
            account,
            order,
            fill,
            position: updated_position,
        })
    }

    pub async fn list_paper_orders(
        &self,
        account_id: &str,
    ) -> Result<Vec<PaperOrder>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT
                id::text AS id,
                account_id::text AS account_id,
                symbol,
                side,
                order_type,
                quantity,
                requested_price,
                filled_quantity,
                average_fill_price,
                status,
                source,
                strategy_id,
                signal_id::text AS signal_id,
                created_at::text AS created_at,
                updated_at::text AS updated_at
            FROM paper_orders
            WHERE account_id = $1::uuid
            ORDER BY created_at DESC, id DESC
            "#,
        )
        .bind(account_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(Self::row_to_paper_order).collect())
    }

    pub async fn list_paper_fills(&self, account_id: &str) -> Result<Vec<PaperFill>, sqlx::Error> {
        self.list_recent_paper_fills(account_id, i64::MAX).await
    }

    pub async fn list_paper_positions(
        &self,
        account_id: &str,
    ) -> Result<Vec<PaperPosition>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT
                id::text AS id,
                account_id::text AS account_id,
                symbol,
                quantity,
                average_price,
                realized_pnl,
                created_at::text AS created_at,
                updated_at::text AS updated_at
            FROM paper_positions
            WHERE account_id = $1::uuid
            ORDER BY symbol ASC, created_at ASC
            "#,
        )
        .bind(account_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(Self::row_to_paper_position).collect())
    }

    pub async fn get_paper_position(
        &self,
        account_id: &str,
        symbol: &str,
    ) -> Result<Option<PaperPosition>, sqlx::Error> {
        let row = sqlx::query(
            r#"
            SELECT
                id::text AS id,
                account_id::text AS account_id,
                symbol,
                quantity,
                average_price,
                realized_pnl,
                created_at::text AS created_at,
                updated_at::text AS updated_at
            FROM paper_positions
            WHERE account_id = $1::uuid AND symbol = $2
            "#,
        )
        .bind(account_id)
        .bind(symbol)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Self::row_to_paper_position))
    }

    pub async fn list_paper_events(
        &self,
        account_id: &str,
    ) -> Result<Vec<PaperAccountEvent>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT
                id::text AS id,
                account_id::text AS account_id,
                event_type,
                message,
                payload::text AS payload,
                created_at::text AS created_at
            FROM paper_account_events
            WHERE account_id = $1::uuid
            ORDER BY created_at DESC, id DESC
            "#,
        )
        .bind(account_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(Self::row_to_paper_account_event)
            .collect())
    }

    pub async fn insert_paper_account_event(
        &self,
        account_id: &str,
        event_type: &str,
        message: &str,
        payload: Option<&str>,
    ) -> Result<PaperAccountEvent, sqlx::Error> {
        let event = PaperAccountEvent {
            id: Self::new_id(),
            account_id: account_id.to_string(),
            event_type: event_type.to_string(),
            message: message.to_string(),
            payload: payload.map(ToOwned::to_owned),
            created_at: Self::now_string(),
        };

        sqlx::query(
            r#"
            INSERT INTO paper_account_events (
                id, account_id, event_type, message, payload, created_at
            ) VALUES ($1::uuid, $2::uuid, $3, $4, $5::jsonb, $6::timestamptz)
            "#,
        )
        .bind(&event.id)
        .bind(&event.account_id)
        .bind(&event.event_type)
        .bind(&event.message)
        .bind(event.payload.as_deref())
        .bind(&event.created_at)
        .execute(&self.pool)
        .await?;

        Ok(event)
    }

    pub async fn cancel_paper_order(
        &self,
        order_id: &str,
    ) -> Result<Option<PaperOrder>, sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        let row = sqlx::query(
            r#"
            SELECT
                id::text AS id,
                account_id::text AS account_id,
                symbol,
                side,
                order_type,
                quantity,
                requested_price,
                filled_quantity,
                average_fill_price,
                status,
                source,
                strategy_id,
                signal_id::text AS signal_id,
                created_at::text AS created_at,
                updated_at::text AS updated_at
            FROM paper_orders
            WHERE id = $1::uuid
            FOR UPDATE
            "#,
        )
        .bind(order_id)
        .fetch_optional(&mut *tx)
        .await?;

        let Some(row) = row else {
            tx.rollback().await?;
            return Ok(None);
        };

        let mut order = Self::row_to_paper_order(row);
        if order.status != "pending" {
            tx.rollback().await?;
            return Ok(Some(order));
        }

        order.status = "cancelled".to_string();
        order.updated_at = Self::now_string();

        sqlx::query(
            r#"
            UPDATE paper_orders
            SET status = 'cancelled', updated_at = $1::timestamptz
            WHERE id = $2::uuid
            "#,
        )
        .bind(&order.updated_at)
        .bind(&order.id)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            r#"
            INSERT INTO paper_account_events (
                id, account_id, event_type, message, payload, created_at
            ) VALUES ($1::uuid, $2::uuid, $3, $4, $5::jsonb, $6::timestamptz)
            "#,
        )
        .bind(Self::new_id())
        .bind(&order.account_id)
        .bind("order_cancelled")
        .bind(format!("Cancelled order {}", order.id))
        .bind(Some(
            serde_json::json!({
                "order_id": order.id,
                "status": order.status,
            })
            .to_string(),
        ))
        .bind(&order.updated_at)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(Some(order))
    }

    pub async fn get_strategy_trading_config(
        &self,
        strategy_id: &str,
    ) -> Result<Option<StrategyTradingConfig>, sqlx::Error> {
        let row = sqlx::query(
            r#"
            SELECT
                strategy_id,
                trading_mode,
                paper_account_id::text AS paper_account_id,
                is_enabled,
                last_started_at::text AS last_started_at,
                last_stopped_at::text AS last_stopped_at,
                created_at::text AS created_at,
                updated_at::text AS updated_at
            FROM strategy_trading_config
            WHERE strategy_id = $1
            "#,
        )
        .bind(strategy_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Self::row_to_strategy_trading_config))
    }

    pub async fn upsert_strategy_trading_config(
        &self,
        config: &StrategyTradingConfig,
    ) -> Result<StrategyTradingConfig, sqlx::Error> {
        let created_at = config.created_at.clone().unwrap_or_else(Self::now_string);
        let updated_at = config.updated_at.clone().unwrap_or_else(Self::now_string);

        let row = sqlx::query(
            r#"
            INSERT INTO strategy_trading_config (
                strategy_id, trading_mode, paper_account_id, is_enabled, last_started_at, last_stopped_at, created_at, updated_at
            ) VALUES ($1, $2, $3::uuid, $4, $5::timestamptz, $6::timestamptz, $7::timestamptz, $8::timestamptz)
            ON CONFLICT (strategy_id) DO UPDATE SET
                trading_mode = EXCLUDED.trading_mode,
                paper_account_id = EXCLUDED.paper_account_id,
                is_enabled = EXCLUDED.is_enabled,
                last_started_at = EXCLUDED.last_started_at,
                last_stopped_at = EXCLUDED.last_stopped_at,
                updated_at = EXCLUDED.updated_at
            RETURNING
                strategy_id,
                trading_mode,
                paper_account_id::text AS paper_account_id,
                is_enabled,
                last_started_at::text AS last_started_at,
                last_stopped_at::text AS last_stopped_at,
                created_at::text AS created_at,
                updated_at::text AS updated_at
            "#,
        )
        .bind(&config.strategy_id)
        .bind(&config.trading_mode)
        .bind(&config.paper_account_id)
        .bind(config.is_enabled)
        .bind(&config.last_started_at)
        .bind(&config.last_stopped_at)
        .bind(created_at)
        .bind(updated_at)
        .fetch_one(&self.pool)
        .await?;

        Ok(Self::row_to_strategy_trading_config(row))
    }

    pub async fn get_strategy_risk_config(
        &self,
        strategy_id: &str,
    ) -> Result<Option<StrategyRiskConfig>, sqlx::Error> {
        let row = sqlx::query(
            r#"
            SELECT
                strategy_id,
                max_dollars_per_trade,
                max_quantity_per_trade,
                max_position_value_per_symbol,
                max_total_exposure,
                max_open_positions,
                max_daily_trades,
                max_daily_loss,
                cooldown_seconds,
                allowlist_symbols,
                blocklist_symbols,
                is_trading_enabled,
                kill_switch_enabled,
                created_at::text AS created_at,
                updated_at::text AS updated_at
            FROM strategy_risk_config
            WHERE strategy_id = $1
            "#,
        )
        .bind(strategy_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Self::row_to_strategy_risk_config))
    }

    pub async fn upsert_strategy_risk_config(
        &self,
        config: &StrategyRiskConfig,
    ) -> Result<StrategyRiskConfig, sqlx::Error> {
        let created_at = config.created_at.clone().unwrap_or_else(Self::now_string);
        let updated_at = config.updated_at.clone().unwrap_or_else(Self::now_string);
        let allowlist_symbols = Self::string_vec_to_json_value(&config.allowlist_symbols);
        let blocklist_symbols = Self::string_vec_to_json_value(&config.blocklist_symbols);

        let row = sqlx::query(
            r#"
            INSERT INTO strategy_risk_config (
                strategy_id, max_dollars_per_trade, max_quantity_per_trade,
                max_position_value_per_symbol, max_total_exposure, max_open_positions,
                max_daily_trades, max_daily_loss, cooldown_seconds, allowlist_symbols,
                blocklist_symbols, is_trading_enabled, kill_switch_enabled, created_at, updated_at
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10::jsonb, $11::jsonb, $12, $13,
                $14::timestamptz, $15::timestamptz
            )
            ON CONFLICT (strategy_id) DO UPDATE SET
                max_dollars_per_trade = EXCLUDED.max_dollars_per_trade,
                max_quantity_per_trade = EXCLUDED.max_quantity_per_trade,
                max_position_value_per_symbol = EXCLUDED.max_position_value_per_symbol,
                max_total_exposure = EXCLUDED.max_total_exposure,
                max_open_positions = EXCLUDED.max_open_positions,
                max_daily_trades = EXCLUDED.max_daily_trades,
                max_daily_loss = EXCLUDED.max_daily_loss,
                cooldown_seconds = EXCLUDED.cooldown_seconds,
                allowlist_symbols = EXCLUDED.allowlist_symbols,
                blocklist_symbols = EXCLUDED.blocklist_symbols,
                is_trading_enabled = EXCLUDED.is_trading_enabled,
                kill_switch_enabled = EXCLUDED.kill_switch_enabled,
                updated_at = EXCLUDED.updated_at
            RETURNING
                strategy_id,
                max_dollars_per_trade,
                max_quantity_per_trade,
                max_position_value_per_symbol,
                max_total_exposure,
                max_open_positions,
                max_daily_trades,
                max_daily_loss,
                cooldown_seconds,
                allowlist_symbols,
                blocklist_symbols,
                is_trading_enabled,
                kill_switch_enabled,
                created_at::text AS created_at,
                updated_at::text AS updated_at
            "#,
        )
        .bind(&config.strategy_id)
        .bind(config.max_dollars_per_trade)
        .bind(config.max_quantity_per_trade)
        .bind(config.max_position_value_per_symbol)
        .bind(config.max_total_exposure)
        .bind(config.max_open_positions)
        .bind(config.max_daily_trades)
        .bind(config.max_daily_loss)
        .bind(config.cooldown_seconds)
        .bind(allowlist_symbols)
        .bind(blocklist_symbols)
        .bind(config.is_trading_enabled)
        .bind(config.kill_switch_enabled)
        .bind(created_at)
        .bind(updated_at)
        .fetch_one(&self.pool)
        .await?;

        Ok(Self::row_to_strategy_risk_config(row))
    }

    pub async fn list_engine_strategy_configs(
        &self,
    ) -> Result<Vec<EngineRunnableStrategy>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT
                stc.strategy_id,
                p.name,
                stc.trading_mode,
                stc.paper_account_id::text AS paper_account_id,
                p.strategy_json,
                p.interval
            FROM strategy_trading_config stc
            INNER JOIN projects p ON p.id = stc.strategy_id
            WHERE stc.trading_mode = 'paper'
              AND stc.is_enabled = TRUE
              AND stc.paper_account_id IS NOT NULL
              AND p.strategy_json <> ''
            ORDER BY stc.updated_at DESC, stc.strategy_id ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        let mut strategies = Vec::new();
        for row in rows {
            let strategy_id: String = row.get("strategy_id");
            let strategy_json: String = row.get("strategy_json");
            let Some(strategy_definition) = Self::parse_strategy_definition(&strategy_json) else {
                continue;
            };
            let symbol_universe = self.get_project_symbols(&strategy_id).await?;
            let risk_config = self
                .get_strategy_risk_config(&strategy_id)
                .await?
                .unwrap_or_else(|| Self::default_strategy_risk_config(&strategy_id));

            if !risk_config.is_trading_enabled || risk_config.kill_switch_enabled {
                continue;
            }

            strategies.push(EngineRunnableStrategy {
                strategy_id,
                name: row.get("name"),
                trading_mode: row.get("trading_mode"),
                paper_account_id: row.get("paper_account_id"),
                timeframe: row.get("interval"),
                risk: strategy_definition.risk.clone(),
                risk_config,
                strategy_definition,
                symbol_universe,
            });
        }

        Ok(strategies)
    }

    pub async fn upsert_strategy_runtime_state(
        &self,
        request: &UpsertStrategyRuntimeStateRequest,
    ) -> Result<StrategyRuntimeState, sqlx::Error> {
        let now = Self::now_string();
        let row = sqlx::query(
            r#"
            INSERT INTO strategy_runtime_state (
                id, strategy_id, paper_account_id, symbol, last_evaluated_at, last_signal,
                last_signal_at, last_order_id, position_state, cooldown_until, created_at, updated_at
            ) VALUES (
                $1::uuid, $2, $3::uuid, $4, $5::timestamptz, $6, $7::timestamptz, $8::uuid,
                $9, $10::timestamptz, $11::timestamptz, $12::timestamptz
            )
            ON CONFLICT (strategy_id, paper_account_id, symbol) DO UPDATE SET
                last_evaluated_at = EXCLUDED.last_evaluated_at,
                last_signal = EXCLUDED.last_signal,
                last_signal_at = EXCLUDED.last_signal_at,
                last_order_id = EXCLUDED.last_order_id,
                position_state = EXCLUDED.position_state,
                cooldown_until = EXCLUDED.cooldown_until,
                updated_at = EXCLUDED.updated_at
            RETURNING
                id::text AS id,
                strategy_id,
                paper_account_id::text AS paper_account_id,
                symbol,
                last_evaluated_at::text AS last_evaluated_at,
                last_signal,
                last_signal_at::text AS last_signal_at,
                last_order_id::text AS last_order_id,
                position_state,
                cooldown_until::text AS cooldown_until,
                created_at::text AS created_at,
                updated_at::text AS updated_at
            "#,
        )
        .bind(Self::new_id())
        .bind(&request.strategy_id)
        .bind(&request.paper_account_id)
        .bind(&request.symbol)
        .bind(&request.last_evaluated_at)
        .bind(&request.last_signal)
        .bind(&request.last_signal_at)
        .bind(&request.last_order_id)
        .bind(&request.position_state)
        .bind(&request.cooldown_until)
        .bind(&now)
        .bind(&now)
        .fetch_one(&self.pool)
        .await?;

        Ok(Self::row_to_strategy_runtime_state(row))
    }

    pub async fn list_strategy_runtime_state(
        &self,
        strategy_id: &str,
    ) -> Result<Vec<StrategyRuntimeState>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT
                id::text AS id,
                strategy_id,
                paper_account_id::text AS paper_account_id,
                symbol,
                last_evaluated_at::text AS last_evaluated_at,
                last_signal,
                last_signal_at::text AS last_signal_at,
                last_order_id::text AS last_order_id,
                position_state,
                cooldown_until::text AS cooldown_until,
                created_at::text AS created_at,
                updated_at::text AS updated_at
            FROM strategy_runtime_state
            WHERE strategy_id = $1
            ORDER BY updated_at DESC, symbol ASC
            "#,
        )
        .bind(strategy_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(Self::row_to_strategy_runtime_state)
            .collect())
    }

    pub async fn get_strategy_runtime_state_for_symbol(
        &self,
        strategy_id: &str,
        paper_account_id: &str,
        symbol: &str,
    ) -> Result<Option<StrategyRuntimeState>, sqlx::Error> {
        let row = sqlx::query(
            r#"
            SELECT
                id::text AS id,
                strategy_id,
                paper_account_id::text AS paper_account_id,
                symbol,
                last_evaluated_at::text AS last_evaluated_at,
                last_signal,
                last_signal_at::text AS last_signal_at,
                last_order_id::text AS last_order_id,
                position_state,
                cooldown_until::text AS cooldown_until,
                created_at::text AS created_at,
                updated_at::text AS updated_at
            FROM strategy_runtime_state
            WHERE strategy_id = $1 AND paper_account_id = $2::uuid AND symbol = $3
            "#,
        )
        .bind(strategy_id)
        .bind(paper_account_id)
        .bind(symbol)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Self::row_to_strategy_runtime_state))
    }

    pub async fn create_strategy_signal(
        &self,
        request: &CreateStrategySignalRequest,
    ) -> Result<StrategySignal, sqlx::Error> {
        let signal = StrategySignal {
            id: Self::new_id(),
            strategy_id: request.strategy_id.clone(),
            paper_account_id: request.paper_account_id.clone(),
            symbol: request.symbol.clone(),
            signal_type: request.signal_type.clone(),
            confidence: request.confidence,
            reason: request.reason.clone(),
            market_price: request.market_price,
            source: request
                .source
                .clone()
                .unwrap_or_else(|| "engine".to_string()),
            status: request
                .status
                .clone()
                .unwrap_or_else(|| "created".to_string()),
            risk_decision: request.risk_decision.clone(),
            risk_reason: request.risk_reason.clone(),
            order_id: request.order_id.clone(),
            created_at: Self::now_string(),
        };

        sqlx::query(
            r#"
            INSERT INTO strategy_signals (
                id, strategy_id, paper_account_id, symbol, signal_type, confidence, reason,
                market_price, source, status, risk_decision, risk_reason, order_id, created_at
            ) VALUES (
                $1::uuid, $2, $3::uuid, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13::uuid, $14::timestamptz
            )
            "#,
        )
        .bind(&signal.id)
        .bind(&signal.strategy_id)
        .bind(&signal.paper_account_id)
        .bind(&signal.symbol)
        .bind(&signal.signal_type)
        .bind(signal.confidence)
        .bind(&signal.reason)
        .bind(signal.market_price)
        .bind(&signal.source)
        .bind(&signal.status)
        .bind(&signal.risk_decision)
        .bind(&signal.risk_reason)
        .bind(&signal.order_id)
        .bind(&signal.created_at)
        .execute(&self.pool)
        .await?;

        Ok(signal)
    }

    pub async fn update_strategy_signal(
        &self,
        signal_id: &str,
        order_id: Option<&str>,
        status: &str,
        risk_decision: Option<&str>,
        risk_reason: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE strategy_signals
            SET order_id = $1::uuid, status = $2, risk_decision = $3, risk_reason = $4
            WHERE id = $5::uuid
            "#,
        )
        .bind(order_id)
        .bind(status)
        .bind(risk_decision)
        .bind(risk_reason)
        .bind(signal_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn update_strategy_signal_status(
        &self,
        signal_id: &str,
        status: &str,
        risk_decision: Option<&str>,
        risk_reason: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE strategy_signals
            SET status = $1, risk_decision = $2, risk_reason = $3
            WHERE id = $4::uuid
            "#,
        )
        .bind(status)
        .bind(risk_decision)
        .bind(risk_reason)
        .bind(signal_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn list_strategy_signals(
        &self,
        strategy_id: &str,
    ) -> Result<Vec<StrategySignal>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT
                id::text AS id,
                strategy_id,
                paper_account_id::text AS paper_account_id,
                symbol,
                signal_type,
                confidence,
                reason,
                market_price,
                source,
                status,
                risk_decision,
                risk_reason,
                order_id::text AS order_id,
                created_at::text AS created_at
            FROM strategy_signals
            WHERE strategy_id = $1
            ORDER BY created_at DESC, id DESC
            LIMIT 100
            "#,
        )
        .bind(strategy_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(Self::row_to_strategy_signal).collect())
    }

    async fn list_open_paper_orders(
        &self,
        account_id: &str,
    ) -> Result<Vec<PaperOrder>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT
                id::text AS id,
                account_id::text AS account_id,
                symbol,
                side,
                order_type,
                quantity,
                requested_price,
                filled_quantity,
                average_fill_price,
                status,
                source,
                strategy_id,
                signal_id::text AS signal_id,
                created_at::text AS created_at,
                updated_at::text AS updated_at
            FROM paper_orders
            WHERE account_id = $1::uuid AND status = 'pending'
            ORDER BY created_at DESC, id DESC
            "#,
        )
        .bind(account_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(Self::row_to_paper_order).collect())
    }

    async fn list_recent_paper_fills(
        &self,
        account_id: &str,
        limit: i64,
    ) -> Result<Vec<PaperFill>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT
                id::text AS id,
                account_id::text AS account_id,
                order_id::text AS order_id,
                symbol,
                side,
                quantity,
                price,
                notional,
                created_at::text AS created_at
            FROM paper_fills
            WHERE account_id = $1::uuid
            ORDER BY created_at DESC, id DESC
            LIMIT $2
            "#,
        )
        .bind(account_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(Self::row_to_paper_fill).collect())
    }

    pub async fn create_project(&self, project: &Project) -> Result<(), sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        sqlx::query(
            r#"
            INSERT INTO projects (
                id, name, description, strategy, strategy_json, strategy_status, created_at, updated_at, interval, range, prepost
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            "#,
        )
        .bind(&project.id)
        .bind(&project.name)
        .bind(&project.description)
        .bind(&project.strategy)
        .bind(&project.strategy_json)
        .bind(&project.strategy_status)
        .bind(&project.created_at)
        .bind(&project.updated_at)
        .bind(&project.interval)
        .bind(&project.range)
        .bind(project.prepost)
        .execute(&mut *tx)
        .await?;

        self.insert_project_symbols(&mut tx, &project.id, &project.symbols)
            .await?;

        tx.commit().await?;
        Ok(())
    }

    pub async fn get_project(&self, id: &str) -> Result<Option<Project>, sqlx::Error> {
        let row = sqlx::query(
            r#"
            SELECT id, name, description, strategy, strategy_json, strategy_status, created_at, updated_at, interval, range, prepost
            FROM projects
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        let Some(row) = row else {
            return Ok(None);
        };

        let symbols = self.get_project_symbols(id).await?;

        Ok(Some(Project {
            id: row.get("id"),
            name: row.get("name"),
            description: row.get("description"),
            strategy: row.get("strategy"),
            strategy_json: row.get("strategy_json"),
            strategy_status: row.get("strategy_status"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            symbols,
            interval: row.get("interval"),
            range: row.get("range"),
            prepost: row.get("prepost"),
        }))
    }

    pub async fn list_projects(&self) -> Result<Vec<Project>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT id, name, description, strategy, strategy_json, strategy_status, created_at, updated_at, interval, range, prepost
            FROM projects
            ORDER BY created_at DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        let mut projects = Vec::with_capacity(rows.len());

        for row in rows {
            let id: String = row.get("id");
            let symbols = self.get_project_symbols(&id).await?;

            projects.push(Project {
                id,
                name: row.get("name"),
                description: row.get("description"),
                strategy: row.get("strategy"),
                strategy_json: row.get("strategy_json"),
                strategy_status: row.get("strategy_status"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
                symbols,
                interval: row.get("interval"),
                range: row.get("range"),
                prepost: row.get("prepost"),
            });
        }

        Ok(projects)
    }

    pub async fn update_project(&self, project: &Project) -> Result<bool, sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        let result = sqlx::query(
            r#"
            UPDATE projects
            SET
                name = $1,
                description = $2,
                strategy = $3,
                strategy_json = $4,
                strategy_status = $5,
                updated_at = $6,
                interval = $7,
                range = $8,
                prepost = $9
            WHERE id = $10
            "#,
        )
        .bind(&project.name)
        .bind(&project.description)
        .bind(&project.strategy)
        .bind(&project.strategy_json)
        .bind(&project.strategy_status)
        .bind(&project.updated_at)
        .bind(&project.interval)
        .bind(&project.range)
        .bind(project.prepost)
        .bind(&project.id)
        .execute(&mut *tx)
        .await?;

        if result.rows_affected() == 0 {
            tx.rollback().await?;
            return Ok(false);
        }

        sqlx::query("DELETE FROM project_symbols WHERE project_id = $1")
            .bind(&project.id)
            .execute(&mut *tx)
            .await?;

        self.insert_project_symbols(&mut tx, &project.id, &project.symbols)
            .await?;

        tx.commit().await?;
        Ok(true)
    }

    pub async fn delete_project(&self, id: &str) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("DELETE FROM projects WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn add_project_symbol(
        &self,
        project_id: &str,
        symbol: &str,
    ) -> Result<(), sqlx::Error> {
        let next_ordinal: i64 = sqlx::query_scalar(
            r#"
            SELECT COALESCE(MAX(ordinal) + 1, 0)
            FROM project_symbols
            WHERE project_id = $1
            "#,
        )
        .bind(project_id)
        .fetch_one(&self.pool)
        .await?;

        sqlx::query(
            r#"
            INSERT INTO project_symbols (project_id, symbol, ordinal)
            VALUES ($1, $2, $3)
            "#,
        )
        .bind(project_id)
        .bind(symbol)
        .bind(next_ordinal)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn remove_project_symbol(
        &self,
        project_id: &str,
        symbol: &str,
    ) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            r#"
            DELETE FROM project_symbols
            WHERE project_id = $1 AND symbol = $2
            "#,
        )
        .bind(project_id)
        .bind(symbol)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn list_active_symbol_configs(&self) -> Result<Vec<ActiveSymbol>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT id, symbol, enabled, created_at, updated_at
            FROM active_symbols
            ORDER BY symbol ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| ActiveSymbol {
                id: row.get("id"),
                symbol: row.get("symbol"),
                enabled: row.get("enabled"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            })
            .collect())
    }

    pub async fn list_active_symbols(&self) -> Result<Vec<String>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT symbol
            FROM active_symbols
            WHERE enabled = TRUE
            ORDER BY symbol ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|row| row.get("symbol")).collect())
    }

    pub async fn insert_engine_heartbeat(
        &self,
        request: &EngineHeartbeatRequest,
    ) -> Result<EngineHeartbeat, sqlx::Error> {
        let heartbeat = EngineHeartbeat {
            id: Self::new_id(),
            engine_name: request.engine_name.clone(),
            status: request.status.clone(),
            timestamp: request.timestamp.clone(),
            created_at: Self::now_string(),
        };

        sqlx::query(
            r#"
            INSERT INTO engine_heartbeats (id, engine_name, status, timestamp, created_at)
            VALUES ($1, $2, $3, $4, $5)
            "#,
        )
        .bind(&heartbeat.id)
        .bind(&heartbeat.engine_name)
        .bind(&heartbeat.status)
        .bind(&heartbeat.timestamp)
        .bind(&heartbeat.created_at)
        .execute(&self.pool)
        .await?;

        Ok(heartbeat)
    }

    pub async fn insert_engine_event(
        &self,
        request: &EngineEventRequest,
    ) -> Result<EngineEvent, sqlx::Error> {
        let event = EngineEvent {
            id: Self::new_id(),
            engine_name: request.engine_name.clone(),
            event_type: request.event_type.clone(),
            symbol: request.symbol.clone(),
            message: request.message.clone(),
            timestamp: request.timestamp.clone(),
            created_at: Self::now_string(),
        };

        sqlx::query(
            r#"
            INSERT INTO engine_events (
                id, engine_name, event_type, symbol, message, timestamp, created_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
        )
        .bind(&event.id)
        .bind(&event.engine_name)
        .bind(&event.event_type)
        .bind(&event.symbol)
        .bind(&event.message)
        .bind(&event.timestamp)
        .bind(&event.created_at)
        .execute(&self.pool)
        .await?;

        Ok(event)
    }

    async fn insert_project_symbols(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        project_id: &str,
        symbols: &[String],
    ) -> Result<(), sqlx::Error> {
        for (ordinal, symbol) in symbols.iter().enumerate() {
            sqlx::query(
                r#"
                INSERT INTO project_symbols (project_id, symbol, ordinal)
                VALUES ($1, $2, $3)
                "#,
            )
            .bind(project_id)
            .bind(symbol)
            .bind(ordinal as i64)
            .execute(&mut **tx)
            .await?;
        }

        Ok(())
    }

    async fn get_project_symbols(&self, project_id: &str) -> Result<Vec<String>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT symbol
            FROM project_symbols
            WHERE project_id = $1
            ORDER BY ordinal ASC
            "#,
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|row| row.get("symbol")).collect())
    }

    pub async fn create_portfolio(&self, portfolio: &Portfolio) -> Result<(), sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        sqlx::query(
            r#"
            INSERT INTO portfolios (
                id, name, description, created_at, updated_at
            ) VALUES ($1, $2, $3, $4, $5)
            "#,
        )
        .bind(&portfolio.id)
        .bind(&portfolio.name)
        .bind(&portfolio.description)
        .bind(&portfolio.created_at)
        .bind(&portfolio.updated_at)
        .execute(&mut *tx)
        .await?;

        self.insert_positions(&mut tx, &portfolio.id, &portfolio.positions)
            .await?;

        tx.commit().await?;
        Ok(())
    }

    pub async fn get_portfolio(&self, id: &str) -> Result<Option<Portfolio>, sqlx::Error> {
        let row = sqlx::query(
            r#"
            SELECT id, name, description, created_at, updated_at
            FROM portfolios
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        let Some(row) = row else {
            return Ok(None);
        };

        let positions = self.get_positions(id).await?;

        Ok(Some(Portfolio {
            id: row.get("id"),
            name: row.get("name"),
            description: row.get("description"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            positions,
        }))
    }

    pub async fn list_portfolios(&self) -> Result<Vec<Portfolio>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT id, name, description, created_at, updated_at
            FROM portfolios
            ORDER BY created_at DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        let mut portfolios = Vec::with_capacity(rows.len());

        for row in rows {
            let id: String = row.get("id");
            let positions = self.get_positions(&id).await?;

            portfolios.push(Portfolio {
                id,
                name: row.get("name"),
                description: row.get("description"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
                positions,
            });
        }

        Ok(portfolios)
    }

    pub async fn update_portfolio(&self, portfolio: &Portfolio) -> Result<bool, sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        let result = sqlx::query(
            r#"
            UPDATE portfolios
            SET
                name = $1,
                description = $2,
                updated_at = $3
            WHERE id = $4
            "#,
        )
        .bind(&portfolio.name)
        .bind(&portfolio.description)
        .bind(&portfolio.updated_at)
        .bind(&portfolio.id)
        .execute(&mut *tx)
        .await?;

        if result.rows_affected() == 0 {
            tx.rollback().await?;
            return Ok(false);
        }

        sqlx::query("DELETE FROM positions WHERE portfolio_id = $1")
            .bind(&portfolio.id)
            .execute(&mut *tx)
            .await?;

        self.insert_positions(&mut tx, &portfolio.id, &portfolio.positions)
            .await?;

        tx.commit().await?;
        Ok(true)
    }

    pub async fn delete_portfolio(&self, id: &str) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("DELETE FROM portfolios WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    async fn insert_positions(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        portfolio_id: &str,
        positions: &[Position],
    ) -> Result<(), sqlx::Error> {
        for (ordinal, position) in positions.iter().enumerate() {
            sqlx::query(
                r#"
                INSERT INTO positions (
                    portfolio_id,
                    symbol,
                    quantity,
                    average_price,
                    position_opened_at,
                    position_closed_at,
                    position_closed_price,
                    ordinal
                ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                "#,
            )
            .bind(portfolio_id)
            .bind(&position.symbol)
            .bind(position.quantity)
            .bind(position.average_price)
            .bind(&position.position_opened_at)
            .bind(&position.position_closed_at)
            .bind(position.position_closed_price)
            .bind(ordinal as i64)
            .execute(&mut **tx)
            .await?;
        }

        Ok(())
    }

    async fn get_positions(&self, portfolio_id: &str) -> Result<Vec<Position>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT
                symbol,
                quantity,
                average_price,
                position_opened_at,
                position_closed_at,
                position_closed_price
            FROM positions
            WHERE portfolio_id = $1
            ORDER BY ordinal ASC, id ASC
            "#,
        )
        .bind(portfolio_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| Position {
                symbol: row.get("symbol"),
                quantity: row.get("quantity"),
                average_price: row.get("average_price"),
                position_opened_at: row.get("position_opened_at"),
                position_closed_at: row.get("position_closed_at"),
                position_closed_price: row.get("position_closed_price"),
            })
            .collect())
    }

    pub async fn list_positions(&self, portfolio_id: &str) -> Result<Vec<Position>, sqlx::Error> {
        self.get_positions(portfolio_id).await
    }

    pub async fn add_position(
        &self,
        portfolio_id: &str,
        position: &Position,
    ) -> Result<(), sqlx::Error> {
        let next_ordinal: i64 = sqlx::query_scalar(
            r#"
            SELECT COALESCE(MAX(ordinal) + 1, 0)
            FROM positions
            WHERE portfolio_id = $1
            "#,
        )
        .bind(portfolio_id)
        .fetch_one(&self.pool)
        .await?;

        sqlx::query(
            r#"
            INSERT INTO positions (
                portfolio_id,
                symbol,
                quantity,
                average_price,
                position_opened_at,
                position_closed_at,
                position_closed_price,
                ordinal
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
        )
        .bind(portfolio_id)
        .bind(&position.symbol)
        .bind(position.quantity)
        .bind(position.average_price)
        .bind(&position.position_opened_at)
        .bind(&position.position_closed_at)
        .bind(position.position_closed_price)
        .bind(next_ordinal)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn update_position(
        &self,
        portfolio_id: &str,
        old_symbol: &str,
        old_position_opened_at: &str,
        updated: &Position,
    ) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            r#"
            UPDATE positions
            SET
                symbol = $1,
                quantity = $2,
                average_price = $3,
                position_opened_at = $4,
                position_closed_at = $5,
                position_closed_price = $6
            WHERE
                portfolio_id = $7
                AND symbol = $8
                AND position_opened_at = $9
            "#,
        )
        .bind(&updated.symbol)
        .bind(updated.quantity)
        .bind(updated.average_price)
        .bind(&updated.position_opened_at)
        .bind(&updated.position_closed_at)
        .bind(updated.position_closed_price)
        .bind(portfolio_id)
        .bind(old_symbol)
        .bind(old_position_opened_at)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn delete_position(
        &self,
        portfolio_id: &str,
        symbol: &str,
        position_opened_at: &str,
    ) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            r#"
            DELETE FROM positions
            WHERE
                portfolio_id = $1
                AND symbol = $2
                AND position_opened_at = $3
            "#,
        )
        .bind(portfolio_id)
        .bind(symbol)
        .bind(position_opened_at)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }
}
