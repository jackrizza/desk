import { useEffect, useMemo, useRef, useState } from "react";
import { createTheme, ThemeProvider } from "@mui/material/styles";
import { BarChart } from "@mui/x-charts/BarChart";
import { Unstable_CandlestickChart } from "@mui/x-charts-premium/CandlestickChart";
import { useNavigate, useParams } from "react-router";
import { ChatPanel } from "../components/ChatPanel";
import { LeftSidebar } from "../components/LeftSidebar";
import { Topbar } from "../components/Topbar";
import { deskApi, type IndicatorResult, type RawStockData, type StockIndicatorsResponse } from "../lib/api";
import { useDeskChat } from "../lib/chat";
import { getResolvedTheme, THEME_CHANGE_EVENT, type Theme } from "../lib/theme";
import { CHAT_OPEN_STORAGE_KEY, usePersistentBoolean } from "../lib/ui-state";

const DEFAULT_INTERVAL = "1d";
const DEFAULT_RANGE = "6mo";
const INTERVAL_OPTIONS = ["1m", "5m", "15m", "30m", "1h", "1d", "1wk"];
const RANGE_OPTIONS = ["1d", "5d", "1mo", "3mo", "6mo", "1y", "2y", "5y"];
const INDICATOR_CATALOG = [
  "ADX/DMS",
  "ATR Bands",
  "ATR Trailing Stops",
  "Accumulation/Distribution",
  "Accumulative Swing Index",
  "Alligator",
  "Anchored VWAP",
  "Aroon",
  "Aroon Oscillator",
  "Average True Range",
  "Awesome Oscillator",
  "Balance of Power",
  "Beta",
  "Bollinger %b",
  "Bollinger Bands",
  "Bollinger Bandwidth",
  "Candlestick Patterns",
  "Center Of Gravity",
  "Central Pivot Range",
  "Chaikin Money Flow",
  "Chaikin Volatility",
  "Chande Forecast Oscillator",
  "Chande Momentum Oscillator",
  "Choppiness Index",
  "Commodity Channel Index",
  "Coppock Curve",
  "Correlation Coefficient",
  "Darvas Box",
  "Detrended Price Oscillator",
  "Disparity Index",
  "Donchian Channel",
  "Donchian Width",
  "Ease of Movement",
  "Ehler Fisher Transform",
  "Elder Force Index",
  "Elder Impulse System",
  "Elder Ray Index",
  "Fractal Chaos Bands",
  "Fractal Chaos Oscillator",
  "Gator Oscillator",
  "Gopalakrishnan Range Index",
  "High Low Bands",
  "High Minus Low",
  "Highest High Value",
  "Historical Volatility",
  "Ichimoku Clouds",
  "Intraday Momentum Index",
  "Keltner Channel",
  "Klinger Volume Oscillator",
  "Linear Reg Forecast",
  "Linear Reg Intercept",
  "Linear Reg R2",
  "Linear Reg Slope",
  "Lowest Low Value",
  "MACD",
  "MACD Divergence",
  "Market Facilitation Index",
  "Mass Index",
  "Median Price",
  "Momentum Indicator",
  "Money Flow Index",
  "Moving Average",
  "Moving Average Deviation",
  "Moving Average Envelope",
  "Negative Volume Index",
  "On Balance Volume",
  "Open Interest",
  "Parabolic SAR",
  "Performance Index",
  "Pivot Points",
  "Positive Volume Index",
  "Pretty Good Oscillator",
  "Price Momentum Oscillator",
  "Price Oscillator",
  "Price Rate of Change",
  "Price Relative",
  "Price Volume Trend",
  "Prime Number Bands",
  "Prime Number Oscillator",
  "Pring's Know Sure Thing",
  "Pring's Special K",
  "Psychological Line",
  "QStick",
  "RAVI",
  "RSI",
  "RSI Divergence",
  "Rainbow Moving Average",
  "Rainbow Oscillator",
  "Random Walk Index",
  "Relative Vigor Index",
  "Relative Volatility",
  "STARC Bands",
  "Schaff Trend Cycle",
  "Shinohara Intensity Ratio",
  "Standard Deviation",
  "Stochastic Divergence",
  "Stochastic Momentum Index",
  "Stochastic RSI",
  "Stochastics",
  "Super Trend",
  "Swing Index",
  "TRIX",
  "Time Series Forecast",
  "Trade Volume Index",
  "Trend Intensity Index",
  "True Range",
  "Twiggs Money Flow",
  "Typical Price",
  "Ulcer Index",
  "Ultimate Oscillator",
  "VWAP",
  "Valuation Lines",
  "Vertical Horizontal Filter",
  "Volume Chart",
  "Volume Oscillator",
  "Volume Profile",
  "Volume Rate of Change",
  "Volume Underlay",
  "Vortex Index",
  "Weighted Close",
  "Williams %R",
  "ZigZag",
] as const;
const SUPPORTED_INDICATORS = new Set([
  "ADX/DMS",
  "ATR Bands",
  "ATR Trailing Stops",
  "Accumulation/Distribution",
  "Accumulative Swing Index",
  "Alligator",
  "Aroon",
  "Aroon Oscillator",
  "Average True Range",
  "Bollinger %b",
  "Bollinger Bands",
  "Bollinger Bandwidth",
  "Chaikin Money Flow",
  "Commodity Channel Index",
  "Donchian Channel",
  "Donchian Width",
  "Ease of Movement",
  "Elder Force Index",
  "Keltner Channel",
  "Klinger Volume Oscillator",
  "MACD",
  "Mass Index",
  "Money Flow Index",
  "Moving Average",
  "Moving Average Envelope",
  "Negative Volume Index",
  "On Balance Volume",
  "Parabolic SAR",
  "Positive Volume Index",
  "Price Oscillator",
  "Price Rate of Change",
  "Price Volume Trend",
  "RSI",
  "Standard Deviation",
  "Stochastics",
  "Super Trend",
  "TRIX",
  "True Range",
  "Twiggs Money Flow",
  "Typical Price",
  "Ulcer Index",
  "Ultimate Oscillator",
  "VWAP",
  "Vortex Index",
  "Williams %R",
  "ZigZag",
]);

type MarketDatum = {
  date: Date;
  dateLabel: string;
  open: number;
  high: number;
  low: number;
  close: number;
  volume: number;
};

type IndicatorChartDatum = {
  date: Date;
  dateLabel: string;
} & Record<string, number | null | Date | string>;

type OverlayLineSeries = {
  id: string;
  dataKey: string;
  label: string;
  color: string;
};

type OverlayPath = {
  id: string;
  label: string;
  color: string;
  d: string;
};

function formatCompact(value: number) {
  if (value >= 1_000_000_000) {
    return `${(value / 1_000_000_000).toFixed(2)}B`;
  }
  if (value >= 1_000_000) {
    return `${(value / 1_000_000).toFixed(2)}M`;
  }
  if (value >= 1_000) {
    return `${(value / 1_000).toFixed(2)}K`;
  }
  return value.toFixed(0);
}

function formatDateLabel(date: Date, interval: string) {
  if (interval.includes("m") || interval.includes("h")) {
    return new Intl.DateTimeFormat("en-US", {
      month: "short",
      day: "numeric",
      hour: "numeric",
      minute: "2-digit",
    }).format(date);
  }

  return new Intl.DateTimeFormat("en-US", {
    month: "short",
    day: "numeric",
  }).format(date);
}

function formatPrice(value: number | null) {
  if (value === null || Number.isNaN(value)) {
    return "--";
  }

  return `$${value.toFixed(2)}`;
}

function formatRefreshDate(value: string | null | undefined) {
  if (!value) {
    return "--";
  }

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

function toggleSelection(current: string[], next: string) {
  return current.includes(next)
    ? current.filter((value) => value !== next)
    : [...current, next];
}

export default function MarketRoute() {
  const params = useParams();
  const navigate = useNavigate();
  const chartAreaRef = useRef<HTMLDivElement | null>(null);
  const [sidebarOpen, setSidebarOpen] = useState(true);
  const [quoteSymbol, setQuoteSymbol] = useState((params.symbol ?? "AAPL").toUpperCase());
  const [stockData, setStockData] = useState<RawStockData | null>(null);
  const [indicatorData, setIndicatorData] = useState<StockIndicatorsResponse | null>(null);
  const [loading, setLoading] = useState(false);
  const [indicatorLoading, setIndicatorLoading] = useState(false);
  const [errorMessage, setErrorMessage] = useState("");
  const [interval, setInterval] = useState(DEFAULT_INTERVAL);
  const [range, setRange] = useState(DEFAULT_RANGE);
  const [indicatorSearch, setIndicatorSearch] = useState("");
  const [isDarkMode, setIsDarkMode] = useState(false);
  const [viewportHeight, setViewportHeight] = useState(900);
  const [chartWidth, setChartWidth] = useState(0);
  const [selectedIndicators, setSelectedIndicators] = useState<string[]>([]);
  const [chatOpen, setChatOpen] = usePersistentBoolean(
    CHAT_OPEN_STORAGE_KEY,
    false,
  );

  const activeSymbol = (params.symbol ?? "AAPL").toUpperCase();

  useEffect(() => {
    setQuoteSymbol(activeSymbol);
  }, [activeSymbol]);

  useEffect(() => {
    const updateTheme = () => {
      setIsDarkMode(getResolvedTheme() === "dark");
    };

    updateTheme();

    const handleThemeChange = (event: Event) => {
      const customEvent = event as CustomEvent<Theme>;
      if (customEvent.detail === "light" || customEvent.detail === "dark") {
        setIsDarkMode(customEvent.detail === "dark");
        return;
      }

      updateTheme();
    };

    const handleStorage = (event: StorageEvent) => {
      if (event.key === "desk-theme") {
        updateTheme();
      }
    };

    window.addEventListener(THEME_CHANGE_EVENT, handleThemeChange as EventListener);
    window.addEventListener("storage", handleStorage);

    return () => {
      window.removeEventListener(THEME_CHANGE_EVENT, handleThemeChange as EventListener);
      window.removeEventListener("storage", handleStorage);
    };
  }, []);

  useEffect(() => {
    const handleResize = () => setViewportHeight(window.innerHeight);

    handleResize();
    window.addEventListener("resize", handleResize);
    return () => window.removeEventListener("resize", handleResize);
  }, []);

  useEffect(() => {
    const element = chartAreaRef.current;
    if (!element) {
      return;
    }

    const observer = new ResizeObserver((entries) => {
      const nextWidth = entries[0]?.contentRect.width ?? 0;
      setChartWidth(nextWidth);
    });

    observer.observe(element);
    setChartWidth(element.getBoundingClientRect().width);

    return () => observer.disconnect();
  }, []);

  useEffect(() => {
    let cancelled = false;

    async function loadMarketData() {
      setLoading(true);
      setErrorMessage("");

      try {
        const nextData = await deskApi.getStockData({
          symbol: activeSymbol,
          range,
          interval,
          prepost: false,
        });

        if (!cancelled) {
          setStockData(nextData);
        }
      } catch (error) {
        if (!cancelled) {
          setErrorMessage(
            error instanceof Error ? error.message : "Failed to load market data.",
          );
        }
      } finally {
        if (!cancelled) {
          setLoading(false);
        }
      }
    }

    void loadMarketData();

    return () => {
      cancelled = true;
    };
  }, [activeSymbol, interval, range]);

  useEffect(() => {
    let cancelled = false;

    async function loadIndicators() {
      if (selectedIndicators.length === 0) {
        setIndicatorData(null);
        return;
      }

      setIndicatorLoading(true);

      try {
        const nextData = await deskApi.getIndicators({
          symbol: activeSymbol,
          range,
          interval,
          prepost: false,
          indicators: selectedIndicators,
        });

        if (!cancelled) {
          setIndicatorData(nextData);
        }
      } catch (error) {
        if (!cancelled) {
          setErrorMessage(
            error instanceof Error ? error.message : "Failed to load indicators.",
          );
        }
      } finally {
        if (!cancelled) {
          setIndicatorLoading(false);
        }
      }
    }

    void loadIndicators();

    return () => {
      cancelled = true;
    };
  }, [activeSymbol, interval, range, selectedIndicators]);

  const dataset = useMemo<MarketDatum[]>(
    () =>
      [...(stockData?.stock_data ?? [])].slice(-60).map((entry) => {
        const date = new Date(entry.date);

        return {
          date,
          dateLabel: entry.date,
          open: Number.parseFloat(entry.open),
          high: Number.parseFloat(entry.high),
          low: Number.parseFloat(entry.low),
          close: Number.parseFloat(entry.close),
          volume: Number.parseInt(entry.volume, 10),
        };
      }),
    [stockData],
  );

  const indicatorOverlayDataset = useMemo<IndicatorChartDatum[]>(() => {
    const lineValueMaps = new Map<string, Map<string, number>>();

    for (const indicator of indicatorData?.indicators ?? []) {
      for (const line of indicator.lines) {
        const overlayKey = `${indicator.key}-${line.key}`;
        lineValueMaps.set(
          overlayKey,
          new Map(line.points.map((point) => [point.date, point.value])),
        );
      }
    }

    return dataset.map((entry) => {
      const row: IndicatorChartDatum = {
        date: entry.date,
        dateLabel: entry.dateLabel,
      };

      for (const [lineKey, valuesByDate] of lineValueMaps) {
        row[lineKey] = valuesByDate.get(entry.dateLabel) ?? null;
      }

      return row;
    });
  }, [dataset, indicatorData]);

  const overlayLineSeries = useMemo<OverlayLineSeries[]>(
    () =>
      (indicatorData?.indicators ?? []).flatMap((indicator, indicatorIndex) =>
        indicator.lines.map((line, lineIndex) => ({
          id: `${indicator.key}-${line.key}`,
          dataKey: `${indicator.key}-${line.key}`,
          label: `${indicator.display_name} - ${line.label}`,
          color:
            (indicatorIndex + lineIndex) % 3 === 0
              ? (isDarkMode ? "#60A5FA" : "#3B82F6")
              : (indicatorIndex + lineIndex) % 3 === 1
                ? (isDarkMode ? "#A78BFA" : "#8B5CF6")
                : (isDarkMode ? "#34D399" : "#10B981"),
        })),
      ),
    [indicatorData, isDarkMode],
  );
  const filteredIndicators = useMemo(() => {
    const query = indicatorSearch.trim().toLowerCase();

    if (!query) {
      return INDICATOR_CATALOG;
    }

    return INDICATOR_CATALOG.filter((indicator) =>
      indicator.toLowerCase().includes(query),
    );
  }, [indicatorSearch]);

  const lastClose = dataset.length > 0 ? dataset[dataset.length - 1].close : null;
  const overlayValues = useMemo(
    () =>
      indicatorOverlayDataset.flatMap((entry) =>
        Object.entries(entry)
          .filter(([key]) => key !== "date" && key !== "dateLabel")
          .map(([, value]) => (typeof value === "number" && Number.isFinite(value) ? value : null))
          .filter((value): value is number => value !== null),
      ),
    [indicatorOverlayDataset],
  );
  const visibleHigh = dataset.length
    ? Math.max(
        ...dataset.map((entry) => entry.high),
        ...(overlayValues.length > 0 ? overlayValues : [Number.NEGATIVE_INFINITY]),
      )
    : null;
  const visibleLow = dataset.length
    ? Math.min(
        ...dataset.map((entry) => entry.low),
        ...(overlayValues.length > 0 ? overlayValues : [Number.POSITIVE_INFINITY]),
      )
    : null;
  const priceRangePadding =
    visibleHigh !== null && visibleLow !== null ? Math.max((visibleHigh - visibleLow) * 0.04, 0.5) : 0;
  const yAxisMin = visibleLow !== null ? visibleLow - priceRangePadding : undefined;
  const yAxisMax = visibleHigh !== null ? visibleHigh + priceRangePadding : undefined;
  const cardHeight = Math.max(viewportHeight - 280, 480);
  const candlestickHeight = Math.max(Math.floor(cardHeight * 0.68), 320);
  const volumeHeight = Math.max(Math.floor(cardHeight * 0.18), 100);
  const candleMargin = { top: 10, right: 24, bottom: 10, left: 70 };
  const overlayPaths = useMemo<OverlayPath[]>(() => {
    if (dataset.length === 0 || overlayLineSeries.length === 0 || chartWidth <= 0) {
      return [];
    }

    const plotWidth = Math.max(chartWidth - candleMargin.left - candleMargin.right, 1);
    const plotHeight = Math.max(candlestickHeight - candleMargin.top - candleMargin.bottom, 1);
    const stepX = plotWidth / dataset.length;
    const minValue = yAxisMin ?? visibleLow ?? 0;
    const maxValue = yAxisMax ?? visibleHigh ?? 1;
    const valueSpan = Math.max(maxValue - minValue, Number.EPSILON);
    const dateToIndex = new Map(dataset.map((entry, index) => [entry.dateLabel, index]));

    return overlayLineSeries.map((series) => {
      const commands: string[] = [];
      let drawing = false;

      for (const row of indicatorOverlayDataset) {
        const rawValue = row[series.dataKey];
        const dataIndex = dateToIndex.get(row.dateLabel);

        if (typeof rawValue !== "number" || !Number.isFinite(rawValue) || dataIndex === undefined) {
          drawing = false;
          continue;
        }

        const x = candleMargin.left + stepX * dataIndex + stepX / 2;
        const normalized = (rawValue - minValue) / valueSpan;
        const y = candleMargin.top + plotHeight - normalized * plotHeight;
        commands.push(`${drawing ? "L" : "M"} ${x.toFixed(2)} ${y.toFixed(2)}`);
        drawing = true;
      }

      return {
        id: series.id,
        label: series.label,
        color: series.color,
        d: commands.join(" "),
      };
    }).filter((path) => path.d.length > 0);
  }, [
    candlestickHeight,
    chartWidth,
    dataset,
    indicatorOverlayDataset,
    overlayLineSeries,
    visibleHigh,
    visibleLow,
    yAxisMax,
    yAxisMin,
  ]);
  const muiTheme = useMemo(
    () =>
      createTheme({
        palette: {
          mode: isDarkMode ? "dark" : "light",
          primary: { main: isDarkMode ? "#60A5FA" : "#3B82F6" },
          secondary: { main: isDarkMode ? "#A78BFA" : "#8B5CF6" },
          background: {
            default: isDarkMode ? "#1F2937" : "#FFFFFF",
            paper: isDarkMode ? "#243244" : "#FFFFFF",
          },
          text: {
            primary: isDarkMode ? "#F9FAFB" : "#1F2937",
            secondary: isDarkMode ? "rgba(249, 250, 251, 0.72)" : "rgba(31, 41, 55, 0.72)",
          },
          success: { main: isDarkMode ? "#34D399" : "#10B981" },
          error: { main: isDarkMode ? "#F87171" : "#EF4444" },
        },
      }),
    [isDarkMode],
  );
  const chat = useDeskChat({
    page: "market",
    symbol: activeSymbol,
    interval,
    range,
    lastClose,
    barCount: dataset.length,
    lastRefreshed: stockData?.last_refreshed ?? null,
    dayHigh: visibleHigh,
    dayLow: visibleLow,
  });

  function handleSearchSubmit() {
    const nextSymbol = quoteSymbol.trim().toUpperCase();
    if (!nextSymbol) {
      return;
    }

    navigate(`/market/${encodeURIComponent(nextSymbol)}`);
  }

  return (
    <main className="app-page min-h-screen pt-16">
      <LeftSidebar
        open={sidebarOpen}
      />
      <Topbar
        onToggleSidebar={() => setSidebarOpen((open) => !open)}
        onToggleChat={() => setChatOpen((open) => !open)}
        sidebarOpen={sidebarOpen}
        chatOpen={chatOpen}
        quoteSymbol={quoteSymbol}
        onQuoteSymbolChange={setQuoteSymbol}
        onQuoteLookup={handleSearchSubmit}
        quoteLoading={loading}
      />

      <section
        className={`ml-0 flex min-h-[calc(100vh-4rem)] flex-col gap-6 p-6 transition-all duration-200 ${
          sidebarOpen ? "md:ml-64" : "md:ml-0"
        } ${chatOpen ? "lg:mr-96 xl:mr-[25vw]" : "lg:mr-0"}`}
      >
        {errorMessage ? (
          <div className="app-alert-error rounded-2xl px-4 py-3 text-sm">
            {errorMessage}
          </div>
        ) : null}

        <section className="flex flex-1 flex-col gap-6">
          <article
            className="app-surface rounded-3xl p-5 shadow-sm"
            style={{ height: `${cardHeight}px` }}
          >
            <div className="mb-4 flex flex-wrap items-center gap-3">
              <MarketStat label="Symbol" value={activeSymbol} />
              <MarketStat label="Last close" value={formatPrice(lastClose)} />
              <MarketStat
                label="Last refreshed"
                value={
                  loading && !stockData?.last_refreshed
                    ? "Loading..."
                    : formatRefreshDate(stockData?.last_refreshed)
                }
              />
              <label className="app-surface-muted flex min-w-[10rem] items-center justify-between gap-2 rounded-2xl px-4 py-3 text-sm">
                <span className="app-text-muted text-xs uppercase tracking-[0.18em]">
                  Interval
                </span>
                <select
                  value={interval}
                  onChange={(event) => setInterval(event.target.value)}
                  className="min-w-0 bg-transparent text-right font-medium outline-none"
                >
                  {INTERVAL_OPTIONS.map((option) => (
                    <option key={option} value={option}>
                      {option}
                    </option>
                  ))}
                </select>
              </label>
              <label className="app-surface-muted flex min-w-[10rem] items-center justify-between gap-2 rounded-2xl px-4 py-3 text-sm">
                <span className="app-text-muted text-xs uppercase tracking-[0.18em]">
                  Range
                </span>
                <select
                  value={range}
                  onChange={(event) => setRange(event.target.value)}
                  className="min-w-0 bg-transparent text-right font-medium outline-none"
                >
                  {RANGE_OPTIONS.map((option) => (
                    <option key={option} value={option}>
                      {option}
                    </option>
                  ))}
                </select>
              </label>
                <details className="relative">
                  <summary className="app-surface-muted flex min-w-[11rem] cursor-pointer items-center justify-between gap-2 rounded-2xl px-4 py-3 text-sm list-none">
                  <span className="app-text-muted text-xs uppercase tracking-[0.18em]">
                    Indicators
                  </span>
                  <span className="font-medium">
                    {selectedIndicators.length > 0 ? selectedIndicators.length : "Select"}
                  </span>
                </summary>
                <div className="app-surface absolute left-0 z-10 mt-2 max-h-96 w-[22rem] overflow-y-auto rounded-2xl border border-[color:var(--color-border)] p-3 shadow-xl">
                  <div className="mb-2 flex items-center justify-between">
                    <p className="text-sm font-medium">Select indicators</p>
                    {selectedIndicators.length > 0 ? (
                      <button
                        type="button"
                        onClick={() => setSelectedIndicators([])}
                        className="app-text-muted text-xs hover:opacity-80"
                      >
                        Clear
                      </button>
                    ) : null}
                  </div>
                  <input
                    type="search"
                    value={indicatorSearch}
                    onChange={(event) => setIndicatorSearch(event.target.value)}
                    placeholder="Search indicators"
                    className="app-input mb-3 w-full rounded-2xl px-3 py-2 text-sm"
                  />
                  <div className="space-y-2">
                    {filteredIndicators.map((indicator) => {
                      const supported = SUPPORTED_INDICATORS.has(indicator);
                      const checked = selectedIndicators.includes(indicator);

                      return (
                        <label
                          key={indicator}
                          className={`flex items-center justify-between gap-3 rounded-2xl px-3 py-2 text-sm ${
                            supported ? "app-surface-muted cursor-pointer" : "bg-transparent opacity-55"
                          }`}
                        >
                          <span>{indicator}</span>
                          <input
                            type="checkbox"
                            checked={checked}
                            disabled={!supported}
                            onChange={() => setSelectedIndicators((current) => toggleSelection(current, indicator))}
                          />
                        </label>
                        );
                      })}
                    {filteredIndicators.length === 0 ? (
                      <div className="app-text-muted rounded-2xl px-3 py-2 text-sm">
                        No indicators match your search.
                      </div>
                    ) : null}
                  </div>
                </div>
              </details>
              <MarketStat
                label="Bars"
                value={
                  indicatorLoading
                    ? "Refreshing..."
                    : loading
                      ? "Updating..."
                      : `${dataset.length}`
                }
              />
            </div>

            <ThemeProvider theme={muiTheme}>
              <div className="app-surface-muted flex h-full min-w-0 flex-col overflow-hidden rounded-2xl p-3">
                <div className="min-h-0 flex-[3]">
                  <div ref={chartAreaRef} className="relative min-w-0 overflow-hidden">
                    <Unstable_CandlestickChart
                      hideLegend
                      dataset={dataset}
                      margin={candleMargin}
                      height={candlestickHeight}
                      xAxis={[
                        {
                          dataKey: "date",
                          scaleType: "band",
                          valueFormatter: (value: Date) => formatDateLabel(value, interval),
                        },
                      ]}
                      yAxis={[
                        {
                          min: yAxisMin,
                          max: yAxisMax,
                          valueFormatter: (value: number) => `$${value.toFixed(2)}`,
                        },
                      ]}
                      series={[
                        {
                          datasetKeys: {
                            open: "open",
                            high: "high",
                            low: "low",
                            close: "close",
                          },
                          valueFormatter: (value, { field }) => {
                            if (value === null) {
                              return "";
                            }

                            const prefix =
                              field === "open"
                                ? "Open"
                                : field === "high"
                                  ? "High"
                                  : field === "low"
                                    ? "Low"
                                    : "Close";

                            return `${prefix}: $${value.toFixed(2)}`;
                          },
                          upColor: (mode) => (mode === "dark" ? "#34D399" : "#10B981"),
                          downColor: (mode) => (mode === "dark" ? "#F87171" : "#EF4444"),
                        },
                      ]}
                    />
                    {overlayPaths.length > 0 && chartWidth > 0 ? (
                      <svg
                        className="pointer-events-none absolute inset-0 h-full w-full"
                        viewBox={`0 0 ${Math.max(chartWidth, 1)} ${candlestickHeight}`}
                        preserveAspectRatio="none"
                        aria-hidden="true"
                      >
                        {overlayPaths.map((path) => (
                          <path
                            key={path.id}
                            d={path.d}
                            fill="none"
                            stroke={path.color}
                            strokeWidth="2.25"
                            strokeLinecap="round"
                            strokeLinejoin="round"
                            vectorEffect="non-scaling-stroke"
                          />
                        ))}
                      </svg>
                    ) : null}
                  </div>
                </div>

                <div className="min-h-0 flex-1 border-t pt-2" style={{ borderColor: "var(--color-border)" }}>
                  <BarChart
                    hideLegend
                    dataset={dataset}
                    margin={{ top: 10, right: 24, bottom: 42, left: 70 }}
                    height={volumeHeight}
                    xAxis={[
                      {
                        dataKey: "date",
                        scaleType: "band",
                        valueFormatter: (value: Date) => formatDateLabel(value, interval),
                      },
                    ]}
                    yAxis={[
                      {
                        valueFormatter: (value: number) => formatCompact(value),
                      },
                    ]}
                    series={[
                      {
                        dataKey: "volume",
                        color: isDarkMode ? "#60A5FA" : "#3B82F6",
                        label: "Volume",
                      },
                    ]}
                  />
                </div>
              </div>
            </ThemeProvider>
            {selectedIndicators.length > 0 ? (
              <div className="app-text-muted mt-3 flex flex-wrap gap-x-4 gap-y-2 text-sm">
                {indicatorData?.indicators.flatMap((indicator, indicatorIndex) =>
                  indicator.lines.map((line, lineIndex) => {
                    const color =
                      (indicatorIndex + lineIndex) % 3 === 0
                        ? (isDarkMode ? "#60A5FA" : "#3B82F6")
                        : (indicatorIndex + lineIndex) % 3 === 1
                          ? (isDarkMode ? "#A78BFA" : "#8B5CF6")
                          : (isDarkMode ? "#34D399" : "#10B981");

                    return (
                      <span key={`${indicator.key}-${line.key}`} className="inline-flex items-center gap-2">
                        <span
                          className="h-2.5 w-2.5 rounded-full"
                          style={{ backgroundColor: color }}
                        />
                        {indicator.display_name} - {line.label}
                      </span>
                    );
                  }),
                )}
              </div>
            ) : null}
          </article>
        </section>
      </section>

      <ChatPanel
        open={chatOpen}
        title={`${activeSymbol} market assistant`}
        messages={chat.messages}
        pending={chat.pending}
        suggestions={chat.suggestions}
        chatTarget={chat.chatTarget}
        chatTraders={chat.chatTraders}
        onChatTargetChange={chat.setChatTarget}
        onClose={() => setChatOpen(false)}
        onSubmit={chat.sendMessage}
        onClear={chat.clearMessages}
      />
    </main>
  );
}

function MarketStat(props: { label: string; value: string }) {
  return (
    <div className="app-surface-muted rounded-2xl px-4 py-3">
      <p className="app-text-muted text-xs uppercase tracking-[0.18em]">
        {props.label}
      </p>
      <p className="mt-2 text-sm font-medium">
        {props.value}
      </p>
    </div>
  );
}
