use models::chat_commands::ChatCommandRequest;
use models::data_sources::{
    BuildDataSourceScriptRequest, CreateDataSourceRequest, UpdateDataSourceRequest,
    UpdateDataSourceScriptRequest, UpdateTraderDataSourcesRequest,
};
use models::engine::{
    ActiveSymbolsResponse, EngineEventRequest, EngineHealthResponse, EngineHeartbeatRequest,
};
use models::paper::{CreatePaperAccountRequest, CreatePaperOrderRequest};
use models::raw::{RawStockData, StockIndicatorsResponse};
use models::{
    channels::{
        CreateChannelMessageRequest, CreateUserChannelMessageRequest, DataScientistChatRequest,
        MdChatRequest, TraderPersonaUpdateRequest, UpdateDataScientistProfileRequest,
        UpdateMdProfileRequest, UpdateUserInvestorProfileRequest,
    },
    portfolio::{Portfolio, Position},
    projects::Project,
    trader::{
        BulkUpsertTraderSymbolsRequest, CreateTraderEventRequest,
        CreateTraderPortfolioProposalRequest, CreateTraderRequest, CreateTraderSymbolRequest,
        CreateTraderTradeProposalRequest, ReviewTraderPortfolioProposalRequest,
        SuggestTraderSymbolsRequest, TraderChatRequest, UpdateTraderRequest,
        UpdateTraderSymbolRequest, UpsertTraderRuntimeStateRequest,
    },
    trading::{
        CreateStrategySignalRequest, UpdateStrategyRiskConfigRequest,
        UpdateStrategySignalStatusRequest, UpdateStrategyTradingConfigRequest,
        UpsertStrategyRuntimeStateRequest,
    },
};
use poem::{
    EndpointExt, Route,
    http::{HeaderValue, Method},
    listener::TcpListener,
    middleware::Cors,
};
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
use tracing_subscriber::FmtSubscriber;

use env_logger;
use log::{error, info};

mod agent_chat;
mod channels;
mod chat_commands;
mod data_sources;
mod helpers;
mod paper;
mod secrets;
mod strategy_execution;
mod strategy_risk;
mod strategy_trading;
mod trader_chat;
mod traders;

use helpers::{
    ActiveSymbolsConfigResponse, ApiTags, BuildDataSourceScriptApiResponse,
    CancelPaperOrderResponse, ChannelMessagesApiResponse, ChatCommandApiResponse,
    CreateChannelMessageResponse, CreateDataSourceResponse, CreatePaperAccountResponse,
    CreatePortfolioResponse, CreatePositionResponse, CreateProjectResponse,
    CreateStrategySignalResponse, CreateTraderEventResponse, CreateTraderResponse,
    DataScientistChatApiResponse, DataScientistProfileApiResponse, DeleteChannelMessagesResponse,
    DeleteDataSourceResponse, DeletePortfolioResponse, DeletePositionResponse,
    DeleteProjectResponse, DeleteTraderResponse, EngineChannelContextApiResponse,
    EngineMutationResponse, EngineStrategyConfigApiResponse, EngineTraderConfigApiResponse,
    ExecutePaperOrderResponse, GetDataSourceEventsResponse, GetDataSourceItemsResponse,
    GetDataSourceResponse, GetDataSourceScriptResponse, GetPaperAccountResponse,
    GetPortfolioResponse, GetPositionResponse, GetProjectResponse, GetStrategyRiskConfigResponse,
    GetStrategyRuntimeStateResponse, GetStrategySignalsResponse, GetStrategyTradingConfigResponse,
    GetTraderEventsResponse, GetTraderResponse, GetTraderRuntimeStateResponse,
    GetTraderTradeProposalsResponse, ListChannelsResponse, ListDataSourcesResponse,
    ListPaperAccountsResponse, ListPaperEventsResponse, ListPaperFillsResponse,
    ListPaperOrdersResponse, ListPaperPositionsResponse, ListPortfoliosResponse,
    ListPositionsResponse, ListProjectsResponse, ListTradersResponse, MdChatApiResponse,
    MdProfileApiResponse, PaperAccountSummaryApiResponse, StrategyRiskMutationResponse,
    SuggestTraderSymbolsApiResponse, TraderChatApiResponse, TraderDataSourcesApiResponse,
    TraderMutationResponse, TraderPersonaApiResponse, TraderPortfolioProposalApiResponse,
    TraderPortfolioProposalsApiResponse, TraderSymbolMutationResponse, TraderSymbolsApiResponse,
    TraderTradeProposalMutationResponse, UpdateDataSourceResponse, UpdateDataSourceScriptResponse,
    UpdatePortfolioResponse, UpdatePositionResponse, UpdateProjectResponse,
    UpdateStrategyRiskConfigResponse, UpdateStrategySignalResponse,
    UpdateStrategyTradingConfigResponse, UpdateTraderResponse, UpsertStrategyRuntimeStateResponse,
    UpsertTraderRuntimeStateResponse, UserInvestorProfileApiResponse, error_message,
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

    #[oai(path = "/chat/commands", method = "post")]
    async fn chat_commands(&self, request: Json<ChatCommandRequest>) -> ChatCommandApiResponse {
        match chat_commands::handle(&self.database, request.0).await {
            Ok(response) => ChatCommandApiResponse::Ok(Json(response)),
            Err(message) if message.contains("required") || message.contains("invalid") => {
                ChatCommandApiResponse::BadRequest(error_message(message))
            }
            Err(message) => ChatCommandApiResponse::InternalError(error_message(message)),
        }
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

    #[oai(path = "/health/live", method = "get", tag = "ApiTags::Engine")]
    async fn health_live(&self) -> Json<EngineHealthResponse> {
        Json(EngineHealthResponse {
            status: "ok".to_string(),
        })
    }

    #[oai(
        path = "/engine/config/symbols",
        method = "get",
        tag = "ApiTags::Engine"
    )]
    async fn engine_config_symbols(&self) -> ActiveSymbolsConfigResponse {
        match self.database.list_active_symbols().await {
            Ok(symbols) => {
                info!("engine active symbols loaded: {:?}", symbols);
                ActiveSymbolsConfigResponse::Ok(Json(ActiveSymbolsResponse { symbols }))
            }
            Err(err) => {
                error!("failed to load engine active symbols: {}", err);
                ActiveSymbolsConfigResponse::InternalError(internal_error(err))
            }
        }
    }

    #[oai(
        path = "/engine/config/strategies",
        method = "get",
        tag = "ApiTags::Engine"
    )]
    async fn engine_config_strategies(&self) -> EngineStrategyConfigApiResponse {
        match strategy_trading::list_engine_strategy_configs(&self.database).await {
            Ok(configs) => EngineStrategyConfigApiResponse::Ok(Json(configs)),
            Err(err) => EngineStrategyConfigApiResponse::InternalError(error_message(err.message)),
        }
    }

    #[oai(
        path = "/engine/config/traders",
        method = "get",
        tag = "ApiTags::Engine"
    )]
    async fn engine_config_traders(&self) -> EngineTraderConfigApiResponse {
        match traders::engine_config(&self.database).await {
            Ok(configs) => EngineTraderConfigApiResponse::Ok(Json(configs)),
            Err(err) => EngineTraderConfigApiResponse::InternalError(error_message(err.message)),
        }
    }

    #[oai(
        path = "/engine/channel-context",
        method = "get",
        tag = "ApiTags::Engine"
    )]
    async fn engine_channel_context(&self) -> EngineChannelContextApiResponse {
        match channels::engine_context(&self.database).await {
            Ok(context) => EngineChannelContextApiResponse::Ok(Json(context)),
            Err(err) => EngineChannelContextApiResponse::InternalError(error_message(err.message)),
        }
    }

    #[oai(
        path = "/engine/channels/:channel_name/messages",
        method = "post",
        tag = "ApiTags::Engine"
    )]
    async fn engine_channel_message(
        &self,
        channel_name: Path<String>,
        request: Json<CreateChannelMessageRequest>,
    ) -> CreateChannelMessageResponse {
        match channels::create_engine_message(&self.database, &channel_name.0, request.0).await {
            Ok(message) => CreateChannelMessageResponse::Created(Json(message)),
            Err(err) => map_channel_message_error(err),
        }
    }

    #[oai(path = "/engine/heartbeat", method = "post", tag = "ApiTags::Engine")]
    async fn engine_heartbeat(
        &self,
        heartbeat: Json<EngineHeartbeatRequest>,
    ) -> EngineMutationResponse {
        let heartbeat = heartbeat.0;

        match self.database.insert_engine_heartbeat(&heartbeat).await {
            Ok(record) => {
                info!(
                    "engine heartbeat persisted: id={}, engine_name={}, status={}, timestamp={}",
                    record.id, record.engine_name, record.status, record.timestamp
                );
                EngineMutationResponse::Ok(Json(EngineHealthResponse {
                    status: "ok".to_string(),
                }))
            }
            Err(err) => {
                error!(
                    "failed to persist engine heartbeat: engine_name={}, status={}, error={}",
                    heartbeat.engine_name, heartbeat.status, err
                );
                EngineMutationResponse::InternalError(internal_error(err))
            }
        }
    }

    #[oai(path = "/engine/events", method = "post", tag = "ApiTags::Engine")]
    async fn engine_events(&self, event: Json<EngineEventRequest>) -> EngineMutationResponse {
        let event = event.0;

        match self.database.insert_engine_event(&event).await {
            Ok(record) => {
                info!(
                    "engine event persisted: id={}, engine_name={}, event_type={}, symbol={:?}, timestamp={}",
                    record.id,
                    record.engine_name,
                    record.event_type,
                    record.symbol,
                    record.timestamp
                );
                EngineMutationResponse::Ok(Json(EngineHealthResponse {
                    status: "ok".to_string(),
                }))
            }
            Err(err) => {
                error!(
                    "failed to persist engine event: engine_name={}, event_type={}, symbol={:?}, error={}",
                    event.engine_name, event.event_type, event.symbol, err
                );
                EngineMutationResponse::InternalError(internal_error(err))
            }
        }
    }

    #[oai(
        path = "/engine/strategy-signals",
        method = "post",
        tag = "ApiTags::Engine"
    )]
    async fn engine_strategy_signals(
        &self,
        request: Json<CreateStrategySignalRequest>,
    ) -> CreateStrategySignalResponse {
        match strategy_execution::create_signal(&self.database, request.0).await {
            Ok(signal) => CreateStrategySignalResponse::Created(Json(signal)),
            Err(message) if message.contains("must") || message.contains("invalid") => {
                CreateStrategySignalResponse::BadRequest(error_message(message))
            }
            Err(message) => CreateStrategySignalResponse::InternalError(error_message(message)),
        }
    }

    #[oai(
        path = "/engine/strategy-signals/:signal_id",
        method = "post",
        tag = "ApiTags::Engine"
    )]
    async fn update_engine_strategy_signal(
        &self,
        signal_id: Path<String>,
        request: Json<UpdateStrategySignalStatusRequest>,
    ) -> UpdateStrategySignalResponse {
        match strategy_execution::update_signal(&self.database, &signal_id.0, request.0).await {
            Ok(()) => UpdateStrategySignalResponse::Ok,
            Err(message) if message.contains("invalid") || message.contains("must") => {
                UpdateStrategySignalResponse::BadRequest(error_message(message))
            }
            Err(message) => UpdateStrategySignalResponse::InternalError(error_message(message)),
        }
    }

    #[oai(
        path = "/engine/strategy-runtime-state",
        method = "post",
        tag = "ApiTags::Engine"
    )]
    async fn engine_strategy_runtime_state(
        &self,
        request: Json<UpsertStrategyRuntimeStateRequest>,
    ) -> UpsertStrategyRuntimeStateResponse {
        match strategy_execution::upsert_runtime_state(&self.database, request.0).await {
            Ok(state) => UpsertStrategyRuntimeStateResponse::Ok(Json(state)),
            Err(message) if message.contains("must") || message.contains("invalid") => {
                UpsertStrategyRuntimeStateResponse::BadRequest(error_message(message))
            }
            Err(message) => {
                UpsertStrategyRuntimeStateResponse::InternalError(error_message(message))
            }
        }
    }

    #[oai(
        path = "/engine/traders/:trader_id/runtime-state",
        method = "post",
        tag = "ApiTags::Engine"
    )]
    async fn engine_trader_runtime_state(
        &self,
        trader_id: Path<String>,
        request: Json<UpsertTraderRuntimeStateRequest>,
    ) -> UpsertTraderRuntimeStateResponse {
        match traders::upsert_runtime_state(&self.database, &trader_id.0, request.0).await {
            Ok(state) => UpsertTraderRuntimeStateResponse::Ok(Json(state)),
            Err(err) => match err.kind {
                traders::TraderErrorKind::BadRequest => {
                    UpsertTraderRuntimeStateResponse::BadRequest(error_message(err.message))
                }
                traders::TraderErrorKind::NotFound => {
                    UpsertTraderRuntimeStateResponse::NotFound(error_message(err.message))
                }
                _ => UpsertTraderRuntimeStateResponse::InternalError(error_message(err.message)),
            },
        }
    }

    #[oai(
        path = "/engine/traders/:trader_id/events",
        method = "post",
        tag = "ApiTags::Engine"
    )]
    async fn engine_trader_events(
        &self,
        trader_id: Path<String>,
        request: Json<CreateTraderEventRequest>,
    ) -> CreateTraderEventResponse {
        match traders::create_event(&self.database, &trader_id.0, request.0).await {
            Ok(event) => CreateTraderEventResponse::Created(Json(event)),
            Err(err) => match err.kind {
                traders::TraderErrorKind::BadRequest => {
                    CreateTraderEventResponse::BadRequest(error_message(err.message))
                }
                traders::TraderErrorKind::NotFound => {
                    CreateTraderEventResponse::NotFound(error_message(err.message))
                }
                _ => CreateTraderEventResponse::InternalError(error_message(err.message)),
            },
        }
    }

    #[oai(
        path = "/engine/traders/:trader_id/trade-proposals",
        method = "post",
        tag = "ApiTags::Engine"
    )]
    async fn engine_trader_trade_proposals(
        &self,
        trader_id: Path<String>,
        request: Json<CreateTraderTradeProposalRequest>,
    ) -> TraderTradeProposalMutationResponse {
        match traders::create_trade_proposal(&self.database, &trader_id.0, request.0).await {
            Ok(proposal) => TraderTradeProposalMutationResponse::Ok(Json(proposal)),
            Err(err) => match err.kind {
                traders::TraderErrorKind::BadRequest => {
                    TraderTradeProposalMutationResponse::BadRequest(error_message(err.message))
                }
                traders::TraderErrorKind::NotFound => {
                    TraderTradeProposalMutationResponse::NotFound(error_message(err.message))
                }
                traders::TraderErrorKind::Conflict => {
                    TraderTradeProposalMutationResponse::Conflict(error_message(err.message))
                }
                traders::TraderErrorKind::Internal => {
                    TraderTradeProposalMutationResponse::InternalError(error_message(err.message))
                }
            },
        }
    }

    #[oai(
        path = "/engine/traders/:trader_id/proposals",
        method = "post",
        tag = "ApiTags::Engine"
    )]
    async fn engine_trader_portfolio_proposals(
        &self,
        trader_id: Path<String>,
        request: Json<CreateTraderPortfolioProposalRequest>,
    ) -> TraderPortfolioProposalApiResponse {
        match traders::create_portfolio_proposal(&self.database, &trader_id.0, request.0).await {
            Ok(proposal) => TraderPortfolioProposalApiResponse::Created(Json(proposal)),
            Err(err) => map_trader_portfolio_proposal_error(err),
        }
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

    #[oai(path = "/channels", method = "get", tag = "ApiTags::Channel")]
    async fn list_channels(&self) -> ListChannelsResponse {
        match channels::list_channels(&self.database).await {
            Ok(channels) => ListChannelsResponse::Ok(Json(channels)),
            Err(err) => ListChannelsResponse::InternalError(error_message(err.message)),
        }
    }

    #[oai(
        path = "/channels/:channel_id/messages",
        method = "get",
        tag = "ApiTags::Channel"
    )]
    async fn list_channel_messages(
        &self,
        channel_id: Path<String>,
        limit: Query<Option<i64>>,
        before: Query<Option<String>>,
        after: Query<Option<String>>,
    ) -> ChannelMessagesApiResponse {
        match channels::list_messages(
            &self.database,
            &channel_id.0,
            limit.0,
            before.0.as_deref(),
            after.0.as_deref(),
        )
        .await
        {
            Ok(messages) => ChannelMessagesApiResponse::Ok(Json(messages)),
            Err(err) => match err.kind {
                channels::ChannelErrorKind::NotFound => {
                    ChannelMessagesApiResponse::NotFound(error_message(err.message))
                }
                _ => ChannelMessagesApiResponse::InternalError(error_message(err.message)),
            },
        }
    }

    #[oai(
        path = "/channels/:channel_id/messages",
        method = "post",
        tag = "ApiTags::Channel"
    )]
    async fn create_channel_message(
        &self,
        channel_id: Path<String>,
        request: Json<CreateUserChannelMessageRequest>,
    ) -> CreateChannelMessageResponse {
        match channels::create_user_message(&self.database, &channel_id.0, request.0).await {
            Ok(message) => CreateChannelMessageResponse::Created(Json(message)),
            Err(err) => map_channel_message_error(err),
        }
    }

    #[oai(
        path = "/channels/messages",
        method = "delete",
        tag = "ApiTags::Settings"
    )]
    async fn clear_channel_messages(&self) -> DeleteChannelMessagesResponse {
        match channels::clear_messages(&self.database).await {
            Ok(_) => DeleteChannelMessagesResponse::Ok,
            Err(err) => DeleteChannelMessagesResponse::InternalError(error_message(err.message)),
        }
    }

    #[oai(path = "/md-profile", method = "get", tag = "ApiTags::Settings")]
    async fn get_md_profile(&self) -> MdProfileApiResponse {
        match channels::get_md_profile(&self.database).await {
            Ok(profile) => MdProfileApiResponse::Ok(Json(profile)),
            Err(err) => MdProfileApiResponse::InternalError(error_message(err.message)),
        }
    }

    #[oai(path = "/md-profile", method = "put", tag = "ApiTags::Settings")]
    async fn update_md_profile(
        &self,
        request: Json<UpdateMdProfileRequest>,
    ) -> MdProfileApiResponse {
        match channels::update_md_profile(&self.database, request.0).await {
            Ok(profile) => MdProfileApiResponse::Ok(Json(profile)),
            Err(err) => MdProfileApiResponse::InternalError(error_message(err.message)),
        }
    }

    #[oai(path = "/md-profile/chat", method = "post", tag = "ApiTags::Settings")]
    async fn md_profile_chat(&self, request: Json<MdChatRequest>) -> MdChatApiResponse {
        match agent_chat::md_chat(&self.database, request.0).await {
            Ok(response) => MdChatApiResponse::Ok(Json(response)),
            Err(err) => match err.kind {
                agent_chat::AgentChatErrorKind::BadRequest => {
                    MdChatApiResponse::BadRequest(error_message(err.message))
                }
                agent_chat::AgentChatErrorKind::Internal => {
                    MdChatApiResponse::InternalError(error_message(err.message))
                }
            },
        }
    }

    #[oai(
        path = "/data-scientist-profile",
        method = "get",
        tag = "ApiTags::Settings"
    )]
    async fn get_data_scientist_profile(&self) -> DataScientistProfileApiResponse {
        match channels::get_data_scientist_profile(&self.database).await {
            Ok(profile) => DataScientistProfileApiResponse::Ok(Json(profile)),
            Err(err) => DataScientistProfileApiResponse::InternalError(error_message(err.message)),
        }
    }

    #[oai(
        path = "/data-scientist-profile",
        method = "put",
        tag = "ApiTags::Settings"
    )]
    async fn update_data_scientist_profile(
        &self,
        request: Json<UpdateDataScientistProfileRequest>,
    ) -> DataScientistProfileApiResponse {
        match channels::update_data_scientist_profile(&self.database, request.0).await {
            Ok(profile) => DataScientistProfileApiResponse::Ok(Json(profile)),
            Err(err) => DataScientistProfileApiResponse::InternalError(error_message(err.message)),
        }
    }

    #[oai(
        path = "/data-scientist-profile/chat",
        method = "post",
        tag = "ApiTags::Settings"
    )]
    async fn data_scientist_profile_chat(
        &self,
        request: Json<DataScientistChatRequest>,
    ) -> DataScientistChatApiResponse {
        match agent_chat::data_scientist_chat(&self.database, request.0).await {
            Ok(response) => DataScientistChatApiResponse::Ok(Json(response)),
            Err(err) => match err.kind {
                agent_chat::AgentChatErrorKind::BadRequest => {
                    DataScientistChatApiResponse::BadRequest(error_message(err.message))
                }
                agent_chat::AgentChatErrorKind::Internal => {
                    DataScientistChatApiResponse::InternalError(error_message(err.message))
                }
            },
        }
    }

    #[oai(
        path = "/settings/investor-profile",
        method = "get",
        tag = "ApiTags::Settings"
    )]
    async fn get_investor_profile(&self) -> UserInvestorProfileApiResponse {
        match channels::get_investor_profile(&self.database).await {
            Ok(profile) => UserInvestorProfileApiResponse::Ok(Json(profile)),
            Err(err) => UserInvestorProfileApiResponse::InternalError(error_message(err.message)),
        }
    }

    #[oai(
        path = "/settings/investor-profile",
        method = "put",
        tag = "ApiTags::Settings"
    )]
    async fn update_investor_profile(
        &self,
        request: Json<UpdateUserInvestorProfileRequest>,
    ) -> UserInvestorProfileApiResponse {
        match channels::update_investor_profile(&self.database, request.0).await {
            Ok(profile) => UserInvestorProfileApiResponse::Ok(Json(profile)),
            Err(err) => UserInvestorProfileApiResponse::InternalError(error_message(err.message)),
        }
    }

    // ----------------------------
    // Paper Trading
    // ----------------------------

    #[oai(path = "/paper/accounts", method = "post", tag = "ApiTags::Paper")]
    async fn create_paper_account(
        &self,
        request: Json<CreatePaperAccountRequest>,
    ) -> CreatePaperAccountResponse {
        match paper::create_account(&self.database, request.0).await {
            Ok(account) => CreatePaperAccountResponse::Created(Json(account)),
            Err(err) => match err.kind {
                paper::PaperErrorKind::BadRequest => {
                    CreatePaperAccountResponse::BadRequest(error_message(err.message))
                }
                paper::PaperErrorKind::Internal => {
                    CreatePaperAccountResponse::InternalError(error_message(err.message))
                }
                _ => CreatePaperAccountResponse::InternalError(error_message(err.message)),
            },
        }
    }

    #[oai(path = "/paper/accounts", method = "get", tag = "ApiTags::Paper")]
    async fn list_paper_accounts(&self) -> ListPaperAccountsResponse {
        match paper::list_accounts(&self.database).await {
            Ok(accounts) => ListPaperAccountsResponse::Ok(Json(accounts)),
            Err(err) => ListPaperAccountsResponse::InternalError(error_message(err.message)),
        }
    }

    #[oai(
        path = "/paper/accounts/:account_id",
        method = "get",
        tag = "ApiTags::Paper"
    )]
    async fn get_paper_account(&self, account_id: Path<String>) -> GetPaperAccountResponse {
        match paper::get_account(&self.database, &account_id.0).await {
            Ok(account) => GetPaperAccountResponse::Ok(Json(account)),
            Err(err) => match err.kind {
                paper::PaperErrorKind::NotFound => {
                    GetPaperAccountResponse::NotFound(error_message(err.message))
                }
                _ => GetPaperAccountResponse::InternalError(error_message(err.message)),
            },
        }
    }

    #[oai(
        path = "/paper/accounts/:account_id/summary",
        method = "get",
        tag = "ApiTags::Paper"
    )]
    async fn get_paper_account_summary(
        &self,
        account_id: Path<String>,
    ) -> PaperAccountSummaryApiResponse {
        match paper::get_account_summary(&self.database, &self.cache, &account_id.0).await {
            Ok(summary) => PaperAccountSummaryApiResponse::Ok(Json(summary)),
            Err(err) => match err.kind {
                paper::PaperErrorKind::NotFound => {
                    PaperAccountSummaryApiResponse::NotFound(error_message(err.message))
                }
                _ => PaperAccountSummaryApiResponse::InternalError(error_message(err.message)),
            },
        }
    }

    #[oai(path = "/paper/orders", method = "post", tag = "ApiTags::Paper")]
    async fn create_paper_order(
        &self,
        request: Json<CreatePaperOrderRequest>,
    ) -> ExecutePaperOrderResponse {
        match paper::execute_paper_market_order(&self.database, &self.cache, request.0).await {
            Ok(result) => ExecutePaperOrderResponse::Ok(Json(result)),
            Err(err) => match err.kind {
                paper::PaperErrorKind::BadRequest => {
                    ExecutePaperOrderResponse::BadRequest(error_message(err.message))
                }
                paper::PaperErrorKind::NotFound => {
                    ExecutePaperOrderResponse::NotFound(error_message(err.message))
                }
                paper::PaperErrorKind::Conflict => {
                    ExecutePaperOrderResponse::Conflict(error_message(err.message))
                }
                paper::PaperErrorKind::Internal => {
                    ExecutePaperOrderResponse::InternalError(error_message(err.message))
                }
            },
        }
    }

    #[oai(
        path = "/paper/accounts/:account_id/orders",
        method = "get",
        tag = "ApiTags::Paper"
    )]
    async fn list_paper_orders(&self, account_id: Path<String>) -> ListPaperOrdersResponse {
        match paper::list_orders(&self.database, &account_id.0).await {
            Ok(orders) => ListPaperOrdersResponse::Ok(Json(orders)),
            Err(err) => match err.kind {
                paper::PaperErrorKind::NotFound => {
                    ListPaperOrdersResponse::NotFound(error_message(err.message))
                }
                _ => ListPaperOrdersResponse::InternalError(error_message(err.message)),
            },
        }
    }

    #[oai(
        path = "/paper/accounts/:account_id/fills",
        method = "get",
        tag = "ApiTags::Paper"
    )]
    async fn list_paper_fills(&self, account_id: Path<String>) -> ListPaperFillsResponse {
        match paper::list_fills(&self.database, &account_id.0).await {
            Ok(fills) => ListPaperFillsResponse::Ok(Json(fills)),
            Err(err) => match err.kind {
                paper::PaperErrorKind::NotFound => {
                    ListPaperFillsResponse::NotFound(error_message(err.message))
                }
                _ => ListPaperFillsResponse::InternalError(error_message(err.message)),
            },
        }
    }

    #[oai(
        path = "/paper/accounts/:account_id/positions",
        method = "get",
        tag = "ApiTags::Paper"
    )]
    async fn list_paper_positions(&self, account_id: Path<String>) -> ListPaperPositionsResponse {
        match paper::list_positions(&self.database, &account_id.0).await {
            Ok(positions) => ListPaperPositionsResponse::Ok(Json(positions)),
            Err(err) => match err.kind {
                paper::PaperErrorKind::NotFound => {
                    ListPaperPositionsResponse::NotFound(error_message(err.message))
                }
                _ => ListPaperPositionsResponse::InternalError(error_message(err.message)),
            },
        }
    }

    #[oai(
        path = "/paper/accounts/:account_id/events",
        method = "get",
        tag = "ApiTags::Paper"
    )]
    async fn list_paper_events(&self, account_id: Path<String>) -> ListPaperEventsResponse {
        match paper::list_events(&self.database, &account_id.0).await {
            Ok(events) => ListPaperEventsResponse::Ok(Json(events)),
            Err(err) => match err.kind {
                paper::PaperErrorKind::NotFound => {
                    ListPaperEventsResponse::NotFound(error_message(err.message))
                }
                _ => ListPaperEventsResponse::InternalError(error_message(err.message)),
            },
        }
    }

    #[oai(
        path = "/paper/orders/:order_id/cancel",
        method = "post",
        tag = "ApiTags::Paper"
    )]
    async fn cancel_paper_order(&self, order_id: Path<String>) -> CancelPaperOrderResponse {
        match paper::cancel_order(&self.database, &order_id.0).await {
            Ok(order) => CancelPaperOrderResponse::Ok(Json(order)),
            Err(err) => match err.kind {
                paper::PaperErrorKind::NotFound => {
                    CancelPaperOrderResponse::NotFound(error_message(err.message))
                }
                paper::PaperErrorKind::Conflict => {
                    CancelPaperOrderResponse::Conflict(error_message(err.message))
                }
                _ => CancelPaperOrderResponse::InternalError(error_message(err.message)),
            },
        }
    }

    #[oai(
        path = "/strategies/:strategy_id/trading-config",
        method = "get",
        tag = "ApiTags::Strategy"
    )]
    async fn get_strategy_trading_config(
        &self,
        strategy_id: Path<String>,
    ) -> GetStrategyTradingConfigResponse {
        match strategy_trading::get_trading_config(&self.database, &strategy_id.0).await {
            Ok(config) => GetStrategyTradingConfigResponse::Ok(Json(config)),
            Err(err) => match err.kind {
                strategy_trading::StrategyTradingErrorKind::NotFound
                | strategy_trading::StrategyTradingErrorKind::BadRequest => {
                    GetStrategyTradingConfigResponse::NotFound(error_message(err.message))
                }
                strategy_trading::StrategyTradingErrorKind::Conflict
                | strategy_trading::StrategyTradingErrorKind::Internal => {
                    GetStrategyTradingConfigResponse::InternalError(error_message(err.message))
                }
            },
        }
    }

    #[oai(
        path = "/strategies/:strategy_id/trading-config",
        method = "put",
        tag = "ApiTags::Strategy"
    )]
    async fn update_strategy_trading_config(
        &self,
        strategy_id: Path<String>,
        request: Json<UpdateStrategyTradingConfigRequest>,
    ) -> UpdateStrategyTradingConfigResponse {
        match strategy_trading::update_trading_config(&self.database, &strategy_id.0, request.0)
            .await
        {
            Ok(config) => UpdateStrategyTradingConfigResponse::Ok(Json(config)),
            Err(err) => match err.kind {
                strategy_trading::StrategyTradingErrorKind::BadRequest => {
                    UpdateStrategyTradingConfigResponse::BadRequest(error_message(err.message))
                }
                strategy_trading::StrategyTradingErrorKind::NotFound => {
                    UpdateStrategyTradingConfigResponse::NotFound(error_message(err.message))
                }
                strategy_trading::StrategyTradingErrorKind::Conflict => {
                    UpdateStrategyTradingConfigResponse::Conflict(error_message(err.message))
                }
                strategy_trading::StrategyTradingErrorKind::Internal => {
                    UpdateStrategyTradingConfigResponse::InternalError(error_message(err.message))
                }
            },
        }
    }

    #[oai(
        path = "/strategies/:strategy_id/risk-config",
        method = "get",
        tag = "ApiTags::Strategy"
    )]
    async fn get_strategy_risk_config(
        &self,
        strategy_id: Path<String>,
    ) -> GetStrategyRiskConfigResponse {
        match strategy_risk::get_risk_config(&self.database, &strategy_id.0).await {
            Ok(config) => GetStrategyRiskConfigResponse::Ok(Json(config)),
            Err(err) => match err.kind {
                strategy_risk::StrategyRiskErrorKind::NotFound => {
                    GetStrategyRiskConfigResponse::NotFound(error_message(err.message))
                }
                strategy_risk::StrategyRiskErrorKind::BadRequest
                | strategy_risk::StrategyRiskErrorKind::Conflict
                | strategy_risk::StrategyRiskErrorKind::Internal => {
                    GetStrategyRiskConfigResponse::InternalError(error_message(err.message))
                }
            },
        }
    }

    #[oai(
        path = "/strategies/:strategy_id/risk-config",
        method = "put",
        tag = "ApiTags::Strategy"
    )]
    async fn update_strategy_risk_config(
        &self,
        strategy_id: Path<String>,
        request: Json<UpdateStrategyRiskConfigRequest>,
    ) -> UpdateStrategyRiskConfigResponse {
        match strategy_risk::update_risk_config(&self.database, &strategy_id.0, request.0).await {
            Ok(config) => UpdateStrategyRiskConfigResponse::Ok(Json(config)),
            Err(err) => match err.kind {
                strategy_risk::StrategyRiskErrorKind::BadRequest => {
                    UpdateStrategyRiskConfigResponse::BadRequest(error_message(err.message))
                }
                strategy_risk::StrategyRiskErrorKind::NotFound => {
                    UpdateStrategyRiskConfigResponse::NotFound(error_message(err.message))
                }
                strategy_risk::StrategyRiskErrorKind::Conflict => {
                    UpdateStrategyRiskConfigResponse::Conflict(error_message(err.message))
                }
                strategy_risk::StrategyRiskErrorKind::Internal => {
                    UpdateStrategyRiskConfigResponse::InternalError(error_message(err.message))
                }
            },
        }
    }

    #[oai(
        path = "/strategies/:strategy_id/kill-switch",
        method = "post",
        tag = "ApiTags::Strategy"
    )]
    async fn trigger_strategy_kill_switch(
        &self,
        strategy_id: Path<String>,
    ) -> StrategyRiskMutationResponse {
        match strategy_risk::trigger_kill_switch(&self.database, &strategy_id.0).await {
            Ok(config) => StrategyRiskMutationResponse::Ok(Json(config)),
            Err(err) => match err.kind {
                strategy_risk::StrategyRiskErrorKind::NotFound => {
                    StrategyRiskMutationResponse::NotFound(error_message(err.message))
                }
                strategy_risk::StrategyRiskErrorKind::Conflict => {
                    StrategyRiskMutationResponse::Conflict(error_message(err.message))
                }
                strategy_risk::StrategyRiskErrorKind::BadRequest
                | strategy_risk::StrategyRiskErrorKind::Internal => {
                    StrategyRiskMutationResponse::InternalError(error_message(err.message))
                }
            },
        }
    }

    #[oai(
        path = "/strategies/:strategy_id/resume-trading",
        method = "post",
        tag = "ApiTags::Strategy"
    )]
    async fn resume_strategy_trading(
        &self,
        strategy_id: Path<String>,
    ) -> StrategyRiskMutationResponse {
        match strategy_risk::resume_trading(&self.database, &strategy_id.0).await {
            Ok(config) => StrategyRiskMutationResponse::Ok(Json(config)),
            Err(err) => match err.kind {
                strategy_risk::StrategyRiskErrorKind::NotFound => {
                    StrategyRiskMutationResponse::NotFound(error_message(err.message))
                }
                strategy_risk::StrategyRiskErrorKind::Conflict => {
                    StrategyRiskMutationResponse::Conflict(error_message(err.message))
                }
                strategy_risk::StrategyRiskErrorKind::BadRequest
                | strategy_risk::StrategyRiskErrorKind::Internal => {
                    StrategyRiskMutationResponse::InternalError(error_message(err.message))
                }
            },
        }
    }

    #[oai(
        path = "/strategies/:strategy_id/runtime-state",
        method = "get",
        tag = "ApiTags::Strategy"
    )]
    async fn get_strategy_runtime_state(
        &self,
        strategy_id: Path<String>,
    ) -> GetStrategyRuntimeStateResponse {
        match strategy_execution::get_runtime_state(&self.database, &strategy_id.0).await {
            Ok(state) => GetStrategyRuntimeStateResponse::Ok(Json(state)),
            Err(message) if message == "strategy not found" => {
                GetStrategyRuntimeStateResponse::NotFound(error_message(message))
            }
            Err(message) => GetStrategyRuntimeStateResponse::InternalError(error_message(message)),
        }
    }

    #[oai(
        path = "/strategies/:strategy_id/signals",
        method = "get",
        tag = "ApiTags::Strategy"
    )]
    async fn get_strategy_signals(&self, strategy_id: Path<String>) -> GetStrategySignalsResponse {
        match strategy_execution::get_signals(&self.database, &strategy_id.0).await {
            Ok(signals) => GetStrategySignalsResponse::Ok(Json(signals)),
            Err(message) if message == "strategy not found" => {
                GetStrategySignalsResponse::NotFound(error_message(message))
            }
            Err(message) => GetStrategySignalsResponse::InternalError(error_message(message)),
        }
    }

    // ----------------------------
    // Traders
    // ----------------------------

    #[oai(path = "/traders", method = "post", tag = "ApiTags::Trader")]
    async fn create_trader(&self, request: Json<CreateTraderRequest>) -> CreateTraderResponse {
        match traders::create_trader(&self.database, request.0).await {
            Ok(trader) => CreateTraderResponse::Created(Json(trader)),
            Err(err) => match err.kind {
                traders::TraderErrorKind::BadRequest => {
                    CreateTraderResponse::BadRequest(error_message(err.message))
                }
                _ => CreateTraderResponse::InternalError(error_message(err.message)),
            },
        }
    }

    #[oai(path = "/traders", method = "get", tag = "ApiTags::Trader")]
    async fn list_traders(&self) -> ListTradersResponse {
        match traders::list_traders(&self.database).await {
            Ok(traders) => ListTradersResponse::Ok(Json(traders)),
            Err(err) => ListTradersResponse::InternalError(error_message(err.message)),
        }
    }

    #[oai(path = "/traders/:trader_id", method = "get", tag = "ApiTags::Trader")]
    async fn get_trader(&self, trader_id: Path<String>) -> GetTraderResponse {
        match traders::get_trader_detail(&self.database, &trader_id.0).await {
            Ok(detail) => GetTraderResponse::Ok(Json(detail)),
            Err(err) => match err.kind {
                traders::TraderErrorKind::NotFound => {
                    GetTraderResponse::NotFound(error_message(err.message))
                }
                _ => GetTraderResponse::InternalError(error_message(err.message)),
            },
        }
    }

    #[oai(
        path = "/traders/:trader_id/persona",
        method = "get",
        tag = "ApiTags::Trader"
    )]
    async fn get_trader_persona(&self, trader_id: Path<String>) -> TraderPersonaApiResponse {
        match channels::get_trader_persona(&self.database, &trader_id.0).await {
            Ok(persona) => TraderPersonaApiResponse::Ok(Json(persona)),
            Err(err) => match err.kind {
                channels::ChannelErrorKind::NotFound => {
                    TraderPersonaApiResponse::NotFound(error_message(err.message))
                }
                _ => TraderPersonaApiResponse::InternalError(error_message(err.message)),
            },
        }
    }

    #[oai(
        path = "/traders/:trader_id/persona",
        method = "put",
        tag = "ApiTags::Trader"
    )]
    async fn update_trader_persona(
        &self,
        trader_id: Path<String>,
        request: Json<TraderPersonaUpdateRequest>,
    ) -> TraderPersonaApiResponse {
        match channels::update_trader_persona(&self.database, &trader_id.0, request.0).await {
            Ok(persona) => TraderPersonaApiResponse::Ok(Json(persona)),
            Err(err) => match err.kind {
                channels::ChannelErrorKind::NotFound => {
                    TraderPersonaApiResponse::NotFound(error_message(err.message))
                }
                _ => TraderPersonaApiResponse::InternalError(error_message(err.message)),
            },
        }
    }

    #[oai(
        path = "/traders/:trader_id/chat",
        method = "post",
        tag = "ApiTags::Trader"
    )]
    async fn trader_chat(
        &self,
        trader_id: Path<String>,
        request: Json<TraderChatRequest>,
    ) -> TraderChatApiResponse {
        match trader_chat::chat(&self.database, &trader_id.0, request.0).await {
            Ok(response) => TraderChatApiResponse::Ok(Json(response)),
            Err(err) => match err.kind {
                trader_chat::TraderChatErrorKind::BadRequest => {
                    TraderChatApiResponse::BadRequest(error_message(err.message))
                }
                trader_chat::TraderChatErrorKind::NotFound => {
                    TraderChatApiResponse::NotFound(error_message(err.message))
                }
                trader_chat::TraderChatErrorKind::Conflict => {
                    TraderChatApiResponse::Conflict(error_message(err.message))
                }
                trader_chat::TraderChatErrorKind::Internal => {
                    TraderChatApiResponse::InternalError(error_message(err.message))
                }
            },
        }
    }

    #[oai(path = "/traders/:trader_id", method = "put", tag = "ApiTags::Trader")]
    async fn update_trader(
        &self,
        trader_id: Path<String>,
        request: Json<UpdateTraderRequest>,
    ) -> UpdateTraderResponse {
        match traders::update_trader(&self.database, &trader_id.0, request.0).await {
            Ok(trader) => UpdateTraderResponse::Ok(Json(trader)),
            Err(err) => match err.kind {
                traders::TraderErrorKind::BadRequest => {
                    UpdateTraderResponse::BadRequest(error_message(err.message))
                }
                traders::TraderErrorKind::NotFound => {
                    UpdateTraderResponse::NotFound(error_message(err.message))
                }
                _ => UpdateTraderResponse::InternalError(error_message(err.message)),
            },
        }
    }

    #[oai(
        path = "/traders/:trader_id",
        method = "delete",
        tag = "ApiTags::Trader"
    )]
    async fn delete_trader(&self, trader_id: Path<String>) -> DeleteTraderResponse {
        match traders::delete_trader(&self.database, &trader_id.0).await {
            Ok(()) => DeleteTraderResponse::Ok,
            Err(err) => match err.kind {
                traders::TraderErrorKind::NotFound => {
                    DeleteTraderResponse::NotFound(error_message(err.message))
                }
                _ => DeleteTraderResponse::InternalError(error_message(err.message)),
            },
        }
    }

    #[oai(
        path = "/traders/:trader_id/start",
        method = "post",
        tag = "ApiTags::Trader"
    )]
    async fn start_trader(&self, trader_id: Path<String>) -> TraderMutationResponse {
        map_trader_mutation(traders::set_status(&self.database, &trader_id.0, "running").await)
    }

    #[oai(
        path = "/traders/:trader_id/stop",
        method = "post",
        tag = "ApiTags::Trader"
    )]
    async fn stop_trader(&self, trader_id: Path<String>) -> TraderMutationResponse {
        map_trader_mutation(traders::set_status(&self.database, &trader_id.0, "stopped").await)
    }

    #[oai(
        path = "/traders/:trader_id/pause",
        method = "post",
        tag = "ApiTags::Trader"
    )]
    async fn pause_trader(&self, trader_id: Path<String>) -> TraderMutationResponse {
        map_trader_mutation(traders::set_status(&self.database, &trader_id.0, "paused").await)
    }

    #[oai(
        path = "/traders/:trader_id/events",
        method = "get",
        tag = "ApiTags::Trader"
    )]
    async fn get_trader_events(&self, trader_id: Path<String>) -> GetTraderEventsResponse {
        match traders::list_events(&self.database, &trader_id.0).await {
            Ok(events) => GetTraderEventsResponse::Ok(Json(events)),
            Err(err) => match err.kind {
                traders::TraderErrorKind::NotFound => {
                    GetTraderEventsResponse::NotFound(error_message(err.message))
                }
                _ => GetTraderEventsResponse::InternalError(error_message(err.message)),
            },
        }
    }

    #[oai(
        path = "/traders/:trader_id/runtime-state",
        method = "get",
        tag = "ApiTags::Trader"
    )]
    async fn get_trader_runtime_state(
        &self,
        trader_id: Path<String>,
    ) -> GetTraderRuntimeStateResponse {
        match traders::get_runtime_state(&self.database, &trader_id.0).await {
            Ok(state) => GetTraderRuntimeStateResponse::Ok(Json(state)),
            Err(err) => match err.kind {
                traders::TraderErrorKind::NotFound => {
                    GetTraderRuntimeStateResponse::NotFound(error_message(err.message))
                }
                _ => GetTraderRuntimeStateResponse::InternalError(error_message(err.message)),
            },
        }
    }

    #[oai(
        path = "/traders/:trader_id/trade-proposals",
        method = "get",
        tag = "ApiTags::Trader"
    )]
    async fn get_trader_trade_proposals(
        &self,
        trader_id: Path<String>,
    ) -> GetTraderTradeProposalsResponse {
        match traders::list_trade_proposals(&self.database, &trader_id.0).await {
            Ok(proposals) => GetTraderTradeProposalsResponse::Ok(Json(proposals)),
            Err(err) => match err.kind {
                traders::TraderErrorKind::NotFound => {
                    GetTraderTradeProposalsResponse::NotFound(error_message(err.message))
                }
                _ => GetTraderTradeProposalsResponse::InternalError(error_message(err.message)),
            },
        }
    }

    #[oai(
        path = "/traders/:trader_id/proposals",
        method = "get",
        tag = "ApiTags::Trader"
    )]
    async fn get_trader_portfolio_proposals(
        &self,
        trader_id: Path<String>,
    ) -> TraderPortfolioProposalsApiResponse {
        match traders::list_portfolio_proposals(&self.database, &trader_id.0).await {
            Ok(response) => TraderPortfolioProposalsApiResponse::Ok(Json(response)),
            Err(err) => map_trader_portfolio_proposals_error(err),
        }
    }

    #[oai(
        path = "/traders/:trader_id/proposals/latest",
        method = "get",
        tag = "ApiTags::Trader"
    )]
    async fn get_latest_trader_portfolio_proposal(
        &self,
        trader_id: Path<String>,
    ) -> TraderPortfolioProposalApiResponse {
        match traders::latest_portfolio_proposal(&self.database, &trader_id.0).await {
            Ok(proposal) => TraderPortfolioProposalApiResponse::Ok(Json(proposal)),
            Err(err) => map_trader_portfolio_proposal_error(err),
        }
    }

    #[oai(
        path = "/traders/:trader_id/proposals/active",
        method = "get",
        tag = "ApiTags::Trader"
    )]
    async fn get_active_trader_portfolio_proposal(
        &self,
        trader_id: Path<String>,
    ) -> TraderPortfolioProposalApiResponse {
        match traders::active_portfolio_proposal(&self.database, &trader_id.0).await {
            Ok(proposal) => TraderPortfolioProposalApiResponse::Ok(Json(proposal)),
            Err(err) => map_trader_portfolio_proposal_error(err),
        }
    }

    #[oai(
        path = "/traders/:trader_id/proposals/:proposal_id",
        method = "get",
        tag = "ApiTags::Trader"
    )]
    async fn get_trader_portfolio_proposal(
        &self,
        trader_id: Path<String>,
        proposal_id: Path<String>,
    ) -> TraderPortfolioProposalApiResponse {
        match traders::get_portfolio_proposal(&self.database, &trader_id.0, &proposal_id.0).await {
            Ok(proposal) => TraderPortfolioProposalApiResponse::Ok(Json(proposal)),
            Err(err) => map_trader_portfolio_proposal_error(err),
        }
    }

    #[oai(
        path = "/traders/:trader_id/proposals/:proposal_id/review",
        method = "post",
        tag = "ApiTags::Trader"
    )]
    async fn review_trader_portfolio_proposal(
        &self,
        trader_id: Path<String>,
        proposal_id: Path<String>,
        request: Json<ReviewTraderPortfolioProposalRequest>,
    ) -> TraderPortfolioProposalApiResponse {
        match traders::review_portfolio_proposal(
            &self.database,
            &trader_id.0,
            &proposal_id.0,
            request.0,
        )
        .await
        {
            Ok(proposal) => TraderPortfolioProposalApiResponse::Ok(Json(proposal)),
            Err(err) => map_trader_portfolio_proposal_error(err),
        }
    }

    #[oai(
        path = "/traders/:trader_id/proposals/:proposal_id/accept",
        method = "post",
        tag = "ApiTags::Trader"
    )]
    async fn accept_trader_portfolio_proposal(
        &self,
        trader_id: Path<String>,
        proposal_id: Path<String>,
        request: Json<ReviewTraderPortfolioProposalRequest>,
    ) -> TraderPortfolioProposalApiResponse {
        match traders::accept_portfolio_proposal(
            &self.database,
            &trader_id.0,
            &proposal_id.0,
            request.0.review_note,
        )
        .await
        {
            Ok(proposal) => TraderPortfolioProposalApiResponse::Ok(Json(proposal)),
            Err(err) => map_trader_portfolio_proposal_error(err),
        }
    }

    #[oai(
        path = "/traders/:trader_id/proposals/:proposal_id/reject",
        method = "post",
        tag = "ApiTags::Trader"
    )]
    async fn reject_trader_portfolio_proposal(
        &self,
        trader_id: Path<String>,
        proposal_id: Path<String>,
        request: Json<ReviewTraderPortfolioProposalRequest>,
    ) -> TraderPortfolioProposalApiResponse {
        match traders::reject_portfolio_proposal(
            &self.database,
            &trader_id.0,
            &proposal_id.0,
            request.0.review_note,
        )
        .await
        {
            Ok(proposal) => TraderPortfolioProposalApiResponse::Ok(Json(proposal)),
            Err(err) => map_trader_portfolio_proposal_error(err),
        }
    }

    #[oai(
        path = "/traders/:trader_id/trade-proposals/:proposal_id/approve",
        method = "post",
        tag = "ApiTags::Trader"
    )]
    async fn approve_trader_trade_proposal(
        &self,
        trader_id: Path<String>,
        proposal_id: Path<String>,
    ) -> TraderTradeProposalMutationResponse {
        match traders::approve_trade_proposal(
            &self.database,
            &self.cache,
            &trader_id.0,
            &proposal_id.0,
        )
        .await
        {
            Ok(proposal) => TraderTradeProposalMutationResponse::Ok(Json(proposal)),
            Err(err) => map_trader_proposal_error(err),
        }
    }

    #[oai(
        path = "/traders/:trader_id/trade-proposals/:proposal_id/reject",
        method = "post",
        tag = "ApiTags::Trader"
    )]
    async fn reject_trader_trade_proposal(
        &self,
        trader_id: Path<String>,
        proposal_id: Path<String>,
    ) -> TraderTradeProposalMutationResponse {
        match traders::reject_trade_proposal(&self.database, &trader_id.0, &proposal_id.0).await {
            Ok(proposal) => TraderTradeProposalMutationResponse::Ok(Json(proposal)),
            Err(err) => map_trader_proposal_error(err),
        }
    }

    #[oai(
        path = "/traders/:trader_id/symbols",
        method = "get",
        tag = "ApiTags::Trader"
    )]
    async fn list_trader_symbols(
        &self,
        trader_id: Path<String>,
        status: Query<Option<String>>,
        asset_type: Query<Option<String>>,
        source: Query<Option<String>>,
    ) -> TraderSymbolsApiResponse {
        match traders::list_symbols(
            &self.database,
            &trader_id.0,
            status.0.as_deref(),
            asset_type.0.as_deref(),
            source.0.as_deref(),
        )
        .await
        {
            Ok(response) => TraderSymbolsApiResponse::Ok(Json(response)),
            Err(err) => map_trader_symbols_error(err),
        }
    }

    #[oai(
        path = "/traders/:trader_id/symbols",
        method = "post",
        tag = "ApiTags::Trader"
    )]
    async fn create_trader_symbol(
        &self,
        trader_id: Path<String>,
        request: Json<CreateTraderSymbolRequest>,
    ) -> TraderSymbolMutationResponse {
        match traders::create_symbol(&self.database, &trader_id.0, request.0).await {
            Ok(symbol) => TraderSymbolMutationResponse::Created(Json(symbol)),
            Err(err) => map_trader_symbol_mutation_error(err),
        }
    }

    #[oai(
        path = "/traders/:trader_id/symbols/:symbol_id",
        method = "put",
        tag = "ApiTags::Trader"
    )]
    async fn update_trader_symbol(
        &self,
        trader_id: Path<String>,
        symbol_id: Path<String>,
        request: Json<UpdateTraderSymbolRequest>,
    ) -> TraderSymbolMutationResponse {
        match traders::update_symbol(&self.database, &trader_id.0, &symbol_id.0, request.0).await {
            Ok(symbol) => TraderSymbolMutationResponse::Ok(Json(symbol)),
            Err(err) => map_trader_symbol_mutation_error(err),
        }
    }

    #[oai(
        path = "/traders/:trader_id/symbols/:symbol_id",
        method = "delete",
        tag = "ApiTags::Trader"
    )]
    async fn delete_trader_symbol(
        &self,
        trader_id: Path<String>,
        symbol_id: Path<String>,
    ) -> TraderSymbolMutationResponse {
        match traders::set_symbol_status(&self.database, &trader_id.0, &symbol_id.0, "archived")
            .await
        {
            Ok(symbol) => TraderSymbolMutationResponse::Ok(Json(symbol)),
            Err(err) => map_trader_symbol_mutation_error(err),
        }
    }

    #[oai(
        path = "/traders/:trader_id/symbols/bulk",
        method = "post",
        tag = "ApiTags::Trader"
    )]
    async fn bulk_upsert_trader_symbols(
        &self,
        trader_id: Path<String>,
        request: Json<BulkUpsertTraderSymbolsRequest>,
    ) -> TraderSymbolsApiResponse {
        match traders::bulk_upsert_symbols(&self.database, &trader_id.0, request.0).await {
            Ok(response) => TraderSymbolsApiResponse::Ok(Json(response)),
            Err(err) => map_trader_symbols_error(err),
        }
    }

    #[oai(
        path = "/traders/:trader_id/symbols/suggest",
        method = "post",
        tag = "ApiTags::Trader"
    )]
    async fn suggest_trader_symbols(
        &self,
        trader_id: Path<String>,
        request: Json<SuggestTraderSymbolsRequest>,
    ) -> SuggestTraderSymbolsApiResponse {
        match traders::suggest_symbols(&self.database, &trader_id.0, request.0).await {
            Ok(response) => SuggestTraderSymbolsApiResponse::Ok(Json(response)),
            Err(err) => map_suggest_trader_symbols_error(err),
        }
    }

    #[oai(
        path = "/traders/:trader_id/symbols/:symbol_id/archive",
        method = "post",
        tag = "ApiTags::Trader"
    )]
    async fn archive_trader_symbol(
        &self,
        trader_id: Path<String>,
        symbol_id: Path<String>,
    ) -> TraderSymbolMutationResponse {
        match traders::set_symbol_status(&self.database, &trader_id.0, &symbol_id.0, "archived")
            .await
        {
            Ok(symbol) => TraderSymbolMutationResponse::Ok(Json(symbol)),
            Err(err) => map_trader_symbol_mutation_error(err),
        }
    }

    #[oai(
        path = "/traders/:trader_id/symbols/:symbol_id/activate",
        method = "post",
        tag = "ApiTags::Trader"
    )]
    async fn activate_trader_symbol(
        &self,
        trader_id: Path<String>,
        symbol_id: Path<String>,
    ) -> TraderSymbolMutationResponse {
        match traders::set_symbol_status(&self.database, &trader_id.0, &symbol_id.0, "active").await
        {
            Ok(symbol) => TraderSymbolMutationResponse::Ok(Json(symbol)),
            Err(err) => map_trader_symbol_mutation_error(err),
        }
    }

    #[oai(
        path = "/traders/:trader_id/symbols/:symbol_id/reject",
        method = "post",
        tag = "ApiTags::Trader"
    )]
    async fn reject_trader_symbol(
        &self,
        trader_id: Path<String>,
        symbol_id: Path<String>,
    ) -> TraderSymbolMutationResponse {
        match traders::set_symbol_status(&self.database, &trader_id.0, &symbol_id.0, "rejected")
            .await
        {
            Ok(symbol) => TraderSymbolMutationResponse::Ok(Json(symbol)),
            Err(err) => map_trader_symbol_mutation_error(err),
        }
    }

    #[oai(
        path = "/traders/:trader_id/data-sources",
        method = "get",
        tag = "ApiTags::Trader"
    )]
    async fn get_trader_data_sources(
        &self,
        trader_id: Path<String>,
    ) -> TraderDataSourcesApiResponse {
        match data_sources::trader_sources(&self.database, &trader_id.0).await {
            Ok(response) => TraderDataSourcesApiResponse::Ok(Json(response)),
            Err(err) => map_data_source_trader_error(err),
        }
    }

    #[oai(
        path = "/traders/:trader_id/data-sources",
        method = "put",
        tag = "ApiTags::Trader"
    )]
    async fn update_trader_data_sources(
        &self,
        trader_id: Path<String>,
        request: Json<UpdateTraderDataSourcesRequest>,
    ) -> TraderDataSourcesApiResponse {
        match data_sources::replace_trader_sources(&self.database, &trader_id.0, request.0).await {
            Ok(response) => TraderDataSourcesApiResponse::Ok(Json(response)),
            Err(err) => map_data_source_trader_error(err),
        }
    }

    // ----------------------------
    // Data Sources
    // ----------------------------

    #[oai(path = "/data-sources", method = "post", tag = "ApiTags::DataSource")]
    async fn create_data_source(
        &self,
        request: Json<CreateDataSourceRequest>,
    ) -> CreateDataSourceResponse {
        match data_sources::create(&self.database, request.0).await {
            Ok(source) => CreateDataSourceResponse::Created(Json(source)),
            Err(err) => match err.kind {
                data_sources::DataSourceErrorKind::BadRequest => {
                    CreateDataSourceResponse::BadRequest(error_message(err.message))
                }
                _ => CreateDataSourceResponse::InternalError(error_message(err.message)),
            },
        }
    }

    #[oai(path = "/data-sources", method = "get", tag = "ApiTags::DataSource")]
    async fn list_data_sources(&self) -> ListDataSourcesResponse {
        match data_sources::list(&self.database).await {
            Ok(sources) => ListDataSourcesResponse::Ok(Json(sources)),
            Err(err) => ListDataSourcesResponse::InternalError(error_message(err.message)),
        }
    }

    #[oai(
        path = "/data-sources/:source_id",
        method = "get",
        tag = "ApiTags::DataSource"
    )]
    async fn get_data_source(&self, source_id: Path<String>) -> GetDataSourceResponse {
        match data_sources::get(&self.database, &source_id.0).await {
            Ok(source) => GetDataSourceResponse::Ok(Json(source)),
            Err(err) => match err.kind {
                data_sources::DataSourceErrorKind::NotFound => {
                    GetDataSourceResponse::NotFound(error_message(err.message))
                }
                _ => GetDataSourceResponse::InternalError(error_message(err.message)),
            },
        }
    }

    #[oai(
        path = "/data-sources/:source_id",
        method = "put",
        tag = "ApiTags::DataSource"
    )]
    async fn update_data_source(
        &self,
        source_id: Path<String>,
        request: Json<UpdateDataSourceRequest>,
    ) -> UpdateDataSourceResponse {
        match data_sources::update(&self.database, &source_id.0, request.0).await {
            Ok(source) => UpdateDataSourceResponse::Ok(Json(source)),
            Err(err) => match err.kind {
                data_sources::DataSourceErrorKind::BadRequest => {
                    UpdateDataSourceResponse::BadRequest(error_message(err.message))
                }
                data_sources::DataSourceErrorKind::NotFound => {
                    UpdateDataSourceResponse::NotFound(error_message(err.message))
                }
                _ => UpdateDataSourceResponse::InternalError(error_message(err.message)),
            },
        }
    }

    #[oai(
        path = "/data-sources/:source_id",
        method = "delete",
        tag = "ApiTags::DataSource"
    )]
    async fn delete_data_source(&self, source_id: Path<String>) -> DeleteDataSourceResponse {
        match data_sources::delete(&self.database, &source_id.0).await {
            Ok(()) => DeleteDataSourceResponse::Ok,
            Err(err) => match err.kind {
                data_sources::DataSourceErrorKind::NotFound => {
                    DeleteDataSourceResponse::NotFound(error_message(err.message))
                }
                _ => DeleteDataSourceResponse::InternalError(error_message(err.message)),
            },
        }
    }

    #[oai(
        path = "/data-sources/:source_id/script",
        method = "get",
        tag = "ApiTags::DataSource"
    )]
    async fn get_data_source_script(&self, source_id: Path<String>) -> GetDataSourceScriptResponse {
        match data_sources::get_script(&self.database, &source_id.0).await {
            Ok(script) => GetDataSourceScriptResponse::Ok(Json(script)),
            Err(err) => map_data_source_script_error(err),
        }
    }

    #[oai(
        path = "/data-sources/:source_id/script",
        method = "put",
        tag = "ApiTags::DataSource"
    )]
    async fn update_data_source_script(
        &self,
        source_id: Path<String>,
        request: Json<UpdateDataSourceScriptRequest>,
    ) -> UpdateDataSourceScriptResponse {
        match data_sources::update_script(&self.database, &source_id.0, request.0).await {
            Ok(script) => UpdateDataSourceScriptResponse::Ok(Json(script)),
            Err(err) => map_update_data_source_script_error(err),
        }
    }

    #[oai(
        path = "/data-sources/:source_id/script/build",
        method = "post",
        tag = "ApiTags::DataSource"
    )]
    async fn build_data_source_script(
        &self,
        source_id: Path<String>,
        request: Json<BuildDataSourceScriptRequest>,
    ) -> BuildDataSourceScriptApiResponse {
        match data_sources::build_script(&self.database, &source_id.0, request.0).await {
            Ok(response) => BuildDataSourceScriptApiResponse::Ok(Json(response)),
            Err(err) => map_build_data_source_script_error(err),
        }
    }

    #[oai(
        path = "/engine/data-sources/:source_id/script",
        method = "get",
        tag = "ApiTags::Engine"
    )]
    async fn get_engine_data_source_script(
        &self,
        source_id: Path<String>,
    ) -> GetDataSourceScriptResponse {
        match data_sources::engine_script(&self.database, &source_id.0).await {
            Ok(script) => GetDataSourceScriptResponse::Ok(Json(script)),
            Err(err) => map_data_source_script_error(err),
        }
    }

    #[oai(
        path = "/data-sources/:source_id/items",
        method = "get",
        tag = "ApiTags::DataSource"
    )]
    async fn get_data_source_items(&self, source_id: Path<String>) -> GetDataSourceItemsResponse {
        match data_sources::items(&self.database, &source_id.0).await {
            Ok(response) => GetDataSourceItemsResponse::Ok(Json(response)),
            Err(err) => match err.kind {
                data_sources::DataSourceErrorKind::NotFound => {
                    GetDataSourceItemsResponse::NotFound(error_message(err.message))
                }
                _ => GetDataSourceItemsResponse::InternalError(error_message(err.message)),
            },
        }
    }

    #[oai(
        path = "/data-sources/:source_id/events",
        method = "get",
        tag = "ApiTags::DataSource"
    )]
    async fn get_data_source_events(&self, source_id: Path<String>) -> GetDataSourceEventsResponse {
        match data_sources::events(&self.database, &source_id.0).await {
            Ok(response) => GetDataSourceEventsResponse::Ok(Json(response)),
            Err(err) => match err.kind {
                data_sources::DataSourceErrorKind::NotFound => {
                    GetDataSourceEventsResponse::NotFound(error_message(err.message))
                }
                _ => GetDataSourceEventsResponse::InternalError(error_message(err.message)),
            },
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

fn map_trader_mutation(
    result: Result<models::trader::Trader, traders::TraderApiError>,
) -> TraderMutationResponse {
    match result {
        Ok(trader) => TraderMutationResponse::Ok(Json(trader)),
        Err(err) => match err.kind {
            traders::TraderErrorKind::BadRequest => {
                TraderMutationResponse::BadRequest(error_message(err.message))
            }
            traders::TraderErrorKind::NotFound => {
                TraderMutationResponse::NotFound(error_message(err.message))
            }
            traders::TraderErrorKind::Conflict => {
                TraderMutationResponse::Conflict(error_message(err.message))
            }
            traders::TraderErrorKind::Internal => {
                TraderMutationResponse::InternalError(error_message(err.message))
            }
        },
    }
}

fn map_channel_message_error(err: channels::ChannelApiError) -> CreateChannelMessageResponse {
    match err.kind {
        channels::ChannelErrorKind::BadRequest => {
            CreateChannelMessageResponse::BadRequest(error_message(err.message))
        }
        channels::ChannelErrorKind::NotFound => {
            CreateChannelMessageResponse::NotFound(error_message(err.message))
        }
        channels::ChannelErrorKind::Internal => {
            CreateChannelMessageResponse::InternalError(error_message(err.message))
        }
    }
}

fn map_trader_proposal_error(err: traders::TraderApiError) -> TraderTradeProposalMutationResponse {
    match err.kind {
        traders::TraderErrorKind::BadRequest => {
            TraderTradeProposalMutationResponse::BadRequest(error_message(err.message))
        }
        traders::TraderErrorKind::NotFound => {
            TraderTradeProposalMutationResponse::NotFound(error_message(err.message))
        }
        traders::TraderErrorKind::Conflict => {
            TraderTradeProposalMutationResponse::Conflict(error_message(err.message))
        }
        traders::TraderErrorKind::Internal => {
            TraderTradeProposalMutationResponse::InternalError(error_message(err.message))
        }
    }
}

fn map_trader_portfolio_proposals_error(
    err: traders::TraderApiError,
) -> TraderPortfolioProposalsApiResponse {
    match err.kind {
        traders::TraderErrorKind::NotFound => {
            TraderPortfolioProposalsApiResponse::NotFound(error_message(err.message))
        }
        traders::TraderErrorKind::BadRequest
        | traders::TraderErrorKind::Conflict
        | traders::TraderErrorKind::Internal => {
            TraderPortfolioProposalsApiResponse::InternalError(error_message(err.message))
        }
    }
}

fn map_trader_portfolio_proposal_error(
    err: traders::TraderApiError,
) -> TraderPortfolioProposalApiResponse {
    match err.kind {
        traders::TraderErrorKind::BadRequest => {
            TraderPortfolioProposalApiResponse::BadRequest(error_message(err.message))
        }
        traders::TraderErrorKind::NotFound => {
            TraderPortfolioProposalApiResponse::NotFound(error_message(err.message))
        }
        traders::TraderErrorKind::Conflict => {
            TraderPortfolioProposalApiResponse::Conflict(error_message(err.message))
        }
        traders::TraderErrorKind::Internal => {
            TraderPortfolioProposalApiResponse::InternalError(error_message(err.message))
        }
    }
}

fn map_trader_symbols_error(err: traders::TraderApiError) -> TraderSymbolsApiResponse {
    match err.kind {
        traders::TraderErrorKind::BadRequest => {
            TraderSymbolsApiResponse::BadRequest(error_message(err.message))
        }
        traders::TraderErrorKind::NotFound => {
            TraderSymbolsApiResponse::NotFound(error_message(err.message))
        }
        traders::TraderErrorKind::Conflict | traders::TraderErrorKind::Internal => {
            TraderSymbolsApiResponse::InternalError(error_message(err.message))
        }
    }
}

fn map_trader_symbol_mutation_error(err: traders::TraderApiError) -> TraderSymbolMutationResponse {
    match err.kind {
        traders::TraderErrorKind::BadRequest => {
            TraderSymbolMutationResponse::BadRequest(error_message(err.message))
        }
        traders::TraderErrorKind::NotFound => {
            TraderSymbolMutationResponse::NotFound(error_message(err.message))
        }
        traders::TraderErrorKind::Conflict => {
            TraderSymbolMutationResponse::Conflict(error_message(err.message))
        }
        traders::TraderErrorKind::Internal => {
            TraderSymbolMutationResponse::InternalError(error_message(err.message))
        }
    }
}

fn map_suggest_trader_symbols_error(
    err: traders::TraderApiError,
) -> SuggestTraderSymbolsApiResponse {
    match err.kind {
        traders::TraderErrorKind::BadRequest => {
            SuggestTraderSymbolsApiResponse::BadRequest(error_message(err.message))
        }
        traders::TraderErrorKind::NotFound => {
            SuggestTraderSymbolsApiResponse::NotFound(error_message(err.message))
        }
        traders::TraderErrorKind::Conflict => {
            SuggestTraderSymbolsApiResponse::Conflict(error_message(err.message))
        }
        traders::TraderErrorKind::Internal => {
            SuggestTraderSymbolsApiResponse::InternalError(error_message(err.message))
        }
    }
}

fn map_data_source_trader_error(
    err: data_sources::DataSourceApiError,
) -> TraderDataSourcesApiResponse {
    match err.kind {
        data_sources::DataSourceErrorKind::BadRequest => {
            TraderDataSourcesApiResponse::BadRequest(error_message(err.message))
        }
        data_sources::DataSourceErrorKind::NotFound => {
            TraderDataSourcesApiResponse::NotFound(error_message(err.message))
        }
        data_sources::DataSourceErrorKind::Internal => {
            TraderDataSourcesApiResponse::InternalError(error_message(err.message))
        }
    }
}

fn map_data_source_script_error(
    err: data_sources::DataSourceApiError,
) -> GetDataSourceScriptResponse {
    match err.kind {
        data_sources::DataSourceErrorKind::BadRequest => {
            GetDataSourceScriptResponse::BadRequest(error_message(err.message))
        }
        data_sources::DataSourceErrorKind::NotFound => {
            GetDataSourceScriptResponse::NotFound(error_message(err.message))
        }
        data_sources::DataSourceErrorKind::Internal => {
            GetDataSourceScriptResponse::InternalError(error_message(err.message))
        }
    }
}

fn map_update_data_source_script_error(
    err: data_sources::DataSourceApiError,
) -> UpdateDataSourceScriptResponse {
    match err.kind {
        data_sources::DataSourceErrorKind::BadRequest => {
            UpdateDataSourceScriptResponse::BadRequest(error_message(err.message))
        }
        data_sources::DataSourceErrorKind::NotFound => {
            UpdateDataSourceScriptResponse::NotFound(error_message(err.message))
        }
        data_sources::DataSourceErrorKind::Internal => {
            UpdateDataSourceScriptResponse::InternalError(error_message(err.message))
        }
    }
}

fn map_build_data_source_script_error(
    err: data_sources::DataSourceApiError,
) -> BuildDataSourceScriptApiResponse {
    match err.kind {
        data_sources::DataSourceErrorKind::BadRequest => {
            BuildDataSourceScriptApiResponse::BadRequest(error_message(err.message))
        }
        data_sources::DataSourceErrorKind::NotFound => {
            BuildDataSourceScriptApiResponse::NotFound(error_message(err.message))
        }
        data_sources::DataSourceErrorKind::Internal => {
            BuildDataSourceScriptApiResponse::InternalError(error_message(err.message))
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .init();
    let _ = FmtSubscriber::builder()
        .with_max_level(tracing::Level::INFO)
        .try_init();

    let api_bind_address =
        env::var("API_BIND_ADDRESS").unwrap_or_else(|_| "0.0.0.0:3000".to_string());
    let api_public_base_url =
        env::var("API_PUBLIC_BASE_URL").unwrap_or_else(|_| "http://localhost:3000/api".to_string());
    let cache_dir = env::var("CACHE_DIR").unwrap_or_else(|_| "cache_data".to_string());
    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://desk:desk@localhost:5432/desk".to_string());
    let cors_allow_origin =
        env::var("CORS_ALLOW_ORIGIN").unwrap_or_else(|_| "http://localhost:5173".to_string());

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
    let cors = Cors::new()
        .allow_origin(HeaderValue::from_str(&cors_allow_origin).map_err(|err| {
            std::io::Error::other(format!("invalid CORS_ALLOW_ORIGIN value: {err}"))
        })?)
        .allow_method(Method::GET)
        .allow_method(Method::POST)
        .allow_method(Method::PUT)
        .allow_method(Method::DELETE)
        .allow_method(Method::OPTIONS)
        .allow_credentials(false);
    let app = Route::new()
        .nest("/api", api_service)
        .nest("/", ui)
        .with(cors);

    info!("Starting server on http://{api_bind_address}");
    poem::Server::new(TcpListener::bind(api_bind_address))
        .run(app)
        .await
}
