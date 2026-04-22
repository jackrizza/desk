use models::raw::{RawStockData, StockIndicatorsResponse};
use models::{
    portfolio::{Portfolio, Position},
    projects::Project,
};
use poem::{Route, listener::TcpListener};
use poem_openapi::{
    OpenApi, OpenApiService,
    param::Path,
    param::Query,
    payload::{Json, PlainText},
};
use std::{env, sync::Arc};

use cache::Cache;
use database::Database;
use stock_data::calculate_indicators;

use env_logger;
use log::{error, info};

mod helpers;

use helpers::{
    ApiTags, CreatePortfolioResponse, CreatePositionResponse, CreateProjectResponse,
    DeletePortfolioResponse, DeletePositionResponse, DeleteProjectResponse, GetPortfolioResponse,
    GetPositionResponse, GetProjectResponse, ListPortfoliosResponse, ListPositionsResponse,
    ListProjectsResponse, UpdatePortfolioResponse, UpdatePositionResponse, UpdateProjectResponse,
    internal_error,
};

struct Api {
    cache: Arc<Cache>,
    database: Arc<Database>,
}

#[OpenApi]
impl Api {
    fn stock_cache_key(symbol: &str, range: &str, interval: &str, prepost: bool) -> String {
        format!("{symbol}_{range}_{interval}_{prepost}")
    }

    #[oai(path = "/hello", method = "get")]
    async fn index(&self, name: Query<Option<String>>) -> PlainText<String> {
        match name.0 {
            Some(name) => PlainText(format!("hello, {}!", name)),
            None => PlainText("hello!".to_string()),
        }
    }

    #[oai(path = "/health", method = "get")]
    async fn health(&self) -> PlainText<&'static str> {
        PlainText("ok")
    }

    #[oai(path = "/stock_data", method = "get")]
    async fn stock_data(
        &self,
        symbol: Query<String>,
        range: Query<String>,
        interval: Query<String>,
        prepost: Query<bool>,
    ) -> Json<RawStockData> {
        let key = Self::stock_cache_key(&symbol.0, &range.0, &interval.0, prepost.0);
        let cache = self.cache.clone();
        let stock_data = cache.check_cache(&key).await;

        match stock_data {
            Ok(data) => {
                let data = data.as_ref();
                Json(data.clone())
            }
            Err(e) => {
                error!("Error fetching stock data: {}", e);
                Json(RawStockData::default())
            }
        }
    }

    #[oai(path = "/indicators", method = "get")]
    async fn indicators(
        &self,
        symbol: Query<String>,
        range: Query<String>,
        interval: Query<String>,
        prepost: Query<bool>,
        indicators: Query<Option<String>>,
    ) -> Json<StockIndicatorsResponse> {
        let requested = indicators
            .0
            .unwrap_or_default()
            .split(',')
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
            .collect::<Vec<String>>();

        let key = Self::stock_cache_key(&symbol.0, &range.0, &interval.0, prepost.0);
        let cache = self.cache.clone();
        let stock_data = cache.check_cache(&key).await;

        match stock_data {
            Ok(data) => Json(calculate_indicators(data.as_ref(), &requested)),
            Err(err) => {
                error!("Error fetching stock indicators: {}", err);
                Json(StockIndicatorsResponse {
                    symbol: symbol.0,
                    last_refreshed: String::new(),
                    interval: interval.0,
                    range: range.0,
                    indicators: Vec::new(),
                    unsupported: requested,
                })
            }
        }
    }

    // ----------------------------
    // Projects
    // ----------------------------

    #[oai(path = "/projects", method = "post", tag = "ApiTags::Project")]
    async fn create_project(&self, project: Json<Project>) -> CreateProjectResponse {
        let project = project.0;

        match self.database.create_project(&project).await {
            Ok(_) => CreateProjectResponse::Created(Json(project)),
            Err(err) => CreateProjectResponse::InternalError(internal_error(err)),
        }
    }

    #[oai(path = "/projects", method = "get", tag = "ApiTags::Project")]
    async fn list_projects(&self) -> ListProjectsResponse {
        match self.database.list_projects().await {
            Ok(projects) => ListProjectsResponse::Ok(Json(projects)),
            Err(err) => ListProjectsResponse::InternalError(internal_error(err)),
        }
    }

    #[oai(
        path = "/projects/:project_id",
        method = "get",
        tag = "ApiTags::Project"
    )]
    async fn get_project(&self, project_id: Path<String>) -> GetProjectResponse {
        match self.database.get_project(&project_id.0).await {
            Ok(Some(project)) => GetProjectResponse::Ok(Json(project)),
            Ok(None) => GetProjectResponse::NotFound,
            Err(err) => GetProjectResponse::InternalError(internal_error(err)),
        }
    }

    #[oai(
        path = "/projects/:project_id",
        method = "put",
        tag = "ApiTags::Project"
    )]
    async fn update_project(
        &self,
        project_id: Path<String>,
        project: Json<Project>,
    ) -> UpdateProjectResponse {
        let mut project = project.0;
        project.id = project_id.0;

        match self.database.update_project(&project).await {
            Ok(true) => UpdateProjectResponse::Ok(Json(project)),
            Ok(false) => UpdateProjectResponse::NotFound,
            Err(err) => UpdateProjectResponse::InternalError(internal_error(err)),
        }
    }

    #[oai(
        path = "/projects/:project_id",
        method = "delete",
        tag = "ApiTags::Project"
    )]
    async fn delete_project(&self, project_id: Path<String>) -> DeleteProjectResponse {
        match self.database.delete_project(&project_id.0).await {
            Ok(true) => DeleteProjectResponse::Ok,
            Ok(false) => DeleteProjectResponse::NotFound,
            Err(err) => DeleteProjectResponse::InternalError(internal_error(err)),
        }
    }

    // ----------------------------
    // Portfolios
    // ----------------------------

    #[oai(path = "/portfolios", method = "post", tag = "ApiTags::Portfolio")]
    async fn create_portfolio(&self, portfolio: Json<Portfolio>) -> CreatePortfolioResponse {
        let portfolio = portfolio.0;

        match self.database.create_portfolio(&portfolio).await {
            Ok(_) => CreatePortfolioResponse::Created(Json(portfolio)),
            Err(err) => CreatePortfolioResponse::InternalError(internal_error(err)),
        }
    }

    #[oai(path = "/portfolios", method = "get", tag = "ApiTags::Portfolio")]
    async fn list_portfolios(&self) -> ListPortfoliosResponse {
        match self.database.list_portfolios().await {
            Ok(portfolios) => ListPortfoliosResponse::Ok(Json(portfolios)),
            Err(err) => ListPortfoliosResponse::InternalError(internal_error(err)),
        }
    }

    #[oai(
        path = "/portfolios/:portfolio_id",
        method = "get",
        tag = "ApiTags::Portfolio"
    )]
    async fn get_portfolio(&self, portfolio_id: Path<String>) -> GetPortfolioResponse {
        match self.database.get_portfolio(&portfolio_id.0).await {
            Ok(Some(portfolio)) => GetPortfolioResponse::Ok(Json(portfolio)),
            Ok(None) => GetPortfolioResponse::NotFound,
            Err(err) => GetPortfolioResponse::InternalError(internal_error(err)),
        }
    }

    #[oai(
        path = "/portfolios/:portfolio_id",
        method = "put",
        tag = "ApiTags::Portfolio"
    )]
    async fn update_portfolio(
        &self,
        portfolio_id: Path<String>,
        portfolio: Json<Portfolio>,
    ) -> UpdatePortfolioResponse {
        let mut portfolio = portfolio.0;
        portfolio.id = portfolio_id.0;

        match self.database.update_portfolio(&portfolio).await {
            Ok(true) => UpdatePortfolioResponse::Ok(Json(portfolio)),
            Ok(false) => UpdatePortfolioResponse::NotFound,
            Err(err) => UpdatePortfolioResponse::InternalError(internal_error(err)),
        }
    }

    #[oai(
        path = "/portfolios/:portfolio_id",
        method = "delete",
        tag = "ApiTags::Portfolio"
    )]
    async fn delete_portfolio(&self, portfolio_id: Path<String>) -> DeletePortfolioResponse {
        match self.database.delete_portfolio(&portfolio_id.0).await {
            Ok(true) => DeletePortfolioResponse::Ok,
            Ok(false) => DeletePortfolioResponse::NotFound,
            Err(err) => DeletePortfolioResponse::InternalError(internal_error(err)),
        }
    }

    // ----------------------------
    // Positions
    // ----------------------------

    #[oai(
        path = "/portfolios/:portfolio_id/positions",
        method = "post",
        tag = "ApiTags::Position"
    )]
    async fn create_position(
        &self,
        portfolio_id: Path<String>,
        position: Json<Position>,
    ) -> CreatePositionResponse {
        let position = position.0;

        match self.database.add_position(&portfolio_id.0, &position).await {
            Ok(_) => CreatePositionResponse::Created(Json(position)),
            Err(err) => CreatePositionResponse::InternalError(internal_error(err)),
        }
    }

    #[oai(
        path = "/portfolios/:portfolio_id/positions",
        method = "get",
        tag = "ApiTags::Position"
    )]
    async fn list_positions(&self, portfolio_id: Path<String>) -> ListPositionsResponse {
        match self.database.list_positions(&portfolio_id.0).await {
            Ok(positions) => ListPositionsResponse::Ok(Json(positions)),
            Err(err) => ListPositionsResponse::InternalError(internal_error(err)),
        }
    }

    #[oai(
        path = "/portfolios/:portfolio_id/positions/:symbol/:position_opened_at",
        method = "get",
        tag = "ApiTags::Position"
    )]
    async fn get_position(
        &self,
        portfolio_id: Path<String>,
        symbol: Path<String>,
        position_opened_at: Path<String>,
    ) -> GetPositionResponse {
        match self.database.list_positions(&portfolio_id.0).await {
            Ok(positions) => {
                let found = positions
                    .into_iter()
                    .find(|p| p.symbol == symbol.0 && p.position_opened_at == position_opened_at.0);

                match found {
                    Some(position) => GetPositionResponse::Ok(Json(position)),
                    None => GetPositionResponse::NotFound,
                }
            }
            Err(err) => GetPositionResponse::InternalError(internal_error(err)),
        }
    }

    #[oai(
        path = "/portfolios/:portfolio_id/positions/:symbol/:position_opened_at",
        method = "put",
        tag = "ApiTags::Position"
    )]
    async fn update_position(
        &self,
        portfolio_id: Path<String>,
        symbol: Path<String>,
        position_opened_at: Path<String>,
        position: Json<Position>,
    ) -> UpdatePositionResponse {
        let position = position.0;

        match self
            .database
            .update_position(&portfolio_id.0, &symbol.0, &position_opened_at.0, &position)
            .await
        {
            Ok(true) => UpdatePositionResponse::Ok(Json(position)),
            Ok(false) => UpdatePositionResponse::NotFound,
            Err(err) => UpdatePositionResponse::InternalError(internal_error(err)),
        }
    }

    #[oai(
        path = "/portfolios/:portfolio_id/positions/:symbol/:position_opened_at",
        method = "delete",
        tag = "ApiTags::Position"
    )]
    async fn delete_position(
        &self,
        portfolio_id: Path<String>,
        symbol: Path<String>,
        position_opened_at: Path<String>,
    ) -> DeletePositionResponse {
        match self
            .database
            .delete_position(&portfolio_id.0, &symbol.0, &position_opened_at.0)
            .await
        {
            Ok(true) => DeletePositionResponse::Ok,
            Ok(false) => DeletePositionResponse::NotFound,
            Err(err) => DeletePositionResponse::InternalError(internal_error(err)),
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .init();

    let api_bind_address =
        env::var("API_BIND_ADDRESS").unwrap_or_else(|_| "0.0.0.0:3000".to_string());
    let api_public_base_url =
        env::var("API_PUBLIC_BASE_URL").unwrap_or_else(|_| "http://localhost:3000/api".to_string());
    let cache_dir = env::var("CACHE_DIR").unwrap_or_else(|_| "cache_data".to_string());
    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://desk:desk@localhost:5432/desk".to_string());

    let api_service = OpenApiService::new(
        Api {
            cache: Arc::new(Cache::new(cache_dir)),
            database: Arc::new(Database::connect(&database_url).await.map_err(|err| {
                std::io::Error::other(format!("failed to connect to database: {err}"))
            })?),
        },
        "Desk API",
        "1.0",
    )
    .server(&api_public_base_url);
    let ui = api_service.swagger_ui();
    let app = Route::new().nest("/api", api_service).nest("/", ui);

    info!("Starting server on http://{api_bind_address}");
    poem::Server::new(TcpListener::bind(api_bind_address))
        .run(app)
        .await
}
