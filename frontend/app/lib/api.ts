export type Project = {
  id: string;
  name: string;
  description: string;
  strategy: string;
  strategy_json: string;
  strategy_status: string;
  created_at: string;
  updated_at: string;
  symbols: string[];
  interval: string;
  range: string;
  prepost: boolean;
};

export type TradingMode = "off" | "paper" | "real";

export type Position = {
  symbol: string;
  quantity: number;
  average_price: number;
  position_opened_at: string;
  position_closed_at: string | null;
  position_closed_price: number | null;
};

export type Portfolio = {
  id: string;
  name: string;
  description: string;
  created_at: string;
  updated_at: string;
  positions: Position[];
};

export type RawStockDataEntry = {
  date: string;
  open: string;
  high: string;
  low: string;
  close: string;
  volume: string;
};

export type RawStockData = {
  symbol: string;
  last_refreshed: string;
  interval: string;
  range: string;
  stock_data: RawStockDataEntry[];
};

export type IndicatorPoint = {
  date: string;
  value: number;
};

export type IndicatorLine = {
  key: string;
  label: string;
  points: IndicatorPoint[];
};

export type IndicatorResult = {
  key: string;
  display_name: string;
  overlay: boolean;
  lines: IndicatorLine[];
};

export type StockIndicatorsResponse = {
  symbol: string;
  last_refreshed: string;
  interval: string;
  range: string;
  indicators: IndicatorResult[];
  unsupported: string[];
};

export interface PaperAccount {
  id: string;
  name: string;
  starting_cash: number;
  cash_balance: number;
  currency: string;
  is_active: boolean;
  created_at: string;
  updated_at: string;
}

export interface LiveAccount {
  id: string;
  name: string;
  is_active: boolean;
}

export type TradingAccountKind = "paper" | "live" | "manual";

export interface TradingAccountOption {
  id: string;
  name: string;
  kind: TradingAccountKind;
  label: string;
  is_active: boolean;
}

export interface PaperPosition {
  id: string;
  account_id: string;
  symbol: string;
  quantity: number;
  average_price: number;
  realized_pnl: number;
  created_at: string;
  updated_at: string;
}

export interface PaperPositionSummary extends PaperPosition {
  current_price?: number | null;
  market_value?: number | null;
  cost_basis: number;
  unrealized_gain: number;
  unrealized_gain_percent: number;
}

export interface PaperOrder {
  id: string;
  account_id: string;
  symbol: string;
  side: string;
  order_type: string;
  quantity: number;
  requested_price?: number | null;
  filled_quantity: number;
  average_fill_price?: number | null;
  status: string;
  source: string;
  trader_id?: string | null;
  strategy_id?: string | null;
  signal_id?: string | null;
  proposal_id?: string | null;
  created_at: string;
  updated_at: string;
}

export interface PaperFill {
  id: string;
  account_id: string;
  order_id: string;
  symbol: string;
  side: string;
  quantity: number;
  price: number;
  notional: number;
  created_at: string;
}

export interface PaperAccountSummaryResponse {
  account: PaperAccount;
  positions: PaperPositionSummary[];
  open_orders: PaperOrder[];
  recent_fills: PaperFill[];
  equity_estimate: number;
  total_cost_basis?: number;
  total_market_value?: number;
  total_unrealized_gain?: number;
  total_unrealized_gain_percent?: number;
}

export interface PaperOrderExecutionResponse {
  order: PaperOrder;
  fill?: PaperFill | null;
  position?: PaperPosition | null;
  cash_balance: number;
}

export interface StrategyTradingConfig {
  strategy_id: string;
  trading_mode: TradingMode;
  paper_account_id: string | null;
  is_enabled: boolean;
  last_started_at?: string | null;
  last_stopped_at?: string | null;
  created_at?: string | null;
  updated_at?: string | null;
}

export interface StrategyRiskConfig {
  strategy_id: string;
  max_dollars_per_trade?: number | null;
  max_quantity_per_trade?: number | null;
  max_position_value_per_symbol?: number | null;
  max_total_exposure?: number | null;
  max_open_positions?: number | null;
  max_daily_trades?: number | null;
  max_daily_loss?: number | null;
  cooldown_seconds: number;
  allowlist_symbols?: string[] | null;
  blocklist_symbols?: string[] | null;
  is_trading_enabled: boolean;
  kill_switch_enabled: boolean;
  created_at?: string | null;
  updated_at?: string | null;
}

export interface UpdateStrategyRiskConfigRequest {
  max_dollars_per_trade?: number | null;
  max_quantity_per_trade?: number | null;
  max_position_value_per_symbol?: number | null;
  max_total_exposure?: number | null;
  max_open_positions?: number | null;
  max_daily_trades?: number | null;
  max_daily_loss?: number | null;
  cooldown_seconds: number;
  allowlist_symbols?: string[] | null;
  blocklist_symbols?: string[] | null;
  is_trading_enabled: boolean;
  kill_switch_enabled: boolean;
}

export interface StrategyCondition {
  indicator: string;
  period?: number | null;
  operator: string;
  value?: number | null;
  compare_indicator?: string | null;
  compare_period?: number | null;
}

export interface StrategyConditionGroup {
  all?: StrategyCondition[] | null;
  any?: StrategyCondition[] | null;
}

export interface StrategyPositionSize {
  type: string;
  quantity?: number | null;
  percent?: number | null;
}

export interface StrategyRisk {
  position_size: StrategyPositionSize;
  max_position_per_symbol?: number | null;
  cooldown_seconds?: number | null;
}

export interface StrategyDefinition {
  version: string;
  entry: StrategyConditionGroup;
  exit: StrategyConditionGroup;
  risk: StrategyRisk;
}

export interface StrategyRuntimeState {
  id: string;
  strategy_id: string;
  paper_account_id: string;
  symbol: string;
  last_evaluated_at?: string | null;
  last_signal?: string | null;
  last_signal_at?: string | null;
  last_order_id?: string | null;
  position_state: string;
  cooldown_until?: string | null;
  created_at: string;
  updated_at: string;
}

export interface StrategySignal {
  id: string;
  strategy_id: string;
  paper_account_id: string;
  symbol: string;
  signal_type: string;
  confidence?: number | null;
  reason: string;
  market_price?: number | null;
  source: string;
  status: string;
  risk_decision?: string | null;
  risk_reason?: string | null;
  order_id?: string | null;
  created_at: string;
}

export interface StrategyRuntimeStateListResponse {
  states: StrategyRuntimeState[];
}

export interface StrategySignalListResponse {
  signals: StrategySignal[];
}

export interface EngineRunnableStrategy {
  strategy_id: string;
  name: string;
  trading_mode: string;
  paper_account_id: string;
  symbol_universe: string[];
  timeframe: string;
  strategy_definition: StrategyDefinition;
  risk: StrategyRisk;
  risk_config: StrategyRiskConfig;
}

export interface UpdateStrategyTradingConfigRequest {
  trading_mode: TradingMode;
  paper_account_id: string | null;
  is_enabled: boolean;
}

export interface EngineStrategyConfigResponse {
  strategies: EngineRunnableStrategy[];
}

export type TraderFreedomLevel = "analyst" | "junior_trader" | "senior_trader";
export type TraderStatus = "stopped" | "running" | "paused";

export interface Trader {
  id: string;
  name: string;
  fundamental_perspective: string;
  freedom_level: TraderFreedomLevel;
  status: TraderStatus;
  default_paper_account_id?: string | null;
  persona?: string | null;
  tone?: string | null;
  communication_style?: string | null;
  is_active: boolean;
  created_at: string;
  updated_at: string;
  started_at?: string | null;
  stopped_at?: string | null;
}

export interface Channel {
  id: string;
  name: string;
  display_name: string;
  description?: string | null;
  is_system: boolean;
  created_at: string;
  updated_at: string;
}

export type ChannelAuthorType = "user" | "trader" | "md" | "system";

export interface ChannelMessage {
  id: string;
  channel_id: string;
  author_type: ChannelAuthorType;
  author_id?: string | null;
  author_name: string;
  role: string;
  content_markdown: string;
  metadata_json?: string | null;
  created_at: string;
}

export interface ChannelMessagesResponse {
  messages: ChannelMessage[];
}

export interface TraderPersona {
  trader_id: string;
  persona?: string | null;
  tone?: string | null;
  communication_style?: string | null;
}

export interface TraderPersonaUpdateRequest {
  persona?: string | null;
  tone?: string | null;
  communication_style?: string | null;
}

export interface TraderMemory {
  id: string;
  trader_id: string;
  memory_type: string;
  topic: string;
  summary: string;
  source_channel_id?: string | null;
  source_message_id?: string | null;
  confidence?: number | null;
  importance: number;
  status: string;
  last_used_at?: string | null;
  created_at: string;
  updated_at: string;
}

export interface CreateTraderMemoryRequest {
  memory_type: string;
  topic: string;
  summary: string;
  source_channel_id?: string | null;
  source_message_id?: string | null;
  confidence?: number | null;
  importance?: number | null;
}

export interface UpdateTraderMemoryRequest {
  memory_type?: string | null;
  topic?: string | null;
  summary?: string | null;
  confidence?: number | null;
  importance?: number | null;
  status?: string | null;
}

export interface TraderMemorySearchResponse {
  memories: TraderMemory[];
}

export interface MdProfile {
  id: string;
  name: string;
  persona: string;
  tone: string;
  communication_style: string;
  created_at: string;
  updated_at: string;
}

export interface UpdateMdProfileRequest {
  name?: string | null;
  persona?: string | null;
  tone?: string | null;
  communication_style?: string | null;
  openai_api_key?: string | null;
}

export interface DataScientistProfile {
  id: string;
  name: string;
  persona: string;
  tone: string;
  communication_style: string;
  created_at: string;
  updated_at: string;
}

export interface UpdateDataScientistProfileRequest {
  name?: string | null;
  persona?: string | null;
  tone?: string | null;
  communication_style?: string | null;
  openai_api_key?: string | null;
}

export interface UserInvestorProfile {
  id: string;
  name?: string | null;
  age?: number | null;
  about?: string | null;
  investment_goals?: string | null;
  risk_tolerance?: string | null;
  time_horizon?: string | null;
  liquidity_needs?: string | null;
  income_needs?: string | null;
  investment_experience?: string | null;
  restrictions?: string | null;
  preferred_sectors?: string | null;
  avoided_sectors?: string | null;
  notes?: string | null;
  created_at: string;
  updated_at: string;
}

export type UpdateUserInvestorProfileRequest = Omit<
  UserInvestorProfile,
  "id" | "created_at" | "updated_at"
>;

export interface TraderInfoSource {
  id: string;
  trader_id: string;
  source_type: string;
  name: string;
  config_json?: string | null;
  enabled: boolean;
  created_at: string;
  updated_at: string;
}

export interface CreateTraderInfoSourceRequest {
  source_type: string;
  name: string;
  config_json?: string | null;
  enabled?: boolean | null;
}

export interface CreateTraderRequest {
  name: string;
  fundamental_perspective: string;
  freedom_level: TraderFreedomLevel;
  default_paper_account_id?: string | null;
  openai_api_key: string;
  info_sources: CreateTraderInfoSourceRequest[];
}

export interface UpdateTraderRequest {
  name?: string;
  fundamental_perspective?: string;
  freedom_level?: TraderFreedomLevel;
  default_paper_account_id?: string | null;
  openai_api_key?: string;
  info_sources?: CreateTraderInfoSourceRequest[];
}

export interface TraderRuntimeState {
  trader_id: string;
  engine_name?: string | null;
  last_heartbeat_at?: string | null;
  last_evaluation_at?: string | null;
  last_error?: string | null;
  current_task?: string | null;
  created_at: string;
  updated_at: string;
}

export interface TraderEvent {
  id: string;
  trader_id: string;
  event_type: string;
  message: string;
  payload?: string | null;
  created_at: string;
}

export interface TraderTradeProposal {
  id: string;
  trader_id: string;
  symbol: string;
  side: "buy" | "sell";
  quantity: number;
  order_type: string;
  reason: string;
  confidence?: number | null;
  status: string;
  reviewed_by?: string | null;
  reviewed_at?: string | null;
  resulting_order_id?: string | null;
  created_at: string;
  updated_at: string;
}

export type TraderSymbolAssetType = "stock" | "etf" | "index" | "crypto" | "other";
export type TraderSymbolStatus = "watching" | "candidate" | "active" | "rejected" | "archived";
export type TraderSymbolSource = "manual" | "ai" | "import" | "engine";

export interface TraderSymbol {
  id: string;
  trader_id: string;
  symbol: string;
  asset_type: TraderSymbolAssetType;
  name?: string | null;
  exchange?: string | null;
  sector?: string | null;
  industry?: string | null;
  notes?: string | null;
  thesis?: string | null;
  fit_score?: number | null;
  status: TraderSymbolStatus;
  source: TraderSymbolSource;
  created_at: string;
  updated_at: string;
}

export interface CreateTraderSymbolRequest {
  symbol: string;
  asset_type?: TraderSymbolAssetType | null;
  name?: string | null;
  exchange?: string | null;
  sector?: string | null;
  industry?: string | null;
  notes?: string | null;
  thesis?: string | null;
  fit_score?: number | null;
  status?: TraderSymbolStatus | null;
  source?: TraderSymbolSource | null;
}

export interface UpdateTraderSymbolRequest {
  asset_type?: TraderSymbolAssetType | null;
  name?: string | null;
  exchange?: string | null;
  sector?: string | null;
  industry?: string | null;
  notes?: string | null;
  thesis?: string | null;
  fit_score?: number | null;
  status?: TraderSymbolStatus | null;
}

export interface TraderSymbolsResponse {
  symbols: TraderSymbol[];
}

export interface SuggestTraderSymbolsRequest {
  max_symbols?: number | null;
  include_etfs?: boolean | null;
  include_stocks?: boolean | null;
  focus?: string | null;
}

export interface SuggestTraderSymbolsResponse {
  suggestions: TraderSymbol[];
}

export interface TraderPortfolioProposal {
  id: string;
  trader_id: string;
  paper_account_id?: string | null;
  title: string;
  summary: string;
  thesis: string;
  status: string;
  plan_state: string;
  confidence?: number | null;
  proposed_actions_json: string;
  source_snapshot_json?: string | null;
  risk_snapshot_json?: string | null;
  market_snapshot_json?: string | null;
  market_basis_json?: string | null;
  invalidation_conditions_json?: string | null;
  change_thresholds_json?: string | null;
  replacement_reason?: string | null;
  created_at: string;
  updated_at: string;
  reviewed_at?: string | null;
  review_note?: string | null;
  accepted_at?: string | null;
  active_until?: string | null;
  expected_duration_seconds?: number | null;
}

export interface TraderPortfolioProposalAction {
  id: string;
  proposal_id: string;
  trader_id: string;
  symbol?: string | null;
  action_type: string;
  side?: string | null;
  quantity?: number | null;
  order_type?: string | null;
  entry_price?: number | null;
  exit_price?: number | null;
  limit_price?: number | null;
  stop_price?: number | null;
  expected_duration_seconds?: number | null;
  enact_by?: string | null;
  market_price_at_creation?: number | null;
  rationale: string;
  confidence?: number | null;
  risk_decision?: string | null;
  status: string;
  created_at: string;
  updated_at: string;
}

export interface TraderPortfolioProposalDetail {
  proposal: TraderPortfolioProposal;
  actions: TraderPortfolioProposalAction[];
}

export interface TraderPortfolioProposalsResponse {
  proposals: TraderPortfolioProposalDetail[];
}

export interface TraderDetail {
  trader: Trader;
  info_sources: TraderInfoSource[];
  runtime_state?: TraderRuntimeState | null;
  recent_events: TraderEvent[];
  tracked_symbols: TraderSymbol[];
}

export interface TraderEventsResponse {
  events: TraderEvent[];
}

export interface TraderTradeProposalsResponse {
  proposals: TraderTradeProposal[];
}

export type DataSourceType =
  | "rss"
  | "web_page"
  | "manual_note"
  | "placeholder_api"
  | "python_script";

export interface DataSource {
  id: string;
  name: string;
  source_type: DataSourceType;
  url?: string | null;
  config_json?: string | null;
  enabled: boolean;
  poll_interval_seconds: number;
  last_checked_at?: string | null;
  last_success_at?: string | null;
  last_error?: string | null;
  created_at: string;
  updated_at: string;
}

export interface CreateDataSourceRequest {
  name: string;
  source_type: DataSourceType;
  url?: string | null;
  config_json?: string | null;
  enabled: boolean;
  poll_interval_seconds?: number | null;
}

export interface UpdateDataSourceRequest {
  name?: string;
  source_type?: DataSourceType;
  url?: string | null;
  config_json?: string | null;
  enabled?: boolean;
  poll_interval_seconds?: number | null;
}

export interface DataSourceItem {
  id: string;
  data_source_id: string;
  external_id?: string | null;
  title: string;
  url?: string | null;
  content?: string | null;
  summary?: string | null;
  raw_payload?: string | null;
  published_at?: string | null;
  discovered_at: string;
  created_at: string;
}

export interface DataSourceEvent {
  id: string;
  data_source_id?: string | null;
  event_type: string;
  message: string;
  payload?: string | null;
  created_at: string;
}

export interface DataSourceItemsResponse {
  items: DataSourceItem[];
}

export interface DataSourceEventsResponse {
  events: DataSourceEvent[];
}

export interface DataSourceScript {
  data_source_id: string;
  language: string;
  script_text: string;
  script_hash?: string | null;
  last_build_status?: string | null;
  last_build_output?: string | null;
  last_built_at?: string | null;
  created_at: string;
  updated_at: string;
}

export interface BuildDataSourceScriptResponse {
  success: boolean;
  status: string;
  output: string;
  script_hash?: string | null;
}

export interface TraderDataSourcesResponse {
  data_sources: DataSource[];
}

export interface ChatCommandAction {
  type: string;
  entity_id?: string | null;
  message?: string | null;
}

export interface ChatCommandResponse {
  reply: string;
  actions: ChatCommandAction[];
  handled: boolean;
  requires_confirmation: boolean;
  confirmation_token?: string | null;
}

export interface TraderChatMessage {
  role: "user" | "assistant";
  content: string;
}

export interface TraderChatResponse {
  reply: string;
  trader_id: string;
  trader_name: string;
  referenced_events: string[];
  referenced_proposals: string[];
  referenced_orders: string[];
  actions: TraderChatAction[];
}

export interface TraderChatAction {
  type: string;
  entity_id?: string | null;
  title?: string | null;
  status?: string | null;
}

export interface AgentChatMessage {
  role: "user" | "assistant";
  content: string;
}

export interface MdChatResponse {
  reply: string;
  referenced_channels: string[];
  referenced_traders: string[];
  referenced_events: string[];
}

export interface DataScientistChatAction {
  type: string;
  entity_id?: string | null;
  name?: string | null;
  source_type?: string | null;
  url?: string | null;
  build_status?: string | null;
  build_output?: string | null;
}

export interface DataScientistChatResponse {
  reply: string;
  actions: DataScientistChatAction[];
}

type RequestOptions = {
  method?: "GET" | "POST" | "PUT" | "DELETE";
  body?: unknown;
};

const API_BASE_URL = import.meta.env.VITE_API_BASE_URL ?? "/api";

async function request<T>(path: string, options: RequestOptions = {}): Promise<T> {
  const response = await fetch(`${API_BASE_URL}${path}`, {
    method: options.method ?? "GET",
    headers: options.body ? { "Content-Type": "application/json" } : undefined,
    body: options.body ? JSON.stringify(options.body) : undefined,
  });

  if (!response.ok) {
    let message = `Request failed with status ${response.status}`;

    try {
      const errorBody = (await response.json()) as { message?: string };
      if (errorBody.message) {
        message = errorBody.message;
      }
    } catch {
      // Some endpoints return an empty body on error/delete responses.
    }

    throw new Error(message);
  }

  if (response.status === 204) {
    return undefined as T;
  }

  const contentType = response.headers.get("content-type") ?? "";
  if (contentType.includes("application/json")) {
    return (await response.json()) as T;
  }

  return (await response.text()) as T;
}

function buildQuery(params: Record<string, string | number | boolean | null | undefined>) {
  const searchParams = new URLSearchParams();

  for (const [key, value] of Object.entries(params)) {
    if (value === undefined || value === null || value === "") {
      continue;
    }

    searchParams.set(key, String(value));
  }

  const query = searchParams.toString();
  return query ? `?${query}` : "";
}

function buildIndicatorQuery(params: {
  symbol: string;
  range: string;
  interval: string;
  prepost: boolean;
  indicators: string[];
}) {
  return buildQuery({
    symbol: params.symbol,
    range: params.range,
    interval: params.interval,
    prepost: params.prepost,
    indicators: params.indicators.join(","),
  });
}

export const deskApi = {
  getHello(name?: string) {
    return request<string>(`/hello${buildQuery({ name })}`);
  },

  getStockData(params: {
    symbol: string;
    range: string;
    interval: string;
    prepost: boolean;
  }) {
    return request<RawStockData>(`/stock_data${buildQuery(params)}`);
  },

  getIndicators(params: {
    symbol: string;
    range: string;
    interval: string;
    prepost: boolean;
    indicators: string[];
  }) {
    return request<StockIndicatorsResponse>(
      `/indicators${buildIndicatorQuery(params)}`,
    );
  },

  listProjects() {
    return request<Project[]>("/projects");
  },

  getProject(projectId: string) {
    return request<Project>(`/projects/${encodeURIComponent(projectId)}`);
  },

  createProject(project: Project) {
    return request<Project>("/projects", { method: "POST", body: project });
  },

  updateProject(projectId: string, project: Project) {
    return request<Project>(`/projects/${encodeURIComponent(projectId)}`, {
      method: "PUT",
      body: project,
    });
  },

  deleteProject(projectId: string) {
    return request<void>(`/projects/${encodeURIComponent(projectId)}`, {
      method: "DELETE",
    });
  },

  listPortfolios() {
    return request<Portfolio[]>("/portfolios");
  },

  getPortfolio(portfolioId: string) {
    return request<Portfolio>(`/portfolios/${encodeURIComponent(portfolioId)}`);
  },

  createPortfolio(portfolio: Portfolio) {
    return request<Portfolio>("/portfolios", { method: "POST", body: portfolio });
  },

  updatePortfolio(portfolioId: string, portfolio: Portfolio) {
    return request<Portfolio>(`/portfolios/${encodeURIComponent(portfolioId)}`, {
      method: "PUT",
      body: portfolio,
    });
  },

  deletePortfolio(portfolioId: string) {
    return request<void>(`/portfolios/${encodeURIComponent(portfolioId)}`, {
      method: "DELETE",
    });
  },

  listPositions(portfolioId: string) {
    return request<Position[]>(
      `/portfolios/${encodeURIComponent(portfolioId)}/positions`,
    );
  },

  getPosition(portfolioId: string, symbol: string, positionOpenedAt: string) {
    return request<Position>(
      `/portfolios/${encodeURIComponent(portfolioId)}/positions/${encodeURIComponent(symbol)}/${encodeURIComponent(positionOpenedAt)}`,
    );
  },

  createPosition(portfolioId: string, position: Position) {
    return request<Position>(
      `/portfolios/${encodeURIComponent(portfolioId)}/positions`,
      {
        method: "POST",
        body: position,
      },
    );
  },

  updatePosition(
    portfolioId: string,
    symbol: string,
    positionOpenedAt: string,
    position: Position,
  ) {
    return request<Position>(
      `/portfolios/${encodeURIComponent(portfolioId)}/positions/${encodeURIComponent(symbol)}/${encodeURIComponent(positionOpenedAt)}`,
      {
        method: "PUT",
        body: position,
      },
    );
  },

  deletePosition(portfolioId: string, symbol: string, positionOpenedAt: string) {
    return request<void>(
      `/portfolios/${encodeURIComponent(portfolioId)}/positions/${encodeURIComponent(symbol)}/${encodeURIComponent(positionOpenedAt)}`,
      {
        method: "DELETE",
      },
    );
  },

  listPaperAccounts() {
    return request<PaperAccount[]>("/paper/accounts");
  },

  async listLiveAccounts() {
    // TODO: Replace this stub with a safe metadata-only live account endpoint when available.
    return [] as LiveAccount[];
  },

  getPaperAccount(accountId: string) {
    return request<PaperAccount>(`/paper/accounts/${encodeURIComponent(accountId)}`);
  },

  getPaperAccountSummary(accountId: string) {
    return request<PaperAccountSummaryResponse>(
      `/paper/accounts/${encodeURIComponent(accountId)}/summary`,
    );
  },

  createPaperAccount(input: { name: string; starting_cash: number }) {
    return request<PaperAccount>("/paper/accounts", {
      method: "POST",
      body: input,
    });
  },

  createPaperOrder(input: {
    account_id: string;
    symbol: string;
    side: "buy" | "sell";
    order_type: "market";
    quantity: number;
    requested_price?: number | null;
    source?: string;
    strategy_id?: string | null;
    signal_id?: string | null;
    trader_id?: string | null;
    proposal_id?: string | null;
  }) {
    return request<PaperOrderExecutionResponse>("/paper/orders", {
      method: "POST",
      body: input,
    });
  },

  listPaperPositions(accountId: string) {
    return request<PaperPosition[]>(
      `/paper/accounts/${encodeURIComponent(accountId)}/positions`,
    );
  },

  listPaperOrders(accountId: string) {
    return request<PaperOrder[]>(
      `/paper/accounts/${encodeURIComponent(accountId)}/orders`,
    );
  },

  listPaperFills(accountId: string) {
    return request<PaperFill[]>(
      `/paper/accounts/${encodeURIComponent(accountId)}/fills`,
    );
  },

  getStrategyTradingConfig(strategyId: string) {
    return request<StrategyTradingConfig>(
      `/strategies/${encodeURIComponent(strategyId)}/trading-config`,
    );
  },

  updateStrategyTradingConfig(
    strategyId: string,
    input: UpdateStrategyTradingConfigRequest,
  ) {
    return request<StrategyTradingConfig>(
      `/strategies/${encodeURIComponent(strategyId)}/trading-config`,
      {
        method: "PUT",
        body: input,
      },
    );
  },

  getStrategyRuntimeState(strategyId: string) {
    return request<StrategyRuntimeStateListResponse>(
      `/strategies/${encodeURIComponent(strategyId)}/runtime-state`,
    );
  },

  getStrategySignals(strategyId: string) {
    return request<StrategySignalListResponse>(
      `/strategies/${encodeURIComponent(strategyId)}/signals`,
    );
  },

  getStrategyRiskConfig(strategyId: string) {
    return request<StrategyRiskConfig>(
      `/strategies/${encodeURIComponent(strategyId)}/risk-config`,
    );
  },

  updateStrategyRiskConfig(
    strategyId: string,
    input: UpdateStrategyRiskConfigRequest,
  ) {
    return request<StrategyRiskConfig>(
      `/strategies/${encodeURIComponent(strategyId)}/risk-config`,
      {
        method: "PUT",
        body: input,
      },
    );
  },

  triggerStrategyKillSwitch(strategyId: string) {
    return request<StrategyRiskConfig>(
      `/strategies/${encodeURIComponent(strategyId)}/kill-switch`,
      {
        method: "POST",
      },
    );
  },

  resumeStrategyTrading(strategyId: string) {
    return request<StrategyRiskConfig>(
      `/strategies/${encodeURIComponent(strategyId)}/resume-trading`,
      {
        method: "POST",
      },
    );
  },

  createTrader(input: CreateTraderRequest) {
    return request<Trader>("/traders", { method: "POST", body: input });
  },

  listTraders() {
    return request<Trader[]>("/traders");
  },

  getTrader(traderId: string) {
    return request<TraderDetail>(`/traders/${encodeURIComponent(traderId)}`);
  },

  updateTrader(traderId: string, input: UpdateTraderRequest) {
    return request<Trader>(`/traders/${encodeURIComponent(traderId)}`, {
      method: "PUT",
      body: input,
    });
  },

  deleteTrader(traderId: string) {
    return request<void>(`/traders/${encodeURIComponent(traderId)}`, {
      method: "DELETE",
    });
  },

  startTrader(traderId: string) {
    return request<Trader>(`/traders/${encodeURIComponent(traderId)}/start`, {
      method: "POST",
    });
  },

  stopTrader(traderId: string) {
    return request<Trader>(`/traders/${encodeURIComponent(traderId)}/stop`, {
      method: "POST",
    });
  },

  pauseTrader(traderId: string) {
    return request<Trader>(`/traders/${encodeURIComponent(traderId)}/pause`, {
      method: "POST",
    });
  },

  getTraderEvents(traderId: string) {
    return request<TraderEventsResponse>(
      `/traders/${encodeURIComponent(traderId)}/events`,
    );
  },

  getTraderRuntimeState(traderId: string) {
    return request<TraderRuntimeState>(
      `/traders/${encodeURIComponent(traderId)}/runtime-state`,
    );
  },

  getTraderTradeProposals(traderId: string) {
    return request<TraderTradeProposalsResponse>(
      `/traders/${encodeURIComponent(traderId)}/trade-proposals`,
    );
  },

  listTraderSymbols(
    traderId: string,
    filters: {
      status?: TraderSymbolStatus | "";
      asset_type?: TraderSymbolAssetType | "";
      source?: TraderSymbolSource | "";
    } = {},
  ) {
    const query = buildQuery(filters);
    return request<TraderSymbolsResponse>(
      `/traders/${encodeURIComponent(traderId)}/symbols${query}`,
    );
  },

  createTraderSymbol(traderId: string, input: CreateTraderSymbolRequest) {
    return request<TraderSymbol>(`/traders/${encodeURIComponent(traderId)}/symbols`, {
      method: "POST",
      body: input,
    });
  },

  updateTraderSymbol(
    traderId: string,
    symbolId: string,
    input: UpdateTraderSymbolRequest,
  ) {
    return request<TraderSymbol>(
      `/traders/${encodeURIComponent(traderId)}/symbols/${encodeURIComponent(symbolId)}`,
      { method: "PUT", body: input },
    );
  },

  archiveTraderSymbol(traderId: string, symbolId: string) {
    return request<TraderSymbol>(
      `/traders/${encodeURIComponent(traderId)}/symbols/${encodeURIComponent(symbolId)}/archive`,
      { method: "POST" },
    );
  },

  activateTraderSymbol(traderId: string, symbolId: string) {
    return request<TraderSymbol>(
      `/traders/${encodeURIComponent(traderId)}/symbols/${encodeURIComponent(symbolId)}/activate`,
      { method: "POST" },
    );
  },

  rejectTraderSymbol(traderId: string, symbolId: string) {
    return request<TraderSymbol>(
      `/traders/${encodeURIComponent(traderId)}/symbols/${encodeURIComponent(symbolId)}/reject`,
      { method: "POST" },
    );
  },

  bulkUpsertTraderSymbols(traderId: string, symbols: CreateTraderSymbolRequest[]) {
    return request<TraderSymbolsResponse>(
      `/traders/${encodeURIComponent(traderId)}/symbols/bulk`,
      { method: "POST", body: { symbols } },
    );
  },

  suggestTraderSymbols(traderId: string, input: SuggestTraderSymbolsRequest) {
    return request<SuggestTraderSymbolsResponse>(
      `/traders/${encodeURIComponent(traderId)}/symbols/suggest`,
      { method: "POST", body: input },
    );
  },

  listTraderProposals(traderId: string) {
    return request<TraderPortfolioProposalsResponse>(
      `/traders/${encodeURIComponent(traderId)}/proposals`,
    );
  },

  getLatestTraderProposal(traderId: string) {
    return request<TraderPortfolioProposalDetail>(
      `/traders/${encodeURIComponent(traderId)}/proposals/latest`,
    );
  },

  getActiveTraderProposal(traderId: string) {
    return request<TraderPortfolioProposalDetail>(
      `/traders/${encodeURIComponent(traderId)}/proposals/active`,
    );
  },

  getTraderProposal(traderId: string, proposalId: string) {
    return request<TraderPortfolioProposalDetail>(
      `/traders/${encodeURIComponent(traderId)}/proposals/${encodeURIComponent(proposalId)}`,
    );
  },

  acceptTraderProposal(traderId: string, proposalId: string, reviewNote?: string) {
    return request<TraderPortfolioProposalDetail>(
      `/traders/${encodeURIComponent(traderId)}/proposals/${encodeURIComponent(proposalId)}/accept`,
      { method: "POST", body: { status: "accepted", review_note: reviewNote ?? null } },
    );
  },

  rejectTraderProposal(traderId: string, proposalId: string, reviewNote?: string) {
    return request<TraderPortfolioProposalDetail>(
      `/traders/${encodeURIComponent(traderId)}/proposals/${encodeURIComponent(proposalId)}/reject`,
      { method: "POST", body: { status: "rejected", review_note: reviewNote ?? null } },
    );
  },

  approveTraderTradeProposal(traderId: string, proposalId: string) {
    return request<TraderTradeProposal>(
      `/traders/${encodeURIComponent(traderId)}/trade-proposals/${encodeURIComponent(proposalId)}/approve`,
      { method: "POST" },
    );
  },

  rejectTraderTradeProposal(traderId: string, proposalId: string) {
    return request<TraderTradeProposal>(
      `/traders/${encodeURIComponent(traderId)}/trade-proposals/${encodeURIComponent(proposalId)}/reject`,
      { method: "POST" },
    );
  },

  createDataSource(input: CreateDataSourceRequest) {
    return request<DataSource>("/data-sources", { method: "POST", body: input });
  },

  listDataSources() {
    return request<DataSource[]>("/data-sources");
  },

  getDataSource(sourceId: string) {
    return request<DataSource>(`/data-sources/${encodeURIComponent(sourceId)}`);
  },

  updateDataSource(sourceId: string, input: UpdateDataSourceRequest) {
    return request<DataSource>(`/data-sources/${encodeURIComponent(sourceId)}`, {
      method: "PUT",
      body: input,
    });
  },

  deleteDataSource(sourceId: string) {
    return request<void>(`/data-sources/${encodeURIComponent(sourceId)}`, {
      method: "DELETE",
    });
  },

  getDataSourceItems(sourceId: string) {
    return request<DataSourceItemsResponse>(
      `/data-sources/${encodeURIComponent(sourceId)}/items`,
    );
  },

  getDataSourceEvents(sourceId: string) {
    return request<DataSourceEventsResponse>(
      `/data-sources/${encodeURIComponent(sourceId)}/events`,
    );
  },

  getDataSourceScript(sourceId: string) {
    return request<DataSourceScript>(
      `/data-sources/${encodeURIComponent(sourceId)}/script`,
    );
  },

  updateDataSourceScript(sourceId: string, input: { script_text: string }) {
    return request<DataSourceScript>(
      `/data-sources/${encodeURIComponent(sourceId)}/script`,
      {
        method: "PUT",
        body: input,
      },
    );
  },

  buildDataSourceScript(sourceId: string, input: { script_text?: string } = {}) {
    return request<BuildDataSourceScriptResponse>(
      `/data-sources/${encodeURIComponent(sourceId)}/script/build`,
      {
        method: "POST",
        body: input,
      },
    );
  },

  getTraderDataSources(traderId: string) {
    return request<TraderDataSourcesResponse>(
      `/traders/${encodeURIComponent(traderId)}/data-sources`,
    );
  },

  updateTraderDataSources(traderId: string, dataSourceIds: string[]) {
    return request<TraderDataSourcesResponse>(
      `/traders/${encodeURIComponent(traderId)}/data-sources`,
      { method: "PUT", body: { data_source_ids: dataSourceIds } },
    );
  },

  sendChatCommand(input: {
    message: string;
    context?: Record<string, unknown>;
    confirmation_token?: string | null;
    confirmed?: boolean;
  }) {
    return request<ChatCommandResponse>("/chat/commands", {
      method: "POST",
      body: input,
    });
  },

  sendTraderChatMessage(
    traderId: string,
    input: {
      message: string;
      conversation?: TraderChatMessage[];
    },
  ) {
    return request<TraderChatResponse>(
      `/traders/${encodeURIComponent(traderId)}/chat`,
      {
        method: "POST",
        body: input,
      },
    );
  },

  listChannels() {
    return request<Channel[]>("/channels");
  },

  listChannelMessages(
    channelId: string,
    params: { limit?: number; before?: string; after?: string } = {},
  ) {
    return request<ChannelMessagesResponse>(
      `/channels/${encodeURIComponent(channelId)}/messages${buildQuery(params)}`,
    );
  },

  postChannelMessage(channelId: string, contentMarkdown: string) {
    return request<ChannelMessage>(
      `/channels/${encodeURIComponent(channelId)}/messages`,
      { method: "POST", body: { content_markdown: contentMarkdown } },
    );
  },

  clearChannelMessages() {
    return request<void>("/channels/messages", { method: "DELETE" });
  },

  getTraderPersona(traderId: string) {
    return request<TraderPersona>(`/traders/${encodeURIComponent(traderId)}/persona`);
  },

  updateTraderPersona(traderId: string, input: TraderPersonaUpdateRequest) {
    return request<TraderPersona>(`/traders/${encodeURIComponent(traderId)}/persona`, {
      method: "PUT",
      body: input,
    });
  },

  listTraderMemories(
    traderId: string,
    filters: { status?: string; memory_type?: string; topic?: string } = {},
  ) {
    return request<TraderMemory[]>(
      `/traders/${encodeURIComponent(traderId)}/memories${buildQuery(filters)}`,
    );
  },

  createTraderMemory(traderId: string, input: CreateTraderMemoryRequest) {
    return request<TraderMemory>(`/traders/${encodeURIComponent(traderId)}/memories`, {
      method: "POST",
      body: input,
    });
  },

  updateTraderMemory(traderId: string, memoryId: string, input: UpdateTraderMemoryRequest) {
    return request<TraderMemory>(
      `/traders/${encodeURIComponent(traderId)}/memories/${encodeURIComponent(memoryId)}`,
      { method: "PUT", body: input },
    );
  },

  archiveTraderMemory(traderId: string, memoryId: string) {
    return request<TraderMemory>(
      `/traders/${encodeURIComponent(traderId)}/memories/${encodeURIComponent(memoryId)}`,
      { method: "DELETE" },
    );
  },

  searchTraderMemories(traderId: string, query: string) {
    return request<TraderMemorySearchResponse>(
      `/traders/${encodeURIComponent(traderId)}/memories/search`,
      { method: "POST", body: { query } },
    );
  },

  getMdProfile() {
    return request<MdProfile>("/md-profile");
  },

  updateMdProfile(input: UpdateMdProfileRequest) {
    return request<MdProfile>("/md-profile", { method: "PUT", body: input });
  },

  sendMdChatMessage(input: {
    message: string;
    conversation?: AgentChatMessage[];
  }) {
    return request<MdChatResponse>("/md-profile/chat", {
      method: "POST",
      body: input,
    });
  },

  getDataScientistProfile() {
    return request<DataScientistProfile>("/data-scientist-profile");
  },

  updateDataScientistProfile(input: UpdateDataScientistProfileRequest) {
    return request<DataScientistProfile>("/data-scientist-profile", {
      method: "PUT",
      body: input,
    });
  },

  sendDataScientistChatMessage(input: {
    message: string;
    conversation?: AgentChatMessage[];
  }) {
    return request<DataScientistChatResponse>("/data-scientist-profile/chat", {
      method: "POST",
      body: input,
    });
  },

  getInvestorProfile() {
    return request<UserInvestorProfile>("/settings/investor-profile");
  },

  updateInvestorProfile(input: UpdateUserInvestorProfileRequest) {
    return request<UserInvestorProfile>("/settings/investor-profile", {
      method: "PUT",
      body: input,
    });
  },
};
