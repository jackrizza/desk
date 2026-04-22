import { useEffect, useMemo, useRef, useState } from "react";
import { useNavigate, useParams } from "react-router";
import { LeftSidebar } from "../components/LeftSidebar";
import { Topbar } from "../components/Topbar";
import { deskApi, type Project, type RawStockData } from "../lib/api";
import { useDeskChat } from "../lib/chat";

type ProjectTab = "build" | "backtest" | "live";
type BuildPanelTab = "chat" | "draft";

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

const BACKTEST_STARTING_CAPITAL = 10_000;

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

function formatPercent(value: number) {
  return `${value >= 0 ? "+" : ""}${value.toFixed(2)}%`;
}

function serialize(value: unknown) {
  return JSON.stringify(value);
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
    const targetPath = `/projects/${encodeURIComponent(selectedProject.id)}/${targetTab}`;

    if (params.projectId !== selectedProject.id || params.tab !== targetTab) {
      navigate(targetPath, { replace: true });
    }
  }, [navigate, params.projectId, params.tab, projects.length, selectedProject]);

  useEffect(() => {
    setStrategyPrompt("");
    setSaveMessage("");
    setBuildPanelTab("chat");
  }, [selectedProject?.id]);

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
  const canEditStrategy = hydrated && Boolean(selectedProject);
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

    navigate(`/projects/${encodeURIComponent(selectedProject.id)}/${tab}`);
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
      const updatedProject = await deskApi.updateProject(selectedProject.id, {
        ...selectedProject,
        strategy: nextStrategy,
        updated_at: new Date().toISOString(),
      });

      setProjects((current) =>
        current.map((project) =>
          project.id === updatedProject.id ? updatedProject : project,
        ),
      );
      setSaveMessage("Strategy saved to this project.");
    } catch (error) {
      setSaveMessage(
        error instanceof Error ? error.message : "Failed to save strategy.",
      );
    } finally {
      setSavePending(false);
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
                <p className="app-text-muted text-xs uppercase tracking-[0.22em]">Project Workspace</p>
                <h1 className="mt-2 text-3xl font-semibold">
                  {selectedProject?.name ?? (loading ? "Loading projects..." : "No projects")}
                </h1>
                <p className="app-text-muted mt-3 text-sm leading-6">
                  {selectedProject?.description || "Use this workspace to turn a project idea into an algorithmic strategy based on the symbols tracked in that project."}
                </p>
              </article>

              <article className="app-surface rounded-3xl p-5 shadow-sm">
                <p className="app-text-muted text-xs uppercase tracking-[0.18em]">Project Signals</p>
                <div className="mt-4 grid gap-3">
                  <InfoTile label="Symbols" value={selectedProject?.symbols.length ? selectedProject.symbols.join(", ") : "None"} />
                  <InfoTile label="Interval" value={selectedProject?.interval ?? "--"} />
                  <InfoTile label="Range" value={selectedProject?.range ?? "--"} />
                  <InfoTile label="Updated" value={selectedProject ? formatTimestamp(selectedProject.updated_at) : "--"} />
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
                <div className="app-surface-muted rounded-2xl p-5">
                  <p className="text-sm leading-7">
                    This tab is reserved for moving a validated strategy into live monitoring or execution for the selected project universe.
                  </p>
                  <p className="app-text-muted mt-4 text-sm leading-7">
                    Planned flow: enable signals, connect risk limits, review live state, and arm the strategy for paper or live trading.
                  </p>
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
