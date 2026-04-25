use models::engine::{ActiveSymbolsResponse, EngineHealthResponse};
use models::{
    channels::{
        Channel, ChannelMessage, ChannelMessagesResponse, DataScientistChatResponse,
        DataScientistProfile, EngineChannelContext, MdChatResponse, MdProfile, TraderPersona,
        UserInvestorProfile,
    },
    chat_commands::ChatCommandResponse,
    data_sources::{
        BuildDataSourceScriptResponse, DataSource, DataSourceEventsResponse,
        DataSourceItemsResponse, DataSourceScript, TraderDataSourcesResponse,
    },
    paper::{
        PaperAccount, PaperAccountEvent, PaperAccountSummaryResponse, PaperFill, PaperOrder,
        PaperOrderExecutionResponse, PaperPosition,
    },
    portfolio::{Portfolio, Position},
    projects::Project,
    trader::{
        EngineTraderConfigResponse, SuggestTraderSymbolsResponse, Trader, TraderChatResponse,
        TraderDetail, TraderEvent, TraderEventsResponse, TraderPortfolioProposalDetail,
        TraderPortfolioProposalsResponse, TraderRuntimeState, TraderSymbol, TraderSymbolsResponse,
        TraderTradeProposal, TraderTradeProposalsResponse,
    },
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
    DataSource,
    Paper,
    Project,
    Portfolio,
    Position,
    Strategy,
    Trader,
    Channel,
    Settings,
}

#[derive(ApiResponse)]
pub enum CreateDataSourceResponse {
    #[oai(status = 201)]
    Created(Json<DataSource>),
    #[oai(status = 400)]
    BadRequest(Json<ErrorBody>),
    #[oai(status = 500)]
    InternalError(Json<ErrorBody>),
}

#[derive(ApiResponse)]
pub enum ListDataSourcesResponse {
    #[oai(status = 200)]
    Ok(Json<Vec<DataSource>>),
    #[oai(status = 500)]
    InternalError(Json<ErrorBody>),
}

#[derive(ApiResponse)]
pub enum GetDataSourceResponse {
    #[oai(status = 200)]
    Ok(Json<DataSource>),
    #[oai(status = 404)]
    NotFound(Json<ErrorBody>),
    #[oai(status = 500)]
    InternalError(Json<ErrorBody>),
}

#[derive(ApiResponse)]
pub enum UpdateDataSourceResponse {
    #[oai(status = 200)]
    Ok(Json<DataSource>),
    #[oai(status = 400)]
    BadRequest(Json<ErrorBody>),
    #[oai(status = 404)]
    NotFound(Json<ErrorBody>),
    #[oai(status = 500)]
    InternalError(Json<ErrorBody>),
}

#[derive(ApiResponse)]
pub enum DeleteDataSourceResponse {
    #[oai(status = 200)]
    Ok,
    #[oai(status = 404)]
    NotFound(Json<ErrorBody>),
    #[oai(status = 500)]
    InternalError(Json<ErrorBody>),
}

#[derive(ApiResponse)]
pub enum GetDataSourceItemsResponse {
    #[oai(status = 200)]
    Ok(Json<DataSourceItemsResponse>),
    #[oai(status = 404)]
    NotFound(Json<ErrorBody>),
    #[oai(status = 500)]
    InternalError(Json<ErrorBody>),
}

#[derive(ApiResponse)]
pub enum GetDataSourceEventsResponse {
    #[oai(status = 200)]
    Ok(Json<DataSourceEventsResponse>),
    #[oai(status = 404)]
    NotFound(Json<ErrorBody>),
    #[oai(status = 500)]
    InternalError(Json<ErrorBody>),
}

#[derive(ApiResponse)]
pub enum GetDataSourceScriptResponse {
    #[oai(status = 200)]
    Ok(Json<DataSourceScript>),
    #[oai(status = 400)]
    BadRequest(Json<ErrorBody>),
    #[oai(status = 404)]
    NotFound(Json<ErrorBody>),
    #[oai(status = 500)]
    InternalError(Json<ErrorBody>),
}

#[derive(ApiResponse)]
pub enum UpdateDataSourceScriptResponse {
    #[oai(status = 200)]
    Ok(Json<DataSourceScript>),
    #[oai(status = 400)]
    BadRequest(Json<ErrorBody>),
    #[oai(status = 404)]
    NotFound(Json<ErrorBody>),
    #[oai(status = 500)]
    InternalError(Json<ErrorBody>),
}

#[derive(ApiResponse)]
pub enum BuildDataSourceScriptApiResponse {
    #[oai(status = 200)]
    Ok(Json<BuildDataSourceScriptResponse>),
    #[oai(status = 400)]
    BadRequest(Json<ErrorBody>),
    #[oai(status = 404)]
    NotFound(Json<ErrorBody>),
    #[oai(status = 500)]
    InternalError(Json<ErrorBody>),
}

#[derive(ApiResponse)]
pub enum TraderDataSourcesApiResponse {
    #[oai(status = 200)]
    Ok(Json<TraderDataSourcesResponse>),
    #[oai(status = 400)]
    BadRequest(Json<ErrorBody>),
    #[oai(status = 404)]
    NotFound(Json<ErrorBody>),
    #[oai(status = 500)]
    InternalError(Json<ErrorBody>),
}

#[derive(ApiResponse)]
pub enum ChatCommandApiResponse {
    #[oai(status = 200)]
    Ok(Json<ChatCommandResponse>),
    #[oai(status = 400)]
    BadRequest(Json<ErrorBody>),
    #[oai(status = 500)]
    InternalError(Json<ErrorBody>),
}

#[derive(Object)]
pub struct ErrorBody {
    pub message: String,
}

#[derive(ApiResponse)]
pub enum ListChannelsResponse {
    #[oai(status = 200)]
    Ok(Json<Vec<Channel>>),
    #[oai(status = 500)]
    InternalError(Json<ErrorBody>),
}

#[derive(ApiResponse)]
pub enum ChannelMessagesApiResponse {
    #[oai(status = 200)]
    Ok(Json<ChannelMessagesResponse>),
    #[oai(status = 404)]
    NotFound(Json<ErrorBody>),
    #[oai(status = 500)]
    InternalError(Json<ErrorBody>),
}

#[derive(ApiResponse)]
pub enum CreateChannelMessageResponse {
    #[oai(status = 201)]
    Created(Json<ChannelMessage>),
    #[oai(status = 400)]
    BadRequest(Json<ErrorBody>),
    #[oai(status = 404)]
    NotFound(Json<ErrorBody>),
    #[oai(status = 500)]
    InternalError(Json<ErrorBody>),
}

#[derive(ApiResponse)]
pub enum DeleteChannelMessagesResponse {
    #[oai(status = 200)]
    Ok,
    #[oai(status = 500)]
    InternalError(Json<ErrorBody>),
}

#[derive(ApiResponse)]
pub enum TraderPersonaApiResponse {
    #[oai(status = 200)]
    Ok(Json<TraderPersona>),
    #[oai(status = 404)]
    NotFound(Json<ErrorBody>),
    #[oai(status = 500)]
    InternalError(Json<ErrorBody>),
}

#[derive(ApiResponse)]
pub enum MdProfileApiResponse {
    #[oai(status = 200)]
    Ok(Json<MdProfile>),
    #[oai(status = 500)]
    InternalError(Json<ErrorBody>),
}

#[derive(ApiResponse)]
pub enum MdChatApiResponse {
    #[oai(status = 200)]
    Ok(Json<MdChatResponse>),
    #[oai(status = 400)]
    BadRequest(Json<ErrorBody>),
    #[oai(status = 500)]
    InternalError(Json<ErrorBody>),
}

#[derive(ApiResponse)]
pub enum DataScientistProfileApiResponse {
    #[oai(status = 200)]
    Ok(Json<DataScientistProfile>),
    #[oai(status = 500)]
    InternalError(Json<ErrorBody>),
}

#[derive(ApiResponse)]
pub enum DataScientistChatApiResponse {
    #[oai(status = 200)]
    Ok(Json<DataScientistChatResponse>),
    #[oai(status = 400)]
    BadRequest(Json<ErrorBody>),
    #[oai(status = 500)]
    InternalError(Json<ErrorBody>),
}

#[derive(ApiResponse)]
pub enum UserInvestorProfileApiResponse {
    #[oai(status = 200)]
    Ok(Json<UserInvestorProfile>),
    #[oai(status = 500)]
    InternalError(Json<ErrorBody>),
}

#[derive(ApiResponse)]
pub enum EngineChannelContextApiResponse {
    #[oai(status = 200)]
    Ok(Json<EngineChannelContext>),
    #[oai(status = 500)]
    InternalError(Json<ErrorBody>),
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
pub enum CreateTraderResponse {
    #[oai(status = 201)]
    Created(Json<Trader>),
    #[oai(status = 400)]
    BadRequest(Json<ErrorBody>),
    #[oai(status = 500)]
    InternalError(Json<ErrorBody>),
}

#[derive(ApiResponse)]
pub enum ListTradersResponse {
    #[oai(status = 200)]
    Ok(Json<Vec<Trader>>),
    #[oai(status = 500)]
    InternalError(Json<ErrorBody>),
}

#[derive(ApiResponse)]
pub enum GetTraderResponse {
    #[oai(status = 200)]
    Ok(Json<TraderDetail>),
    #[oai(status = 404)]
    NotFound(Json<ErrorBody>),
    #[oai(status = 500)]
    InternalError(Json<ErrorBody>),
}

#[derive(ApiResponse)]
pub enum UpdateTraderResponse {
    #[oai(status = 200)]
    Ok(Json<Trader>),
    #[oai(status = 400)]
    BadRequest(Json<ErrorBody>),
    #[oai(status = 404)]
    NotFound(Json<ErrorBody>),
    #[oai(status = 500)]
    InternalError(Json<ErrorBody>),
}

#[derive(ApiResponse)]
pub enum DeleteTraderResponse {
    #[oai(status = 200)]
    Ok,
    #[oai(status = 404)]
    NotFound(Json<ErrorBody>),
    #[oai(status = 500)]
    InternalError(Json<ErrorBody>),
}

#[derive(ApiResponse)]
pub enum TraderMutationResponse {
    #[oai(status = 200)]
    Ok(Json<Trader>),
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
pub enum GetTraderEventsResponse {
    #[oai(status = 200)]
    Ok(Json<TraderEventsResponse>),
    #[oai(status = 404)]
    NotFound(Json<ErrorBody>),
    #[oai(status = 500)]
    InternalError(Json<ErrorBody>),
}

#[derive(ApiResponse)]
pub enum GetTraderRuntimeStateResponse {
    #[oai(status = 200)]
    Ok(Json<TraderRuntimeState>),
    #[oai(status = 404)]
    NotFound(Json<ErrorBody>),
    #[oai(status = 500)]
    InternalError(Json<ErrorBody>),
}

#[derive(ApiResponse)]
pub enum GetTraderTradeProposalsResponse {
    #[oai(status = 200)]
    Ok(Json<TraderTradeProposalsResponse>),
    #[oai(status = 404)]
    NotFound(Json<ErrorBody>),
    #[oai(status = 500)]
    InternalError(Json<ErrorBody>),
}

#[derive(ApiResponse)]
pub enum TraderTradeProposalMutationResponse {
    #[oai(status = 200)]
    Ok(Json<TraderTradeProposal>),
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
pub enum TraderSymbolsApiResponse {
    #[oai(status = 200)]
    Ok(Json<TraderSymbolsResponse>),
    #[oai(status = 400)]
    BadRequest(Json<ErrorBody>),
    #[oai(status = 404)]
    NotFound(Json<ErrorBody>),
    #[oai(status = 500)]
    InternalError(Json<ErrorBody>),
}

#[derive(ApiResponse)]
pub enum TraderSymbolMutationResponse {
    #[oai(status = 200)]
    Ok(Json<TraderSymbol>),
    #[oai(status = 201)]
    Created(Json<TraderSymbol>),
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
pub enum TraderPortfolioProposalsApiResponse {
    #[oai(status = 200)]
    Ok(Json<TraderPortfolioProposalsResponse>),
    #[oai(status = 404)]
    NotFound(Json<ErrorBody>),
    #[oai(status = 500)]
    InternalError(Json<ErrorBody>),
}

#[derive(ApiResponse)]
pub enum TraderPortfolioProposalApiResponse {
    #[oai(status = 200)]
    Ok(Json<TraderPortfolioProposalDetail>),
    #[oai(status = 201)]
    Created(Json<TraderPortfolioProposalDetail>),
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
pub enum SuggestTraderSymbolsApiResponse {
    #[oai(status = 200)]
    Ok(Json<SuggestTraderSymbolsResponse>),
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
pub enum TraderChatApiResponse {
    #[oai(status = 200)]
    Ok(Json<TraderChatResponse>),
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
pub enum EngineTraderConfigApiResponse {
    #[oai(status = 200)]
    Ok(Json<EngineTraderConfigResponse>),
    #[oai(status = 500)]
    InternalError(Json<ErrorBody>),
}

#[derive(ApiResponse)]
pub enum UpsertTraderRuntimeStateResponse {
    #[oai(status = 200)]
    Ok(Json<TraderRuntimeState>),
    #[oai(status = 400)]
    BadRequest(Json<ErrorBody>),
    #[oai(status = 404)]
    NotFound(Json<ErrorBody>),
    #[oai(status = 500)]
    InternalError(Json<ErrorBody>),
}

#[derive(ApiResponse)]
pub enum CreateTraderEventResponse {
    #[oai(status = 201)]
    Created(Json<TraderEvent>),
    #[oai(status = 400)]
    BadRequest(Json<ErrorBody>),
    #[oai(status = 404)]
    NotFound(Json<ErrorBody>),
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
