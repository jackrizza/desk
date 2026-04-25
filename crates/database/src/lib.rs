use chrono::Utc;
use models::{
    channels::{
        Channel, ChannelMessage, CreateChannelMessageRequest, CreateTraderMemoryRequest,
        DataScientistProfile, EngineChannelContext, MdProfile, TraderMemory,
        TraderMemorySearchResponse, TraderPersona, TraderPersonaUpdateRequest,
        UpdateDataScientistProfileRequest, UpdateMdProfileRequest, UpdateTraderMemoryRequest,
        UpdateUserInvestorProfileRequest, UserInvestorProfile,
    },
    data_sources::{
        CreateDataSourceRequest, DataSource, DataSourceEvent, DataSourceItem, DataSourceScript,
        EngineTraderDataSource, TraderDataSourceAssignment, UpdateDataSourceRequest,
    },
    engine::{
        ActiveSymbol, EngineEvent, EngineEventRequest, EngineHeartbeat, EngineHeartbeatRequest,
    },
    paper::{PaperAccount, PaperAccountEvent, PaperFill, PaperOrder, PaperPosition},
    portfolio::{Portfolio, Position},
    projects::Project,
    trader::{
        CreateTraderEventRequest, CreateTraderInfoSourceRequest,
        CreateTraderPortfolioProposalActionRequest, CreateTraderPortfolioProposalRequest,
        CreateTraderSymbolRequest, CreateTraderTradeProposalRequest, EngineRunnableTrader, Trader,
        TraderEvent, TraderInfoSource, TraderPortfolioProposal, TraderPortfolioProposalAction,
        TraderPortfolioProposalDetail, TraderRuntimeState, TraderSymbol, TraderTradeProposal,
        UpdateTraderSymbolRequest, UpsertTraderRuntimeStateRequest,
    },
    trading::{
        CreateStrategySignalRequest, EngineRunnableStrategy, StrategyDefinition,
        StrategyRiskConfig, StrategyRuntimeState, StrategySignal, StrategyTradingConfig,
        UpsertStrategyRuntimeStateRequest,
    },
};
use serde_json::Value;
use sqlx::{PgPool, Postgres, QueryBuilder, Row, Transaction, postgres::PgPoolOptions};
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
    pub trader_id: Option<String>,
    pub strategy_id: Option<String>,
    pub signal_id: Option<String>,
    pub proposal_id: Option<String>,
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
                trader_id UUID NULL,
                strategy_id TEXT NULL,
                signal_id UUID NULL,
                proposal_id UUID NULL,
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
            ADD COLUMN IF NOT EXISTS trader_id UUID NULL
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
            ALTER TABLE paper_orders
            ADD COLUMN IF NOT EXISTS proposal_id UUID NULL
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
            "CREATE INDEX IF NOT EXISTS idx_paper_orders_trader_id ON paper_orders(trader_id);",
            "CREATE INDEX IF NOT EXISTS idx_paper_orders_proposal_id ON paper_orders(proposal_id);",
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

        self.init_trader_tables().await?;
        self.init_scrapper_tables().await?;
        self.seed_active_symbols().await?;

        Ok(())
    }

    async fn init_scrapper_tables(&self) -> Result<(), sqlx::Error> {
        for statement in [
            "CREATE SCHEMA IF NOT EXISTS scrapper;",
            r#"
            CREATE TABLE IF NOT EXISTS scrapper.data_sources (
                id UUID PRIMARY KEY,
                name TEXT NOT NULL,
                source_type TEXT NOT NULL CHECK (source_type IN ('rss', 'web_page', 'manual_note', 'placeholder_api', 'python_script')),
                url TEXT NULL,
                config_json JSONB NULL,
                enabled BOOLEAN NOT NULL DEFAULT TRUE,
                poll_interval_seconds INTEGER NOT NULL DEFAULT 30,
                last_checked_at TIMESTAMPTZ NULL,
                last_success_at TIMESTAMPTZ NULL,
                last_error TEXT NULL,
                created_at TIMESTAMPTZ NOT NULL,
                updated_at TIMESTAMPTZ NOT NULL
            );
            "#,
            "ALTER TABLE scrapper.data_sources DROP CONSTRAINT IF EXISTS data_sources_source_type_check;",
            r#"
            ALTER TABLE scrapper.data_sources
            ADD CONSTRAINT data_sources_source_type_check
            CHECK (source_type IN ('rss', 'web_page', 'manual_note', 'placeholder_api', 'python_script'));
            "#,
            r#"
            CREATE TABLE IF NOT EXISTS scrapper.data_source_scripts (
                data_source_id UUID PRIMARY KEY REFERENCES scrapper.data_sources(id) ON DELETE CASCADE,
                language TEXT NOT NULL DEFAULT 'python',
                script_text TEXT NOT NULL,
                script_hash TEXT NULL,
                last_build_status TEXT NULL CHECK (last_build_status IS NULL OR last_build_status IN ('not_built', 'success', 'failed')),
                last_build_output TEXT NULL,
                last_built_at TIMESTAMPTZ NULL,
                created_at TIMESTAMPTZ NOT NULL,
                updated_at TIMESTAMPTZ NOT NULL
            );
            "#,
            r#"
            CREATE TABLE IF NOT EXISTS scrapper.data_source_items (
                id UUID PRIMARY KEY,
                data_source_id UUID NOT NULL REFERENCES scrapper.data_sources(id) ON DELETE CASCADE,
                external_id TEXT NULL,
                title TEXT NOT NULL,
                url TEXT NULL,
                content TEXT NULL,
                summary TEXT NULL,
                raw_payload JSONB NULL,
                published_at TIMESTAMPTZ NULL,
                discovered_at TIMESTAMPTZ NOT NULL,
                created_at TIMESTAMPTZ NOT NULL,
                UNIQUE(data_source_id, external_id)
            );
            "#,
            r#"
            CREATE TABLE IF NOT EXISTS scrapper.data_source_events (
                id UUID PRIMARY KEY,
                data_source_id UUID NULL REFERENCES scrapper.data_sources(id) ON DELETE SET NULL,
                event_type TEXT NOT NULL,
                message TEXT NOT NULL,
                payload JSONB NULL,
                created_at TIMESTAMPTZ NOT NULL
            );
            "#,
            r#"
            CREATE TABLE IF NOT EXISTS scrapper.trader_data_sources (
                trader_id UUID NOT NULL,
                data_source_id UUID NOT NULL REFERENCES scrapper.data_sources(id) ON DELETE CASCADE,
                enabled BOOLEAN NOT NULL DEFAULT TRUE,
                created_at TIMESTAMPTZ NOT NULL,
                updated_at TIMESTAMPTZ NOT NULL,
                PRIMARY KEY (trader_id, data_source_id)
            );
            "#,
            "CREATE INDEX IF NOT EXISTS idx_data_sources_enabled ON scrapper.data_sources(enabled);",
            "CREATE INDEX IF NOT EXISTS idx_data_sources_source_type ON scrapper.data_sources(source_type);",
            "CREATE INDEX IF NOT EXISTS idx_data_source_items_source_published ON scrapper.data_source_items(data_source_id, published_at DESC);",
            "CREATE INDEX IF NOT EXISTS idx_data_source_items_source_discovered ON scrapper.data_source_items(data_source_id, discovered_at DESC);",
            "CREATE INDEX IF NOT EXISTS idx_data_source_events_source_created ON scrapper.data_source_events(data_source_id, created_at DESC);",
        ] {
            sqlx::query(statement).execute(&self.pool).await?;
        }

        Ok(())
    }

    async fn init_trader_tables(&self) -> Result<(), sqlx::Error> {
        for statement in [
            r#"
            CREATE TABLE IF NOT EXISTS traders (
                id UUID PRIMARY KEY,
                name TEXT NOT NULL,
                fundamental_perspective TEXT NOT NULL,
                freedom_level TEXT NOT NULL CHECK (freedom_level IN ('analyst', 'junior_trader', 'senior_trader')),
                status TEXT NOT NULL DEFAULT 'stopped' CHECK (status IN ('stopped', 'running', 'paused')),
                default_paper_account_id UUID NULL REFERENCES paper_accounts(id) ON DELETE SET NULL,
                is_active BOOLEAN NOT NULL DEFAULT TRUE,
                created_at TIMESTAMPTZ NOT NULL,
                updated_at TIMESTAMPTZ NOT NULL,
                started_at TIMESTAMPTZ NULL,
                stopped_at TIMESTAMPTZ NULL
            );
            "#,
            r#"
            ALTER TABLE traders ADD COLUMN IF NOT EXISTS persona TEXT NULL;
            "#,
            r#"
            ALTER TABLE traders ADD COLUMN IF NOT EXISTS tone TEXT NULL;
            "#,
            r#"
            ALTER TABLE traders ADD COLUMN IF NOT EXISTS communication_style TEXT NULL;
            "#,
            r#"
            CREATE TABLE IF NOT EXISTS channels (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                display_name TEXT NOT NULL,
                description TEXT NULL,
                is_system BOOLEAN NOT NULL DEFAULT FALSE,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            "#,
            r#"
            CREATE TABLE IF NOT EXISTS channel_messages (
                id TEXT PRIMARY KEY,
                channel_id TEXT NOT NULL,
                author_type TEXT NOT NULL CHECK (author_type IN ('user', 'trader', 'md', 'system')),
                author_id TEXT NULL,
                author_name TEXT NOT NULL,
                role TEXT NOT NULL DEFAULT 'message' CHECK (role IN ('message', 'question', 'answer', 'alert', 'proposal', 'review', 'system')),
                content_markdown TEXT NOT NULL,
                metadata_json TEXT NULL,
                created_at TEXT NOT NULL
            );
            "#,
            r#"
            CREATE TABLE IF NOT EXISTS md_profile (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL DEFAULT 'MD',
                persona TEXT NOT NULL,
                tone TEXT NOT NULL,
                communication_style TEXT NOT NULL,
                openai_api_key_secret TEXT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            "#,
            r#"
            ALTER TABLE md_profile ADD COLUMN IF NOT EXISTS openai_api_key_secret TEXT NULL;
            "#,
            r#"
            CREATE TABLE IF NOT EXISTS data_scientist_profile (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL DEFAULT 'Data Scientist',
                persona TEXT NOT NULL,
                tone TEXT NOT NULL,
                communication_style TEXT NOT NULL,
                openai_api_key_secret TEXT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            "#,
            r#"
            ALTER TABLE data_scientist_profile ADD COLUMN IF NOT EXISTS openai_api_key_secret TEXT NULL;
            "#,
            r#"
            CREATE TABLE IF NOT EXISTS user_investor_profile (
                id TEXT PRIMARY KEY,
                name TEXT NULL,
                age INTEGER NULL,
                about TEXT NULL,
                investment_goals TEXT NULL,
                risk_tolerance TEXT NULL,
                time_horizon TEXT NULL,
                liquidity_needs TEXT NULL,
                income_needs TEXT NULL,
                investment_experience TEXT NULL,
                restrictions TEXT NULL,
                preferred_sectors TEXT NULL,
                avoided_sectors TEXT NULL,
                notes TEXT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            "#,
            r#"
            CREATE TABLE IF NOT EXISTS trader_memories (
                id TEXT PRIMARY KEY,
                trader_id TEXT NOT NULL,
                memory_type TEXT NOT NULL CHECK (memory_type IN ('answer', 'review', 'decision', 'user_preference', 'risk_note', 'data_note', 'proposal_note', 'channel_resolution')),
                topic TEXT NOT NULL,
                summary TEXT NOT NULL,
                source_channel_id TEXT NULL,
                source_message_id TEXT NULL,
                confidence DOUBLE PRECISION NULL,
                importance BIGINT NOT NULL DEFAULT 3,
                status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'archived', 'superseded')),
                last_used_at TEXT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            "#,
            r#"
            CREATE TABLE IF NOT EXISTS trader_memory_links (
                id TEXT PRIMARY KEY,
                trader_id TEXT NOT NULL,
                memory_id TEXT NOT NULL,
                channel_message_id TEXT NOT NULL,
                link_type TEXT NOT NULL CHECK (link_type IN ('created_from', 'referenced_by', 'prevented_repeat')),
                created_at TEXT NOT NULL
            );
            "#,
            r#"
            CREATE TABLE IF NOT EXISTS trader_info_sources (
                id UUID PRIMARY KEY,
                trader_id UUID NOT NULL REFERENCES traders(id) ON DELETE CASCADE,
                source_type TEXT NOT NULL,
                name TEXT NOT NULL,
                config_json JSONB NULL,
                enabled BOOLEAN NOT NULL DEFAULT TRUE,
                created_at TIMESTAMPTZ NOT NULL,
                updated_at TIMESTAMPTZ NOT NULL
            );
            "#,
            r#"
            CREATE TABLE IF NOT EXISTS trader_secrets (
                trader_id UUID PRIMARY KEY REFERENCES traders(id) ON DELETE CASCADE,
                openai_api_key_secret TEXT NOT NULL,
                created_at TIMESTAMPTZ NOT NULL,
                updated_at TIMESTAMPTZ NOT NULL
            );
            "#,
            r#"
            CREATE TABLE IF NOT EXISTS trader_runtime_state (
                trader_id UUID PRIMARY KEY REFERENCES traders(id) ON DELETE CASCADE,
                engine_name TEXT NULL,
                last_heartbeat_at TIMESTAMPTZ NULL,
                last_evaluation_at TIMESTAMPTZ NULL,
                last_error TEXT NULL,
                current_task TEXT NULL,
                created_at TIMESTAMPTZ NOT NULL,
                updated_at TIMESTAMPTZ NOT NULL
            );
            "#,
            r#"
            CREATE TABLE IF NOT EXISTS trader_events (
                id UUID PRIMARY KEY,
                trader_id UUID NOT NULL REFERENCES traders(id) ON DELETE CASCADE,
                event_type TEXT NOT NULL,
                message TEXT NOT NULL,
                payload JSONB NULL,
                created_at TIMESTAMPTZ NOT NULL
            );
            "#,
            r#"
            CREATE TABLE IF NOT EXISTS trader_trade_proposals (
                id UUID PRIMARY KEY,
                trader_id UUID NOT NULL REFERENCES traders(id) ON DELETE CASCADE,
                symbol TEXT NOT NULL,
                side TEXT NOT NULL CHECK (side IN ('buy', 'sell')),
                quantity DOUBLE PRECISION NOT NULL,
                order_type TEXT NOT NULL DEFAULT 'market',
                reason TEXT NOT NULL,
                confidence DOUBLE PRECISION NULL,
                status TEXT NOT NULL DEFAULT 'pending_review',
                reviewed_by TEXT NULL,
                reviewed_at TIMESTAMPTZ NULL,
                resulting_order_id UUID NULL REFERENCES paper_orders(id) ON DELETE SET NULL,
                created_at TIMESTAMPTZ NOT NULL,
                updated_at TIMESTAMPTZ NOT NULL
            );
            "#,
            r#"
            CREATE TABLE IF NOT EXISTS trader_symbols (
                id UUID PRIMARY KEY,
                trader_id UUID NOT NULL REFERENCES traders(id) ON DELETE CASCADE,
                symbol TEXT NOT NULL,
                asset_type TEXT NOT NULL DEFAULT 'stock' CHECK (asset_type IN ('stock', 'etf', 'index', 'crypto', 'other')),
                name TEXT NULL,
                exchange TEXT NULL,
                sector TEXT NULL,
                industry TEXT NULL,
                notes TEXT NULL,
                thesis TEXT NULL,
                fit_score DOUBLE PRECISION NULL,
                status TEXT NOT NULL DEFAULT 'watching' CHECK (status IN ('watching', 'candidate', 'active', 'rejected', 'archived')),
                source TEXT NOT NULL DEFAULT 'manual' CHECK (source IN ('manual', 'ai', 'import', 'engine')),
                created_at TIMESTAMPTZ NOT NULL,
                updated_at TIMESTAMPTZ NOT NULL,
                UNIQUE(trader_id, symbol)
            );
            "#,
            r#"
            CREATE TABLE IF NOT EXISTS trader_portfolio_proposals (
                id UUID PRIMARY KEY,
                trader_id UUID NOT NULL REFERENCES traders(id) ON DELETE CASCADE,
                paper_account_id UUID NULL REFERENCES paper_accounts(id) ON DELETE SET NULL,
                title TEXT NOT NULL,
                summary TEXT NOT NULL,
                thesis TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'proposed' CHECK (status IN ('proposed', 'accepted', 'rejected', 'superseded', 'executed', 'expired')),
                plan_state TEXT NOT NULL DEFAULT 'draft' CHECK (plan_state IN ('draft', 'active', 'superseded', 'completed', 'invalidated', 'rejected')),
                confidence DOUBLE PRECISION NULL,
                proposed_actions_json JSONB NOT NULL,
                source_snapshot_json JSONB NULL,
                risk_snapshot_json JSONB NULL,
                market_snapshot_json JSONB NULL,
                market_basis_json JSONB NULL,
                invalidation_conditions_json JSONB NULL,
                change_thresholds_json JSONB NULL,
                replacement_reason TEXT NULL,
                created_at TIMESTAMPTZ NOT NULL,
                updated_at TIMESTAMPTZ NOT NULL,
                reviewed_at TIMESTAMPTZ NULL,
                review_note TEXT NULL,
                accepted_at TIMESTAMPTZ NULL,
                active_until TIMESTAMPTZ NULL,
                expected_duration_seconds BIGINT NULL
            );
            "#,
            r#"
            CREATE TABLE IF NOT EXISTS trader_portfolio_proposal_actions (
                id UUID PRIMARY KEY,
                proposal_id UUID NOT NULL REFERENCES trader_portfolio_proposals(id) ON DELETE CASCADE,
                trader_id UUID NOT NULL REFERENCES traders(id) ON DELETE CASCADE,
                symbol TEXT NULL,
                action_type TEXT NOT NULL CHECK (action_type IN ('buy', 'sell', 'hold', 'watch', 'activate_symbol', 'reject_symbol', 'reduce', 'increase', 'no_action')),
                side TEXT NULL,
                quantity DOUBLE PRECISION NULL,
                order_type TEXT NULL,
                entry_price DOUBLE PRECISION NULL,
                exit_price DOUBLE PRECISION NULL,
                limit_price DOUBLE PRECISION NULL,
                stop_price DOUBLE PRECISION NULL,
                expected_duration_seconds BIGINT NULL,
                enact_by TIMESTAMPTZ NULL,
                market_price_at_creation DOUBLE PRECISION NULL,
                rationale TEXT NOT NULL,
                confidence DOUBLE PRECISION NULL,
                risk_decision TEXT NULL,
                status TEXT NOT NULL DEFAULT 'proposed',
                created_at TIMESTAMPTZ NOT NULL,
                updated_at TIMESTAMPTZ NOT NULL
            );
            "#,
            "CREATE INDEX IF NOT EXISTS idx_traders_status ON traders(status);",
            "CREATE INDEX IF NOT EXISTS idx_traders_freedom_level ON traders(freedom_level);",
            "CREATE INDEX IF NOT EXISTS idx_trader_events_trader_id_created_at ON trader_events(trader_id, created_at DESC);",
            "CREATE INDEX IF NOT EXISTS idx_trader_trade_proposals_trader_id_status ON trader_trade_proposals(trader_id, status);",
            "CREATE INDEX IF NOT EXISTS idx_trader_runtime_state_last_heartbeat_at ON trader_runtime_state(last_heartbeat_at DESC);",
            "CREATE INDEX IF NOT EXISTS idx_trader_symbols_trader_id ON trader_symbols(trader_id);",
            "CREATE INDEX IF NOT EXISTS idx_trader_symbols_symbol ON trader_symbols(symbol);",
            "CREATE INDEX IF NOT EXISTS idx_trader_symbols_status ON trader_symbols(status);",
            "CREATE INDEX IF NOT EXISTS idx_trader_symbols_asset_type ON trader_symbols(asset_type);",
            "CREATE INDEX IF NOT EXISTS idx_trader_symbols_fit_score ON trader_symbols(fit_score DESC);",
            "CREATE INDEX IF NOT EXISTS idx_trader_portfolio_proposals_trader_created ON trader_portfolio_proposals(trader_id, created_at DESC);",
            "CREATE INDEX IF NOT EXISTS idx_trader_portfolio_proposals_trader_status ON trader_portfolio_proposals(trader_id, status);",
            "CREATE INDEX IF NOT EXISTS idx_trader_portfolio_proposal_actions_proposal ON trader_portfolio_proposal_actions(proposal_id);",
            "CREATE INDEX IF NOT EXISTS idx_trader_portfolio_proposal_actions_trader_symbol ON trader_portfolio_proposal_actions(trader_id, symbol);",
            "CREATE INDEX IF NOT EXISTS idx_trader_portfolio_proposal_actions_status ON trader_portfolio_proposal_actions(status);",
            "CREATE INDEX IF NOT EXISTS idx_channels_name ON channels(name);",
            "CREATE INDEX IF NOT EXISTS idx_channel_messages_channel_created ON channel_messages(channel_id, created_at);",
            "CREATE INDEX IF NOT EXISTS idx_channel_messages_author ON channel_messages(author_type, author_id);",
            "CREATE INDEX IF NOT EXISTS idx_channel_messages_role ON channel_messages(role);",
            "CREATE INDEX IF NOT EXISTS idx_trader_memories_trader_id ON trader_memories(trader_id);",
            "CREATE INDEX IF NOT EXISTS idx_trader_memories_topic ON trader_memories(topic);",
            "CREATE INDEX IF NOT EXISTS idx_trader_memories_type ON trader_memories(memory_type);",
            "CREATE INDEX IF NOT EXISTS idx_trader_memories_status ON trader_memories(status);",
            "CREATE INDEX IF NOT EXISTS idx_trader_memories_last_used ON trader_memories(last_used_at);",
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_trader_memory_links_unique ON trader_memory_links(trader_id, channel_message_id, link_type);",
            "CREATE INDEX IF NOT EXISTS idx_trader_memory_links_trader_id ON trader_memory_links(trader_id);",
            "CREATE INDEX IF NOT EXISTS idx_trader_memory_links_memory_id ON trader_memory_links(memory_id);",
            "ALTER TABLE trader_portfolio_proposals ADD COLUMN IF NOT EXISTS plan_state TEXT NOT NULL DEFAULT 'draft';",
            "ALTER TABLE trader_portfolio_proposals ADD COLUMN IF NOT EXISTS accepted_at TIMESTAMPTZ NULL;",
            "ALTER TABLE trader_portfolio_proposals ADD COLUMN IF NOT EXISTS active_until TIMESTAMPTZ NULL;",
            "ALTER TABLE trader_portfolio_proposals ADD COLUMN IF NOT EXISTS expected_duration_seconds BIGINT NULL;",
            "ALTER TABLE trader_portfolio_proposals ADD COLUMN IF NOT EXISTS market_basis_json JSONB NULL;",
            "ALTER TABLE trader_portfolio_proposals ADD COLUMN IF NOT EXISTS invalidation_conditions_json JSONB NULL;",
            "ALTER TABLE trader_portfolio_proposals ADD COLUMN IF NOT EXISTS change_thresholds_json JSONB NULL;",
            "ALTER TABLE trader_portfolio_proposals ADD COLUMN IF NOT EXISTS replacement_reason TEXT NULL;",
            "ALTER TABLE trader_portfolio_proposal_actions ADD COLUMN IF NOT EXISTS entry_price DOUBLE PRECISION NULL;",
            "ALTER TABLE trader_portfolio_proposal_actions ADD COLUMN IF NOT EXISTS exit_price DOUBLE PRECISION NULL;",
            "ALTER TABLE trader_portfolio_proposal_actions ADD COLUMN IF NOT EXISTS limit_price DOUBLE PRECISION NULL;",
            "ALTER TABLE trader_portfolio_proposal_actions ADD COLUMN IF NOT EXISTS stop_price DOUBLE PRECISION NULL;",
            "ALTER TABLE trader_portfolio_proposal_actions ADD COLUMN IF NOT EXISTS expected_duration_seconds BIGINT NULL;",
            "ALTER TABLE trader_portfolio_proposal_actions ADD COLUMN IF NOT EXISTS enact_by TIMESTAMPTZ NULL;",
            "ALTER TABLE trader_portfolio_proposal_actions ADD COLUMN IF NOT EXISTS market_price_at_creation DOUBLE PRECISION NULL;",
        ] {
            sqlx::query(statement).execute(&self.pool).await?;
        }

        self.seed_channels().await?;
        self.seed_md_profile().await?;
        self.seed_data_scientist_profile().await?;
        self.seed_user_investor_profile().await?;

        Ok(())
    }

    async fn seed_channels(&self) -> Result<(), sqlx::Error> {
        let now = Self::now_string();
        for (name, display_name, description) in [
            ("general", "#general", "Broad coordination and discussion."),
            (
                "data_analysis",
                "#data_analysis",
                "Data quality, source gaps, and analysis questions.",
            ),
            (
                "trading",
                "#trading",
                "Trade proposals, portfolio plans, and risk coordination.",
            ),
        ] {
            sqlx::query(
                r#"
                INSERT INTO channels (id, name, display_name, description, is_system, created_at, updated_at)
                VALUES ($1, $2, $3, $4, TRUE, $5, $6)
                ON CONFLICT (name) DO UPDATE
                SET display_name = EXCLUDED.display_name,
                    description = EXCLUDED.description,
                    is_system = TRUE,
                    updated_at = EXCLUDED.updated_at
                "#,
            )
            .bind(name)
            .bind(name)
            .bind(display_name)
            .bind(description)
            .bind(&now)
            .bind(&now)
            .execute(&self.pool)
            .await?;
        }
        Ok(())
    }

    async fn seed_md_profile(&self) -> Result<(), sqlx::Error> {
        let now = Self::now_string();
        sqlx::query(
            r#"
            INSERT INTO md_profile (id, name, persona, tone, communication_style, created_at, updated_at)
            VALUES ($1, 'MD', $2, $3, $4, $5, $6)
            ON CONFLICT (id) DO NOTHING
            "#,
        )
        .bind("default")
        .bind("The MD is a managing director responsible for monitoring trader reasoning, reducing drift, asking clarifying questions, identifying weak assumptions, and encouraging collaboration. The MD does not place trades.")
        .bind("Direct, calm, skeptical, risk-aware, and constructive.")
        .bind("Monitors channel discussion, asks clarifying questions, summarizes disagreements, and reminds traders of risk and user context without executing trades.")
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn seed_data_scientist_profile(&self) -> Result<(), sqlx::Error> {
        let now = Self::now_string();
        sqlx::query(
            r#"
            INSERT INTO data_scientist_profile (id, name, persona, tone, communication_style, created_at, updated_at)
            VALUES ($1, 'Data Scientist', $2, $3, $4, $5, $6)
            ON CONFLICT (id) DO NOTHING
            "#,
        )
        .bind("default")
        .bind("The Data Scientist designs, debugs, and improves data sources for the trading application. It can inspect user-provided URLs, reason about extraction strategies, and create Python Script data sources. It does not trade.")
        .bind("Technical, precise, practical, and concise.")
        .bind("Explains extraction assumptions, script limitations, build results, and next steps without placing trades or requesting secrets.")
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn seed_user_investor_profile(&self) -> Result<(), sqlx::Error> {
        let now = Self::now_string();
        sqlx::query(
            r#"
            INSERT INTO user_investor_profile (id, created_at, updated_at)
            VALUES ($1, $2, $3)
            ON CONFLICT (id) DO NOTHING
            "#,
        )
        .bind("default")
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await?;
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
            trader_id: row.get("trader_id"),
            strategy_id: row.get("strategy_id"),
            signal_id: row.get("signal_id"),
            proposal_id: row.get("proposal_id"),
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

    fn row_to_trader(row: sqlx::postgres::PgRow) -> Trader {
        Trader {
            id: row.get("id"),
            name: row.get("name"),
            fundamental_perspective: row.get("fundamental_perspective"),
            freedom_level: row.get("freedom_level"),
            status: row.get("status"),
            default_paper_account_id: row.get("default_paper_account_id"),
            persona: row.try_get("persona").ok(),
            tone: row.try_get("tone").ok(),
            communication_style: row.try_get("communication_style").ok(),
            is_active: row.get("is_active"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            started_at: row.get("started_at"),
            stopped_at: row.get("stopped_at"),
        }
    }

    fn row_to_channel(row: sqlx::postgres::PgRow) -> Channel {
        Channel {
            id: row.get("id"),
            name: row.get("name"),
            display_name: row.get("display_name"),
            description: row.get("description"),
            is_system: row.get("is_system"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        }
    }

    fn row_to_channel_message(row: sqlx::postgres::PgRow) -> ChannelMessage {
        ChannelMessage {
            id: row.get("id"),
            channel_id: row.get("channel_id"),
            author_type: row.get("author_type"),
            author_id: row.get("author_id"),
            author_name: row.get("author_name"),
            role: row.get("role"),
            content_markdown: row.get("content_markdown"),
            metadata_json: row.get("metadata_json"),
            created_at: row.get("created_at"),
        }
    }

    fn row_to_trader_memory(row: sqlx::postgres::PgRow) -> TraderMemory {
        TraderMemory {
            id: row.get("id"),
            trader_id: row.get("trader_id"),
            memory_type: row.get("memory_type"),
            topic: row.get("topic"),
            summary: row.get("summary"),
            source_channel_id: row.get("source_channel_id"),
            source_message_id: row.get("source_message_id"),
            confidence: row.get("confidence"),
            importance: row.get("importance"),
            status: row.get("status"),
            last_used_at: row.get("last_used_at"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        }
    }

    fn row_to_md_profile(row: sqlx::postgres::PgRow) -> MdProfile {
        MdProfile {
            id: row.get("id"),
            name: row.get("name"),
            persona: row.get("persona"),
            tone: row.get("tone"),
            communication_style: row.get("communication_style"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        }
    }

    fn row_to_data_scientist_profile(row: sqlx::postgres::PgRow) -> DataScientistProfile {
        DataScientistProfile {
            id: row.get("id"),
            name: row.get("name"),
            persona: row.get("persona"),
            tone: row.get("tone"),
            communication_style: row.get("communication_style"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        }
    }

    fn row_to_user_investor_profile(row: sqlx::postgres::PgRow) -> UserInvestorProfile {
        UserInvestorProfile {
            id: row.get("id"),
            name: row.get("name"),
            age: row.get::<Option<i32>, _>("age").map(i64::from),
            about: row.get("about"),
            investment_goals: row.get("investment_goals"),
            risk_tolerance: row.get("risk_tolerance"),
            time_horizon: row.get("time_horizon"),
            liquidity_needs: row.get("liquidity_needs"),
            income_needs: row.get("income_needs"),
            investment_experience: row.get("investment_experience"),
            restrictions: row.get("restrictions"),
            preferred_sectors: row.get("preferred_sectors"),
            avoided_sectors: row.get("avoided_sectors"),
            notes: row.get("notes"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        }
    }

    fn row_to_trader_info_source(row: sqlx::postgres::PgRow) -> TraderInfoSource {
        let config_json = row
            .try_get::<Option<String>, _>("config_json")
            .ok()
            .flatten()
            .or_else(|| {
                row.try_get::<Option<Value>, _>("config_json")
                    .ok()
                    .flatten()
                    .map(|value| value.to_string())
            });

        TraderInfoSource {
            id: row.get("id"),
            trader_id: row.get("trader_id"),
            source_type: row.get("source_type"),
            name: row.get("name"),
            config_json,
            enabled: row.get("enabled"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        }
    }

    fn row_to_trader_runtime_state(row: sqlx::postgres::PgRow) -> TraderRuntimeState {
        TraderRuntimeState {
            trader_id: row.get("trader_id"),
            engine_name: row.get("engine_name"),
            last_heartbeat_at: row.get("last_heartbeat_at"),
            last_evaluation_at: row.get("last_evaluation_at"),
            last_error: row.get("last_error"),
            current_task: row.get("current_task"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        }
    }

    fn row_to_engine_event(row: sqlx::postgres::PgRow) -> EngineEvent {
        EngineEvent {
            id: row.get("id"),
            engine_name: row.get("engine_name"),
            event_type: row.get("event_type"),
            symbol: row.get("symbol"),
            message: row.get("message"),
            timestamp: row.get("timestamp"),
            created_at: row.get("created_at"),
        }
    }

    fn row_to_trader_event(row: sqlx::postgres::PgRow) -> TraderEvent {
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

        TraderEvent {
            id: row.get("id"),
            trader_id: row.get("trader_id"),
            event_type: row.get("event_type"),
            message: row.get("message"),
            payload,
            created_at: row.get("created_at"),
        }
    }

    fn row_to_trader_trade_proposal(row: sqlx::postgres::PgRow) -> TraderTradeProposal {
        TraderTradeProposal {
            id: row.get("id"),
            trader_id: row.get("trader_id"),
            symbol: row.get("symbol"),
            side: row.get("side"),
            quantity: row.get("quantity"),
            order_type: row.get("order_type"),
            reason: row.get("reason"),
            confidence: row.get("confidence"),
            status: row.get("status"),
            reviewed_by: row.get("reviewed_by"),
            reviewed_at: row.get("reviewed_at"),
            resulting_order_id: row.get("resulting_order_id"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        }
    }

    fn row_to_trader_symbol(row: sqlx::postgres::PgRow) -> TraderSymbol {
        TraderSymbol {
            id: row.get("id"),
            trader_id: row.get("trader_id"),
            symbol: row.get("symbol"),
            asset_type: row.get("asset_type"),
            name: row.get("name"),
            exchange: row.get("exchange"),
            sector: row.get("sector"),
            industry: row.get("industry"),
            notes: row.get("notes"),
            thesis: row.get("thesis"),
            fit_score: row.get("fit_score"),
            status: row.get("status"),
            source: row.get("source"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        }
    }

    fn json_column_to_string(row: &sqlx::postgres::PgRow, column: &str) -> Option<String> {
        row.try_get::<Option<String>, _>(column)
            .ok()
            .flatten()
            .or_else(|| {
                row.try_get::<Option<Value>, _>(column)
                    .ok()
                    .flatten()
                    .map(|value| value.to_string())
            })
    }

    fn row_to_trader_portfolio_proposal(row: sqlx::postgres::PgRow) -> TraderPortfolioProposal {
        let proposed_actions_json = Self::json_column_to_string(&row, "proposed_actions_json")
            .unwrap_or_else(|| "[]".to_string());
        TraderPortfolioProposal {
            id: row.get("id"),
            trader_id: row.get("trader_id"),
            paper_account_id: row.get("paper_account_id"),
            title: row.get("title"),
            summary: row.get("summary"),
            thesis: row.get("thesis"),
            status: row.get("status"),
            plan_state: row.get("plan_state"),
            confidence: row.get("confidence"),
            proposed_actions_json,
            source_snapshot_json: Self::json_column_to_string(&row, "source_snapshot_json"),
            risk_snapshot_json: Self::json_column_to_string(&row, "risk_snapshot_json"),
            market_snapshot_json: Self::json_column_to_string(&row, "market_snapshot_json"),
            market_basis_json: Self::json_column_to_string(&row, "market_basis_json"),
            invalidation_conditions_json: Self::json_column_to_string(
                &row,
                "invalidation_conditions_json",
            ),
            change_thresholds_json: Self::json_column_to_string(&row, "change_thresholds_json"),
            replacement_reason: row.get("replacement_reason"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            reviewed_at: row.get("reviewed_at"),
            review_note: row.get("review_note"),
            accepted_at: row.get("accepted_at"),
            active_until: row.get("active_until"),
            expected_duration_seconds: row.get("expected_duration_seconds"),
        }
    }

    fn row_to_trader_portfolio_proposal_action(
        row: sqlx::postgres::PgRow,
    ) -> TraderPortfolioProposalAction {
        TraderPortfolioProposalAction {
            id: row.get("id"),
            proposal_id: row.get("proposal_id"),
            trader_id: row.get("trader_id"),
            symbol: row.get("symbol"),
            action_type: row.get("action_type"),
            side: row.get("side"),
            quantity: row.get("quantity"),
            order_type: row.get("order_type"),
            entry_price: row.get("entry_price"),
            exit_price: row.get("exit_price"),
            limit_price: row.get("limit_price"),
            stop_price: row.get("stop_price"),
            expected_duration_seconds: row.get("expected_duration_seconds"),
            enact_by: row.get("enact_by"),
            market_price_at_creation: row.get("market_price_at_creation"),
            rationale: row.get("rationale"),
            confidence: row.get("confidence"),
            risk_decision: row.get("risk_decision"),
            status: row.get("status"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        }
    }

    fn row_to_data_source(row: sqlx::postgres::PgRow) -> DataSource {
        let config_json = row
            .try_get::<Option<String>, _>("config_json")
            .ok()
            .flatten()
            .or_else(|| {
                row.try_get::<Option<Value>, _>("config_json")
                    .ok()
                    .flatten()
                    .map(|value| value.to_string())
            });
        DataSource {
            id: row.get("id"),
            name: row.get("name"),
            source_type: row.get("source_type"),
            url: row.get("url"),
            config_json,
            enabled: row.get("enabled"),
            poll_interval_seconds: row.get::<i32, _>("poll_interval_seconds") as i64,
            last_checked_at: row.get("last_checked_at"),
            last_success_at: row.get("last_success_at"),
            last_error: row.get("last_error"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        }
    }

    fn row_to_data_source_item(row: sqlx::postgres::PgRow) -> DataSourceItem {
        let raw_payload = row
            .try_get::<Option<String>, _>("raw_payload")
            .ok()
            .flatten()
            .or_else(|| {
                row.try_get::<Option<Value>, _>("raw_payload")
                    .ok()
                    .flatten()
                    .map(|value| value.to_string())
            });
        DataSourceItem {
            id: row.get("id"),
            data_source_id: row.get("data_source_id"),
            external_id: row.get("external_id"),
            title: row.get("title"),
            url: row.get("url"),
            content: row.get("content"),
            summary: row.get("summary"),
            raw_payload,
            published_at: row.get("published_at"),
            discovered_at: row.get("discovered_at"),
            created_at: row.get("created_at"),
        }
    }

    fn row_to_data_source_event(row: sqlx::postgres::PgRow) -> DataSourceEvent {
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
        DataSourceEvent {
            id: row.get("id"),
            data_source_id: row.get("data_source_id"),
            event_type: row.get("event_type"),
            message: row.get("message"),
            payload,
            created_at: row.get("created_at"),
        }
    }

    fn row_to_data_source_script(row: sqlx::postgres::PgRow) -> DataSourceScript {
        DataSourceScript {
            data_source_id: row.get("data_source_id"),
            language: row.get("language"),
            script_text: row.get("script_text"),
            script_hash: row.get("script_hash"),
            last_build_status: row.get("last_build_status"),
            last_build_output: row.get("last_build_output"),
            last_built_at: row.get("last_built_at"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
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
            trader_id: params.trader_id.clone(),
            strategy_id: params.strategy_id.clone(),
            signal_id: params.signal_id.clone(),
            proposal_id: params.proposal_id.clone(),
            created_at: now.clone(),
            updated_at: now.clone(),
        };

        sqlx::query(
            r#"
            INSERT INTO paper_orders (
                id, account_id, symbol, side, order_type, quantity, requested_price,
                filled_quantity, average_fill_price, status, source, trader_id, strategy_id,
                signal_id, proposal_id, created_at, updated_at
            ) VALUES (
                $1::uuid, $2::uuid, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12::uuid, $13,
                $14::uuid, $15::uuid, $16::timestamptz, $17::timestamptz
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
        .bind(&order.trader_id)
        .bind(&order.strategy_id)
        .bind(&order.signal_id)
        .bind(&order.proposal_id)
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
                trader_id::text AS trader_id,
                strategy_id,
                signal_id::text AS signal_id,
                proposal_id::text AS proposal_id,
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
                trader_id::text AS trader_id,
                strategy_id,
                signal_id::text AS signal_id,
                proposal_id::text AS proposal_id,
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

    pub async fn create_trader(
        &self,
        trader: &Trader,
        info_sources: &[CreateTraderInfoSourceRequest],
        openai_api_key_secret: &str,
    ) -> Result<Trader, sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        sqlx::query(
            r#"
            INSERT INTO traders (
                id, name, fundamental_perspective, freedom_level, status,
                default_paper_account_id, persona, tone, communication_style,
                is_active, created_at, updated_at, started_at, stopped_at
            ) VALUES (
                $1::uuid, $2, $3, $4, $5, $6::uuid, $7, $8, $9, $10,
                $11::timestamptz, $12::timestamptz, $13::timestamptz, $14::timestamptz
            )
            "#,
        )
        .bind(&trader.id)
        .bind(&trader.name)
        .bind(&trader.fundamental_perspective)
        .bind(&trader.freedom_level)
        .bind(&trader.status)
        .bind(&trader.default_paper_account_id)
        .bind(&trader.persona)
        .bind(&trader.tone)
        .bind(&trader.communication_style)
        .bind(trader.is_active)
        .bind(&trader.created_at)
        .bind(&trader.updated_at)
        .bind(&trader.started_at)
        .bind(&trader.stopped_at)
        .execute(&mut *tx)
        .await?;

        Self::upsert_trader_secret_in_tx(&mut tx, &trader.id, openai_api_key_secret).await?;
        Self::replace_trader_info_sources_in_tx(&mut tx, &trader.id, info_sources).await?;
        Self::insert_trader_event_in_tx(
            &mut tx,
            &trader.id,
            "trader_created",
            "Trader created",
            None,
        )
        .await?;

        tx.commit().await?;
        self.get_trader(&trader.id)
            .await?
            .ok_or(sqlx::Error::RowNotFound)
    }

    pub async fn list_traders(&self) -> Result<Vec<Trader>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT id::text AS id, name, fundamental_perspective, freedom_level, status,
                   default_paper_account_id::text AS default_paper_account_id, is_active,
                   persona, tone, communication_style,
                   created_at::text AS created_at, updated_at::text AS updated_at,
                   started_at::text AS started_at, stopped_at::text AS stopped_at
            FROM traders
            WHERE is_active = TRUE
            ORDER BY updated_at DESC, name ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(Self::row_to_trader).collect())
    }

    pub async fn get_trader(&self, trader_id: &str) -> Result<Option<Trader>, sqlx::Error> {
        let row = sqlx::query(
            r#"
            SELECT id::text AS id, name, fundamental_perspective, freedom_level, status,
                   default_paper_account_id::text AS default_paper_account_id, is_active,
                   persona, tone, communication_style,
                   created_at::text AS created_at, updated_at::text AS updated_at,
                   started_at::text AS started_at, stopped_at::text AS stopped_at
            FROM traders
            WHERE id = $1::uuid AND is_active = TRUE
            "#,
        )
        .bind(trader_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Self::row_to_trader))
    }

    pub async fn get_trader_openai_api_key(
        &self,
        trader_id: &str,
    ) -> Result<Option<String>, sqlx::Error> {
        let row = sqlx::query(
            r#"
            SELECT openai_api_key_secret
            FROM trader_secrets
            WHERE trader_id = $1::uuid
            "#,
        )
        .bind(trader_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|row| row.get("openai_api_key_secret")))
    }

    pub async fn list_recent_paper_orders_for_trader(
        &self,
        trader_id: &str,
        limit: i64,
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
                trader_id::text AS trader_id,
                strategy_id,
                signal_id::text AS signal_id,
                proposal_id::text AS proposal_id,
                created_at::text AS created_at,
                updated_at::text AS updated_at
            FROM paper_orders
            WHERE trader_id = $1::uuid
            ORDER BY created_at DESC, id DESC
            LIMIT $2
            "#,
        )
        .bind(trader_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(Self::row_to_paper_order).collect())
    }

    pub async fn list_recent_paper_fills_for_trader(
        &self,
        trader_id: &str,
        limit: i64,
    ) -> Result<Vec<PaperFill>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT
                pf.id::text AS id,
                pf.account_id::text AS account_id,
                pf.order_id::text AS order_id,
                pf.symbol,
                pf.side,
                pf.quantity,
                pf.price,
                pf.notional,
                pf.created_at::text AS created_at
            FROM paper_fills pf
            INNER JOIN paper_orders po ON po.id = pf.order_id
            WHERE po.trader_id = $1::uuid
            ORDER BY pf.created_at DESC, pf.id DESC
            LIMIT $2
            "#,
        )
        .bind(trader_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(Self::row_to_paper_fill).collect())
    }

    pub async fn update_trader(
        &self,
        trader_id: &str,
        name: Option<&str>,
        perspective: Option<&str>,
        freedom_level: Option<&str>,
        default_paper_account_id: Option<&str>,
        openai_api_key_secret: Option<&str>,
        info_sources: Option<&[CreateTraderInfoSourceRequest]>,
    ) -> Result<Option<Trader>, sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        let now = Self::now_string();

        let row = sqlx::query(
            r#"
            UPDATE traders
            SET name = COALESCE($2, name),
                fundamental_perspective = COALESCE($3, fundamental_perspective),
                freedom_level = COALESCE($4, freedom_level),
                default_paper_account_id = CASE WHEN $5 THEN $6::uuid ELSE default_paper_account_id END,
                updated_at = $7::timestamptz
            WHERE id = $1::uuid AND is_active = TRUE
            RETURNING id::text AS id, name, fundamental_perspective, freedom_level, status,
                      default_paper_account_id::text AS default_paper_account_id, is_active,
                      persona, tone, communication_style,
                      created_at::text AS created_at, updated_at::text AS updated_at,
                      started_at::text AS started_at, stopped_at::text AS stopped_at
            "#,
        )
        .bind(trader_id)
        .bind(name)
        .bind(perspective)
        .bind(freedom_level)
        .bind(default_paper_account_id.is_some())
        .bind(default_paper_account_id)
        .bind(&now)
        .fetch_optional(&mut *tx)
        .await?;

        let Some(row) = row else {
            tx.rollback().await?;
            return Ok(None);
        };

        if let Some(secret) = openai_api_key_secret {
            Self::upsert_trader_secret_in_tx(&mut tx, trader_id, secret).await?;
        }
        if let Some(sources) = info_sources {
            Self::replace_trader_info_sources_in_tx(&mut tx, trader_id, sources).await?;
        }
        Self::insert_trader_event_in_tx(
            &mut tx,
            trader_id,
            "trader_updated",
            "Trader updated",
            None,
        )
        .await?;

        tx.commit().await?;
        Ok(Some(Self::row_to_trader(row)))
    }

    pub async fn set_trader_status(
        &self,
        trader_id: &str,
        status: &str,
        event_type: &str,
        message: &str,
    ) -> Result<Option<Trader>, sqlx::Error> {
        let now = Self::now_string();
        let row = sqlx::query(
            r#"
            UPDATE traders
            SET status = $2,
                started_at = CASE WHEN $2 = 'running' THEN $3::timestamptz ELSE started_at END,
                stopped_at = CASE WHEN $2 = 'stopped' THEN $3::timestamptz ELSE stopped_at END,
                updated_at = $3::timestamptz
            WHERE id = $1::uuid AND is_active = TRUE
            RETURNING id::text AS id, name, fundamental_perspective, freedom_level, status,
                      default_paper_account_id::text AS default_paper_account_id, is_active,
                      persona, tone, communication_style,
                      created_at::text AS created_at, updated_at::text AS updated_at,
                      started_at::text AS started_at, stopped_at::text AS stopped_at
            "#,
        )
        .bind(trader_id)
        .bind(status)
        .bind(&now)
        .fetch_optional(&self.pool)
        .await?;

        if row.is_some() {
            self.create_trader_event(
                trader_id,
                &CreateTraderEventRequest {
                    event_type: event_type.to_string(),
                    message: message.to_string(),
                    payload: None,
                },
            )
            .await?;
        }

        Ok(row.map(Self::row_to_trader))
    }

    pub async fn delete_trader(&self, trader_id: &str) -> Result<bool, sqlx::Error> {
        let now = Self::now_string();
        let result = sqlx::query(
            r#"
            UPDATE traders
            SET is_active = FALSE, status = 'stopped', stopped_at = $2::timestamptz,
                updated_at = $2::timestamptz
            WHERE id = $1::uuid AND is_active = TRUE
            "#,
        )
        .bind(trader_id)
        .bind(&now)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() > 0 {
            self.create_trader_event(
                trader_id,
                &CreateTraderEventRequest {
                    event_type: "trader_deleted".to_string(),
                    message: "Trader deleted".to_string(),
                    payload: None,
                },
            )
            .await?;
        }

        Ok(result.rows_affected() > 0)
    }

    pub async fn list_trader_info_sources(
        &self,
        trader_id: &str,
    ) -> Result<Vec<TraderInfoSource>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT id::text AS id, trader_id::text AS trader_id, source_type, name,
                   config_json, enabled, created_at::text AS created_at, updated_at::text AS updated_at
            FROM trader_info_sources
            WHERE trader_id = $1::uuid
            ORDER BY created_at ASC
            "#,
        )
        .bind(trader_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(Self::row_to_trader_info_source)
            .collect())
    }

    pub async fn get_trader_runtime_state(
        &self,
        trader_id: &str,
    ) -> Result<Option<TraderRuntimeState>, sqlx::Error> {
        let row = sqlx::query(
            r#"
            SELECT trader_id::text AS trader_id, engine_name,
                   last_heartbeat_at::text AS last_heartbeat_at,
                   last_evaluation_at::text AS last_evaluation_at,
                   last_error, current_task, created_at::text AS created_at,
                   updated_at::text AS updated_at
            FROM trader_runtime_state
            WHERE trader_id = $1::uuid
            "#,
        )
        .bind(trader_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Self::row_to_trader_runtime_state))
    }

    pub async fn upsert_trader_runtime_state(
        &self,
        trader_id: &str,
        request: &UpsertTraderRuntimeStateRequest,
    ) -> Result<TraderRuntimeState, sqlx::Error> {
        let now = Self::now_string();
        let row = sqlx::query(
            r#"
            INSERT INTO trader_runtime_state (
                trader_id, engine_name, last_heartbeat_at, last_evaluation_at,
                last_error, current_task, created_at, updated_at
            ) VALUES (
                $1::uuid, $2, $3::timestamptz, $4::timestamptz, $5, $6,
                $7::timestamptz, $8::timestamptz
            )
            ON CONFLICT (trader_id) DO UPDATE SET
                engine_name = EXCLUDED.engine_name,
                last_heartbeat_at = EXCLUDED.last_heartbeat_at,
                last_evaluation_at = EXCLUDED.last_evaluation_at,
                last_error = EXCLUDED.last_error,
                current_task = EXCLUDED.current_task,
                updated_at = EXCLUDED.updated_at
            RETURNING trader_id::text AS trader_id, engine_name,
                      last_heartbeat_at::text AS last_heartbeat_at,
                      last_evaluation_at::text AS last_evaluation_at,
                      last_error, current_task, created_at::text AS created_at,
                      updated_at::text AS updated_at
            "#,
        )
        .bind(trader_id)
        .bind(&request.engine_name)
        .bind(&request.last_heartbeat_at)
        .bind(&request.last_evaluation_at)
        .bind(&request.last_error)
        .bind(&request.current_task)
        .bind(&now)
        .bind(&now)
        .fetch_one(&self.pool)
        .await?;

        Ok(Self::row_to_trader_runtime_state(row))
    }

    pub async fn create_trader_event(
        &self,
        trader_id: &str,
        request: &CreateTraderEventRequest,
    ) -> Result<TraderEvent, sqlx::Error> {
        let event = TraderEvent {
            id: Self::new_id(),
            trader_id: trader_id.to_string(),
            event_type: request.event_type.clone(),
            message: request.message.clone(),
            payload: request.payload.clone(),
            created_at: Self::now_string(),
        };

        sqlx::query(
            r#"
            INSERT INTO trader_events (id, trader_id, event_type, message, payload, created_at)
            VALUES ($1::uuid, $2::uuid, $3, $4, $5::jsonb, $6::timestamptz)
            "#,
        )
        .bind(&event.id)
        .bind(&event.trader_id)
        .bind(&event.event_type)
        .bind(&event.message)
        .bind(&event.payload)
        .bind(&event.created_at)
        .execute(&self.pool)
        .await?;

        Ok(event)
    }

    pub async fn list_trader_events(
        &self,
        trader_id: &str,
        limit: i64,
    ) -> Result<Vec<TraderEvent>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT id::text AS id, trader_id::text AS trader_id, event_type, message,
                   payload, created_at::text AS created_at
            FROM trader_events
            WHERE trader_id = $1::uuid
            ORDER BY created_at DESC, id DESC
            LIMIT $2
            "#,
        )
        .bind(trader_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(Self::row_to_trader_event).collect())
    }

    pub async fn create_trader_trade_proposal(
        &self,
        trader_id: &str,
        request: &CreateTraderTradeProposalRequest,
    ) -> Result<TraderTradeProposal, sqlx::Error> {
        let now = Self::now_string();
        let row = sqlx::query(
            r#"
            INSERT INTO trader_trade_proposals (
                id, trader_id, symbol, side, quantity, order_type, reason, confidence,
                status, created_at, updated_at
            ) VALUES (
                $1::uuid, $2::uuid, $3, $4, $5, $6, $7, $8, 'pending_review',
                $9::timestamptz, $10::timestamptz
            )
            RETURNING id::text AS id, trader_id::text AS trader_id, symbol, side, quantity,
                      order_type, reason, confidence, status, reviewed_by,
                      reviewed_at::text AS reviewed_at, resulting_order_id::text AS resulting_order_id,
                      created_at::text AS created_at, updated_at::text AS updated_at
            "#,
        )
        .bind(Self::new_id())
        .bind(trader_id)
        .bind(request.symbol.trim().to_ascii_uppercase())
        .bind(request.side.trim().to_ascii_lowercase())
        .bind(request.quantity)
        .bind(
            request
                .order_type
                .clone()
                .unwrap_or_else(|| "market".to_string()),
        )
        .bind(&request.reason)
        .bind(request.confidence)
        .bind(&now)
        .bind(&now)
        .fetch_one(&self.pool)
        .await?;

        Ok(Self::row_to_trader_trade_proposal(row))
    }

    pub async fn list_trader_trade_proposals(
        &self,
        trader_id: &str,
    ) -> Result<Vec<TraderTradeProposal>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT id::text AS id, trader_id::text AS trader_id, symbol, side, quantity,
                   order_type, reason, confidence, status, reviewed_by,
                   reviewed_at::text AS reviewed_at, resulting_order_id::text AS resulting_order_id,
                   created_at::text AS created_at, updated_at::text AS updated_at
            FROM trader_trade_proposals
            WHERE trader_id = $1::uuid
            ORDER BY created_at DESC, id DESC
            LIMIT 100
            "#,
        )
        .bind(trader_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(Self::row_to_trader_trade_proposal)
            .collect())
    }

    pub async fn get_trader_trade_proposal(
        &self,
        trader_id: &str,
        proposal_id: &str,
    ) -> Result<Option<TraderTradeProposal>, sqlx::Error> {
        let row = sqlx::query(
            r#"
            SELECT id::text AS id, trader_id::text AS trader_id, symbol, side, quantity,
                   order_type, reason, confidence, status, reviewed_by,
                   reviewed_at::text AS reviewed_at, resulting_order_id::text AS resulting_order_id,
                   created_at::text AS created_at, updated_at::text AS updated_at
            FROM trader_trade_proposals
            WHERE trader_id = $1::uuid AND id = $2::uuid
            "#,
        )
        .bind(trader_id)
        .bind(proposal_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Self::row_to_trader_trade_proposal))
    }

    pub async fn update_trader_trade_proposal_review(
        &self,
        trader_id: &str,
        proposal_id: &str,
        status: &str,
        resulting_order_id: Option<&str>,
    ) -> Result<Option<TraderTradeProposal>, sqlx::Error> {
        let now = Self::now_string();
        let row = sqlx::query(
            r#"
            UPDATE trader_trade_proposals
            SET status = $3, reviewed_by = 'local_user', reviewed_at = $4::timestamptz,
                resulting_order_id = $5::uuid, updated_at = $4::timestamptz
            WHERE trader_id = $1::uuid AND id = $2::uuid
            RETURNING id::text AS id, trader_id::text AS trader_id, symbol, side, quantity,
                      order_type, reason, confidence, status, reviewed_by,
                      reviewed_at::text AS reviewed_at, resulting_order_id::text AS resulting_order_id,
                      created_at::text AS created_at, updated_at::text AS updated_at
            "#,
        )
        .bind(trader_id)
        .bind(proposal_id)
        .bind(status)
        .bind(&now)
        .bind(resulting_order_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Self::row_to_trader_trade_proposal))
    }

    pub async fn list_trader_symbols(
        &self,
        trader_id: &str,
        status: Option<&str>,
        asset_type: Option<&str>,
        source: Option<&str>,
    ) -> Result<Vec<TraderSymbol>, sqlx::Error> {
        let mut query = QueryBuilder::<Postgres>::new(
            r#"
            SELECT id::text AS id, trader_id::text AS trader_id, symbol, asset_type,
                   name, exchange, sector, industry, notes, thesis, fit_score, status,
                   source, created_at::text AS created_at, updated_at::text AS updated_at
            FROM trader_symbols
            WHERE trader_id =
            "#,
        );
        query.push_bind(trader_id);
        query.push("::uuid");

        if let Some(status) = status {
            query.push(" AND status = ");
            query.push_bind(status);
        }
        if let Some(asset_type) = asset_type {
            query.push(" AND asset_type = ");
            query.push_bind(asset_type);
        }
        if let Some(source) = source {
            query.push(" AND source = ");
            query.push_bind(source);
        }

        query.push(
            r#"
            ORDER BY
                CASE status
                    WHEN 'active' THEN 1
                    WHEN 'watching' THEN 2
                    WHEN 'candidate' THEN 3
                    WHEN 'rejected' THEN 4
                    WHEN 'archived' THEN 5
                    ELSE 6
                END,
                fit_score DESC NULLS LAST,
                symbol ASC
            "#,
        );

        let rows = query.build().fetch_all(&self.pool).await?;
        Ok(rows.into_iter().map(Self::row_to_trader_symbol).collect())
    }

    pub async fn upsert_trader_symbol(
        &self,
        trader_id: &str,
        request: &CreateTraderSymbolRequest,
    ) -> Result<TraderSymbol, sqlx::Error> {
        let now = Self::now_string();
        let symbol = request.symbol.trim().to_ascii_uppercase();
        let asset_type = request.asset_type.as_deref().unwrap_or("stock").trim();
        let status = request.status.as_deref().unwrap_or("watching").trim();
        let source = request.source.as_deref().unwrap_or("manual").trim();
        let row = sqlx::query(
            r#"
            INSERT INTO trader_symbols (
                id, trader_id, symbol, asset_type, name, exchange, sector, industry,
                notes, thesis, fit_score, status, source, created_at, updated_at
            ) VALUES (
                $1::uuid, $2::uuid, $3, $4, $5, $6, $7, $8, $9, $10, $11,
                $12, $13, $14::timestamptz, $15::timestamptz
            )
            ON CONFLICT (trader_id, symbol) DO UPDATE SET
                asset_type = EXCLUDED.asset_type,
                name = COALESCE(EXCLUDED.name, trader_symbols.name),
                exchange = COALESCE(EXCLUDED.exchange, trader_symbols.exchange),
                sector = COALESCE(EXCLUDED.sector, trader_symbols.sector),
                industry = COALESCE(EXCLUDED.industry, trader_symbols.industry),
                notes = COALESCE(EXCLUDED.notes, trader_symbols.notes),
                thesis = COALESCE(EXCLUDED.thesis, trader_symbols.thesis),
                fit_score = COALESCE(EXCLUDED.fit_score, trader_symbols.fit_score),
                status = EXCLUDED.status,
                source = EXCLUDED.source,
                updated_at = EXCLUDED.updated_at
            RETURNING id::text AS id, trader_id::text AS trader_id, symbol, asset_type,
                      name, exchange, sector, industry, notes, thesis, fit_score, status,
                      source, created_at::text AS created_at, updated_at::text AS updated_at
            "#,
        )
        .bind(Self::new_id())
        .bind(trader_id)
        .bind(symbol)
        .bind(asset_type)
        .bind(
            request
                .name
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty()),
        )
        .bind(
            request
                .exchange
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty()),
        )
        .bind(
            request
                .sector
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty()),
        )
        .bind(
            request
                .industry
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty()),
        )
        .bind(
            request
                .notes
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty()),
        )
        .bind(
            request
                .thesis
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty()),
        )
        .bind(request.fit_score)
        .bind(status)
        .bind(source)
        .bind(&now)
        .bind(&now)
        .fetch_one(&self.pool)
        .await?;

        Ok(Self::row_to_trader_symbol(row))
    }

    pub async fn update_trader_symbol(
        &self,
        trader_id: &str,
        symbol_id: &str,
        request: &UpdateTraderSymbolRequest,
    ) -> Result<Option<TraderSymbol>, sqlx::Error> {
        let now = Self::now_string();
        let row = sqlx::query(
            r#"
            UPDATE trader_symbols
            SET asset_type = COALESCE($3, asset_type),
                name = COALESCE($4, name),
                exchange = COALESCE($5, exchange),
                sector = COALESCE($6, sector),
                industry = COALESCE($7, industry),
                notes = COALESCE($8, notes),
                thesis = COALESCE($9, thesis),
                fit_score = COALESCE($10, fit_score),
                status = COALESCE($11, status),
                updated_at = $12::timestamptz
            WHERE trader_id = $1::uuid AND id = $2::uuid
            RETURNING id::text AS id, trader_id::text AS trader_id, symbol, asset_type,
                      name, exchange, sector, industry, notes, thesis, fit_score, status,
                      source, created_at::text AS created_at, updated_at::text AS updated_at
            "#,
        )
        .bind(trader_id)
        .bind(symbol_id)
        .bind(request.asset_type.as_deref().map(str::trim))
        .bind(request.name.as_deref().map(str::trim))
        .bind(request.exchange.as_deref().map(str::trim))
        .bind(request.sector.as_deref().map(str::trim))
        .bind(request.industry.as_deref().map(str::trim))
        .bind(request.notes.as_deref().map(str::trim))
        .bind(request.thesis.as_deref().map(str::trim))
        .bind(request.fit_score)
        .bind(request.status.as_deref().map(str::trim))
        .bind(&now)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Self::row_to_trader_symbol))
    }

    pub async fn set_trader_symbol_status(
        &self,
        trader_id: &str,
        symbol_id: &str,
        status: &str,
    ) -> Result<Option<TraderSymbol>, sqlx::Error> {
        self.update_trader_symbol(
            trader_id,
            symbol_id,
            &UpdateTraderSymbolRequest {
                status: Some(status.to_string()),
                ..Default::default()
            },
        )
        .await
    }

    pub async fn create_trader_portfolio_proposal(
        &self,
        trader_id: &str,
        request: &CreateTraderPortfolioProposalRequest,
    ) -> Result<TraderPortfolioProposalDetail, sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        let now = Self::now_string();
        let proposal_id = Self::new_id();
        let proposed_actions_json =
            serde_json::to_string(&request.proposed_actions).unwrap_or_else(|_| "[]".to_string());

        sqlx::query(
            r#"
            UPDATE trader_portfolio_proposals
            SET status = 'superseded', plan_state = 'superseded', updated_at = $2::timestamptz
            WHERE trader_id = $1::uuid AND status = 'proposed'
            "#,
        )
        .bind(trader_id)
        .bind(&now)
        .execute(&mut *tx)
        .await?;

        let proposal_row = sqlx::query(
            r#"
            INSERT INTO trader_portfolio_proposals (
                id, trader_id, paper_account_id, title, summary, thesis, status, plan_state,
                confidence, proposed_actions_json, source_snapshot_json, risk_snapshot_json,
                market_snapshot_json, market_basis_json, invalidation_conditions_json,
                change_thresholds_json, replacement_reason, expected_duration_seconds,
                created_at, updated_at
            ) VALUES (
                $1::uuid, $2::uuid, $3::uuid, $4, $5, $6, 'proposed', 'draft',
                $7, $8::jsonb, $9::jsonb, $10::jsonb, $11::jsonb, $12::jsonb,
                $13::jsonb, $14::jsonb, $15, $16, $17::timestamptz, $18::timestamptz
            )
            RETURNING id::text AS id, trader_id::text AS trader_id,
                      paper_account_id::text AS paper_account_id, title, summary, thesis,
                      status, plan_state, confidence, proposed_actions_json, source_snapshot_json,
                      risk_snapshot_json, market_snapshot_json,
                      market_basis_json, invalidation_conditions_json, change_thresholds_json,
                      replacement_reason,
                      created_at::text AS created_at, updated_at::text AS updated_at,
                      reviewed_at::text AS reviewed_at, review_note,
                      accepted_at::text AS accepted_at, active_until::text AS active_until,
                      expected_duration_seconds
            "#,
        )
        .bind(&proposal_id)
        .bind(trader_id)
        .bind(&request.paper_account_id)
        .bind(request.title.trim())
        .bind(request.summary.trim())
        .bind(request.thesis.trim())
        .bind(request.confidence)
        .bind(&proposed_actions_json)
        .bind(&request.source_snapshot_json)
        .bind(&request.risk_snapshot_json)
        .bind(&request.market_snapshot_json)
        .bind(&request.market_basis_json)
        .bind(&request.invalidation_conditions_json)
        .bind(&request.change_thresholds_json)
        .bind(&request.replacement_reason)
        .bind(request.expected_duration_seconds)
        .bind(&now)
        .bind(&now)
        .fetch_one(&mut *tx)
        .await?;

        let mut actions = Vec::new();
        for action in &request.proposed_actions {
            actions.push(
                Self::insert_trader_portfolio_proposal_action_in_tx(
                    &mut tx,
                    &proposal_id,
                    trader_id,
                    action,
                    &now,
                )
                .await?,
            );
        }

        Self::insert_trader_event_in_tx(
            &mut tx,
            trader_id,
            "proposal_created",
            &format!("Created portfolio proposal {}", request.title.trim()),
            Some(&format!(r#"{{"proposal_id":"{}"}}"#, proposal_id)),
        )
        .await?;

        tx.commit().await?;
        Ok(TraderPortfolioProposalDetail {
            proposal: Self::row_to_trader_portfolio_proposal(proposal_row),
            actions,
        })
    }

    async fn insert_trader_portfolio_proposal_action_in_tx(
        tx: &mut Transaction<'_, Postgres>,
        proposal_id: &str,
        trader_id: &str,
        action: &CreateTraderPortfolioProposalActionRequest,
        now: &str,
    ) -> Result<TraderPortfolioProposalAction, sqlx::Error> {
        let row = sqlx::query(
            r#"
            INSERT INTO trader_portfolio_proposal_actions (
                id, proposal_id, trader_id, symbol, action_type, side, quantity,
                order_type, entry_price, exit_price, limit_price, stop_price,
                expected_duration_seconds, enact_by, market_price_at_creation,
                rationale, confidence, risk_decision, status, created_at, updated_at
            ) VALUES (
                $1::uuid, $2::uuid, $3::uuid, $4, $5, $6, $7, $8, $9, $10,
                $11, $12, $13, $14::timestamptz, $15, $16, $17, $18,
                'proposed', $19::timestamptz, $20::timestamptz
            )
            RETURNING id::text AS id, proposal_id::text AS proposal_id, trader_id::text AS trader_id,
                      symbol, action_type, side, quantity, order_type, entry_price, exit_price,
                      limit_price, stop_price, expected_duration_seconds, enact_by::text AS enact_by,
                      market_price_at_creation, rationale, confidence,
                      risk_decision, status, created_at::text AS created_at, updated_at::text AS updated_at
            "#,
        )
        .bind(Self::new_id())
        .bind(proposal_id)
        .bind(trader_id)
        .bind(
            action
                .symbol
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_ascii_uppercase),
        )
        .bind(action.action_type.trim())
        .bind(action.side.as_deref().map(str::trim))
        .bind(action.quantity)
        .bind(action.order_type.as_deref().map(str::trim))
        .bind(action.entry_price)
        .bind(action.exit_price)
        .bind(action.limit_price)
        .bind(action.stop_price)
        .bind(action.expected_duration_seconds)
        .bind(&action.enact_by)
        .bind(action.market_price_at_creation)
        .bind(action.rationale.trim())
        .bind(action.confidence)
        .bind(action.risk_decision.as_deref().map(str::trim))
        .bind(now)
        .bind(now)
        .fetch_one(&mut **tx)
        .await?;
        Ok(Self::row_to_trader_portfolio_proposal_action(row))
    }

    pub async fn list_trader_portfolio_proposals(
        &self,
        trader_id: &str,
    ) -> Result<Vec<TraderPortfolioProposalDetail>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT id::text AS id, trader_id::text AS trader_id,
                   paper_account_id::text AS paper_account_id, title, summary, thesis,
                   status, plan_state, confidence, proposed_actions_json, source_snapshot_json,
                   risk_snapshot_json, market_snapshot_json, market_basis_json,
                   invalidation_conditions_json, change_thresholds_json, replacement_reason,
                   created_at::text AS created_at, updated_at::text AS updated_at,
                   reviewed_at::text AS reviewed_at, review_note,
                   accepted_at::text AS accepted_at, active_until::text AS active_until,
                   expected_duration_seconds
            FROM trader_portfolio_proposals
            WHERE trader_id = $1::uuid
            ORDER BY created_at DESC, id DESC
            LIMIT 100
            "#,
        )
        .bind(trader_id)
        .fetch_all(&self.pool)
        .await?;

        let mut details = Vec::new();
        for row in rows {
            let proposal = Self::row_to_trader_portfolio_proposal(row);
            let actions = self
                .list_trader_portfolio_proposal_actions(&proposal.id)
                .await?;
            details.push(TraderPortfolioProposalDetail { proposal, actions });
        }
        Ok(details)
    }

    pub async fn get_latest_trader_portfolio_proposal(
        &self,
        trader_id: &str,
    ) -> Result<Option<TraderPortfolioProposalDetail>, sqlx::Error> {
        let row = sqlx::query(
            r#"
            SELECT id::text AS id, trader_id::text AS trader_id,
                   paper_account_id::text AS paper_account_id, title, summary, thesis,
                   status, plan_state, confidence, proposed_actions_json, source_snapshot_json,
                   risk_snapshot_json, market_snapshot_json, market_basis_json,
                   invalidation_conditions_json, change_thresholds_json, replacement_reason,
                   created_at::text AS created_at, updated_at::text AS updated_at,
                   reviewed_at::text AS reviewed_at, review_note,
                   accepted_at::text AS accepted_at, active_until::text AS active_until,
                   expected_duration_seconds
            FROM trader_portfolio_proposals
            WHERE trader_id = $1::uuid AND status <> 'superseded'
            ORDER BY created_at DESC, id DESC
            LIMIT 1
            "#,
        )
        .bind(trader_id)
        .fetch_optional(&self.pool)
        .await?;
        let Some(row) = row else {
            return Ok(None);
        };
        let proposal = Self::row_to_trader_portfolio_proposal(row);
        let actions = self
            .list_trader_portfolio_proposal_actions(&proposal.id)
            .await?;
        Ok(Some(TraderPortfolioProposalDetail { proposal, actions }))
    }

    pub async fn get_active_trader_portfolio_proposal(
        &self,
        trader_id: &str,
    ) -> Result<Option<TraderPortfolioProposalDetail>, sqlx::Error> {
        let row = sqlx::query(
            r#"
            SELECT id::text AS id, trader_id::text AS trader_id,
                   paper_account_id::text AS paper_account_id, title, summary, thesis,
                   status, plan_state, confidence, proposed_actions_json, source_snapshot_json,
                   risk_snapshot_json, market_snapshot_json, market_basis_json,
                   invalidation_conditions_json, change_thresholds_json, replacement_reason,
                   created_at::text AS created_at, updated_at::text AS updated_at,
                   reviewed_at::text AS reviewed_at, review_note,
                   accepted_at::text AS accepted_at, active_until::text AS active_until,
                   expected_duration_seconds
            FROM trader_portfolio_proposals
            WHERE trader_id = $1::uuid AND status = 'accepted' AND plan_state = 'active'
            ORDER BY accepted_at DESC NULLS LAST, created_at DESC, id DESC
            LIMIT 1
            "#,
        )
        .bind(trader_id)
        .fetch_optional(&self.pool)
        .await?;
        let Some(row) = row else {
            return Ok(None);
        };
        let proposal = Self::row_to_trader_portfolio_proposal(row);
        let actions = self
            .list_trader_portfolio_proposal_actions(&proposal.id)
            .await?;
        Ok(Some(TraderPortfolioProposalDetail { proposal, actions }))
    }

    pub async fn get_trader_portfolio_proposal(
        &self,
        trader_id: &str,
        proposal_id: &str,
    ) -> Result<Option<TraderPortfolioProposalDetail>, sqlx::Error> {
        let row = sqlx::query(
            r#"
            SELECT id::text AS id, trader_id::text AS trader_id,
                   paper_account_id::text AS paper_account_id, title, summary, thesis,
                   status, plan_state, confidence, proposed_actions_json, source_snapshot_json,
                   risk_snapshot_json, market_snapshot_json, market_basis_json,
                   invalidation_conditions_json, change_thresholds_json, replacement_reason,
                   created_at::text AS created_at, updated_at::text AS updated_at,
                   reviewed_at::text AS reviewed_at, review_note,
                   accepted_at::text AS accepted_at, active_until::text AS active_until,
                   expected_duration_seconds
            FROM trader_portfolio_proposals
            WHERE trader_id = $1::uuid AND id = $2::uuid
            "#,
        )
        .bind(trader_id)
        .bind(proposal_id)
        .fetch_optional(&self.pool)
        .await?;
        let Some(row) = row else {
            return Ok(None);
        };
        let proposal = Self::row_to_trader_portfolio_proposal(row);
        let actions = self
            .list_trader_portfolio_proposal_actions(&proposal.id)
            .await?;
        Ok(Some(TraderPortfolioProposalDetail { proposal, actions }))
    }

    async fn list_trader_portfolio_proposal_actions(
        &self,
        proposal_id: &str,
    ) -> Result<Vec<TraderPortfolioProposalAction>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT id::text AS id, proposal_id::text AS proposal_id, trader_id::text AS trader_id,
                   symbol, action_type, side, quantity, order_type, entry_price, exit_price,
                   limit_price, stop_price, expected_duration_seconds, enact_by::text AS enact_by,
                   market_price_at_creation, rationale, confidence,
                   risk_decision, status, created_at::text AS created_at, updated_at::text AS updated_at
            FROM trader_portfolio_proposal_actions
            WHERE proposal_id = $1::uuid
            ORDER BY created_at ASC, id ASC
            "#,
        )
        .bind(proposal_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(Self::row_to_trader_portfolio_proposal_action)
            .collect())
    }

    pub async fn review_trader_portfolio_proposal(
        &self,
        trader_id: &str,
        proposal_id: &str,
        status: &str,
        review_note: Option<&str>,
    ) -> Result<Option<TraderPortfolioProposalDetail>, sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        let now = Self::now_string();
        if status == "accepted" {
            sqlx::query(
                r#"
                UPDATE trader_portfolio_proposals
                SET plan_state = 'superseded', status = 'superseded', updated_at = $2::timestamptz
                WHERE trader_id = $1::uuid AND plan_state = 'active'
                "#,
            )
            .bind(trader_id)
            .bind(&now)
            .execute(&mut *tx)
            .await?;
        }
        let plan_state = match status {
            "accepted" => "active",
            "rejected" => "rejected",
            "superseded" => "superseded",
            "expired" => "invalidated",
            "executed" => "completed",
            _ => "draft",
        };
        let row = sqlx::query(
            r#"
            UPDATE trader_portfolio_proposals
            SET status = $3,
                plan_state = $4,
                accepted_at = CASE WHEN $3 = 'accepted' THEN $5::timestamptz ELSE accepted_at END,
                active_until = CASE
                    WHEN $3 = 'accepted' AND expected_duration_seconds IS NOT NULL
                    THEN $5::timestamptz + (expected_duration_seconds || ' seconds')::interval
                    ELSE active_until
                END,
                reviewed_at = $5::timestamptz,
                review_note = $6,
                updated_at = $5::timestamptz
            WHERE trader_id = $1::uuid AND id = $2::uuid
            RETURNING id::text AS id, trader_id::text AS trader_id,
                      paper_account_id::text AS paper_account_id, title, summary, thesis,
                      status, plan_state, confidence, proposed_actions_json, source_snapshot_json,
                      risk_snapshot_json, market_snapshot_json, market_basis_json,
                      invalidation_conditions_json, change_thresholds_json, replacement_reason,
                      created_at::text AS created_at, updated_at::text AS updated_at,
                      reviewed_at::text AS reviewed_at, review_note,
                      accepted_at::text AS accepted_at, active_until::text AS active_until,
                      expected_duration_seconds
            "#,
        )
        .bind(trader_id)
        .bind(proposal_id)
        .bind(status)
        .bind(plan_state)
        .bind(&now)
        .bind(review_note)
        .fetch_optional(&mut *tx)
        .await?;

        let Some(row) = row else {
            return Ok(None);
        };
        let proposal = Self::row_to_trader_portfolio_proposal(row);
        let action_rows = sqlx::query(
            r#"
            SELECT id::text AS id, proposal_id::text AS proposal_id, trader_id::text AS trader_id,
                   symbol, action_type, side, quantity, order_type, entry_price, exit_price,
                   limit_price, stop_price, expected_duration_seconds, enact_by::text AS enact_by,
                   market_price_at_creation, rationale, confidence,
                   risk_decision, status, created_at::text AS created_at, updated_at::text AS updated_at
            FROM trader_portfolio_proposal_actions
            WHERE proposal_id = $1::uuid
            ORDER BY created_at ASC, id ASC
            "#,
        )
        .bind(&proposal.id)
        .fetch_all(&mut *tx)
        .await?;
        tx.commit().await?;
        let actions = action_rows
            .into_iter()
            .map(Self::row_to_trader_portfolio_proposal_action)
            .collect();
        Ok(Some(TraderPortfolioProposalDetail { proposal, actions }))
    }

    pub async fn list_engine_trader_configs(
        &self,
    ) -> Result<Vec<EngineRunnableTrader>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT t.id::text AS id, t.name, t.fundamental_perspective, t.freedom_level,
                   t.default_paper_account_id::text AS default_paper_account_id,
                   t.persona, t.tone, t.communication_style,
                   s.openai_api_key_secret
            FROM traders t
            INNER JOIN trader_secrets s ON s.trader_id = t.id
            WHERE t.is_active = TRUE AND t.status = 'running'
            ORDER BY t.updated_at DESC, t.name ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        let mut traders = Vec::new();
        for row in rows {
            let id: String = row.get("id");
            traders.push(EngineRunnableTrader {
                id: id.clone(),
                name: row.get("name"),
                fundamental_perspective: row.get("fundamental_perspective"),
                freedom_level: row.get("freedom_level"),
                default_paper_account_id: row.get("default_paper_account_id"),
                persona: row.try_get("persona").ok(),
                tone: row.try_get("tone").ok(),
                communication_style: row.try_get("communication_style").ok(),
                info_sources: self.list_trader_info_sources(&id).await?,
                data_sources: self.list_engine_trader_data_sources(&id).await?,
                tracked_symbols: self
                    .list_trader_symbols(&id, None, None, None)
                    .await?
                    .into_iter()
                    .filter(|symbol| symbol.status == "active" || symbol.status == "watching")
                    .collect(),
                openai_api_key: row.get("openai_api_key_secret"),
            });
        }

        Ok(traders)
    }

    pub async fn list_channels(&self) -> Result<Vec<Channel>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT id, name, display_name, description, is_system, created_at, updated_at
            FROM channels
            ORDER BY
                CASE name
                    WHEN 'general' THEN 0
                    WHEN 'data_analysis' THEN 1
                    WHEN 'trading' THEN 2
                    ELSE 3
                END,
                display_name ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(Self::row_to_channel).collect())
    }

    pub async fn get_channel(&self, channel_id: &str) -> Result<Option<Channel>, sqlx::Error> {
        let row = sqlx::query(
            r#"
            SELECT id, name, display_name, description, is_system, created_at, updated_at
            FROM channels
            WHERE id = $1 OR name = $1
            "#,
        )
        .bind(channel_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(Self::row_to_channel))
    }

    pub async fn list_channel_messages(
        &self,
        channel_id: &str,
        limit: Option<i64>,
        before: Option<&str>,
        after: Option<&str>,
    ) -> Result<Vec<ChannelMessage>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT id, channel_id, author_type, author_id, author_name, role,
                   content_markdown, metadata_json, created_at
            FROM channel_messages
            WHERE channel_id = $1
              AND ($2::text IS NULL OR created_at < $2)
              AND ($3::text IS NULL OR created_at > $3)
            ORDER BY created_at ASC, id ASC
            LIMIT $4
            "#,
        )
        .bind(channel_id)
        .bind(before)
        .bind(after)
        .bind(limit.unwrap_or(200).clamp(1, 500))
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(Self::row_to_channel_message).collect())
    }

    pub async fn create_channel_message(
        &self,
        channel_id: &str,
        request: &CreateChannelMessageRequest,
    ) -> Result<ChannelMessage, sqlx::Error> {
        let row = sqlx::query(
            r#"
            INSERT INTO channel_messages (
                id, channel_id, author_type, author_id, author_name, role,
                content_markdown, metadata_json, created_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING id, channel_id, author_type, author_id, author_name, role,
                      content_markdown, metadata_json, created_at
            "#,
        )
        .bind(Self::new_id())
        .bind(channel_id)
        .bind(request.author_type.trim())
        .bind(
            request
                .author_id
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty()),
        )
        .bind(
            request
                .author_name
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .unwrap_or("Unknown"),
        )
        .bind(request.role.as_deref().unwrap_or("message").trim())
        .bind(request.content_markdown.trim())
        .bind(request.metadata_json.as_deref())
        .bind(Self::now_string())
        .fetch_one(&self.pool)
        .await?;
        Ok(Self::row_to_channel_message(row))
    }

    pub async fn clear_channel_messages(&self) -> Result<u64, sqlx::Error> {
        let result = sqlx::query("DELETE FROM channel_messages")
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected())
    }

    pub async fn latest_channel_message_for_author(
        &self,
        channel_id: &str,
        author_type: &str,
        author_id: Option<&str>,
    ) -> Result<Option<ChannelMessage>, sqlx::Error> {
        let row = sqlx::query(
            r#"
            SELECT id, channel_id, author_type, author_id, author_name, role,
                   content_markdown, metadata_json, created_at
            FROM channel_messages
            WHERE channel_id = $1
              AND author_type = $2
              AND (($3::text IS NULL AND author_id IS NULL) OR author_id = $3)
            ORDER BY created_at DESC, id DESC
            LIMIT 1
            "#,
        )
        .bind(channel_id)
        .bind(author_type)
        .bind(author_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(Self::row_to_channel_message))
    }

    pub async fn list_trader_memories(
        &self,
        trader_id: &str,
        status: Option<&str>,
        memory_type: Option<&str>,
        topic: Option<&str>,
    ) -> Result<Vec<TraderMemory>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT id, trader_id, memory_type, topic, summary, source_channel_id,
                   source_message_id, confidence, importance, status, last_used_at,
                   created_at, updated_at
            FROM trader_memories
            WHERE trader_id = $1
              AND ($2::text IS NULL OR $2 = '__all__' OR status = $2)
              AND ($3::text IS NULL OR memory_type = $3)
              AND ($4::text IS NULL OR topic ILIKE '%' || $4 || '%')
            ORDER BY importance DESC, created_at DESC
            "#,
        )
        .bind(trader_id)
        .bind(status.unwrap_or("active"))
        .bind(memory_type)
        .bind(topic)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(Self::row_to_trader_memory).collect())
    }

    pub async fn search_trader_memories(
        &self,
        trader_id: &str,
        query: &str,
        limit: Option<i64>,
    ) -> Result<TraderMemorySearchResponse, sqlx::Error> {
        let terms = query
            .split_whitespace()
            .take(8)
            .map(|term| term.trim_matches(|ch: char| !ch.is_ascii_alphanumeric()))
            .filter(|term| term.len() >= 3)
            .collect::<Vec<_>>();
        let search = if terms.is_empty() {
            query.trim().to_string()
        } else {
            terms.join(" ")
        };
        let rows = sqlx::query(
            r#"
            SELECT id, trader_id, memory_type, topic, summary, source_channel_id,
                   source_message_id, confidence, importance, status, last_used_at,
                   created_at, updated_at
            FROM trader_memories
            WHERE trader_id = $1
              AND status = 'active'
              AND ($2::text = '' OR topic ILIKE '%' || $2 || '%' OR summary ILIKE '%' || $2 || '%')
            ORDER BY importance DESC, COALESCE(last_used_at, created_at) DESC
            LIMIT $3
            "#,
        )
        .bind(trader_id)
        .bind(search)
        .bind(limit.unwrap_or(5).clamp(1, 25))
        .fetch_all(&self.pool)
        .await?;
        Ok(TraderMemorySearchResponse {
            memories: rows.into_iter().map(Self::row_to_trader_memory).collect(),
        })
    }

    pub async fn create_trader_memory(
        &self,
        trader_id: &str,
        request: &CreateTraderMemoryRequest,
    ) -> Result<TraderMemory, sqlx::Error> {
        let now = Self::now_string();
        let row = sqlx::query(
            r#"
            INSERT INTO trader_memories (
                id, trader_id, memory_type, topic, summary, source_channel_id,
                source_message_id, confidence, importance, status, last_used_at,
                created_at, updated_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 'active', NULL, $10, $10)
            RETURNING id, trader_id, memory_type, topic, summary, source_channel_id,
                      source_message_id, confidence, importance, status, last_used_at,
                      created_at, updated_at
            "#,
        )
        .bind(Self::new_id())
        .bind(trader_id)
        .bind(request.memory_type.trim())
        .bind(request.topic.trim())
        .bind(request.summary.trim())
        .bind(request.source_channel_id.as_deref())
        .bind(request.source_message_id.as_deref())
        .bind(request.confidence)
        .bind(request.importance.unwrap_or(3).clamp(1, 5))
        .bind(now)
        .fetch_one(&self.pool)
        .await?;
        Ok(Self::row_to_trader_memory(row))
    }

    pub async fn update_trader_memory(
        &self,
        trader_id: &str,
        memory_id: &str,
        request: &UpdateTraderMemoryRequest,
    ) -> Result<Option<TraderMemory>, sqlx::Error> {
        let row = sqlx::query(
            r#"
            UPDATE trader_memories
            SET memory_type = COALESCE($3, memory_type),
                topic = COALESCE($4, topic),
                summary = COALESCE($5, summary),
                confidence = COALESCE($6, confidence),
                importance = COALESCE($7, importance),
                status = COALESCE($8, status),
                updated_at = $9
            WHERE trader_id = $1 AND id = $2
            RETURNING id, trader_id, memory_type, topic, summary, source_channel_id,
                      source_message_id, confidence, importance, status, last_used_at,
                      created_at, updated_at
            "#,
        )
        .bind(trader_id)
        .bind(memory_id)
        .bind(request.memory_type.as_deref())
        .bind(request.topic.as_deref())
        .bind(request.summary.as_deref())
        .bind(request.confidence)
        .bind(request.importance)
        .bind(request.status.as_deref())
        .bind(Self::now_string())
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(Self::row_to_trader_memory))
    }

    pub async fn archive_trader_memory(
        &self,
        trader_id: &str,
        memory_id: &str,
    ) -> Result<Option<TraderMemory>, sqlx::Error> {
        self.update_trader_memory(
            trader_id,
            memory_id,
            &UpdateTraderMemoryRequest {
                status: Some("archived".to_string()),
                ..Default::default()
            },
        )
        .await
    }

    pub async fn mark_trader_memory_used(
        &self,
        trader_id: &str,
        memory_id: &str,
    ) -> Result<Option<TraderMemory>, sqlx::Error> {
        let now = Self::now_string();
        let row = sqlx::query(
            r#"
            UPDATE trader_memories
            SET last_used_at = $3, updated_at = $3
            WHERE trader_id = $1 AND id = $2
            RETURNING id, trader_id, memory_type, topic, summary, source_channel_id,
                      source_message_id, confidence, importance, status, last_used_at,
                      created_at, updated_at
            "#,
        )
        .bind(trader_id)
        .bind(memory_id)
        .bind(now)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(Self::row_to_trader_memory))
    }

    pub async fn create_trader_memory_link(
        &self,
        trader_id: &str,
        memory_id: &str,
        channel_message_id: &str,
        link_type: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO trader_memory_links (id, trader_id, memory_id, channel_message_id, link_type, created_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (trader_id, channel_message_id, link_type) DO NOTHING
            "#,
        )
        .bind(Self::new_id())
        .bind(trader_id)
        .bind(memory_id)
        .bind(channel_message_id)
        .bind(link_type)
        .bind(Self::now_string())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn trader_memory_link_exists(
        &self,
        trader_id: &str,
        channel_message_id: &str,
        link_type: &str,
    ) -> Result<bool, sqlx::Error> {
        let row = sqlx::query(
            r#"
            SELECT 1
            FROM trader_memory_links
            WHERE trader_id = $1 AND channel_message_id = $2 AND link_type = $3
            LIMIT 1
            "#,
        )
        .bind(trader_id)
        .bind(channel_message_id)
        .bind(link_type)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.is_some())
    }

    pub async fn get_trader_persona(
        &self,
        trader_id: &str,
    ) -> Result<Option<TraderPersona>, sqlx::Error> {
        let row = sqlx::query(
            r#"
            SELECT id::text AS trader_id, persona, tone, communication_style
            FROM traders
            WHERE id = $1::uuid AND is_active = TRUE
            "#,
        )
        .bind(trader_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|row| TraderPersona {
            trader_id: row.get("trader_id"),
            persona: row.get("persona"),
            tone: row.get("tone"),
            communication_style: row.get("communication_style"),
        }))
    }

    pub async fn update_trader_persona(
        &self,
        trader_id: &str,
        request: &TraderPersonaUpdateRequest,
    ) -> Result<Option<TraderPersona>, sqlx::Error> {
        let now = Self::now_string();
        let row = sqlx::query(
            r#"
            UPDATE traders
            SET persona = COALESCE($2, persona),
                tone = COALESCE($3, tone),
                communication_style = COALESCE($4, communication_style),
                updated_at = $5::timestamptz
            WHERE id = $1::uuid AND is_active = TRUE
            RETURNING id::text AS trader_id, persona, tone, communication_style
            "#,
        )
        .bind(trader_id)
        .bind(request.persona.as_deref())
        .bind(request.tone.as_deref())
        .bind(request.communication_style.as_deref())
        .bind(&now)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|row| TraderPersona {
            trader_id: row.get("trader_id"),
            persona: row.get("persona"),
            tone: row.get("tone"),
            communication_style: row.get("communication_style"),
        }))
    }

    pub async fn get_md_profile(&self) -> Result<MdProfile, sqlx::Error> {
        self.seed_md_profile().await?;
        let row = sqlx::query(
            r#"
            SELECT id, name, persona, tone, communication_style, created_at, updated_at
            FROM md_profile
            WHERE id = 'default'
            "#,
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(Self::row_to_md_profile(row))
    }

    pub async fn update_md_profile(
        &self,
        request: &UpdateMdProfileRequest,
    ) -> Result<MdProfile, sqlx::Error> {
        let now = Self::now_string();
        let row = sqlx::query(
            r#"
            UPDATE md_profile
            SET name = COALESCE($1, name),
                persona = COALESCE($2, persona),
                tone = COALESCE($3, tone),
                communication_style = COALESCE($4, communication_style),
                openai_api_key_secret = CASE WHEN $5 THEN $6 ELSE openai_api_key_secret END,
                updated_at = $7
            WHERE id = 'default'
            RETURNING id, name, persona, tone, communication_style, created_at, updated_at
            "#,
        )
        .bind(request.name.as_deref())
        .bind(request.persona.as_deref())
        .bind(request.tone.as_deref())
        .bind(request.communication_style.as_deref())
        .bind(
            request
                .openai_api_key
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .is_some(),
        )
        .bind(
            request
                .openai_api_key
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty()),
        )
        .bind(&now)
        .fetch_one(&self.pool)
        .await?;
        Ok(Self::row_to_md_profile(row))
    }

    pub async fn get_data_scientist_profile(&self) -> Result<DataScientistProfile, sqlx::Error> {
        self.seed_data_scientist_profile().await?;
        let row = sqlx::query(
            r#"
            SELECT id, name, persona, tone, communication_style, created_at, updated_at
            FROM data_scientist_profile
            WHERE id = 'default'
            "#,
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(Self::row_to_data_scientist_profile(row))
    }

    pub async fn update_data_scientist_profile(
        &self,
        request: &UpdateDataScientistProfileRequest,
    ) -> Result<DataScientistProfile, sqlx::Error> {
        let now = Self::now_string();
        let row = sqlx::query(
            r#"
            UPDATE data_scientist_profile
            SET name = COALESCE($1, name),
                persona = COALESCE($2, persona),
                tone = COALESCE($3, tone),
                communication_style = COALESCE($4, communication_style),
                openai_api_key_secret = CASE WHEN $5 THEN $6 ELSE openai_api_key_secret END,
                updated_at = $7
            WHERE id = 'default'
            RETURNING id, name, persona, tone, communication_style, created_at, updated_at
            "#,
        )
        .bind(request.name.as_deref())
        .bind(request.persona.as_deref())
        .bind(request.tone.as_deref())
        .bind(request.communication_style.as_deref())
        .bind(
            request
                .openai_api_key
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .is_some(),
        )
        .bind(
            request
                .openai_api_key
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty()),
        )
        .bind(&now)
        .fetch_one(&self.pool)
        .await?;
        Ok(Self::row_to_data_scientist_profile(row))
    }

    pub async fn get_data_scientist_openai_api_key(&self) -> Result<Option<String>, sqlx::Error> {
        self.seed_data_scientist_profile().await?;
        let row = sqlx::query(
            r#"
            SELECT openai_api_key_secret
            FROM data_scientist_profile
            WHERE id = 'default'
            "#,
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(row.get("openai_api_key_secret"))
    }

    pub async fn get_user_investor_profile(&self) -> Result<UserInvestorProfile, sqlx::Error> {
        self.seed_user_investor_profile().await?;
        let row = sqlx::query(
            r#"
            SELECT id, name, age, about, investment_goals, risk_tolerance, time_horizon,
                   liquidity_needs, income_needs, investment_experience, restrictions,
                   preferred_sectors, avoided_sectors, notes, created_at, updated_at
            FROM user_investor_profile
            WHERE id = 'default'
            "#,
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(Self::row_to_user_investor_profile(row))
    }

    pub async fn get_md_openai_api_key(&self) -> Result<Option<String>, sqlx::Error> {
        self.seed_md_profile().await?;
        let row = sqlx::query(
            r#"
            SELECT openai_api_key_secret
            FROM md_profile
            WHERE id = 'default'
            "#,
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(row.get("openai_api_key_secret"))
    }

    pub async fn update_user_investor_profile(
        &self,
        request: &UpdateUserInvestorProfileRequest,
    ) -> Result<UserInvestorProfile, sqlx::Error> {
        let now = Self::now_string();
        let row = sqlx::query(
            r#"
            UPDATE user_investor_profile
            SET name = $1,
                age = $2,
                about = $3,
                investment_goals = $4,
                risk_tolerance = $5,
                time_horizon = $6,
                liquidity_needs = $7,
                income_needs = $8,
                investment_experience = $9,
                restrictions = $10,
                preferred_sectors = $11,
                avoided_sectors = $12,
                notes = $13,
                updated_at = $14
            WHERE id = 'default'
            RETURNING id, name, age, about, investment_goals, risk_tolerance, time_horizon,
                      liquidity_needs, income_needs, investment_experience, restrictions,
                      preferred_sectors, avoided_sectors, notes, created_at, updated_at
            "#,
        )
        .bind(request.name.as_deref())
        .bind(request.age.map(|age| age as i32))
        .bind(request.about.as_deref())
        .bind(request.investment_goals.as_deref())
        .bind(request.risk_tolerance.as_deref())
        .bind(request.time_horizon.as_deref())
        .bind(request.liquidity_needs.as_deref())
        .bind(request.income_needs.as_deref())
        .bind(request.investment_experience.as_deref())
        .bind(request.restrictions.as_deref())
        .bind(request.preferred_sectors.as_deref())
        .bind(request.avoided_sectors.as_deref())
        .bind(request.notes.as_deref())
        .bind(&now)
        .fetch_one(&self.pool)
        .await?;
        Ok(Self::row_to_user_investor_profile(row))
    }

    pub async fn engine_channel_context(&self) -> Result<EngineChannelContext, sqlx::Error> {
        let channels = self.list_channels().await?;
        let mut recent_messages = Vec::new();
        for channel in &channels {
            recent_messages.extend(
                self.list_channel_messages(&channel.id, Some(50), None, None)
                    .await?,
            );
        }
        let rows = sqlx::query(
            r#"
            SELECT id::text AS trader_id, persona, tone, communication_style
            FROM traders
            WHERE is_active = TRUE
            ORDER BY name ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;
        let memory_rows = sqlx::query(
            r#"
            SELECT id, trader_id, memory_type, topic, summary, source_channel_id,
                   source_message_id, confidence, importance, status, last_used_at,
                   created_at, updated_at
            FROM trader_memories
            WHERE status = 'active'
            ORDER BY importance DESC, COALESCE(last_used_at, created_at) DESC
            LIMIT 200
            "#,
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(EngineChannelContext {
            channels,
            recent_messages,
            md_profile: self.get_md_profile().await?,
            md_openai_api_key: self.get_md_openai_api_key().await?,
            user_investor_profile: self.get_user_investor_profile().await?,
            trader_personas: rows
                .into_iter()
                .map(|row| TraderPersona {
                    trader_id: row.get("trader_id"),
                    persona: row.get("persona"),
                    tone: row.get("tone"),
                    communication_style: row.get("communication_style"),
                })
                .collect(),
            trader_memories: memory_rows
                .into_iter()
                .map(Self::row_to_trader_memory)
                .collect(),
        })
    }

    pub async fn list_recent_engine_events(
        &self,
        limit: i64,
    ) -> Result<Vec<EngineEvent>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT id, engine_name, event_type, symbol, message, timestamp, created_at
            FROM engine_events
            ORDER BY created_at DESC, id DESC
            LIMIT $1
            "#,
        )
        .bind(limit.clamp(1, 100))
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(Self::row_to_engine_event).collect())
    }

    pub async fn create_data_source(
        &self,
        request: &CreateDataSourceRequest,
    ) -> Result<DataSource, sqlx::Error> {
        let now = Self::now_string();
        let row = sqlx::query(
            r#"
            INSERT INTO scrapper.data_sources (
                id, name, source_type, url, config_json, enabled, poll_interval_seconds, created_at, updated_at
            ) VALUES ($1::uuid, $2, $3, $4, $5::jsonb, $6, $7, $8::timestamptz, $9::timestamptz)
            RETURNING id::text AS id, name, source_type, url, config_json, enabled,
                      poll_interval_seconds, last_checked_at::text AS last_checked_at,
                      last_success_at::text AS last_success_at, last_error,
                      created_at::text AS created_at, updated_at::text AS updated_at
            "#,
        )
        .bind(Self::new_id())
        .bind(request.name.trim())
        .bind(request.source_type.trim())
        .bind(request.url.as_deref().map(str::trim).filter(|value| !value.is_empty()))
        .bind(&request.config_json)
        .bind(request.enabled)
        .bind(request.poll_interval_seconds.unwrap_or(30) as i32)
        .bind(&now)
        .bind(&now)
        .fetch_one(&self.pool)
        .await?;
        Ok(Self::row_to_data_source(row))
    }

    pub async fn list_data_sources(&self) -> Result<Vec<DataSource>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT id::text AS id, name, source_type, url, config_json, enabled,
                   poll_interval_seconds, last_checked_at::text AS last_checked_at,
                   last_success_at::text AS last_success_at, last_error,
                   created_at::text AS created_at, updated_at::text AS updated_at
            FROM scrapper.data_sources
            ORDER BY updated_at DESC, name ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(Self::row_to_data_source).collect())
    }

    pub async fn get_data_source(
        &self,
        source_id: &str,
    ) -> Result<Option<DataSource>, sqlx::Error> {
        let row = sqlx::query(
            r#"
            SELECT id::text AS id, name, source_type, url, config_json, enabled,
                   poll_interval_seconds, last_checked_at::text AS last_checked_at,
                   last_success_at::text AS last_success_at, last_error,
                   created_at::text AS created_at, updated_at::text AS updated_at
            FROM scrapper.data_sources
            WHERE id = $1::uuid
            "#,
        )
        .bind(source_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(Self::row_to_data_source))
    }

    pub async fn update_data_source(
        &self,
        source_id: &str,
        request: &UpdateDataSourceRequest,
    ) -> Result<Option<DataSource>, sqlx::Error> {
        let now = Self::now_string();
        let row = sqlx::query(
            r#"
            UPDATE scrapper.data_sources
            SET name = COALESCE($2, name),
                source_type = COALESCE($3, source_type),
                url = CASE WHEN $4 THEN $5 ELSE url END,
                config_json = CASE WHEN $6 THEN $7::jsonb ELSE config_json END,
                enabled = COALESCE($8, enabled),
                poll_interval_seconds = COALESCE($9, poll_interval_seconds),
                updated_at = $10::timestamptz
            WHERE id = $1::uuid
            RETURNING id::text AS id, name, source_type, url, config_json, enabled,
                      poll_interval_seconds, last_checked_at::text AS last_checked_at,
                      last_success_at::text AS last_success_at, last_error,
                      created_at::text AS created_at, updated_at::text AS updated_at
            "#,
        )
        .bind(source_id)
        .bind(request.name.as_deref().map(str::trim))
        .bind(request.source_type.as_deref().map(str::trim))
        .bind(request.url.is_some())
        .bind(request.url.as_deref().map(str::trim))
        .bind(request.config_json.is_some())
        .bind(&request.config_json)
        .bind(request.enabled)
        .bind(request.poll_interval_seconds.map(|value| value as i32))
        .bind(&now)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(Self::row_to_data_source))
    }

    pub async fn disable_data_source(&self, source_id: &str) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            "UPDATE scrapper.data_sources SET enabled = FALSE, updated_at = $2::timestamptz WHERE id = $1::uuid",
        )
        .bind(source_id)
        .bind(Self::now_string())
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn get_data_source_script(
        &self,
        source_id: &str,
    ) -> Result<Option<DataSourceScript>, sqlx::Error> {
        let row = sqlx::query(
            r#"
            SELECT data_source_id::text AS data_source_id, language, script_text, script_hash,
                   last_build_status, last_build_output, last_built_at::text AS last_built_at,
                   created_at::text AS created_at, updated_at::text AS updated_at
            FROM scrapper.data_source_scripts
            WHERE data_source_id = $1::uuid
            "#,
        )
        .bind(source_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(Self::row_to_data_source_script))
    }

    pub async fn upsert_data_source_script(
        &self,
        source_id: &str,
        script_text: &str,
        script_hash: &str,
    ) -> Result<DataSourceScript, sqlx::Error> {
        let now = Self::now_string();
        let row = sqlx::query(
            r#"
            INSERT INTO scrapper.data_source_scripts (
                data_source_id, language, script_text, script_hash, last_build_status, created_at, updated_at
            ) VALUES ($1::uuid, 'python', $2, $3, 'not_built', $4::timestamptz, $5::timestamptz)
            ON CONFLICT (data_source_id) DO UPDATE
            SET script_text = EXCLUDED.script_text,
                script_hash = EXCLUDED.script_hash,
                updated_at = EXCLUDED.updated_at
            RETURNING data_source_id::text AS data_source_id, language, script_text, script_hash,
                      last_build_status, last_build_output, last_built_at::text AS last_built_at,
                      created_at::text AS created_at, updated_at::text AS updated_at
            "#,
        )
        .bind(source_id)
        .bind(script_text)
        .bind(script_hash)
        .bind(&now)
        .bind(&now)
        .fetch_one(&self.pool)
        .await?;
        Ok(Self::row_to_data_source_script(row))
    }

    pub async fn update_data_source_script_build(
        &self,
        source_id: &str,
        status: &str,
        output: &str,
        script_hash: Option<&str>,
    ) -> Result<Option<DataSourceScript>, sqlx::Error> {
        let now = Self::now_string();
        let row = sqlx::query(
            r#"
            UPDATE scrapper.data_source_scripts
            SET last_build_status = $2,
                last_build_output = $3,
                last_built_at = $4::timestamptz,
                script_hash = COALESCE($5, script_hash),
                updated_at = $4::timestamptz
            WHERE data_source_id = $1::uuid
            RETURNING data_source_id::text AS data_source_id, language, script_text, script_hash,
                      last_build_status, last_build_output, last_built_at::text AS last_built_at,
                      created_at::text AS created_at, updated_at::text AS updated_at
            "#,
        )
        .bind(source_id)
        .bind(status)
        .bind(output)
        .bind(&now)
        .bind(script_hash)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(Self::row_to_data_source_script))
    }

    pub async fn list_data_source_items(
        &self,
        source_id: &str,
        limit: i64,
    ) -> Result<Vec<DataSourceItem>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT id::text AS id, data_source_id::text AS data_source_id, external_id,
                   title, url, content, summary, raw_payload, published_at::text AS published_at,
                   discovered_at::text AS discovered_at, created_at::text AS created_at
            FROM scrapper.data_source_items
            WHERE data_source_id = $1::uuid
            ORDER BY discovered_at DESC, id DESC
            LIMIT $2
            "#,
        )
        .bind(source_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(Self::row_to_data_source_item)
            .collect())
    }

    pub async fn list_data_source_events(
        &self,
        source_id: &str,
        limit: i64,
    ) -> Result<Vec<DataSourceEvent>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT id::text AS id, data_source_id::text AS data_source_id, event_type,
                   message, payload, created_at::text AS created_at
            FROM scrapper.data_source_events
            WHERE data_source_id = $1::uuid
            ORDER BY created_at DESC, id DESC
            LIMIT $2
            "#,
        )
        .bind(source_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(Self::row_to_data_source_event)
            .collect())
    }

    pub async fn list_trader_data_sources(
        &self,
        trader_id: &str,
    ) -> Result<Vec<DataSource>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT ds.id::text AS id, ds.name, ds.source_type, ds.url, ds.config_json, ds.enabled,
                   ds.poll_interval_seconds, ds.last_checked_at::text AS last_checked_at,
                   ds.last_success_at::text AS last_success_at, ds.last_error,
                   ds.created_at::text AS created_at, ds.updated_at::text AS updated_at
            FROM scrapper.trader_data_sources tds
            INNER JOIN scrapper.data_sources ds ON ds.id = tds.data_source_id
            WHERE tds.trader_id = $1::uuid AND tds.enabled = TRUE
            ORDER BY ds.name ASC
            "#,
        )
        .bind(trader_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(Self::row_to_data_source).collect())
    }

    pub async fn replace_trader_data_sources(
        &self,
        trader_id: &str,
        source_ids: &[String],
    ) -> Result<Vec<TraderDataSourceAssignment>, sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("DELETE FROM scrapper.trader_data_sources WHERE trader_id = $1::uuid")
            .bind(trader_id)
            .execute(&mut *tx)
            .await?;
        let mut assignments = Vec::new();
        for source_id in source_ids {
            let now = Self::now_string();
            sqlx::query(
                r#"
                INSERT INTO scrapper.trader_data_sources (trader_id, data_source_id, enabled, created_at, updated_at)
                VALUES ($1::uuid, $2::uuid, TRUE, $3::timestamptz, $4::timestamptz)
                "#,
            )
            .bind(trader_id)
            .bind(source_id)
            .bind(&now)
            .bind(&now)
            .execute(&mut *tx)
            .await?;
            assignments.push(TraderDataSourceAssignment {
                trader_id: trader_id.to_string(),
                data_source_id: source_id.clone(),
                enabled: true,
                created_at: now.clone(),
                updated_at: now,
            });
        }
        tx.commit().await?;
        Ok(assignments)
    }

    async fn list_engine_trader_data_sources(
        &self,
        trader_id: &str,
    ) -> Result<Vec<EngineTraderDataSource>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT ds.id::text AS id, ds.name, ds.source_type
            FROM scrapper.trader_data_sources tds
            INNER JOIN scrapper.data_sources ds ON ds.id = tds.data_source_id
            WHERE tds.trader_id = $1::uuid AND tds.enabled = TRUE AND ds.enabled = TRUE
            ORDER BY ds.name ASC
            "#,
        )
        .bind(trader_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|row| EngineTraderDataSource {
                id: row.get("id"),
                name: row.get("name"),
                source_type: row.get("source_type"),
            })
            .collect())
    }

    async fn upsert_trader_secret_in_tx(
        tx: &mut Transaction<'_, Postgres>,
        trader_id: &str,
        openai_api_key_secret: &str,
    ) -> Result<(), sqlx::Error> {
        // TODO(security): encrypt this value or move it to a Key Vault/secret manager.
        let now = Self::now_string();
        sqlx::query(
            r#"
            INSERT INTO trader_secrets (trader_id, openai_api_key_secret, created_at, updated_at)
            VALUES ($1::uuid, $2, $3::timestamptz, $4::timestamptz)
            ON CONFLICT (trader_id) DO UPDATE SET
                openai_api_key_secret = EXCLUDED.openai_api_key_secret,
                updated_at = EXCLUDED.updated_at
            "#,
        )
        .bind(trader_id)
        .bind(openai_api_key_secret)
        .bind(&now)
        .bind(&now)
        .execute(&mut **tx)
        .await?;

        Ok(())
    }

    async fn replace_trader_info_sources_in_tx(
        tx: &mut Transaction<'_, Postgres>,
        trader_id: &str,
        info_sources: &[CreateTraderInfoSourceRequest],
    ) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM trader_info_sources WHERE trader_id = $1::uuid")
            .bind(trader_id)
            .execute(&mut **tx)
            .await?;

        for source in info_sources {
            let now = Self::now_string();
            sqlx::query(
                r#"
                INSERT INTO trader_info_sources (
                    id, trader_id, source_type, name, config_json, enabled, created_at, updated_at
                ) VALUES ($1::uuid, $2::uuid, $3, $4, $5::jsonb, $6, $7::timestamptz, $8::timestamptz)
                "#,
            )
            .bind(Self::new_id())
            .bind(trader_id)
            .bind(source.source_type.trim())
            .bind(source.name.trim())
            .bind(&source.config_json)
            .bind(source.enabled.unwrap_or(true))
            .bind(&now)
            .bind(&now)
            .execute(&mut **tx)
            .await?;
        }

        Ok(())
    }

    async fn insert_trader_event_in_tx(
        tx: &mut Transaction<'_, Postgres>,
        trader_id: &str,
        event_type: &str,
        message: &str,
        payload: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO trader_events (id, trader_id, event_type, message, payload, created_at)
            VALUES ($1::uuid, $2::uuid, $3, $4, $5::jsonb, $6::timestamptz)
            "#,
        )
        .bind(Self::new_id())
        .bind(trader_id)
        .bind(event_type)
        .bind(message)
        .bind(payload)
        .bind(Self::now_string())
        .execute(&mut **tx)
        .await?;

        Ok(())
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
                trader_id::text AS trader_id,
                strategy_id,
                signal_id::text AS signal_id,
                proposal_id::text AS proposal_id,
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
