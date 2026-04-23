use models::engine::{ActiveSymbolsResponse, EngineHealthResponse};
use models::{
    paper::{
        PaperAccount, PaperAccountEvent, PaperAccountSummaryResponse, PaperFill, PaperOrder,
        PaperOrderExecutionResponse, PaperPosition,
    },
    portfolio::{Portfolio, Position},
    projects::Project,
    trading::{
        EngineStrategyConfigResponse, StrategyRiskConfig, StrategyRuntimeState,
        StrategyRuntimeStateListResponse, StrategySignal, StrategySignalListResponse,
        StrategyTradingConfig,
    },
};
use poem_openapi::{ApiResponse, Object, Tags, payload::Json};

#[derive(Tags)]
pub enum ApiTags {
    Engine,
    Paper,
    Project,
    Portfolio,
    Position,
    Strategy,
}

#[derive(Object)]
pub struct ErrorBody {
    pub message: String,
}

#[derive(ApiResponse)]
pub enum CreateProjectResponse {
    #[oai(status = 201)]
    Created(Json<Project>),
    #[oai(status = 500)]
    InternalError(Json<ErrorBody>),
}

#[derive(ApiResponse)]
pub enum GetProjectResponse {
    #[oai(status = 200)]
    Ok(Json<Project>),
    #[oai(status = 404)]
    NotFound,
    #[oai(status = 500)]
    InternalError(Json<ErrorBody>),
}

#[derive(ApiResponse)]
pub enum ListProjectsResponse {
    #[oai(status = 200)]
    Ok(Json<Vec<Project>>),
    #[oai(status = 500)]
    InternalError(Json<ErrorBody>),
}

#[derive(ApiResponse)]
pub enum UpdateProjectResponse {
    #[oai(status = 200)]
    Ok(Json<Project>),
    #[oai(status = 404)]
    NotFound,
    #[oai(status = 500)]
    InternalError(Json<ErrorBody>),
}

#[derive(ApiResponse)]
pub enum DeleteProjectResponse {
    #[oai(status = 200)]
    Ok,
    #[oai(status = 404)]
    NotFound,
    #[oai(status = 500)]
    InternalError(Json<ErrorBody>),
}

#[derive(ApiResponse)]
pub enum CreatePortfolioResponse {
    #[oai(status = 201)]
    Created(Json<Portfolio>),
    #[oai(status = 500)]
    InternalError(Json<ErrorBody>),
}

#[derive(ApiResponse)]
pub enum GetPortfolioResponse {
    #[oai(status = 200)]
    Ok(Json<Portfolio>),
    #[oai(status = 404)]
    NotFound,
    #[oai(status = 500)]
    InternalError(Json<ErrorBody>),
}

#[derive(ApiResponse)]
pub enum ListPortfoliosResponse {
    #[oai(status = 200)]
    Ok(Json<Vec<Portfolio>>),
    #[oai(status = 500)]
    InternalError(Json<ErrorBody>),
}

#[derive(ApiResponse)]
pub enum UpdatePortfolioResponse {
    #[oai(status = 200)]
    Ok(Json<Portfolio>),
    #[oai(status = 404)]
    NotFound,
    #[oai(status = 500)]
    InternalError(Json<ErrorBody>),
}

#[derive(ApiResponse)]
pub enum DeletePortfolioResponse {
    #[oai(status = 200)]
    Ok,
    #[oai(status = 404)]
    NotFound,
    #[oai(status = 500)]
    InternalError(Json<ErrorBody>),
}

#[derive(ApiResponse)]
pub enum CreatePositionResponse {
    #[oai(status = 201)]
    Created(Json<Position>),
    #[oai(status = 500)]
    InternalError(Json<ErrorBody>),
}

#[derive(ApiResponse)]
pub enum GetPositionResponse {
    #[oai(status = 200)]
    Ok(Json<Position>),
    #[oai(status = 404)]
    NotFound,
    #[oai(status = 500)]
    InternalError(Json<ErrorBody>),
}

#[derive(ApiResponse)]
pub enum ListPositionsResponse {
    #[oai(status = 200)]
    Ok(Json<Vec<Position>>),
    #[oai(status = 500)]
    InternalError(Json<ErrorBody>),
}

#[derive(ApiResponse)]
pub enum UpdatePositionResponse {
    #[oai(status = 200)]
    Ok(Json<Position>),
    #[oai(status = 404)]
    NotFound,
    #[oai(status = 500)]
    InternalError(Json<ErrorBody>),
}

#[derive(ApiResponse)]
pub enum DeletePositionResponse {
    #[oai(status = 200)]
    Ok,
    #[oai(status = 404)]
    NotFound,
    #[oai(status = 500)]
    InternalError(Json<ErrorBody>),
}

#[derive(ApiResponse)]
pub enum ActiveSymbolsConfigResponse {
    #[oai(status = 200)]
    Ok(Json<ActiveSymbolsResponse>),
    #[oai(status = 500)]
    InternalError(Json<ErrorBody>),
}

#[derive(ApiResponse)]
pub enum EngineMutationResponse {
    #[oai(status = 200)]
    Ok(Json<EngineHealthResponse>),
    #[oai(status = 500)]
    InternalError(Json<ErrorBody>),
}

#[derive(ApiResponse)]
pub enum CreateStrategySignalResponse {
    #[oai(status = 201)]
    Created(Json<StrategySignal>),
    #[oai(status = 400)]
    BadRequest(Json<ErrorBody>),
    #[oai(status = 500)]
    InternalError(Json<ErrorBody>),
}

#[derive(ApiResponse)]
pub enum UpdateStrategySignalResponse {
    #[oai(status = 200)]
    Ok,
    #[oai(status = 400)]
    BadRequest(Json<ErrorBody>),
    #[oai(status = 500)]
    InternalError(Json<ErrorBody>),
}

#[derive(ApiResponse)]
pub enum UpsertStrategyRuntimeStateResponse {
    #[oai(status = 200)]
    Ok(Json<StrategyRuntimeState>),
    #[oai(status = 400)]
    BadRequest(Json<ErrorBody>),
    #[oai(status = 500)]
    InternalError(Json<ErrorBody>),
}

#[derive(ApiResponse)]
pub enum CreatePaperAccountResponse {
    #[oai(status = 201)]
    Created(Json<PaperAccount>),
    #[oai(status = 400)]
    BadRequest(Json<ErrorBody>),
    #[oai(status = 500)]
    InternalError(Json<ErrorBody>),
}

#[derive(ApiResponse)]
pub enum ListPaperAccountsResponse {
    #[oai(status = 200)]
    Ok(Json<Vec<PaperAccount>>),
    #[oai(status = 500)]
    InternalError(Json<ErrorBody>),
}

#[derive(ApiResponse)]
pub enum GetPaperAccountResponse {
    #[oai(status = 200)]
    Ok(Json<PaperAccount>),
    #[oai(status = 404)]
    NotFound(Json<ErrorBody>),
    #[oai(status = 500)]
    InternalError(Json<ErrorBody>),
}

#[derive(ApiResponse)]
pub enum PaperAccountSummaryApiResponse {
    #[oai(status = 200)]
    Ok(Json<PaperAccountSummaryResponse>),
    #[oai(status = 404)]
    NotFound(Json<ErrorBody>),
    #[oai(status = 500)]
    InternalError(Json<ErrorBody>),
}

#[derive(ApiResponse)]
pub enum ExecutePaperOrderResponse {
    #[oai(status = 200)]
    Ok(Json<PaperOrderExecutionResponse>),
    #[oai(status = 400)]
    BadRequest(Json<ErrorBody>),
    #[oai(status = 404)]
    NotFound(Json<ErrorBody>),
    #[oai(status = 409)]
    Conflict(Json<ErrorBody>),
    #[oai(status = 500)]
    InternalError(Json<ErrorBody>),
}

#[derive(ApiResponse)]
pub enum ListPaperOrdersResponse {
    #[oai(status = 200)]
    Ok(Json<Vec<PaperOrder>>),
    #[oai(status = 404)]
    NotFound(Json<ErrorBody>),
    #[oai(status = 500)]
    InternalError(Json<ErrorBody>),
}

#[derive(ApiResponse)]
pub enum ListPaperFillsResponse {
    #[oai(status = 200)]
    Ok(Json<Vec<PaperFill>>),
    #[oai(status = 404)]
    NotFound(Json<ErrorBody>),
    #[oai(status = 500)]
    InternalError(Json<ErrorBody>),
}

#[derive(ApiResponse)]
pub enum ListPaperPositionsResponse {
    #[oai(status = 200)]
    Ok(Json<Vec<PaperPosition>>),
    #[oai(status = 404)]
    NotFound(Json<ErrorBody>),
    #[oai(status = 500)]
    InternalError(Json<ErrorBody>),
}

#[derive(ApiResponse)]
pub enum ListPaperEventsResponse {
    #[oai(status = 200)]
    Ok(Json<Vec<PaperAccountEvent>>),
    #[oai(status = 404)]
    NotFound(Json<ErrorBody>),
    #[oai(status = 500)]
    InternalError(Json<ErrorBody>),
}

#[derive(ApiResponse)]
pub enum CancelPaperOrderResponse {
    #[oai(status = 200)]
    Ok(Json<PaperOrder>),
    #[oai(status = 404)]
    NotFound(Json<ErrorBody>),
    #[oai(status = 409)]
    Conflict(Json<ErrorBody>),
    #[oai(status = 500)]
    InternalError(Json<ErrorBody>),
}

#[derive(ApiResponse)]
pub enum GetStrategyTradingConfigResponse {
    #[oai(status = 200)]
    Ok(Json<StrategyTradingConfig>),
    #[oai(status = 404)]
    NotFound(Json<ErrorBody>),
    #[oai(status = 500)]
    InternalError(Json<ErrorBody>),
}

#[derive(ApiResponse)]
pub enum UpdateStrategyTradingConfigResponse {
    #[oai(status = 200)]
    Ok(Json<StrategyTradingConfig>),
    #[oai(status = 400)]
    BadRequest(Json<ErrorBody>),
    #[oai(status = 404)]
    NotFound(Json<ErrorBody>),
    #[oai(status = 409)]
    Conflict(Json<ErrorBody>),
    #[oai(status = 500)]
    InternalError(Json<ErrorBody>),
}

#[derive(ApiResponse)]
pub enum GetStrategyRiskConfigResponse {
    #[oai(status = 200)]
    Ok(Json<StrategyRiskConfig>),
    #[oai(status = 404)]
    NotFound(Json<ErrorBody>),
    #[oai(status = 500)]
    InternalError(Json<ErrorBody>),
}

#[derive(ApiResponse)]
pub enum UpdateStrategyRiskConfigResponse {
    #[oai(status = 200)]
    Ok(Json<StrategyRiskConfig>),
    #[oai(status = 400)]
    BadRequest(Json<ErrorBody>),
    #[oai(status = 404)]
    NotFound(Json<ErrorBody>),
    #[oai(status = 409)]
    Conflict(Json<ErrorBody>),
    #[oai(status = 500)]
    InternalError(Json<ErrorBody>),
}

#[derive(ApiResponse)]
pub enum StrategyRiskMutationResponse {
    #[oai(status = 200)]
    Ok(Json<StrategyRiskConfig>),
    #[oai(status = 404)]
    NotFound(Json<ErrorBody>),
    #[oai(status = 409)]
    Conflict(Json<ErrorBody>),
    #[oai(status = 500)]
    InternalError(Json<ErrorBody>),
}

#[derive(ApiResponse)]
pub enum EngineStrategyConfigApiResponse {
    #[oai(status = 200)]
    Ok(Json<EngineStrategyConfigResponse>),
    #[oai(status = 500)]
    InternalError(Json<ErrorBody>),
}

#[derive(ApiResponse)]
pub enum GetStrategyRuntimeStateResponse {
    #[oai(status = 200)]
    Ok(Json<StrategyRuntimeStateListResponse>),
    #[oai(status = 404)]
    NotFound(Json<ErrorBody>),
    #[oai(status = 500)]
    InternalError(Json<ErrorBody>),
}

#[derive(ApiResponse)]
pub enum GetStrategySignalsResponse {
    #[oai(status = 200)]
    Ok(Json<StrategySignalListResponse>),
    #[oai(status = 404)]
    NotFound(Json<ErrorBody>),
    #[oai(status = 500)]
    InternalError(Json<ErrorBody>),
}

pub fn internal_error<E: std::fmt::Display>(err: E) -> Json<ErrorBody> {
    Json(ErrorBody {
        message: err.to_string(),
    })
}

pub fn error_message(message: impl Into<String>) -> Json<ErrorBody> {
    Json(ErrorBody {
        message: message.into(),
    })
}
