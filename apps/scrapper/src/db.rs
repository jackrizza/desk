use anyhow::Result;
use chrono::Utc;
use models::data_sources::DataSource;
use sqlx::{PgPool, Row, postgres::PgPoolOptions};
use uuid::Uuid;

use crate::types::ScrappedItem;

#[derive(Clone)]
pub struct ScrapperDb {
    pool: PgPool,
}

impl ScrapperDb {
    pub async fn connect(database_url: &str) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await?;
        let db = Self { pool };
        db.migrate().await?;
        Ok(db)
    }

    pub async fn migrate(&self) -> Result<()> {
        for statement in [
            "CREATE SCHEMA IF NOT EXISTS scrapper;",
            r#"
            CREATE TABLE IF NOT EXISTS scrapper.data_sources (
                id UUID PRIMARY KEY,
                name TEXT NOT NULL,
                source_type TEXT NOT NULL CHECK (source_type IN ('rss', 'web_page', 'manual_note', 'placeholder_api')),
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

    pub async fn due_sources(&self) -> Result<Vec<DataSource>> {
        let rows = sqlx::query(
            r#"
            SELECT id::text AS id, name, source_type, url, config_json::text AS config_json, enabled,
                   poll_interval_seconds, last_checked_at::text AS last_checked_at,
                   last_success_at::text AS last_success_at, last_error,
                   created_at::text AS created_at, updated_at::text AS updated_at
            FROM scrapper.data_sources
            WHERE enabled = TRUE
              AND (
                last_checked_at IS NULL
                OR last_checked_at <= now() - (poll_interval_seconds || ' seconds')::interval
              )
            ORDER BY COALESCE(last_checked_at, created_at) ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|row| DataSource {
                id: row.get("id"),
                name: row.get("name"),
                source_type: row.get("source_type"),
                url: row.get("url"),
                config_json: row.get("config_json"),
                enabled: row.get("enabled"),
                poll_interval_seconds: row.get::<i32, _>("poll_interval_seconds") as i64,
                last_checked_at: row.get("last_checked_at"),
                last_success_at: row.get("last_success_at"),
                last_error: row.get("last_error"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            })
            .collect())
    }

    pub async fn insert_item(&self, source_id: &str, item: &ScrappedItem) -> Result<bool> {
        let now = Utc::now().to_rfc3339();
        let result = sqlx::query(
            r#"
            INSERT INTO scrapper.data_source_items (
                id, data_source_id, external_id, title, url, content, summary, raw_payload,
                published_at, discovered_at, created_at
            ) VALUES (
                $1::uuid, $2::uuid, $3, $4, $5, $6, $7, $8::jsonb, $9::timestamptz,
                $10::timestamptz, $11::timestamptz
            )
            ON CONFLICT (data_source_id, external_id) DO NOTHING
            "#,
        )
        .bind(Uuid::now_v7().to_string())
        .bind(source_id)
        .bind(&item.external_id)
        .bind(&item.title)
        .bind(&item.url)
        .bind(&item.content)
        .bind(&item.summary)
        .bind(&item.raw_payload)
        .bind(&item.published_at)
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn mark_success(&self, source_id: &str) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            "UPDATE scrapper.data_sources SET last_checked_at = $2::timestamptz, last_success_at = $2::timestamptz, last_error = NULL, updated_at = $2::timestamptz WHERE id = $1::uuid",
        )
        .bind(source_id)
        .bind(now)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn mark_error(&self, source_id: &str, error: &str) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            "UPDATE scrapper.data_sources SET last_checked_at = $2::timestamptz, last_error = $3, updated_at = $2::timestamptz WHERE id = $1::uuid",
        )
        .bind(source_id)
        .bind(now)
        .bind(error)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn event(
        &self,
        source_id: Option<&str>,
        event_type: &str,
        message: &str,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO scrapper.data_source_events (id, data_source_id, event_type, message, created_at) VALUES ($1::uuid, $2::uuid, $3, $4, $5::timestamptz)",
        )
        .bind(Uuid::now_v7().to_string())
        .bind(source_id)
        .bind(event_type)
        .bind(message)
        .bind(Utc::now().to_rfc3339())
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
