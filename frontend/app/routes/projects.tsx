import { useEffect, useMemo, useRef, useState } from "react";
import { useNavigate, useParams } from "react-router";
import { LeftSidebar } from "../components/LeftSidebar";
import { Topbar } from "../components/Topbar";
import {
  deskApi,
  type PaperAccount,
  type PaperAccountSummaryResponse,
  type Project,
  type RawStockData,
  type StrategyDefinition,
  type StrategyRiskConfig,
  type StrategyRuntimeState,
  type StrategySignal,
  type StrategyTradingConfig,
  type TradingMode,
} from "../lib/api";
import { useDeskChat } from "../lib/chat";
import { paperSummariesEqual } from "../lib/stable-compare";

type ProjectTab = "build" | "backtest" | "live";
type BuildPanelTab = "chat" | "draft";
type ProjectSignalsFormState = {
  symbols: string;
  interval: string;
  range: string;
  prepost: boolean;
};

type BacktestBar = {
  date: string;
  open: number;
  high: number;
  low: number;
  close: number;
};

type BacktestTrade = {
  entryIndex: number;
  exitIndex: number;
  entryPrice: number;
  exitPrice: number;
  shares: number;
  profitLoss: number;
  returnPct: number;
};

type BacktestResult = {
  symbol: string;
  bars: BacktestBar[];
  trades: BacktestTrade[];
  finalEquity: number;
  totalReturnPct: number;
  winRate: number;
  tradeCount: number;
  strategyMode: string;
};

type StrategyTradingSettings = {
  trading_mode: TradingMode;
  paper_account_id: string | null;
  is_enabled: boolean;
  last_started_at?: string | null;
  last_stopped_at?: string | null;
};

type ProjectTradingSettingsMap = Record<string, StrategyTradingSettings>;

type PaperOrderFormState = {
  symbol: string;
  side: "buy" | "sell";
  quantity: string;
};

type RiskConfigFormState = {
  max_dollars_per_trade: string;
  max_quantity_per_trade: string;
  max_position_value_per_symbol: string;
  max_total_exposure: string;
  max_open_positions: string;
  max_daily_trades: string;
  max_daily_loss: string;
  cooldown_seconds: string;
  allowlist_symbols: string;
  blocklist_symbols: string;
  is_trading_enabled: boolean;
  kill_switch_enabled: boolean;
};

const BACKTEST_STARTING_CAPITAL = 10_000;
const INTERVAL_OPTIONS = ["1m", "5m", "15m", "30m", "1h", "1d", "1wk"];
const RANGE_OPTIONS = ["1d", "5d", "1mo", "3mo", "6mo", "1y", "2y", "5y"];
const DEFAULT_PAPER_ORDER_FORM: PaperOrderFormState = {
  symbol: "",
  side: "buy",
  quantity: "1",
};
const DEFAULT_TRADING_SETTINGS: StrategyTradingSettings = {
  trading_mode: "off",
  paper_account_id: null,
  is_enabled: false,
  last_started_at: null,
  last_stopped_at: null,
};

const DEFAULT_RISK_FORM: RiskConfigFormState = {
  max_dollars_per_trade: "",
  max_quantity_per_trade: "",
  max_position_value_per_symbol: "",
  max_total_exposure: "",
  max_open_positions: "",
  max_daily_trades: "",
  max_daily_loss: "",
  cooldown_seconds: "300",
  allowlist_symbols: "",
  blocklist_symbols: "",
  is_trading_enabled: true,
  kill_switch_enabled: false,
};

function toSignalsFormState(project: Project | null): ProjectSignalsFormState {
  return {
    symbols: project?.symbols.join(", ") ?? "",
    interval: project?.interval ?? "1d",
    range: project?.range ?? "1mo",
    prepost: project?.prepost ?? false,
  };
}

function formatTimestamp(value: string) {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return value;
  }

  return new Intl.DateTimeFormat("en-US", {
    month: "short",
    day: "numeric",
    year: "numeric",
    hour: "numeric",
    minute: "2-digit",
  }).format(date);
}

function formatCurrency(value: number) {
  return new Intl.NumberFormat("en-US", {
    style: "currency",
    currency: "USD",
    maximumFractionDigits: 2,
  }).format(value);
}

function formatCurrencyWithCode(value: number, currency = "USD") {
  return new Intl.NumberFormat("en-US", {
    style: "currency",
    currency,
    maximumFractionDigits: 2,
  }).format(value);
}

function formatPercent(value: number) {
  return `${value >= 0 ? "+" : ""}${value.toFixed(2)}%`;
}

function formatNumber(value: number, maximumFractionDigits = 2) {
  return new Intl.NumberFormat("en-US", {
    maximumFractionDigits,
  }).format(value);
}

function serialize(value: unknown) {
  return JSON.stringify(value);
}

function toTradingSettings(config: StrategyTradingConfig): StrategyTradingSettings {
  return {
    trading_mode: config.trading_mode,
    paper_account_id: config.paper_account_id,
    is_enabled: config.is_enabled,
    last_started_at: config.last_started_at ?? null,
    last_stopped_at: config.last_stopped_at ?? null,
  };
}

function parseStrategyDefinition(raw: string | null | undefined): StrategyDefinition | null {
  if (!raw) {
    return null;
  }

  try {
    return JSON.parse(raw) as StrategyDefinition;
  } catch {
    return null;
  }
}

function buildStrategyDefinition(strategyDraft: string): StrategyDefinition {
  const normalized = strategyDraft.toLowerCase();
  const baseRisk = {
    position_size: {
      type: "fixed_quantity",
      quantity: 1,
    },
    max_position_per_symbol: 1,
    cooldown_seconds: 3600,
  };

  if (
    normalized.includes("mean reversion")
    || normalized.includes("oversold")
    || normalized.includes("bollinger")
    || normalized.includes("rsi")
  ) {
    return {
      version: "1",
      entry: {
        all: [
          {
            indicator: "rsi",
            period: 14,
            operator: "less_than",
            value: 30,
          },
          {
            indicator: "close",
            operator: "less_than",
            compare_indicator: "sma",
            compare_period: 20,
          },
        ],
      },
      exit: {
        any: [
          {
            indicator: "rsi",
            period: 14,
            operator: "greater_than",
            value: 55,
          },
          {
            indicator: "close",
            operator: "greater_than",
            compare_indicator: "sma",
            compare_period: 20,
          },
        ],
      },
      risk: baseRisk,
    };
  }

  if (
    normalized.includes("breakout")
    || normalized.includes("momentum")
    || normalized.includes("trend")
  ) {
    return {
      version: "1",
      entry: {
        all: [
          {
            indicator: "close",
            operator: "greater_than",
            compare_indicator: "sma",
            compare_period: 20,
          },
          {
            indicator: "sma",
            period: 20,
            operator: "greater_than",
            compare_indicator: "sma",
            compare_period: 50,
          },
        ],
      },
      exit: {
        any: [
          {
            indicator: "close",
            operator: "less_than",
            compare_indicator: "sma",
            compare_period: 20,
          },
          {
            indicator: "rsi",
            period: 14,
            operator: "greater_than",
            value: 75,
          },
        ],
      },
      risk: {
        ...baseRisk,
        cooldown_seconds: 1800,
      },
    };
  }

  return {
    version: "1",
    entry: {
      all: [
        {
          indicator: "sma",
          period: 20,
          operator: "crosses_above",
          compare_indicator: "sma",
          compare_period: 50,
        },
      ],
    },
    exit: {
      any: [
        {
          indicator: "sma",
          period: 20,
          operator: "crosses_below",
          compare_indicator: "sma",
          compare_period: 50,
        },
      ],
    },
    risk: baseRisk,
  };
}

function summarizeStrategyDefinition(definition: StrategyDefinition | null) {
  if (!definition) {
    return "No executable strategy definition has been saved yet.";
  }

  const entryCount = (definition.entry.all?.length ?? 0) + (definition.entry.any?.length ?? 0);
  const exitCount = (definition.exit.all?.length ?? 0) + (definition.exit.any?.length ?? 0);
  const quantity = definition.risk.position_size.quantity ?? 0;

  return `Engine-ready v${definition.version} strategy with ${entryCount} entry rules, ${exitCount} exit rules, fixed quantity ${quantity}, and cooldown ${definition.risk.cooldown_seconds ?? 0}s.`;
}

function toRiskConfigForm(config: StrategyRiskConfig): RiskConfigFormState {
  return {
    max_dollars_per_trade: config.max_dollars_per_trade?.toString() ?? "",
    max_quantity_per_trade: config.max_quantity_per_trade?.toString() ?? "",
    max_position_value_per_symbol: config.max_position_value_per_symbol?.toString() ?? "",
    max_total_exposure: config.max_total_exposure?.toString() ?? "",
    max_open_positions: config.max_open_positions?.toString() ?? "",
    max_daily_trades: config.max_daily_trades?.toString() ?? "",
    max_daily_loss: config.max_daily_loss?.toString() ?? "",
    cooldown_seconds: config.cooldown_seconds.toString(),
    allowlist_symbols: config.allowlist_symbols?.join(", ") ?? "",
    blocklist_symbols: config.blocklist_symbols?.join(", ") ?? "",
    is_trading_enabled: config.is_trading_enabled,
    kill_switch_enabled: config.kill_switch_enabled,
  };
}

function parseOptionalNumber(value: string) {
  const trimmed = value.trim();
  if (!trimmed) {
    return null;
  }

  const parsed = Number(trimmed);
  return Number.isFinite(parsed) ? parsed : Number.NaN;
}

function parseOptionalInteger(value: string) {
  const trimmed = value.trim();
  if (!trimmed) {
    return null;
  }

  const parsed = Number(trimmed);
  return Number.isInteger(parsed) ? parsed : Number.NaN;
}

function parseSymbolsCsv(value: string) {
  const symbols = value
    .split(",")
    .map((symbol) => symbol.trim().toUpperCase())
    .filter(Boolean);

  return symbols.length > 0 ? symbols : null;
}

function normalizeProjectTab(value?: string): ProjectTab {
  return value === "backtest" || value === "live" ? value : "build";
}

function summarizeLatestAssistantMessage(messages: ReturnType<typeof useDeskChat>["messages"]) {
  const assistantMessages = messages.filter((message) => message.role === "assistant");
  return assistantMessages.at(-1)?.content ?? "";
}

function toBacktestBars(stockData: RawStockData): BacktestBar[] {
  return stockData.stock_data
    .map((entry) => ({
      date: entry.date,
      open: Number(entry.open),
      high: Number(entry.high),
      low: Number(entry.low),
      close: Number(entry.close),
    }))
    .filter((entry) =>
      [entry.open, entry.high, entry.low, entry.close].every((value) => Number.isFinite(value)),
    );
}

function movingAverage(values: number[], index: number, window: number) {
  if (index + 1 < window) {
    return null;
  }

  const slice = values.slice(index + 1 - window, index + 1);
  return slice.reduce((sum, value) => sum + value, 0) / window;
}

function stdDev(values: number[], index: number, window: number) {
  if (index + 1 < window) {
    return null;
  }

  const slice = values.slice(index + 1 - window, index + 1);
  const mean = slice.reduce((sum, value) => sum + value, 0) / slice.length;
  const variance = slice.reduce((sum, value) => sum + (value - mean) ** 2, 0) / slice.length;
  return Math.sqrt(variance);
}

function inferStrategyMode(strategyDraft: string) {
  const normalized = strategyDraft.toLowerCase();

  if (
    normalized.includes("mean reversion")
    || normalized.includes("oversold")
    || normalized.includes("bollinger")
  ) {
    return "Mean reversion";
  }

  if (
    normalized.includes("breakout")
    || normalized.includes("momentum")
    || normalized.includes("trend")
  ) {
    return "Breakout momentum";
  }

  return "Moving-average crossover";
}

function simulateBacktest(symbol: string, stockData: RawStockData, strategyDraft: string): BacktestResult {
  const bars = toBacktestBars(stockData);
  const closes = bars.map((bar) => bar.close);
  const mode = inferStrategyMode(strategyDraft);
  const trades: BacktestTrade[] = [];
  let cash = BACKTEST_STARTING_CAPITAL;
  let activeTrade: { entryIndex: number; entryPrice: number; shares: number } | null = null;

  for (let index = 1; index < bars.length; index += 1) {
    const close = bars[index].close;
    const prevClose = bars[index - 1].close;
    const sma10 = movingAverage(closes, index, 10);
    const prevSma10 = movingAverage(closes, index - 1, 10);
    const sma20 = movingAverage(closes, index, 20);
    const deviation = stdDev(closes, index, 20);
    const breakoutHigh = index >= 10 ? Math.max(...bars.slice(index - 10, index).map((bar) => bar.high)) : null;

    const enterBreakout = breakoutHigh !== null && prevClose <= breakoutHigh && close > breakoutHigh;
    const enterMeanReversion = sma20 !== null && deviation !== null && close < sma20 - deviation * 1.5;
    const enterCross = sma10 !== null && prevSma10 !== null && prevClose <= prevSma10 && close > sma10;

    const exitBreakout = sma10 !== null && close < sma10;
    const exitMeanReversion = sma20 !== null && close >= sma20;
    const exitCross = sma10 !== null && prevClose >= (prevSma10 ?? sma10) && close < sma10;

    const shouldEnter = activeTrade === null && (
      (mode === "Breakout momentum" && enterBreakout)
      || (mode === "Mean reversion" && enterMeanReversion)
      || (mode === "Moving-average crossover" && enterCross)
    );

    if (shouldEnter) {
      const shares = Math.floor(cash / close);
      if (shares > 0) {
        cash -= shares * close;
        activeTrade = {
          entryIndex: index,
          entryPrice: close,
          shares,
        };
      }
      continue;
    }

    if (activeTrade !== null) {
      const maxHold = index - activeTrade.entryIndex >= 12;
      const shouldExit = maxHold
        || (mode === "Breakout momentum" && exitBreakout)
        || (mode === "Mean reversion" && exitMeanReversion)
        || (mode === "Moving-average crossover" && exitCross);

      if (shouldExit) {
        const proceeds = activeTrade.shares * close;
        cash += proceeds;
        const profitLoss = proceeds - activeTrade.shares * activeTrade.entryPrice;
        trades.push({
          entryIndex: activeTrade.entryIndex,
          exitIndex: index,
          entryPrice: activeTrade.entryPrice,
          exitPrice: close,
          shares: activeTrade.shares,
          profitLoss,
          returnPct: ((close - activeTrade.entryPrice) / activeTrade.entryPrice) * 100,
        });
        activeTrade = null;
      }
    }
  }

  if (activeTrade !== null && bars.length > 0) {
    const finalClose = bars[bars.length - 1].close;
    const proceeds = activeTrade.shares * finalClose;
    cash += proceeds;
    const profitLoss = proceeds - activeTrade.shares * activeTrade.entryPrice;
    trades.push({
      entryIndex: activeTrade.entryIndex,
      exitIndex: bars.length - 1,
      entryPrice: activeTrade.entryPrice,
      exitPrice: finalClose,
      shares: activeTrade.shares,
      profitLoss,
      returnPct: ((finalClose - activeTrade.entryPrice) / activeTrade.entryPrice) * 100,
    });
  }

  const wins = trades.filter((trade) => trade.profitLoss > 0).length;
  const finalEquity = cash;

  return {
    symbol,
    bars,
    trades,
    finalEquity,
    totalReturnPct: ((finalEquity - BACKTEST_STARTING_CAPITAL) / BACKTEST_STARTING_CAPITAL) * 100,
    winRate: trades.length ? (wins / trades.length) * 100 : 0,
    tradeCount: trades.length,
    strategyMode: mode,
  };
}

function buildChartPath(bars: BacktestBar[], width: number, height: number) {
  if (bars.length === 0) {
    return "";
  }

  const minPrice = Math.min(...bars.map((bar) => bar.low));
  const maxPrice = Math.max(...bars.map((bar) => bar.high));
  const span = Math.max(maxPrice - minPrice, Number.EPSILON);

  return bars
    .map((bar, index) => {
      const x = bars.length === 1 ? width / 2 : (index / (bars.length - 1)) * width;
      const y = height - ((bar.close - minPrice) / span) * height;
      return `${index === 0 ? "M" : "L"} ${x.toFixed(2)} ${y.toFixed(2)}`;
    })
    .join(" ");
}

function buildRefineStrategyPrompt(
  project: Project | null,
  strategyDraft: string,
  results: BacktestResult[],
) {
  const currentWinRate = results.length
    ? results.reduce((sum, result) => sum + result.winRate, 0) / results.length
    : 0;
  const weakSymbols = results
    .filter((result) => result.winRate < 50 || result.totalReturnPct <= 0)
    .sort((left, right) => left.winRate - right.winRate)
    .slice(0, 3)
    .map((result) =>
      `${result.symbol}: win rate ${result.winRate.toFixed(1)}%, return ${formatPercent(result.totalReturnPct)}, trades ${result.tradeCount}`,
    );
  const strongSymbols = results
    .filter((result) => result.winRate >= 50 && result.totalReturnPct > 0)
    .sort((left, right) => right.winRate - left.winRate)
    .slice(0, 2)
    .map((result) =>
      `${result.symbol}: win rate ${result.winRate.toFixed(1)}%, return ${formatPercent(result.totalReturnPct)}`,
    );

  return [
    `Refine the saved strategy for project "${project?.name ?? "Current Project"}" based on this backtest review.`,
    "",
    "Goal:",
    "- Raise the overall win rate above 50% while keeping the strategy rules practical and systematic.",
    "",
    "Current strategy draft:",
    strategyDraft || "No saved draft yet.",
    "",
    "Backtest observations:",
    `- Average win rate across project symbols: ${currentWinRate.toFixed(1)}%`,
    `- Symbols tested: ${results.length > 0 ? results.map((result) => result.symbol).join(", ") : "None"}`,
    `- Weak symbols to study: ${weakSymbols.length > 0 ? weakSymbols.join(" | ") : "None identified"}`,
    `- Stronger symbols to preserve: ${strongSymbols.length > 0 ? strongSymbols.join(" | ") : "None identified"}`,
    "",
    "Please rewrite the strategy with tighter entry filters, clearer exits, and better trade selection so the win rate has a better chance of moving above 50%. Call out exactly what should change in setup, entry, exit, and risk management.",
  ].join("\n");
}

export default function ProjectsRoute() {
  const params = useParams();
  const navigate = useNavigate();
  const [sidebarOpen, setSidebarOpen] = useState(true);
  const [quoteSymbol, setQuoteSymbol] = useState("AAPL");
  const [projects, setProjects] = useState<Project[]>([]);
  const [loading, setLoading] = useState(true);
  const [hydrated, setHydrated] = useState(false);
  const [errorMessage, setErrorMessage] = useState("");
  const [strategyPrompt, setStrategyPrompt] = useState("");
  const [buildPanelTab, setBuildPanelTab] = useState<BuildPanelTab>("chat");
  const [backtestLoading, setBacktestLoading] = useState(false);
  const [backtestResults, setBacktestResults] = useState<BacktestResult[]>([]);
  const [savePending, setSavePending] = useState(false);
  const [saveMessage, setSaveMessage] = useState("");
  const [signalsForm, setSignalsForm] = useState<ProjectSignalsFormState>(
    toSignalsFormState(null),
  );
  const [signalsSavePending, setSignalsSavePending] = useState(false);
  const [signalsSaveMessage, setSignalsSaveMessage] = useState("");
  const [tradingSettingsByProject, setTradingSettingsByProject] = useState<ProjectTradingSettingsMap>({});
  const [tradingConfigLoading, setTradingConfigLoading] = useState(false);
  const [liveModeMessage, setLiveModeMessage] = useState("");
  const [paperAccounts, setPaperAccounts] = useState<PaperAccount[]>([]);
  const [paperAccountsLoading, setPaperAccountsLoading] = useState(false);
  const [paperAccountsError, setPaperAccountsError] = useState("");
  const [paperSummary, setPaperSummary] = useState<PaperAccountSummaryResponse | null>(null);
  const [paperSummaryLoading, setPaperSummaryLoading] = useState(false);
  const [paperSummaryError, setPaperSummaryError] = useState("");
  const [riskConfig, setRiskConfig] = useState<StrategyRiskConfig | null>(null);
  const [riskConfigForm, setRiskConfigForm] = useState<RiskConfigFormState>(DEFAULT_RISK_FORM);
  const [riskConfigLoading, setRiskConfigLoading] = useState(false);
  const [riskConfigSaving, setRiskConfigSaving] = useState(false);
  const [riskConfigMessage, setRiskConfigMessage] = useState("");
  const [runtimeStates, setRuntimeStates] = useState<StrategyRuntimeState[]>([]);
  const [runtimeStatesLoading, setRuntimeStatesLoading] = useState(false);
  const [runtimeStatesError, setRuntimeStatesError] = useState("");
  const [strategySignals, setStrategySignals] = useState<StrategySignal[]>([]);
  const [strategySignalsLoading, setStrategySignalsLoading] = useState(false);
  const [strategySignalsError, setStrategySignalsError] = useState("");
  const [createPaperAccountPending, setCreatePaperAccountPending] = useState(false);
  const [createPaperAccountMessage, setCreatePaperAccountMessage] = useState("");
  const [paperOrderForm, setPaperOrderForm] = useState<PaperOrderFormState>(DEFAULT_PAPER_ORDER_FORM);
  const [paperOrderPending, setPaperOrderPending] = useState(false);
  const [paperOrderMessage, setPaperOrderMessage] = useState("");
  const projectsSnapshotRef = useRef("");
  const backtestSignatureRef = useRef("");
  const chatScrollRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    setHydrated(true);
  }, []);

  useEffect(() => {
    let cancelled = false;

    async function loadProjects() {
      try {
        const nextProjects = await deskApi.listProjects();
        if (cancelled) {
          return;
        }

        const nextSnapshot = serialize(nextProjects);
        if (projectsSnapshotRef.current !== nextSnapshot) {
          projectsSnapshotRef.current = nextSnapshot;
          setProjects(nextProjects);
        }
        setErrorMessage("");
      } catch (error) {
        if (!cancelled) {
          setErrorMessage(
            error instanceof Error ? error.message : "Failed to load projects.",
          );
          setProjects([]);
        }
      } finally {
        if (!cancelled) {
          setLoading(false);
        }
      }
    }

    void loadProjects();
    const intervalId = window.setInterval(() => {
      void loadProjects();
    }, 2000);

    return () => {
      cancelled = true;
      window.clearInterval(intervalId);
    };
  }, []);

  const selectedProject = useMemo(() => {
    if (!projects.length) {
      return null;
    }

    return projects.find((project) => project.id === params.projectId) ?? projects[0] ?? null;
  }, [params.projectId, projects]);
  const activeTab = normalizeProjectTab(params.tab);

  useEffect(() => {
    if (!projects.length || !selectedProject) {
      return;
    }

    const targetTab = normalizeProjectTab(params.tab);
    const targetPath = `/strategies/${encodeURIComponent(selectedProject.id)}/${targetTab}`;

    if (params.projectId !== selectedProject.id || params.tab !== targetTab) {
      navigate(targetPath, { replace: true });
    }
  }, [navigate, params.projectId, params.tab, projects.length, selectedProject]);

  useEffect(() => {
    setStrategyPrompt("");
    setSaveMessage("");
    setBuildPanelTab("chat");
    setLiveModeMessage("");
    setCreatePaperAccountMessage("");
    setPaperOrderMessage("");
    setPaperOrderForm(DEFAULT_PAPER_ORDER_FORM);
  }, [selectedProject?.id]);

  useEffect(() => {
    setSignalsForm(toSignalsFormState(selectedProject));
    setSignalsSaveMessage("");
  }, [
    selectedProject?.id,
    selectedProject?.symbols,
    selectedProject?.interval,
    selectedProject?.range,
    selectedProject?.prepost,
  ]);

  const chat = useDeskChat({
    page: "project",
    projectId: selectedProject?.id ?? "unassigned",
    projectName: selectedProject?.name ?? null,
    description: selectedProject?.description ?? null,
    symbols: selectedProject?.symbols ?? [],
    interval: selectedProject?.interval ?? null,
    range: selectedProject?.range ?? null,
    prepost: selectedProject?.prepost ?? null,
  });

  const assistantDraft = summarizeLatestAssistantMessage(chat.messages);
  const strategyDraft = assistantDraft.trim() || selectedProject?.strategy || "";
  const executableStrategy = useMemo(
    () => parseStrategyDefinition(selectedProject?.strategy_json),
    [selectedProject?.strategy_json],
  );
  const canEditStrategy = hydrated && Boolean(selectedProject);
  const currentTradingSettings = selectedProject
    ? tradingSettingsByProject[selectedProject.id] ?? DEFAULT_TRADING_SETTINGS
    : DEFAULT_TRADING_SETTINGS;
  const selectedPaperAccountId = currentTradingSettings.paper_account_id;
  const latestRuntimeState = runtimeStates[0] ?? null;
  const latestStrategySignal = strategySignals[0] ?? null;
  const latestBlockedSignal = strategySignals.find((signal) => signal.status === "blocked_by_risk") ?? null;
  const backtestSignature = useMemo(
    () =>
      serialize({
        activeTab,
        projectId: selectedProject?.id ?? "",
        symbols: selectedProject?.symbols ?? [],
        interval: selectedProject?.interval ?? "",
        range: selectedProject?.range ?? "",
        prepost: selectedProject?.prepost ?? false,
        strategyDraft,
      }),
    [
      activeTab,
      selectedProject?.id,
      selectedProject?.symbols,
      selectedProject?.interval,
      selectedProject?.range,
      selectedProject?.prepost,
      strategyDraft,
    ],
  );

  useEffect(() => {
    if (!selectedProject) {
      return;
    }

    let cancelled = false;
    setTradingConfigLoading(true);

    async function loadTradingConfig() {
      try {
        // TODO: Switch from project id to a dedicated persisted strategy id once strategy records are split from projects.
        const config = await deskApi.getStrategyTradingConfig(selectedProject.id);
        if (cancelled) {
          return;
        }

        setTradingSettingsByProject((current) => ({
          ...current,
          [selectedProject.id]: toTradingSettings(config),
        }));
        setLiveModeMessage("");
      } catch (error) {
        if (!cancelled) {
          setTradingSettingsByProject((current) => ({
            ...current,
            [selectedProject.id]: DEFAULT_TRADING_SETTINGS,
          }));
          setLiveModeMessage(
            error instanceof Error ? error.message : "Failed to load trading configuration.",
          );
        }
      } finally {
        if (!cancelled) {
          setTradingConfigLoading(false);
        }
      }
    }

    void loadTradingConfig();

    return () => {
      cancelled = true;
    };
  }, [selectedProject?.id]);

  useEffect(() => {
    if (!selectedProject) {
      setRiskConfig(null);
      setRiskConfigForm(DEFAULT_RISK_FORM);
      setRiskConfigLoading(false);
      setRiskConfigMessage("");
      return;
    }

    let cancelled = false;
    setRiskConfigLoading(true);

    async function loadRiskConfig() {
      try {
        const config = await deskApi.getStrategyRiskConfig(selectedProject.id);
        if (cancelled) {
          return;
        }

        setRiskConfig(config);
        setRiskConfigForm(toRiskConfigForm(config));
        setRiskConfigMessage("");
      } catch (error) {
        if (!cancelled) {
          setRiskConfig(null);
          setRiskConfigForm(DEFAULT_RISK_FORM);
          setRiskConfigMessage(
            error instanceof Error ? error.message : "Failed to load risk config.",
          );
        }
      } finally {
        if (!cancelled) {
          setRiskConfigLoading(false);
        }
      }
    }

    void loadRiskConfig();

    return () => {
      cancelled = true;
    };
  }, [selectedProject?.id]);

  useEffect(() => {
    if (buildPanelTab !== "chat") {
      return;
    }

    const container = chatScrollRef.current;
    if (!container) {
      return;
    }

    container.scrollTop = container.scrollHeight;
  }, [buildPanelTab, chat.messages, chat.pending]);

  useEffect(() => {
    let cancelled = false;

    async function loadBacktest() {
      if (activeTab !== "backtest" || !selectedProject?.symbols.length) {
        setBacktestResults([]);
        setBacktestLoading(false);
        backtestSignatureRef.current = "";
        return;
      }

      if (backtestSignatureRef.current === backtestSignature) {
        return;
      }

      backtestSignatureRef.current = backtestSignature;
      setBacktestLoading(true);

      try {
        const nextResults = await Promise.all(
          selectedProject.symbols.map(async (symbol) => {
            const stockData = await deskApi.getStockData({
              symbol,
              range: selectedProject.range,
              interval: selectedProject.interval,
              prepost: selectedProject.prepost,
            });

            return simulateBacktest(symbol, stockData, strategyDraft);
          }),
        );

        if (!cancelled) {
          setBacktestResults(nextResults);
          setErrorMessage("");
        }
      } catch (error) {
        if (!cancelled) {
          setErrorMessage(
            error instanceof Error ? error.message : "Failed to load backtest data.",
          );
          setBacktestResults([]);
        }
      } finally {
        if (!cancelled) {
          setBacktestLoading(false);
        }
      }
    }

    void loadBacktest();

    return () => {
      cancelled = true;
    };
  }, [
    activeTab,
    backtestSignature,
    selectedProject?.symbols,
    selectedProject?.range,
    selectedProject?.interval,
    selectedProject?.prepost,
    strategyDraft,
  ]);

  useEffect(() => {
    if (activeTab !== "live" || currentTradingSettings.trading_mode !== "paper") {
      setPaperAccounts([]);
      setPaperAccountsLoading(false);
      setPaperAccountsError("");
      return;
    }

    let cancelled = false;
    setPaperAccountsLoading(true);
    setPaperAccountsError("");

    async function loadPaperAccounts() {
      try {
        const accounts = await deskApi.listPaperAccounts();
        if (cancelled) {
          return;
        }

        setPaperAccounts(accounts);
      } catch (error) {
        if (!cancelled) {
          setPaperAccounts([]);
          setPaperAccountsError(
            error instanceof Error ? error.message : "Failed to load paper accounts.",
          );
        }
      } finally {
        if (!cancelled) {
          setPaperAccountsLoading(false);
        }
      }
    }

    void loadPaperAccounts();

    return () => {
      cancelled = true;
    };
  }, [activeTab, currentTradingSettings.trading_mode]);

  useEffect(() => {
    if (
      !selectedProject
      || currentTradingSettings.trading_mode !== "paper"
      || paperAccountsLoading
      || paperAccounts.length === 0
    ) {
      return;
    }

    const hasSelectedAccount = selectedPaperAccountId
      ? paperAccounts.some((account) => account.id === selectedPaperAccountId)
      : false;
    if (hasSelectedAccount) {
      return;
    }

    const nextAccountId = paperAccounts[0]?.id ?? null;
    if (nextAccountId) {
      void persistTradingSettings(
        {
          trading_mode: "paper",
          paper_account_id: nextAccountId,
          is_enabled: false,
          last_started_at: currentTradingSettings.last_started_at ?? null,
          last_stopped_at: currentTradingSettings.last_stopped_at ?? null,
        },
        "Paper account updated for this strategy.",
      );
    }
  }, [
    currentTradingSettings.trading_mode,
    paperAccounts,
    paperAccountsLoading,
    selectedPaperAccountId,
    selectedProject,
  ]);

  useEffect(() => {
    if (activeTab !== "live" || currentTradingSettings.trading_mode !== "paper" || !selectedPaperAccountId) {
      setPaperSummary(null);
      setPaperSummaryLoading(false);
      setPaperSummaryError("");
      return;
    }

    let cancelled = false;
    setPaperSummaryLoading(true);
    setPaperSummaryError("");

    async function loadPaperSummary() {
      try {
        const nextSummary = await deskApi.getPaperAccountSummary(selectedPaperAccountId);
        if (cancelled) {
          return;
        }

        setPaperSummary((current) => {
          if (paperSummariesEqual(current, nextSummary)) {
            if (import.meta.env.DEV) {
              console.debug("Paper summary unchanged; skipping state update");
            }
            return current;
          }

          return nextSummary;
        });
        setPaperSummaryError("");
      } catch (error) {
        if (!cancelled) {
          setPaperSummaryError(
            error instanceof Error ? error.message : "Failed to load paper account summary.",
          );
        }
      } finally {
        if (!cancelled) {
          setPaperSummaryLoading(false);
        }
      }
    }

    void loadPaperSummary();

    return () => {
      cancelled = true;
    };
  }, [activeTab, currentTradingSettings.trading_mode, selectedPaperAccountId]);

  useEffect(() => {
    if (activeTab !== "live" || !selectedProject) {
      setRuntimeStates([]);
      setRuntimeStatesLoading(false);
      setRuntimeStatesError("");
      setStrategySignals([]);
      setStrategySignalsLoading(false);
      setStrategySignalsError("");
      return;
    }

    let cancelled = false;

    async function loadExecutionState() {
      setRuntimeStatesLoading(true);
      setStrategySignalsLoading(true);

      try {
        const [statesResponse, signalsResponse] = await Promise.all([
          deskApi.getStrategyRuntimeState(selectedProject.id),
          deskApi.getStrategySignals(selectedProject.id),
        ]);

        if (cancelled) {
          return;
        }

        setRuntimeStates(statesResponse.states);
        setRuntimeStatesError("");
        setStrategySignals(signalsResponse.signals);
        setStrategySignalsError("");
      } catch (error) {
        if (!cancelled) {
          const message =
            error instanceof Error ? error.message : "Failed to load engine strategy activity.";
          setRuntimeStates([]);
          setStrategySignals([]);
          setRuntimeStatesError(message);
          setStrategySignalsError(message);
        }
      } finally {
        if (!cancelled) {
          setRuntimeStatesLoading(false);
          setStrategySignalsLoading(false);
        }
      }
    }

    void loadExecutionState();
    const intervalId = window.setInterval(() => {
      void loadExecutionState();
    }, 5000);

    return () => {
      cancelled = true;
      window.clearInterval(intervalId);
    };
  }, [activeTab, selectedProject?.id, currentTradingSettings.is_enabled, currentTradingSettings.paper_account_id]);

  function handleLookup() {
    const nextSymbol = quoteSymbol.trim().toUpperCase();
    if (!nextSymbol) {
      return;
    }

    navigate(`/market/${encodeURIComponent(nextSymbol)}`);
  }

  function navigateToProjectTab(tab: ProjectTab) {
    if (!selectedProject) {
      return;
    }

    navigate(`/strategies/${encodeURIComponent(selectedProject.id)}/${tab}`);
  }

  function updateProjectTradingSettings(nextSettings: StrategyTradingSettings) {
    if (!selectedProject) {
      return;
    }

    setTradingSettingsByProject((current) => ({
      ...current,
      [selectedProject.id]: nextSettings,
    }));
  }

  async function persistTradingSettings(nextSettings: StrategyTradingSettings, successMessage: string) {
    if (!selectedProject) {
      return;
    }

    const previousSettings = currentTradingSettings;
    updateProjectTradingSettings(nextSettings);
    setTradingConfigLoading(true);
    setLiveModeMessage("");

    try {
      const saved = await deskApi.updateStrategyTradingConfig(selectedProject.id, {
        trading_mode: nextSettings.trading_mode,
        paper_account_id: nextSettings.paper_account_id,
        is_enabled: nextSettings.is_enabled,
      });

      updateProjectTradingSettings(toTradingSettings(saved));
      setLiveModeMessage(successMessage);
    } catch (error) {
      updateProjectTradingSettings(previousSettings);
      setLiveModeMessage(
        error instanceof Error ? error.message : "Failed to save trading configuration.",
      );
    } finally {
      setTradingConfigLoading(false);
    }
  }

  function handleTradingModeChange(mode: TradingMode) {
    if (!selectedProject) {
      return;
    }

    if (mode === "real") {
      setLiveModeMessage("Real trading is disabled. Paper trading is available for testing.");
      return;
    }

    const nextSettings: StrategyTradingSettings = {
      trading_mode: mode,
      paper_account_id: mode === "paper" ? selectedPaperAccountId : null,
      is_enabled: mode === "paper" ? currentTradingSettings.is_enabled && Boolean(selectedPaperAccountId) : false,
      last_started_at: currentTradingSettings.last_started_at ?? null,
      last_stopped_at: currentTradingSettings.last_stopped_at ?? null,
    };
    void persistTradingSettings(
      nextSettings,
      mode === "off"
        ? "Strategy is not trading."
        : "Paper mode is selected. Choose an account and enable the strategy when you are ready.",
    );
    setCreatePaperAccountMessage("");
    setPaperOrderMessage("");
  }

  function handlePaperAccountSelection(accountId: string) {
    void persistTradingSettings({
      trading_mode: "paper",
      paper_account_id: accountId || null,
      is_enabled: Boolean(accountId) && currentTradingSettings.is_enabled,
      last_started_at: currentTradingSettings.last_started_at ?? null,
      last_stopped_at: currentTradingSettings.last_stopped_at ?? null,
    }, "Paper account updated for this strategy.");
  }

  function handleEnableStrategy() {
    if (!executableStrategy) {
      setLiveModeMessage("Save the strategy first so the engine has an executable strategy definition.");
      return;
    }

    if (!selectedPaperAccountId) {
      setLiveModeMessage("Select a paper account before enabling this strategy.");
      return;
    }

    void persistTradingSettings(
      {
        trading_mode: "paper",
        paper_account_id: selectedPaperAccountId,
        is_enabled: true,
        last_started_at: currentTradingSettings.last_started_at ?? null,
        last_stopped_at: currentTradingSettings.last_stopped_at ?? null,
      },
      "Paper trading enabled. Engine will evaluate this strategy in the background.",
    );
  }

  function handleDisableStrategy() {
    void persistTradingSettings(
      {
        trading_mode: currentTradingSettings.trading_mode === "paper" ? "paper" : "off",
        paper_account_id: currentTradingSettings.trading_mode === "paper" ? selectedPaperAccountId : null,
        is_enabled: false,
        last_started_at: currentTradingSettings.last_started_at ?? null,
        last_stopped_at: currentTradingSettings.last_stopped_at ?? null,
      },
      currentTradingSettings.trading_mode === "paper"
        ? "Paper trading disabled. The strategy remains configured but the engine will stop submitting orders."
        : "Strategy is not trading.",
    );
  }

  async function handleSaveRiskConfig() {
    if (!selectedProject) {
      return;
    }

    const maxDollarsPerTrade = parseOptionalNumber(riskConfigForm.max_dollars_per_trade);
    const maxQuantityPerTrade = parseOptionalNumber(riskConfigForm.max_quantity_per_trade);
    const maxPositionValuePerSymbol = parseOptionalNumber(riskConfigForm.max_position_value_per_symbol);
    const maxTotalExposure = parseOptionalNumber(riskConfigForm.max_total_exposure);
    const maxOpenPositions = parseOptionalInteger(riskConfigForm.max_open_positions);
    const maxDailyTrades = parseOptionalInteger(riskConfigForm.max_daily_trades);
    const maxDailyLoss = parseOptionalNumber(riskConfigForm.max_daily_loss);
    const cooldownSeconds = parseOptionalInteger(riskConfigForm.cooldown_seconds);

    if (
      [maxDollarsPerTrade, maxQuantityPerTrade, maxPositionValuePerSymbol, maxTotalExposure, maxDailyLoss].some(
        (value) => Number.isNaN(value),
      )
      || [maxOpenPositions, maxDailyTrades, cooldownSeconds].some((value) => Number.isNaN(value))
    ) {
      setRiskConfigMessage("Risk limits must be valid numbers.");
      return;
    }

    setRiskConfigSaving(true);
    setRiskConfigMessage("");

    try {
      const config = await deskApi.updateStrategyRiskConfig(selectedProject.id, {
        max_dollars_per_trade: maxDollarsPerTrade,
        max_quantity_per_trade: maxQuantityPerTrade,
        max_position_value_per_symbol: maxPositionValuePerSymbol,
        max_total_exposure: maxTotalExposure,
        max_open_positions: maxOpenPositions,
        max_daily_trades: maxDailyTrades,
        max_daily_loss: maxDailyLoss,
        cooldown_seconds: cooldownSeconds ?? 0,
        allowlist_symbols: parseSymbolsCsv(riskConfigForm.allowlist_symbols),
        blocklist_symbols: parseSymbolsCsv(riskConfigForm.blocklist_symbols),
        is_trading_enabled: riskConfigForm.is_trading_enabled,
        kill_switch_enabled: riskConfigForm.kill_switch_enabled,
      });

      setRiskConfig(config);
      setRiskConfigForm(toRiskConfigForm(config));
      setRiskConfigMessage("Risk controls saved.");
    } catch (error) {
      setRiskConfigMessage(
        error instanceof Error ? error.message : "Failed to save risk controls.",
      );
    } finally {
      setRiskConfigSaving(false);
    }
  }

  async function handleKillSwitch() {
    if (!selectedProject || !window.confirm("Trigger the kill switch and immediately disable this strategy?")) {
      return;
    }

    setRiskConfigSaving(true);
    setRiskConfigMessage("");
    try {
      const config = await deskApi.triggerStrategyKillSwitch(selectedProject.id);
      setRiskConfig(config);
      setRiskConfigForm(toRiskConfigForm(config));
      setRiskConfigMessage("Kill switch activated. This strategy is now disabled.");
    } catch (error) {
      setRiskConfigMessage(
        error instanceof Error ? error.message : "Failed to activate kill switch.",
      );
    } finally {
      setRiskConfigSaving(false);
    }
  }

  async function handleResumeTrading() {
    if (!selectedProject) {
      return;
    }

    setRiskConfigSaving(true);
    setRiskConfigMessage("");
    try {
      const config = await deskApi.resumeStrategyTrading(selectedProject.id);
      setRiskConfig(config);
      setRiskConfigForm(toRiskConfigForm(config));
      setRiskConfigMessage("Risk config resumed trading for this strategy.");
    } catch (error) {
      setRiskConfigMessage(
        error instanceof Error ? error.message : "Failed to resume strategy trading.",
      );
    } finally {
      setRiskConfigSaving(false);
    }
  }

  async function handleStrategySubmit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const prompt = strategyPrompt.trim();
    if (!prompt) {
      return;
    }

    setStrategyPrompt("");
    await chat.sendMessage(prompt);
  }

  async function handleSaveStrategy() {
    if (!selectedProject) {
      return;
    }

    const nextStrategy = strategyDraft.trim();
    if (!nextStrategy) {
      setSaveMessage("Build a strategy draft before saving it.");
      return;
    }

    setSavePending(true);
    setSaveMessage("");

    try {
      const strategyDefinition = buildStrategyDefinition(nextStrategy);
      const updatedProject = await deskApi.updateProject(selectedProject.id, {
        ...selectedProject,
        strategy: nextStrategy,
        strategy_json: JSON.stringify(strategyDefinition),
        strategy_status: "ready",
        updated_at: new Date().toISOString(),
      });

      setProjects((current) =>
        current.map((project) =>
          project.id === updatedProject.id ? updatedProject : project,
        ),
      );
      setSaveMessage("Strategy saved with an executable engine config.");
    } catch (error) {
      setSaveMessage(
        error instanceof Error ? error.message : "Failed to save strategy.",
      );
    } finally {
      setSavePending(false);
    }
  }

  async function handleSaveSignals() {
    if (!selectedProject) {
      return;
    }

    const nextSymbols = signalsForm.symbols
      .split(",")
      .map((symbol) => symbol.trim().toUpperCase())
      .filter(Boolean);

    if (nextSymbols.length === 0) {
      setSignalsSaveMessage("Add at least one symbol before saving strategy signals.");
      return;
    }

    const nextInterval = signalsForm.interval.trim();
    const nextRange = signalsForm.range.trim();

    if (!nextInterval || !nextRange) {
      setSignalsSaveMessage("Interval and range are required.");
      return;
    }

    setSignalsSavePending(true);
    setSignalsSaveMessage("");

    try {
      const updatedProject = await deskApi.updateProject(selectedProject.id, {
        ...selectedProject,
        symbols: nextSymbols,
        interval: nextInterval,
        range: nextRange,
        prepost: signalsForm.prepost,
        updated_at: new Date().toISOString(),
      });

      setProjects((current) =>
        current.map((project) =>
          project.id === updatedProject.id ? updatedProject : project,
        ),
      );
      setSignalsForm(toSignalsFormState(updatedProject));
      setSignalsSaveMessage("Strategy signals updated.");
    } catch (error) {
      setSignalsSaveMessage(
        error instanceof Error ? error.message : "Failed to update strategy signals.",
      );
    } finally {
      setSignalsSavePending(false);
    }
  }

  function handleRefineStrategyFromBacktest() {
    const prompt = buildRefineStrategyPrompt(
      selectedProject,
      strategyDraft,
      backtestResults,
    );

    setStrategyPrompt(prompt);
    navigateToProjectTab("build");
    setSaveMessage("Backtest review has been turned into a refinement prompt.");
  }

  async function handleCreatePaperAccount() {
    if (!selectedProject) {
      return;
    }

    setCreatePaperAccountPending(true);
    setCreatePaperAccountMessage("");
    setPaperAccountsError("");

    try {
      const account = await deskApi.createPaperAccount({
        name: "Default Paper Account",
        starting_cash: 100000,
      });

      setPaperAccounts((current) => [...current, account]);
      await persistTradingSettings({
        trading_mode: "paper",
        paper_account_id: account.id,
        is_enabled: false,
        last_started_at: currentTradingSettings.last_started_at ?? null,
        last_stopped_at: currentTradingSettings.last_stopped_at ?? null,
      }, "Paper account created and linked. Enable the strategy when you are ready.");
      setCreatePaperAccountMessage("Paper account created.");
    } catch (error) {
      setCreatePaperAccountMessage(
        error instanceof Error ? error.message : "Failed to create paper account.",
      );
    } finally {
      setCreatePaperAccountPending(false);
    }
  }

  async function handlePaperOrderSubmit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();

    if (!selectedPaperAccountId) {
      setPaperOrderMessage("Select a paper account before submitting an order.");
      return;
    }

    const symbol = paperOrderForm.symbol.trim().toUpperCase();
    const quantity = Number(paperOrderForm.quantity);
    if (!symbol) {
      setPaperOrderMessage("Symbol is required.");
      return;
    }

    if (!Number.isFinite(quantity) || quantity <= 0) {
      setPaperOrderMessage("Quantity must be greater than zero.");
      return;
    }

    setPaperOrderPending(true);
    setPaperOrderMessage("");

    try {
      await deskApi.createPaperOrder({
        account_id: selectedPaperAccountId,
        symbol,
        side: paperOrderForm.side,
        order_type: "market",
        quantity,
        source: "ui",
      });

      setPaperOrderForm(DEFAULT_PAPER_ORDER_FORM);
      setPaperOrderMessage("Paper order submitted.");
      const nextSummary = await deskApi.getPaperAccountSummary(selectedPaperAccountId);
      setPaperSummary((current) => {
        if (paperSummariesEqual(current, nextSummary)) {
          if (import.meta.env.DEV) {
            console.debug("Paper summary unchanged after order refresh; skipping state update");
          }
          return current;
        }

        return nextSummary;
      });
      setPaperSummaryError("");
    } catch (error) {
      setPaperOrderMessage(
        error instanceof Error ? error.message : "Failed to submit paper order.",
      );
    } finally {
      setPaperOrderPending(false);
    }
  }

  return (
    <main className="app-page min-h-screen pt-16">
      <LeftSidebar open={sidebarOpen} />
      <Topbar
        onToggleSidebar={() => setSidebarOpen((open) => !open)}
        sidebarOpen={sidebarOpen}
        quoteSymbol={quoteSymbol}
        onQuoteSymbolChange={setQuoteSymbol}
        onQuoteLookup={handleLookup}
        quoteLoading={false}
      />

      <section
        className={`ml-0 flex min-h-[calc(100vh-4rem)] flex-col gap-6 p-6 transition-all duration-200 ${
          sidebarOpen ? "md:ml-64" : "md:ml-0"
        }`}
      >
        {errorMessage ? (
          <div className="app-alert-error rounded-2xl px-4 py-3 text-sm">
            {errorMessage}
          </div>
        ) : null}

        <div className="flex flex-wrap gap-3">
          {([
            { key: "build", label: "Build" },
            { key: "backtest", label: "Backtest" },
            { key: "live", label: "Live" },
          ] as Array<{ key: ProjectTab; label: string }>).map((tab) => (
            <button
              key={tab.key}
              type="button"
              onClick={() => navigateToProjectTab(tab.key)}
              className="rounded-full px-4 py-2 text-sm font-medium transition"
              style={
                activeTab === tab.key
                  ? {
                      background: "color-mix(in srgb, var(--color-primary) 14%, transparent)",
                      color: "var(--color-primary)",
                    }
                  : undefined
              }
            >
              {tab.label}
            </button>
          ))}
        </div>

        <div className={activeTab === "build" ? "grid gap-6 xl:grid-cols-[22rem_minmax(0,1fr)]" : "grid gap-6"}>
          {activeTab === "build" ? (
            <aside className="space-y-6 self-start xl:sticky xl:top-20">
              <article className="app-surface rounded-3xl p-5 shadow-sm">
                <p className="app-text-muted text-xs uppercase tracking-[0.22em]">Strategy Workspace</p>
                <h1 className="mt-2 text-3xl font-semibold">
                  {selectedProject?.name ?? (loading ? "Loading strategies..." : "No strategies")}
                </h1>
                <p className="app-text-muted mt-3 text-sm leading-6">
                  {selectedProject?.description || "Use this workspace to turn a strategy idea into an algorithmic strategy based on the symbols tracked in this strategy."}
                </p>
              </article>

              <article className="app-surface rounded-3xl p-5 shadow-sm">
                <p className="app-text-muted text-xs uppercase tracking-[0.18em]">Strategy Signals</p>
                <div className="mt-4 space-y-3">
                  <label className="block">
                    <span className="mb-2 block text-sm font-medium">Symbols</span>
                    <textarea
                      value={signalsForm.symbols}
                      onChange={(event) =>
                        setSignalsForm((current) => ({
                          ...current,
                          symbols: event.target.value,
                        }))}
                      rows={3}
                      placeholder="AAPL, MSFT, NVDA"
                      className="app-input w-full rounded-2xl px-4 py-3 text-sm"
                      disabled={!selectedProject || signalsSavePending}
                    />
                  </label>

                  <label className="block">
                    <span className="mb-2 block text-sm font-medium">Interval</span>
                    <select
                      value={signalsForm.interval}
                      onChange={(event) =>
                        setSignalsForm((current) => ({
                          ...current,
                          interval: event.target.value,
                        }))}
                      className="app-input w-full rounded-2xl px-4 py-3 text-sm"
                      disabled={!selectedProject || signalsSavePending}
                    >
                      {INTERVAL_OPTIONS.map((option) => (
                        <option key={option} value={option}>
                          {option}
                        </option>
                      ))}
                    </select>
                  </label>

                  <label className="block">
                    <span className="mb-2 block text-sm font-medium">Range</span>
                    <select
                      value={signalsForm.range}
                      onChange={(event) =>
                        setSignalsForm((current) => ({
                          ...current,
                          range: event.target.value,
                        }))}
                      className="app-input w-full rounded-2xl px-4 py-3 text-sm"
                      disabled={!selectedProject || signalsSavePending}
                    >
                      {RANGE_OPTIONS.map((option) => (
                        <option key={option} value={option}>
                          {option}
                        </option>
                      ))}
                    </select>
                  </label>

                  <label className="flex items-center gap-3 rounded-2xl px-1 py-1 text-sm">
                    <input
                      type="checkbox"
                      checked={signalsForm.prepost}
                      onChange={(event) =>
                        setSignalsForm((current) => ({
                          ...current,
                          prepost: event.target.checked,
                        }))}
                      className="h-4 w-4"
                      disabled={!selectedProject || signalsSavePending}
                    />
                    <span>Include pre/post market data</span>
                  </label>

                  <InfoTile label="Updated" value={selectedProject ? formatTimestamp(selectedProject.updated_at) : "--"} />

                  {signalsSaveMessage ? (
                    <p className="app-text-muted text-sm">{signalsSaveMessage}</p>
                  ) : null}

                  <div className="flex justify-end">
                    <button
                      type="button"
                      onClick={handleSaveSignals}
                      disabled={!selectedProject || signalsSavePending}
                      className="app-button-primary rounded-full px-4 py-2 text-sm font-medium disabled:cursor-not-allowed disabled:opacity-60"
                    >
                      {signalsSavePending ? "Saving..." : "Save signals"}
                    </button>
                  </div>
                </div>
              </article>

              <article className="app-surface rounded-3xl p-5 shadow-sm">
                <p className="app-text-muted text-xs uppercase tracking-[0.18em]">Prompt Ideas</p>
                <div className="mt-4 flex flex-wrap gap-2">
                  {chat.suggestions.map((suggestion) => (
                    <button
                      key={suggestion}
                      type="button"
                      onClick={() => setStrategyPrompt(suggestion)}
                      className="app-button-secondary rounded-full px-3 py-2 text-xs font-medium"
                    >
                      {suggestion}
                    </button>
                  ))}
                </div>
              </article>
            </aside>
          ) : null}

          <section className={`grid min-w-0 gap-6 ${activeTab === "build" ? "xl:h-[calc(95vh-8.5rem)]" : ""}`}>
            {activeTab === "build" ? (
              <>
                <article className="app-surface flex min-h-[calc(95vh-10rem)] flex-col rounded-3xl p-5 shadow-sm xl:h-[calc(95vh-8.5rem)] xl:min-h-0">
                  <div className="mb-4 flex flex-wrap items-center justify-between gap-3">
                    <div>
                      <p className="app-text-muted text-xs uppercase tracking-[0.22em]">Strategy Builder</p>
                      <h2 className="mt-2 text-2xl font-semibold">Chat your rules into shape</h2>
                    </div>
                    {hydrated ? (
                      <div className="flex flex-wrap items-center gap-2">
                        <button
                          type="button"
                          onClick={handleSaveStrategy}
                          disabled={!canEditStrategy || savePending || !strategyDraft.trim()}
                          className="app-button-primary rounded-full px-4 py-2 text-sm font-medium disabled:cursor-not-allowed disabled:opacity-60"
                        >
                          {savePending ? "Saving..." : "Save strategy"}
                        </button>
                        <button
                          type="button"
                          onClick={chat.clearMessages}
                          className="app-button-secondary rounded-full px-4 py-2 text-sm font-medium"
                        >
                          Reset chat
                        </button>
                      </div>
                    ) : (
                      <div className="flex flex-wrap items-center gap-2">
                        <div className="app-surface-muted h-10 w-28 rounded-full" />
                        <div className="app-surface-muted h-10 w-24 rounded-full" />
                      </div>
                    )}
                  </div>

                  <div className="mb-4 flex flex-wrap gap-2">
                    {([
                      { key: "chat", label: "Chat" },
                      { key: "draft", label: "Working draft" },
                    ] as Array<{ key: BuildPanelTab; label: string }>).map((tab) => (
                      <button
                        key={tab.key}
                        type="button"
                        onClick={() => setBuildPanelTab(tab.key)}
                        className="rounded-full px-4 py-2 text-sm font-medium transition"
                        style={
                          buildPanelTab === tab.key
                            ? {
                                background: "color-mix(in srgb, var(--color-primary) 14%, transparent)",
                                color: "var(--color-primary)",
                              }
                            : undefined
                        }
                      >
                        {tab.label}
                      </button>
                    ))}
                  </div>

                  {buildPanelTab === "chat" ? (
                    <div className="app-surface-muted min-h-0 flex-1 rounded-2xl p-4">
                      <div ref={chatScrollRef} className="h-full overflow-y-auto pr-1">
                        <div className="space-y-4">
                          {chat.messages.map((message) => (
                            <article
                              key={message.id}
                              className={`max-w-[85%] rounded-2xl px-4 py-3 ${
                                message.role === "user" ? "ml-auto app-button-primary" : "app-surface"
                              }`}
                            >
                              <div className="mb-2 flex items-center justify-between gap-3">
                                <p className="text-xs font-semibold uppercase tracking-[0.16em]">
                                  {message.role === "user" ? "You" : "Desk"}
                                </p>
                                <p className="app-text-muted text-xs">{formatTimestamp(message.createdAt)}</p>
                              </div>
                              <p className="whitespace-pre-wrap text-sm leading-6">{message.content}</p>
                            </article>
                          ))}
                          {chat.pending ? (
                            <div className="app-surface max-w-[85%] rounded-2xl px-4 py-3">
                              <p className="app-text-muted text-sm">Desk is drafting your strategy...</p>
                            </div>
                          ) : null}
                        </div>
                      </div>
                    </div>
                  ) : (
                    <div className="app-surface-muted flex min-h-0 flex-1 flex-col rounded-2xl p-4">
                      <div className="mb-4 flex items-center justify-between gap-3">
                        <div>
                          <p className="app-text-muted text-xs uppercase tracking-[0.22em]">Working Draft</p>
                          <p className="mt-2 text-sm">
                            Keep the saved strategy tight and execution-ready.
                          </p>
                        </div>
                        {saveMessage ? (
                          <p className="app-text-muted text-sm">{saveMessage}</p>
                        ) : null}
                      </div>
                      <div className="app-surface min-h-0 flex-1 overflow-y-auto rounded-2xl p-4">
                        <p className="whitespace-pre-wrap text-sm leading-7">
                          {strategyDraft || "Start describing the pattern you want, and Desk will turn it into a strategy outline."}
                        </p>
                      </div>
                      <div className="mt-4 grid gap-4 lg:grid-cols-2">
                        <InfoTile
                          label="Strategy Status"
                          value={selectedProject?.strategy_status ?? "draft"}
                        />
                        <InfoTile
                          label="Executable Config"
                          value={summarizeStrategyDefinition(executableStrategy)}
                        />
                      </div>
                    </div>
                  )}

                  {hydrated ? (
                    <form className="mt-4 space-y-3" onSubmit={handleStrategySubmit}>
                      <textarea
                        value={strategyPrompt}
                        onChange={(event) => setStrategyPrompt(event.target.value)}
                        rows={5}
                        placeholder="Describe the trading pattern you want to build, for example: Buy when one of these symbols closes above the 20-day high on expanding volume, then trail a stop under the 10-day low."
                        className="app-input w-full rounded-2xl px-4 py-3 text-sm"
                        disabled={!canEditStrategy}
                      />
                      <div className="flex justify-end">
                        <button
                          type="submit"
                          disabled={!canEditStrategy || chat.pending}
                          className="app-button-primary rounded-full px-5 py-2.5 text-sm font-medium disabled:cursor-not-allowed disabled:opacity-60"
                        >
                          Build strategy
                        </button>
                      </div>
                    </form>
                  ) : (
                    <div className="mt-4 space-y-3">
                      <div className="app-surface-muted h-36 rounded-2xl" />
                      <div className="flex justify-end">
                        <div className="app-surface-muted h-10 w-32 rounded-full" />
                      </div>
                    </div>
                  )}
                </article>
              </>
            ) : null}

            {activeTab === "backtest" ? (
              <div className="grid gap-6">
                <div className="flex justify-end">
                  <button
                    type="button"
                    onClick={handleRefineStrategyFromBacktest}
                    disabled={backtestLoading || backtestResults.length === 0}
                    className="app-button-primary rounded-full px-5 py-2.5 text-sm font-medium disabled:cursor-not-allowed disabled:opacity-60"
                  >
                    Refine strategy based on backtest
                  </button>
                </div>

                {backtestResults.map((result) => (
                  <article key={result.symbol} className="app-surface rounded-3xl p-5 shadow-sm">
                    <div className="mb-4 flex flex-wrap items-center justify-between gap-3">
                      <div>
                        <h3 className="mt-2 text-2xl font-semibold">{result.symbol}</h3>
                        <p className="app-text-muted mt-2 text-sm">Engine: {result.strategyMode}</p>
                      </div>
                      <div className="app-text-muted text-sm">
                        {backtestLoading ? "Running preview backtest..." : `${formatCurrency(BACKTEST_STARTING_CAPITAL)} starting capital`}
                      </div>
                    </div>

                    <div className="app-surface-muted rounded-2xl p-4">
                      <SymbolBacktestChart result={result} />
                    </div>

                    <div className="mt-4 grid gap-4 lg:grid-cols-4">
                      <InfoTile label="Final equity" value={formatCurrency(result.finalEquity)} />
                      <InfoTile label="Return" value={formatPercent(result.totalReturnPct)} />
                      <InfoTile label="Win rate" value={`${result.winRate.toFixed(1)}%`} />
                      <InfoTile label="Trades" value={`${result.tradeCount}`} />
                    </div>
                  </article>
                ))}

                {!backtestLoading && backtestResults.length === 0 ? (
                  <article className="app-surface rounded-3xl p-5 shadow-sm">
                    <div className="app-surface-muted rounded-2xl p-5">
                      <p className="text-sm leading-7">
                        Add symbols to this project and save a strategy draft to unlock the backtest preview.
                      </p>
                    </div>
                  </article>
                ) : null}
              </div>
            ) : null}

            {activeTab === "live" ? (
              <article className="app-surface rounded-3xl p-5 shadow-sm">
                <div className="mb-4">
                  <p className="app-text-muted text-xs uppercase tracking-[0.22em]">Live</p>
                  <h2 className="mt-2 text-2xl font-semibold">Prepare execution</h2>
                </div>
                <div className="space-y-5">
                  <div className="app-surface-muted rounded-2xl p-5">
                    <div className="flex flex-wrap items-start justify-between gap-4">
                      <div>
                        <p className="text-sm font-medium">Trading mode</p>
                        <p className="app-text-muted mt-2 text-sm leading-6">
                          Choose how this strategy should be armed for execution. Paper mode persists in the backend so the engine can keep running after the browser closes.
                        </p>
                      </div>
                      <div className="trading-mode-toggle" role="tablist" aria-label="Trading mode">
                        {([
                          { key: "off", label: "Off", disabled: false },
                          { key: "paper", label: "Paper", disabled: false },
                          { key: "real", label: "Real", disabled: true },
                        ] as Array<{ key: TradingMode; label: string; disabled: boolean }>).map((option) => (
                          <button
                            key={option.key}
                            type="button"
                            className={[
                              "trading-mode-toggle__button",
                              currentTradingSettings.trading_mode === option.key ? "trading-mode-toggle__button--active" : "",
                              option.disabled ? "trading-mode-toggle__button--disabled" : "",
                            ].filter(Boolean).join(" ")}
                            onClick={() => handleTradingModeChange(option.key)}
                            aria-pressed={currentTradingSettings.trading_mode === option.key}
                            title={option.disabled ? "Coming later" : undefined}
                            disabled={tradingConfigLoading || option.disabled}
                          >
                            {option.label}
                          </button>
                        ))}
                      </div>
                    </div>

                    <div className="mt-4 flex flex-wrap items-center gap-3">
                      <button
                        type="button"
                        onClick={handleEnableStrategy}
                        disabled={
                          tradingConfigLoading
                          || currentTradingSettings.trading_mode !== "paper"
                          || !executableStrategy
                          || !selectedPaperAccountId
                          || currentTradingSettings.is_enabled
                        }
                        className="app-button-primary rounded-full px-4 py-2 text-sm font-medium disabled:cursor-not-allowed disabled:opacity-60"
                      >
                        Enable Strategy
                      </button>
                      <button
                        type="button"
                        onClick={handleDisableStrategy}
                        disabled={
                          tradingConfigLoading
                          || (currentTradingSettings.trading_mode === "off" && !currentTradingSettings.is_enabled)
                        }
                        className="app-button-secondary rounded-full px-4 py-2 text-sm font-medium disabled:cursor-not-allowed disabled:opacity-60"
                      >
                        Disable Strategy
                      </button>
                    </div>

                    <div className="mt-4 space-y-2 text-sm">
                      {currentTradingSettings.trading_mode === "off" ? (
                        <p>Strategy is not trading.</p>
                      ) : null}
                      {currentTradingSettings.trading_mode === "paper" ? (
                        <p>
                          {currentTradingSettings.is_enabled
                            ? "Paper trading is enabled. The engine can evaluate this strategy in the background."
                            : "Paper mode is selected. Choose an account and enable the strategy to let the engine trade automatically."}
                        </p>
                      ) : null}
                      <p className="app-text-muted">
                        Real mode is visible for planning, but live broker execution is not implemented yet.
                      </p>
                      {!executableStrategy ? (
                        <p className="app-text-muted">
                          Save the current strategy draft first so the engine has a structured execution config to run.
                        </p>
                      ) : null}
                      {tradingConfigLoading ? (
                        <p className="app-text-muted">Saving trading configuration...</p>
                      ) : null}
                      {liveModeMessage ? <p className="app-text-muted">{liveModeMessage}</p> : null}
                    </div>
                  </div>

                  {currentTradingSettings.trading_mode === "paper" ? (
                    <div className="space-y-5">
                      <section className="app-surface-muted rounded-2xl p-5">
                        <div className="flex flex-wrap items-end justify-between gap-4">
                          <div>
                            <p className="text-sm font-medium">Paper account</p>
                            <p className="app-text-muted mt-2 text-sm">
                              Connect this strategy to a simulated account. The selected account is saved through the backend so the engine keeps running after the browser closes.
                            </p>
                          </div>
                          {paperAccounts.length > 0 ? (
                            <label className="block min-w-[16rem]">
                              <span className="mb-2 block text-sm font-medium">Active paper account</span>
                              <select
                                value={selectedPaperAccountId ?? ""}
                                onChange={(event) => handlePaperAccountSelection(event.target.value)}
                                className="app-input w-full rounded-2xl px-4 py-3 text-sm"
                                disabled={paperAccountsLoading || tradingConfigLoading}
                              >
                                <option value="">Select a paper account</option>
                                {paperAccounts.map((account) => (
                                  <option key={account.id} value={account.id}>
                                    {account.name}
                                  </option>
                                ))}
                              </select>
                            </label>
                          ) : null}
                        </div>

                        {paperAccountsLoading ? (
                          <p className="app-text-muted mt-4 text-sm">Loading paper accounts...</p>
                        ) : null}
                        {paperAccountsError ? (
                          <p className="app-alert-error mt-4 rounded-2xl px-4 py-3 text-sm">{paperAccountsError}</p>
                        ) : null}

                        {!paperAccountsLoading && !paperAccountsError && paperAccounts.length === 0 ? (
                          <div className="mt-4 rounded-2xl border border-dashed px-4 py-4 text-sm" style={{ borderColor: "var(--color-border)" }}>
                            <p>No paper accounts are available yet.</p>
                            <p className="app-text-muted mt-2">
                              Create a default paper account with starting cash of {formatCurrency(100000)}.
                            </p>
                            <div className="mt-4 flex flex-wrap items-center gap-3">
                              <button
                                type="button"
                                onClick={handleCreatePaperAccount}
                                disabled={createPaperAccountPending || tradingConfigLoading}
                                className="app-button-primary rounded-full px-4 py-2 text-sm font-medium disabled:cursor-not-allowed disabled:opacity-60"
                              >
                                {createPaperAccountPending ? "Creating..." : "Create default paper account"}
                              </button>
                              {createPaperAccountMessage ? (
                                <p className="app-text-muted text-sm">{createPaperAccountMessage}</p>
                              ) : null}
                            </div>
                          </div>
                        ) : null}

                        {!paperAccountsLoading && paperAccounts.length > 0 && createPaperAccountMessage ? (
                          <p className="app-text-muted mt-4 text-sm">{createPaperAccountMessage}</p>
                        ) : null}
                      </section>

                      <section className="app-surface-muted rounded-2xl p-5">
                        <div className="flex flex-wrap items-center justify-between gap-3">
                          <div>
                            <p className="text-sm font-medium">Engine status</p>
                            <p className="app-text-muted mt-2 text-sm">
                              This section reflects persisted strategy execution state from the backend, not browser-local state.
                            </p>
                          </div>
                        </div>

                        <div className="mt-4 grid gap-4 md:grid-cols-2 xl:grid-cols-4">
                          <InfoTile
                            label="Mode"
                            value={currentTradingSettings.trading_mode.toUpperCase()}
                          />
                          <InfoTile
                            label="Enabled"
                            value={currentTradingSettings.is_enabled ? "Yes" : "No"}
                          />
                          <InfoTile
                            label="Latest Signal"
                            value={latestStrategySignal?.signal_type ?? "none"}
                          />
                          <InfoTile
                            label="Latest State"
                            value={latestRuntimeState?.position_state ?? "flat"}
                          />
                        </div>

                        <div className="mt-4 grid gap-4 md:grid-cols-2 xl:grid-cols-3">
                          <InfoTile
                            label="Risk Trading"
                            value={riskConfig?.is_trading_enabled ? "Enabled" : "Disabled"}
                          />
                          <InfoTile
                            label="Kill Switch"
                            value={riskConfig?.kill_switch_enabled ? "ACTIVE" : "Off"}
                          />
                          <InfoTile
                            label="Last Risk Block"
                            value={latestBlockedSignal?.risk_reason ?? "None"}
                          />
                        </div>

                        <div className="mt-4 grid gap-4 md:grid-cols-2">
                          <InfoTile
                            label="Last Started"
                            value={currentTradingSettings.last_started_at ? formatTimestamp(currentTradingSettings.last_started_at) : "--"}
                          />
                          <InfoTile
                            label="Last Stopped"
                            value={currentTradingSettings.last_stopped_at ? formatTimestamp(currentTradingSettings.last_stopped_at) : "--"}
                          />
                        </div>

                        {runtimeStatesLoading || strategySignalsLoading ? (
                          <p className="app-text-muted mt-4 text-sm">Refreshing runtime state from the engine...</p>
                        ) : null}
                        {runtimeStatesError ? (
                          <p className="app-alert-error mt-4 rounded-2xl px-4 py-3 text-sm">{runtimeStatesError}</p>
                        ) : null}
                        {!runtimeStatesError && strategySignalsError ? (
                          <p className="app-alert-error mt-4 rounded-2xl px-4 py-3 text-sm">{strategySignalsError}</p>
                        ) : null}
                      </section>

                      <section className="app-surface-muted rounded-2xl p-5">
                        <div className="flex flex-wrap items-start justify-between gap-4">
                          <div>
                            <p className="text-sm font-medium">Risk controls</p>
                            <p className="app-text-muted mt-2 text-sm">
                              These limits are enforced before the engine submits paper orders.
                            </p>
                            <p className="app-text-muted mt-1 text-sm">
                              Kill switch immediately disables this strategy.
                            </p>
                          </div>
                          <div className="flex flex-wrap gap-3">
                            <button
                              type="button"
                              onClick={handleResumeTrading}
                              disabled={riskConfigSaving || riskConfigLoading}
                              className="app-button-secondary rounded-full px-4 py-2 text-sm font-medium disabled:cursor-not-allowed disabled:opacity-60"
                            >
                              Resume Trading
                            </button>
                            <button
                              type="button"
                              onClick={handleKillSwitch}
                              disabled={riskConfigSaving || riskConfigLoading}
                              className="app-button-danger rounded-full px-4 py-2 text-sm font-medium disabled:cursor-not-allowed disabled:opacity-60"
                            >
                              Kill Switch
                            </button>
                          </div>
                        </div>

                        {riskConfigLoading ? (
                          <p className="app-text-muted mt-4 text-sm">Loading risk controls...</p>
                        ) : null}

                        <div className="mt-4 grid gap-4 md:grid-cols-2 xl:grid-cols-3">
                          <label className="block">
                            <span className="mb-2 block text-sm font-medium">Max dollars per trade</span>
                            <input
                              value={riskConfigForm.max_dollars_per_trade}
                              onChange={(event) => setRiskConfigForm((current) => ({ ...current, max_dollars_per_trade: event.target.value }))}
                              className="app-input w-full rounded-2xl px-4 py-3 text-sm"
                              disabled={riskConfigSaving || riskConfigLoading}
                            />
                          </label>
                          <label className="block">
                            <span className="mb-2 block text-sm font-medium">Max quantity per trade</span>
                            <input
                              value={riskConfigForm.max_quantity_per_trade}
                              onChange={(event) => setRiskConfigForm((current) => ({ ...current, max_quantity_per_trade: event.target.value }))}
                              className="app-input w-full rounded-2xl px-4 py-3 text-sm"
                              disabled={riskConfigSaving || riskConfigLoading}
                            />
                          </label>
                          <label className="block">
                            <span className="mb-2 block text-sm font-medium">Max position value per symbol</span>
                            <input
                              value={riskConfigForm.max_position_value_per_symbol}
                              onChange={(event) => setRiskConfigForm((current) => ({ ...current, max_position_value_per_symbol: event.target.value }))}
                              className="app-input w-full rounded-2xl px-4 py-3 text-sm"
                              disabled={riskConfigSaving || riskConfigLoading}
                            />
                          </label>
                          <label className="block">
                            <span className="mb-2 block text-sm font-medium">Max total exposure</span>
                            <input
                              value={riskConfigForm.max_total_exposure}
                              onChange={(event) => setRiskConfigForm((current) => ({ ...current, max_total_exposure: event.target.value }))}
                              className="app-input w-full rounded-2xl px-4 py-3 text-sm"
                              disabled={riskConfigSaving || riskConfigLoading}
                            />
                          </label>
                          <label className="block">
                            <span className="mb-2 block text-sm font-medium">Max open positions</span>
                            <input
                              value={riskConfigForm.max_open_positions}
                              onChange={(event) => setRiskConfigForm((current) => ({ ...current, max_open_positions: event.target.value }))}
                              className="app-input w-full rounded-2xl px-4 py-3 text-sm"
                              disabled={riskConfigSaving || riskConfigLoading}
                            />
                          </label>
                          <label className="block">
                            <span className="mb-2 block text-sm font-medium">Max daily trades</span>
                            <input
                              value={riskConfigForm.max_daily_trades}
                              onChange={(event) => setRiskConfigForm((current) => ({ ...current, max_daily_trades: event.target.value }))}
                              className="app-input w-full rounded-2xl px-4 py-3 text-sm"
                              disabled={riskConfigSaving || riskConfigLoading}
                            />
                          </label>
                          <label className="block">
                            <span className="mb-2 block text-sm font-medium">Max daily loss</span>
                            <input
                              value={riskConfigForm.max_daily_loss}
                              onChange={(event) => setRiskConfigForm((current) => ({ ...current, max_daily_loss: event.target.value }))}
                              className="app-input w-full rounded-2xl px-4 py-3 text-sm"
                              disabled={riskConfigSaving || riskConfigLoading}
                            />
                          </label>
                          <label className="block">
                            <span className="mb-2 block text-sm font-medium">Cooldown seconds</span>
                            <input
                              value={riskConfigForm.cooldown_seconds}
                              onChange={(event) => setRiskConfigForm((current) => ({ ...current, cooldown_seconds: event.target.value }))}
                              className="app-input w-full rounded-2xl px-4 py-3 text-sm"
                              disabled={riskConfigSaving || riskConfigLoading}
                            />
                          </label>
                          <label className="block">
                            <span className="mb-2 block text-sm font-medium">Allowlist symbols</span>
                            <input
                              value={riskConfigForm.allowlist_symbols}
                              onChange={(event) => setRiskConfigForm((current) => ({ ...current, allowlist_symbols: event.target.value }))}
                              className="app-input w-full rounded-2xl px-4 py-3 text-sm"
                              placeholder="AAPL, MSFT"
                              disabled={riskConfigSaving || riskConfigLoading}
                            />
                          </label>
                          <label className="block md:col-span-2 xl:col-span-3">
                            <span className="mb-2 block text-sm font-medium">Blocklist symbols</span>
                            <input
                              value={riskConfigForm.blocklist_symbols}
                              onChange={(event) => setRiskConfigForm((current) => ({ ...current, blocklist_symbols: event.target.value }))}
                              className="app-input w-full rounded-2xl px-4 py-3 text-sm"
                              placeholder="TSLA, GME"
                              disabled={riskConfigSaving || riskConfigLoading}
                            />
                          </label>
                        </div>

                        <div className="mt-4 flex flex-wrap gap-4">
                          <label className="flex items-center gap-3 text-sm">
                            <input
                              type="checkbox"
                              checked={riskConfigForm.is_trading_enabled}
                              onChange={(event) => setRiskConfigForm((current) => ({ ...current, is_trading_enabled: event.target.checked }))}
                              disabled={riskConfigSaving || riskConfigLoading}
                            />
                            Trading enabled
                          </label>
                          <label className="flex items-center gap-3 text-sm">
                            <input
                              type="checkbox"
                              checked={riskConfigForm.kill_switch_enabled}
                              onChange={(event) => setRiskConfigForm((current) => ({ ...current, kill_switch_enabled: event.target.checked }))}
                              disabled={riskConfigSaving || riskConfigLoading}
                            />
                            Kill switch enabled
                          </label>
                        </div>

                        <div className="mt-4 flex flex-wrap items-center gap-3">
                          <button
                            type="button"
                            onClick={handleSaveRiskConfig}
                            disabled={riskConfigSaving || riskConfigLoading}
                            className="app-button-primary rounded-full px-4 py-2 text-sm font-medium disabled:cursor-not-allowed disabled:opacity-60"
                          >
                            {riskConfigSaving ? "Saving..." : "Save risk controls"}
                          </button>
                          {riskConfigMessage ? (
                            <p className="app-text-muted text-sm">{riskConfigMessage}</p>
                          ) : null}
                        </div>
                      </section>

                      {selectedPaperAccountId ? (
                        <section className="app-surface-muted rounded-2xl p-5">
                          <div className="flex flex-wrap items-center justify-between gap-3">
                            <div>
                              <p className="text-sm font-medium">Paper account summary</p>
                              <p className="app-text-muted mt-2 text-sm">
                                Review cash, equity, open positions, and recent fills for the selected account.
                              </p>
                            </div>
                            {paperSummary?.account ? (
                              <p className="app-text-muted text-sm">
                                {paperSummary.account.name}
                              </p>
                            ) : null}
                          </div>

                          {paperSummaryLoading ? (
                            <p className="app-text-muted mt-4 text-sm">Loading account summary...</p>
                          ) : null}
                          {paperSummaryError ? (
                            <p className="app-alert-error mt-4 rounded-2xl px-4 py-3 text-sm">{paperSummaryError}</p>
                          ) : null}

                          {paperSummary ? (
                            <div className="mt-4 space-y-5">
                              <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-4">
                                <InfoTile
                                  label="Cash Balance"
                                  value={formatCurrencyWithCode(
                                    paperSummary.account.cash_balance,
                                    paperSummary.account.currency,
                                  )}
                                />
                                <InfoTile
                                  label="Estimated Equity"
                                  value={formatCurrencyWithCode(
                                    paperSummary.equity_estimate,
                                    paperSummary.account.currency,
                                  )}
                                />
                                <InfoTile
                                  label="Positions Count"
                                  value={formatNumber(paperSummary.positions.length, 0)}
                                />
                                <InfoTile
                                  label="Recent Fills Count"
                                  value={formatNumber(paperSummary.recent_fills.length, 0)}
                                />
                              </div>

                              <details className="app-surface rounded-2xl px-4 py-3">
                                <summary className="cursor-pointer text-sm font-medium">
                                  Manual test order
                                </summary>
                                <p className="app-text-muted mt-3 text-sm">
                                  Manual orders stay available for testing, but the primary workflow is engine-driven paper trading from the saved strategy.
                                </p>
                                <form className="mt-4 grid gap-3 md:grid-cols-[minmax(0,1fr)_12rem_10rem_auto]" onSubmit={handlePaperOrderSubmit}>
                                  <label className="block">
                                    <span className="mb-2 block text-sm font-medium">Symbol</span>
                                    <input
                                      value={paperOrderForm.symbol}
                                      onChange={(event) =>
                                        setPaperOrderForm((current) => ({
                                          ...current,
                                          symbol: event.target.value,
                                        }))}
                                      className="app-input w-full rounded-2xl px-4 py-3 text-sm"
                                      placeholder="AAPL"
                                      disabled={paperOrderPending || tradingConfigLoading}
                                    />
                                  </label>
                                  <label className="block">
                                    <span className="mb-2 block text-sm font-medium">Side</span>
                                    <select
                                      value={paperOrderForm.side}
                                      onChange={(event) =>
                                        setPaperOrderForm((current) => ({
                                          ...current,
                                          side: event.target.value as "buy" | "sell",
                                        }))}
                                      className="app-input w-full rounded-2xl px-4 py-3 text-sm"
                                      disabled={paperOrderPending || tradingConfigLoading}
                                    >
                                      <option value="buy">Buy</option>
                                      <option value="sell">Sell</option>
                                    </select>
                                  </label>
                                  <label className="block">
                                    <span className="mb-2 block text-sm font-medium">Quantity</span>
                                    <input
                                      value={paperOrderForm.quantity}
                                      onChange={(event) =>
                                        setPaperOrderForm((current) => ({
                                          ...current,
                                          quantity: event.target.value,
                                        }))}
                                      type="number"
                                      min="0"
                                      step="0.01"
                                      className="app-input w-full rounded-2xl px-4 py-3 text-sm"
                                      disabled={paperOrderPending || tradingConfigLoading}
                                    />
                                  </label>
                                  <div className="flex items-end">
                                    <button
                                      type="submit"
                                      disabled={paperOrderPending || tradingConfigLoading}
                                      className="app-button-primary w-full rounded-full px-4 py-3 text-sm font-medium disabled:cursor-not-allowed disabled:opacity-60"
                                    >
                                      {paperOrderPending ? "Submitting..." : "Submit Paper Order"}
                                    </button>
                                  </div>
                                </form>
                                {paperOrderMessage ? (
                                  <p className="app-text-muted mt-3 text-sm">{paperOrderMessage}</p>
                                ) : null}
                              </details>

                              <div className="grid gap-5 xl:grid-cols-2">
                                <CompactTable
                                  title="Open positions"
                                  emptyMessage="No open positions in this paper account."
                                  columns={["Symbol", "Quantity", "Avg Price", "Realized P/L"]}
                                  rows={paperSummary.positions.map((position) => [
                                    position.symbol,
                                    formatNumber(position.quantity, 4),
                                    formatCurrencyWithCode(position.average_price, paperSummary.account.currency),
                                    formatCurrencyWithCode(position.realized_pnl, paperSummary.account.currency),
                                  ])}
                                />
                                <CompactTable
                                  title="Recent fills"
                                  emptyMessage="No recent fills yet."
                                  columns={["Time", "Symbol", "Side", "Qty", "Price"]}
                                  rows={paperSummary.recent_fills.map((fill) => [
                                    formatTimestamp(fill.created_at),
                                    fill.symbol,
                                    fill.side.toUpperCase(),
                                    formatNumber(fill.quantity, 4),
                                    formatCurrencyWithCode(fill.price, paperSummary.account.currency),
                                  ])}
                                />
                              </div>

                              <div className="grid gap-5 xl:grid-cols-2">
                                <CompactTable
                                  title="Runtime state"
                                  emptyMessage="No runtime state recorded yet. Enable the strategy and wait for the engine to evaluate it."
                                  columns={["Symbol", "State", "Last Signal", "Last Evaluated"]} 
                                  rows={runtimeStates.map((state) => [
                                    state.symbol,
                                    state.position_state,
                                    state.last_signal ?? "none",
                                    state.last_evaluated_at ? formatTimestamp(state.last_evaluated_at) : "--",
                                  ])}
                                />
                                <CompactTable
                                  title="Recent signals"
                                  emptyMessage="No strategy signals recorded yet."
                                  columns={["Time", "Symbol", "Signal", "Reason"]}
                                  rows={strategySignals.slice(0, 10).map((signal) => [
                                    formatTimestamp(signal.created_at),
                                    signal.symbol,
                                    signal.signal_type,
                                    signal.reason,
                                  ])}
                                />
                              </div>

                              <CompactTable
                                title="Recent blocked signals"
                                emptyMessage="No signals have been blocked by risk controls."
                                columns={["Time", "Symbol", "Signal", "Risk Reason"]}
                                rows={strategySignals
                                  .filter((signal) => signal.status === "blocked_by_risk")
                                  .slice(0, 10)
                                  .map((signal) => [
                                    formatTimestamp(signal.created_at),
                                    signal.symbol,
                                    signal.signal_type,
                                    signal.risk_reason ?? signal.reason,
                                  ])}
                              />
                            </div>
                          ) : null}
                        </section>
                      ) : null}
                    </div>
                  ) : null}
                </div>
              </article>
            ) : null}
          </section>
        </div>
      </section>
    </main>
  );
}

function InfoTile({ label, value }: { label: string; value: string }) {
  return (
    <div className="app-surface-muted rounded-2xl px-4 py-3">
      <p className="app-text-muted text-xs uppercase tracking-[0.18em]">{label}</p>
      <p className="mt-2 text-sm font-medium leading-6">{value}</p>
    </div>
  );
}

function CompactTable({
  title,
  columns,
  rows,
  emptyMessage,
}: {
  title: string;
  columns: string[];
  rows: string[][];
  emptyMessage: string;
}) {
  return (
    <div className="app-surface rounded-2xl p-4">
      <div className="mb-3 flex items-center justify-between gap-3">
        <p className="text-sm font-medium">{title}</p>
        <p className="app-text-muted text-xs uppercase tracking-[0.18em]">{rows.length} rows</p>
      </div>

      {rows.length === 0 ? (
        <p className="app-text-muted text-sm">{emptyMessage}</p>
      ) : (
        <div className="overflow-x-auto">
          <table className="min-w-full text-left text-sm">
            <thead>
              <tr className="app-text-muted border-b" style={{ borderColor: "var(--color-border)" }}>
                {columns.map((column) => (
                  <th key={column} className="px-3 py-2 text-xs font-medium uppercase tracking-[0.18em]">
                    {column}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {rows.map((row, index) => (
                <tr key={`${title}-${index}`} className="border-b last:border-b-0" style={{ borderColor: "var(--color-border)" }}>
                  {row.map((value, cellIndex) => (
                    <td key={`${title}-${index}-${cellIndex}`} className="px-3 py-2.5 align-top">
                      {value}
                    </td>
                  ))}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}

function SymbolBacktestChart({ result }: { result: BacktestResult }) {
  const width = 980;
  const height = 280;
  const path = buildChartPath(result.bars, width, height);
  const minPrice = result.bars.length ? Math.min(...result.bars.map((bar) => bar.low)) : 0;
  const maxPrice = result.bars.length ? Math.max(...result.bars.map((bar) => bar.high)) : 1;
  const span = Math.max(maxPrice - minPrice, Number.EPSILON);

  return (
    <div className="space-y-4">
      <div className="overflow-x-auto">
        <svg viewBox={`0 0 ${width} ${height}`} className="h-72 min-w-[60rem] w-full" preserveAspectRatio="none">
          {result.trades.map((trade) => {
            const startX = result.bars.length <= 1 ? 0 : (trade.entryIndex / (result.bars.length - 1)) * width;
            const endX = result.bars.length <= 1 ? width : (trade.exitIndex / (result.bars.length - 1)) * width;
            return (
              <rect
                key={`${trade.entryIndex}-${trade.exitIndex}`}
                x={startX}
                y={0}
                width={Math.max(endX - startX, 6)}
                height={height}
                fill={trade.profitLoss >= 0 ? "rgba(16, 185, 129, 0.12)" : "rgba(239, 68, 68, 0.12)"}
              />
            );
          })}
          <path d={path} fill="none" stroke="var(--color-primary)" strokeWidth="2.5" vectorEffect="non-scaling-stroke" />
          {result.trades.map((trade) => {
            const entryX = result.bars.length <= 1 ? width / 2 : (trade.entryIndex / (result.bars.length - 1)) * width;
            const exitX = result.bars.length <= 1 ? width / 2 : (trade.exitIndex / (result.bars.length - 1)) * width;
            const entryY = height - ((trade.entryPrice - minPrice) / span) * height;
            const exitY = height - ((trade.exitPrice - minPrice) / span) * height;
            return (
              <g key={`${trade.entryIndex}-${trade.exitIndex}-markers`}>
                <circle cx={entryX} cy={entryY} r="4.5" fill="#10B981" />
                <circle cx={exitX} cy={exitY} r="4.5" fill="#EF4444" />
              </g>
            );
          })}
        </svg>
      </div>
      <div className="grid gap-3 md:grid-cols-3">
        {result.trades.slice(0, 3).map((trade) => (
          <div key={`${trade.entryIndex}-${trade.exitIndex}-summary`} className="app-surface rounded-2xl px-4 py-3">
            <p className="app-text-muted text-xs uppercase tracking-[0.18em]">
              Trade {trade.entryIndex + 1} to {trade.exitIndex + 1}
            </p>
            <p className="mt-2 text-sm font-medium">
              {formatCurrency(trade.entryPrice)} to {formatCurrency(trade.exitPrice)}
            </p>
            <p className="app-text-muted mt-2 text-sm">
              {formatPercent(trade.returnPct)} ({formatCurrency(trade.profitLoss)})
            </p>
          </div>
        ))}
        {result.trades.length === 0 ? (
          <div className="app-surface rounded-2xl px-4 py-3 md:col-span-3">
            <p className="app-text-muted text-sm">No prototype trades were generated for this symbol with the current draft.</p>
          </div>
        ) : null}
      </div>
    </div>
  );
}
