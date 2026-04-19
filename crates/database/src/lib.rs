use models::{
    portfolio::{Portfolio, Position},
    projects::Project,
};
use sqlx::{Row, Sqlite, SqlitePool, Transaction, sqlite::SqlitePoolOptions};

pub struct Database {
    pool: SqlitePool,
}

impl Database {
    pub async fn connect(database_url: &str) -> Result<Self, sqlx::Error> {
        let pool = SqlitePoolOptions::new()
            .max_connections(10)
            .connect(database_url)
            .await?;

        sqlx::query("PRAGMA foreign_keys = ON;")
            .execute(&pool)
            .await?;

        let db = Self { pool };
        db.init().await?;
        Ok(db)
    }

    pub fn pool(&self) -> &SqlitePool {
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
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                interval TEXT NOT NULL,
                range TEXT NOT NULL,
                prepost INTEGER NOT NULL CHECK (prepost IN (0, 1))
            );
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            ALTER TABLE projects
            ADD COLUMN strategy TEXT NOT NULL DEFAULT ''
            "#,
        )
        .execute(&self.pool)
        .await
        .ok();

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS project_symbols (
                project_id TEXT NOT NULL,
                symbol TEXT NOT NULL,
                ordinal INTEGER NOT NULL,
                PRIMARY KEY (project_id, ordinal),
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
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                portfolio_id TEXT NOT NULL,
                symbol TEXT NOT NULL,
                quantity REAL NOT NULL,
                average_price REAL NOT NULL,
                position_opened_at TEXT NOT NULL,
                position_closed_at TEXT NULL,
                position_closed_price REAL NULL,
                ordinal INTEGER NOT NULL,
                FOREIGN KEY (portfolio_id) REFERENCES portfolios(id) ON DELETE CASCADE,
                UNIQUE (portfolio_id, symbol, position_opened_at)
            );
            "#,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    // ----------------------------
    // Project CRUD
    // ----------------------------

    pub async fn create_project(&self, project: &Project) -> Result<(), sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        sqlx::query(
            r#"
            INSERT INTO projects (
                id, name, description, strategy, created_at, updated_at, interval, range, prepost
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&project.id)
        .bind(&project.name)
        .bind(&project.description)
        .bind(&project.strategy)
        .bind(&project.created_at)
        .bind(&project.updated_at)
        .bind(&project.interval)
        .bind(&project.range)
        .bind(project.prepost as i64)
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
            SELECT id, name, description, strategy, created_at, updated_at, interval, range, prepost
            FROM projects
            WHERE id = ?
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
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            symbols,
            interval: row.get("interval"),
            range: row.get("range"),
            prepost: row.get::<i64, _>("prepost") != 0,
        }))
    }

    pub async fn list_projects(&self) -> Result<Vec<Project>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT id, name, description, strategy, created_at, updated_at, interval, range, prepost
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
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
                symbols,
                interval: row.get("interval"),
                range: row.get("range"),
                prepost: row.get::<i64, _>("prepost") != 0,
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
                name = ?,
                description = ?,
                strategy = ?,
                updated_at = ?,
                interval = ?,
                range = ?,
                prepost = ?
            WHERE id = ?
            "#,
        )
        .bind(&project.name)
        .bind(&project.description)
        .bind(&project.strategy)
        .bind(&project.updated_at)
        .bind(&project.interval)
        .bind(&project.range)
        .bind(project.prepost as i64)
        .bind(&project.id)
        .execute(&mut *tx)
        .await?;

        if result.rows_affected() == 0 {
            tx.rollback().await?;
            return Ok(false);
        }

        sqlx::query("DELETE FROM project_symbols WHERE project_id = ?")
            .bind(&project.id)
            .execute(&mut *tx)
            .await?;

        self.insert_project_symbols(&mut tx, &project.id, &project.symbols)
            .await?;

        tx.commit().await?;
        Ok(true)
    }

    pub async fn delete_project(&self, id: &str) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("DELETE FROM projects WHERE id = ?")
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
            WHERE project_id = ?
            "#,
        )
        .bind(project_id)
        .fetch_one(&self.pool)
        .await?;

        sqlx::query(
            r#"
            INSERT INTO project_symbols (project_id, symbol, ordinal)
            VALUES (?, ?, ?)
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
            WHERE project_id = ? AND symbol = ?
            "#,
        )
        .bind(project_id)
        .bind(symbol)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    async fn insert_project_symbols(
        &self,
        tx: &mut Transaction<'_, Sqlite>,
        project_id: &str,
        symbols: &[String],
    ) -> Result<(), sqlx::Error> {
        for (ordinal, symbol) in symbols.iter().enumerate() {
            sqlx::query(
                r#"
                INSERT INTO project_symbols (project_id, symbol, ordinal)
                VALUES (?, ?, ?)
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
            WHERE project_id = ?
            ORDER BY ordinal ASC
            "#,
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|row| row.get("symbol")).collect())
    }

    // ----------------------------
    // Portfolio CRUD
    // ----------------------------

    pub async fn create_portfolio(&self, portfolio: &Portfolio) -> Result<(), sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        sqlx::query(
            r#"
            INSERT INTO portfolios (
                id, name, description, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?)
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
            WHERE id = ?
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
                name = ?,
                description = ?,
                updated_at = ?
            WHERE id = ?
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

        sqlx::query("DELETE FROM positions WHERE portfolio_id = ?")
            .bind(&portfolio.id)
            .execute(&mut *tx)
            .await?;

        self.insert_positions(&mut tx, &portfolio.id, &portfolio.positions)
            .await?;

        tx.commit().await?;
        Ok(true)
    }

    pub async fn delete_portfolio(&self, id: &str) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("DELETE FROM portfolios WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    async fn insert_positions(
        &self,
        tx: &mut Transaction<'_, Sqlite>,
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
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
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
            WHERE portfolio_id = ?
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

    // ----------------------------
    // Position CRUD
    // ----------------------------
    //
    // Since Position has no ID field in your model, this uses:
    // (portfolio_id, symbol, position_opened_at)
    // as the logical identifier for update/delete.
    //

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
            WHERE portfolio_id = ?
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
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
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
                symbol = ?,
                quantity = ?,
                average_price = ?,
                position_opened_at = ?,
                position_closed_at = ?,
                position_closed_price = ?
            WHERE
                portfolio_id = ?
                AND symbol = ?
                AND position_opened_at = ?
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
                portfolio_id = ?
                AND symbol = ?
                AND position_opened_at = ?
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
