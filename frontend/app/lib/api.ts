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
  strategy_id?: string | null;
  signal_id?: string | null;
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
  positions: PaperPosition[];
  open_orders: PaperOrder[];
  recent_fills: PaperFill[];
  equity_estimate: number;
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

function buildQuery(params: Record<string, string | number | boolean | undefined>) {
  const searchParams = new URLSearchParams();

  for (const [key, value] of Object.entries(params)) {
    if (value === undefined) {
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
};
