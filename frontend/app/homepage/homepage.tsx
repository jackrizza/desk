import { useEffect, useMemo, useRef, useState } from "react";
import { useNavigate, useSearchParams } from "react-router";
import { EmptyState, ErrorInline, LoadingInline, RefreshBadge } from "../components/AppFeedback";
import { ChatPanel } from "../components/ChatPanel";
import { LeftSidebar } from "../components/LeftSidebar";
import { SignedValue } from "../components/SignedValue";
import { Topbar } from "../components/Topbar";
import {
  deskApi,
  type Portfolio,
  type Position,
  type Project,
  type TradingAccountKind,
} from "../lib/api";
import { usePaperAccountSummary } from "../hooks/usePaperAccountSummary";
import { usePortfolioAccounts } from "../hooks/usePortfolioAccounts";
import { useDeskChat } from "../lib/chat";
import {
  getPaperPositionMetrics,
  getPaperSummaryMetrics,
} from "../lib/paper-summary";
import { CHAT_OPEN_STORAGE_KEY, usePersistentBoolean } from "../lib/ui-state";
import { useAppState } from "../state/useAppState";

type ProjectFormState = {
  id: string;
  name: string;
  description: string;
  symbols: string;
  interval: string;
  range: string;
  prepost: boolean;
};

type PortfolioFormState = {
  id: string;
  name: string;
  description: string;
};

type PositionFormState = {
  symbol: string;
  quantity: string;
  averagePrice: string;
  positionOpenedAt: string;
  positionClosedAt: string;
  positionClosedPrice: string;
};

type DisplayPosition = Position & {
  portfolioId: string;
  portfolioName: string;
};

type ClosePositionFormState = {
  positionClosedAt: string;
  positionClosedPrice: string;
};

const MANUAL_ACCOUNT_ID = "manual";

const emptyProjectForm: ProjectFormState = {
  id: "",
  name: "",
  description: "",
  symbols: "",
  interval: "1d",
  range: "1mo",
  prepost: false,
};

const emptyPortfolioForm: PortfolioFormState = {
  id: "",
  name: "",
  description: "",
};

const emptyPositionForm: PositionFormState = {
  symbol: "",
  quantity: "",
  averagePrice: "",
  positionOpenedAt: "",
  positionClosedAt: "",
  positionClosedPrice: "",
};

const emptyClosePositionForm: ClosePositionFormState = {
  positionClosedAt: "",
  positionClosedPrice: "",
};

function formatTimestamp(value: string) {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return value;
  }

  return date.toLocaleString();
}

function formatCurrency(value: number) {
  return new Intl.NumberFormat("en-US", {
    style: "currency",
    currency: "USD",
    maximumFractionDigits: 2,
  }).format(value);
}

function formatSignedCurrency(value: number) {
  const safeValue = Number.isFinite(value) ? value : 0;
  const absoluteValue = Math.abs(safeValue);
  const prefix = safeValue > 0 ? "+" : safeValue < 0 ? "-" : "";
  return `${prefix}${formatCurrency(absoluteValue)}`;
}

function getLatestClose(positionHistory: {
  stock_data: Array<{ close: string }>;
}) {
  const latestEntry = positionHistory.stock_data.at(-1);
  if (!latestEntry) {
    return null;
  }

  const close = Number(latestEntry.close);
  return Number.isFinite(close) ? close : null;
}

function makeTimestamp() {
  return new Date().toISOString();
}

function makeEntityId(prefix: "project" | "portfolio") {
  const randomPart = Math.random().toString(36).slice(2, 8);
  return `${prefix}-${Date.now().toString(36)}-${randomPart}`;
}

function toDateTimeLocalValue(value: string) {
  if (!value) {
    return "";
  }

  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return value;
  }

  const localDate = new Date(date.getTime() - date.getTimezoneOffset() * 60000);
  return localDate.toISOString().slice(0, 16);
}

function fromDateTimeLocalValue(value: string) {
  if (!value) {
    return "";
  }

  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return value;
  }

  return date.toISOString();
}

function positionKey(position: Pick<Position, "symbol" | "position_opened_at">) {
  return `${position.symbol}::${position.position_opened_at}`;
}

function displayPositionKey(position: DisplayPosition) {
  return `${position.portfolioId}::${position.symbol}::${position.position_opened_at}`;
}

function serialize(value: unknown) {
  return JSON.stringify(value);
}

function formatPercent(value: number) {
  const safeValue = Number.isFinite(value) ? value : 0;
  const prefix = safeValue > 0 ? "+" : "";
  return `${prefix}${safeValue.toFixed(2)}%`;
}

function toProjectPayload(form: ProjectFormState, existing?: Project): Project {
  const timestamp = makeTimestamp();

  return {
    id: form.id.trim(),
    name: form.name.trim(),
    description: form.description.trim(),
    strategy: existing?.strategy ?? "",
    strategy_json: existing?.strategy_json ?? "{}",
    strategy_status: existing?.strategy_status ?? "draft",
    created_at: existing?.created_at ?? timestamp,
    updated_at: timestamp,
    symbols: form.symbols
      .split(",")
      .map((symbol) => symbol.trim().toUpperCase())
      .filter(Boolean),
    interval: form.interval.trim(),
    range: form.range.trim(),
    prepost: form.prepost,
  };
}

function toPortfolioPayload(
  form: PortfolioFormState,
  existing?: Portfolio,
): Portfolio {
  const timestamp = makeTimestamp();

  return {
    id: form.id.trim(),
    name: form.name.trim(),
    description: form.description.trim(),
    created_at: existing?.created_at ?? timestamp,
    updated_at: timestamp,
    positions: existing?.positions ?? [],
  };
}

function toPositionPayload(form: PositionFormState): Position {
  return {
    symbol: form.symbol.trim().toUpperCase(),
    quantity: Number(form.quantity),
    average_price: Number(form.averagePrice),
    position_opened_at: form.positionOpenedAt.trim(),
    position_closed_at: form.positionClosedAt.trim() || null,
    position_closed_price: form.positionClosedPrice.trim()
      ? Number(form.positionClosedPrice)
      : null,
  };
}

export function Homepage() {
  const { uiState, setSelectedPaperAccountId, setSelectedPortfolioAccount } =
    useAppState();
  const { accountOptions, paperAccountsQuery, liveAccountsQuery } =
    usePortfolioAccounts();
  const selectedTradingAccount = uiState.selectedPortfolioAccount ?? {
    kind: "manual" as const,
    id: MANUAL_ACCOUNT_ID,
  };
  const selectedPaperAccountId = selectedTradingAccount.kind === "paper"
    ? selectedTradingAccount.id
    : null;
  const paperSummaryQuery = usePaperAccountSummary(selectedPaperAccountId);
  const [sidebarOpen, setSidebarOpen] = useState(true);
  const [helloMessage, setHelloMessage] = useState("Connecting to Desk API...");
  const [projects, setProjects] = useState<Project[]>([]);
  const [portfolios, setPortfolios] = useState<Portfolio[]>([]);
  const [positions, setPositions] = useState<DisplayPosition[]>([]);
  const [selectedProjectId, setSelectedProjectId] = useState<string>("");
  const [selectedPortfolioIds, setSelectedPortfolioIds] = useState<string[]>([]);
  const [selectedPortfolioId, setSelectedPortfolioId] = useState<string>("");
  const [selectedProject, setSelectedProject] = useState<Project | null>(null);
  const [selectedPortfolio, setSelectedPortfolio] = useState<Portfolio | null>(null);
  const [projectForm, setProjectForm] = useState<ProjectFormState>(emptyProjectForm);
  const [portfolioForm, setPortfolioForm] =
    useState<PortfolioFormState>(emptyPortfolioForm);
  const [positionForm, setPositionForm] =
    useState<PositionFormState>(emptyPositionForm);
  const [closePositionModalOpen, setClosePositionModalOpen] = useState(false);
  const [positionToClose, setPositionToClose] = useState<DisplayPosition | null>(null);
  const [closePositionForm, setClosePositionForm] =
    useState<ClosePositionFormState>(emptyClosePositionForm);
  const [quoteSymbol, setQuoteSymbol] = useState("AAPL");
  const [summaryMessage, setSummaryMessage] = useState("");
  const [errorMessage, setErrorMessage] = useState("");
  const [loading, setLoading] = useState(false);
  const [allPortfolioNav, setAllPortfolioNav] = useState(0);
  const [navCoverage, setNavCoverage] = useState({ priced: 0, total: 0 });
  const [navLoading, setNavLoading] = useState(false);
  const [allPortfolioGainAmount, setAllPortfolioGainAmount] = useState(0);
  const [allPortfolioGainPercentage, setAllPortfolioGainPercentage] = useState(0);
  const [projectModalOpen, setProjectModalOpen] = useState(false);
  const [portfolioModalOpen, setPortfolioModalOpen] = useState(false);
  const [chatOpen, setChatOpen] = usePersistentBoolean(
    CHAT_OPEN_STORAGE_KEY,
    false,
  );
  const navigate = useNavigate();
  const [searchParams, setSearchParams] = useSearchParams();
  const syncInFlightRef = useRef(false);
  const stateSnapshotRef = useRef({
    projects: "",
    portfolios: "",
    positions: "",
    selectedProject: "",
    selectedPortfolio: "",
  });
  const selectedProjectIdRef = useRef(selectedProjectId);
  const selectedPortfolioIdsRef = useRef(selectedPortfolioIds);
  const selectedPortfolioIdRef = useRef(selectedPortfolioId);
  const portfolioSelectionInitializedRef = useRef(false);

  const totalOpenPositions = useMemo(
    () => positions.filter((position) => !position.position_closed_at).length,
    [positions],
  );
  const scopedPortfolios = useMemo(
    () =>
      portfolios.filter(
        (portfolio) =>
          selectedPortfolioIds.length === 0 ||
          selectedPortfolioIds.includes(portfolio.id),
      ),
    [portfolios, selectedPortfolioIds],
  );
  const allOpenPortfolioPositions = useMemo(
    () =>
      scopedPortfolios.flatMap((portfolio) =>
        (portfolio.positions ?? []).filter((position) => !position.position_closed_at),
      ),
    [scopedPortfolios],
  );
  const allPortfolioPositions = useMemo(
    () => scopedPortfolios.flatMap((portfolio) => portfolio.positions ?? []),
    [scopedPortfolios],
  );
  const selectedPortfolioNames = useMemo(
    () =>
      portfolios
        .filter((portfolio) => selectedPortfolioIds.includes(portfolio.id))
        .map((portfolio) => portfolio.name),
    [portfolios, selectedPortfolioIds],
  );
  const positionAllocations = useMemo(() => {
    const palette = [
      "#0f766e",
      "#2563eb",
      "#9333ea",
      "#ea580c",
      "#dc2626",
      "#0891b2",
      "#65a30d",
      "#ca8a04",
    ];
    const openPositions = positions.filter((position) => !position.position_closed_at);
    const totalValue = openPositions.reduce(
      (sum, position) => sum + position.quantity * position.average_price,
      0,
    );

    const slices = openPositions.map((position, index) => {
      const value = position.quantity * position.average_price;
      const percentage = totalValue > 0 ? (value / totalValue) * 100 : 0;

      return {
        key: positionKey(position),
        symbol: position.symbol,
        value,
        percentage,
        color: palette[index % palette.length],
      };
    });

    let currentAngle = 0;
    const gradientStops = slices
      .map((slice) => {
        const start = currentAngle;
        currentAngle += slice.percentage;
        return `${slice.color} ${start}% ${currentAngle}%`;
      })
      .join(", ");

    return {
      totalValue,
      slices,
      background:
        gradientStops ||
        "rgba(148, 163, 184, 0.25) 0% 100%",
    };
  }, [positions]);
  const portfolioSelectionLabel = useMemo(() => {
    if (!portfolios.length) {
      return "No portfolios";
    }
    if (selectedPortfolioIds.length === portfolios.length) {
      return "All portfolios";
    }
    if (selectedPortfolioNames.length <= 2) {
      return selectedPortfolioNames.join(", ");
    }
    return `${selectedPortfolioNames.slice(0, 2).join(", ")} +${selectedPortfolioNames.length - 2}`;
  }, [portfolios.length, selectedPortfolioIds.length, selectedPortfolioNames]);
  const selectedAccountOption = useMemo(
    () =>
      accountOptions.find(
        (option) =>
          option.kind === selectedTradingAccount.kind
          && option.id === selectedTradingAccount.id,
      ) ?? null,
    [accountOptions, selectedTradingAccount],
  );
  const isManualAccountSelected = selectedTradingAccount.kind === "manual";
  const isLiveAccountSelected = selectedTradingAccount.kind === "live";
  const selectedPaperSummary = paperSummaryQuery.data ?? null;
  const selectedPaperAccount = selectedPaperSummary?.account ?? null;
  const selectedPaperPositions = selectedPaperSummary?.positions ?? [];
  const paperEquityEstimate = selectedPaperSummary?.equity_estimate ?? 0;
  const paperStartingCash = selectedPaperAccount?.starting_cash ?? 0;
  const paperTotalReturn = paperEquityEstimate - paperStartingCash;
  const paperTotalReturnPercent = paperStartingCash > 0
    ? (paperTotalReturn / paperStartingCash) * 100
    : 0;
  const selectedPaperMetrics = selectedPaperSummary
    ? getPaperSummaryMetrics(selectedPaperSummary)
    : null;
  const chat = useDeskChat({
    page: "home",
    projectCount: projects.length,
    portfolioCount: portfolios.length,
    portfolioScopeLabel: portfolioSelectionLabel,
    nav: allPortfolioNav,
    gainPercentage: allPortfolioGainPercentage,
    totalPositions: positions.length,
    openPositions: totalOpenPositions,
    selectedProjectName: selectedProject?.name ?? null,
    topPositions: positions.slice(0, 5).map((position) => ({
      symbol: position.symbol,
      portfolioName: position.portfolioName,
      quantity: position.quantity,
      averagePrice: position.average_price,
      status: position.position_closed_at ? "closed" : "open",
    })),
  });

  useEffect(() => {
    selectedProjectIdRef.current = selectedProjectId;
  }, [selectedProjectId]);

  useEffect(() => {
    selectedPortfolioIdsRef.current = selectedPortfolioIds;
  }, [selectedPortfolioIds]);

  useEffect(() => {
    selectedPortfolioIdRef.current = selectedPortfolioId;
  }, [selectedPortfolioId]);

  useEffect(() => {
    if (!accountOptions.length) {
      return;
    }

    const currentExists = accountOptions.some(
      (option) =>
        option.kind === selectedTradingAccount.kind
        && option.id === selectedTradingAccount.id,
    );
    if (currentExists) {
      return;
    }

    const fallbackOption = accountOptions.find((option) => option.kind === "manual")
      ?? accountOptions.find((option) => option.kind === "paper")
      ?? accountOptions[0];
    setSelectedPortfolioAccount(
      fallbackOption
        ? { kind: fallbackOption.kind, id: fallbackOption.id }
        : { kind: "manual", id: MANUAL_ACCOUNT_ID },
    );
  }, [accountOptions, selectedTradingAccount, setSelectedPortfolioAccount]);

  useEffect(() => {
    const nextPaperAccountId =
      selectedTradingAccount.kind === "paper" ? selectedTradingAccount.id : null;
    if (uiState.selectedPaperAccountId !== nextPaperAccountId) {
      setSelectedPaperAccountId(nextPaperAccountId);
    }
  }, [
    selectedTradingAccount,
    setSelectedPaperAccountId,
    uiState.selectedPaperAccountId,
  ]);

  async function syncData(options?: { showLoading?: boolean; showSummary?: boolean }) {
    if (syncInFlightRef.current) {
      return;
    }

    syncInFlightRef.current = true;
    if (options?.showLoading) {
      setLoading(true);
    }

    try {
      const [nextProjects, nextPortfolios] = await Promise.all([
        deskApi.listProjects(),
        deskApi.listPortfolios(),
      ]);

      const nextSelectedProjectId =
        nextProjects.some((project) => project.id === selectedProjectIdRef.current)
          ? selectedProjectIdRef.current
          : nextProjects[0]?.id ?? "";
      const availablePortfolioIds = nextPortfolios.map((portfolio) => portfolio.id);
      const filteredPortfolioIds = selectedPortfolioIdsRef.current.filter(
        (portfolioId) => availablePortfolioIds.includes(portfolioId),
      );
      const nextSelectedPortfolioIds =
        selectedPortfolioIdsRef.current.length === 0 &&
        !portfolioSelectionInitializedRef.current
          ? availablePortfolioIds
          : filteredPortfolioIds;
      const nextSelectedPortfolioId =
        nextSelectedPortfolioIds.includes(selectedPortfolioIdRef.current)
          ? selectedPortfolioIdRef.current
          : nextSelectedPortfolioIds[0] ?? "";

      const nextPositionGroups = await Promise.all(
        nextSelectedPortfolioIds.map(async (portfolioId) => {
          const portfolio = nextPortfolios.find((item) => item.id === portfolioId);
          const portfolioPositions = await deskApi.listPositions(portfolioId);

          return portfolioPositions.map((position) => ({
            ...position,
            portfolioId,
            portfolioName: portfolio?.name ?? portfolioId,
          }));
        }),
      );
      const nextPositions = nextPositionGroups.flat();

      const [nextSelectedProject, nextSelectedPortfolio] = await Promise.all([
        nextSelectedProjectId
          ? deskApi.getProject(nextSelectedProjectId)
          : Promise.resolve(null),
        nextSelectedPortfolioId
          ? deskApi.getPortfolio(nextSelectedPortfolioId)
          : Promise.resolve(null),
      ]);

      let changed = false;

      const nextProjectsSignature = serialize(nextProjects);
      if (stateSnapshotRef.current.projects !== nextProjectsSignature) {
        stateSnapshotRef.current.projects = nextProjectsSignature;
        setProjects(nextProjects);
        changed = true;
      }

      const nextPortfoliosSignature = serialize(nextPortfolios);
      if (stateSnapshotRef.current.portfolios !== nextPortfoliosSignature) {
        stateSnapshotRef.current.portfolios = nextPortfoliosSignature;
        setPortfolios(nextPortfolios);
        changed = true;
      }

      const nextPositionsSignature = serialize(nextPositions);
      if (stateSnapshotRef.current.positions !== nextPositionsSignature) {
        stateSnapshotRef.current.positions = nextPositionsSignature;
        setPositions(nextPositions);
        changed = true;
      }

      const nextSelectedProjectSignature = serialize(nextSelectedProject);
      if (stateSnapshotRef.current.selectedProject !== nextSelectedProjectSignature) {
        stateSnapshotRef.current.selectedProject = nextSelectedProjectSignature;
        setSelectedProject(nextSelectedProject);
        changed = true;
      }

      const nextSelectedPortfolioSignature = serialize(nextSelectedPortfolio);
      if (
        stateSnapshotRef.current.selectedPortfolio !== nextSelectedPortfolioSignature
      ) {
        stateSnapshotRef.current.selectedPortfolio = nextSelectedPortfolioSignature;
        setSelectedPortfolio(nextSelectedPortfolio);
        changed = true;
      }

      if (selectedProjectIdRef.current !== nextSelectedProjectId) {
        selectedProjectIdRef.current = nextSelectedProjectId;
        setSelectedProjectId(nextSelectedProjectId);
        changed = true;
      }

      const nextSelectedPortfolioIdsSignature = serialize(nextSelectedPortfolioIds);
      const currentSelectedPortfolioIdsSignature = serialize(
        selectedPortfolioIdsRef.current,
      );
      if (currentSelectedPortfolioIdsSignature !== nextSelectedPortfolioIdsSignature) {
        selectedPortfolioIdsRef.current = nextSelectedPortfolioIds;
        portfolioSelectionInitializedRef.current = true;
        setSelectedPortfolioIds(nextSelectedPortfolioIds);
        changed = true;
      }

      if (selectedPortfolioIdRef.current !== nextSelectedPortfolioId) {
        selectedPortfolioIdRef.current = nextSelectedPortfolioId;
        setSelectedPortfolioId(nextSelectedPortfolioId);
        changed = true;
      }

      if (changed && options?.showSummary) {
        setSummaryMessage(
          `Dashboard synced with the Poem OpenAPI server (${nextProjects.length} projects, ${nextPortfolios.length} portfolios).`,
        );
      }
    } catch (error) {
      const message =
        error instanceof Error ? error.message : "Failed to load Desk API data.";
      setErrorMessage(message);
    } finally {
      if (options?.showLoading) {
        setLoading(false);
      }
      syncInFlightRef.current = false;
    }
  }

  useEffect(() => {
    deskApi
      .getHello("trader")
      .then(setHelloMessage)
      .catch(() => setHelloMessage("Desk API is reachable, but hello failed."));

    void syncData({ showLoading: true });

    const intervalId = window.setInterval(() => {
      void syncData();
    }, 1000);

    return () => window.clearInterval(intervalId);
  }, []);

  useEffect(() => {
    void syncData();
  }, [selectedProjectId, selectedPortfolioId, selectedPortfolioIds]);

  useEffect(() => {
    if (searchParams.get("createProject") !== "1") {
      const queryProjectId = searchParams.get("projectId");
      if (!queryProjectId) {
        return;
      }

      if (queryProjectId !== selectedProjectIdRef.current) {
        selectedProjectIdRef.current = queryProjectId;
        setSelectedProjectId(queryProjectId);
      }
      return;
    }

    setProjectModalOpen(true);
    setSearchParams((currentParams) => {
      const nextParams = new URLSearchParams(currentParams);
      nextParams.delete("createProject");
      return nextParams;
    });
  }, [searchParams, setSearchParams]);

  useEffect(() => {
    let cancelled = false;

    async function refreshAllPortfolioNav() {
      if (!allPortfolioPositions.length) {
        setAllPortfolioNav(0);
        setAllPortfolioGainAmount(0);
        setAllPortfolioGainPercentage(0);
        setNavCoverage({ priced: 0, total: 0 });
        setNavLoading(false);
        return;
      }

      setNavLoading(true);

      try {
        const uniqueSymbols = [
          ...new Set(
            allOpenPortfolioPositions.map((position) => position.symbol),
          ),
        ];
        const symbolPrices = new Map<string, number>();

        await Promise.all(
          uniqueSymbols.map(async (symbol) => {
            try {
              const quote = await deskApi.getStockData({
                symbol,
                range: "1d",
                interval: "1m",
                prepost: false,
              });
              const latestClose = getLatestClose(quote);
              if (latestClose !== null) {
                symbolPrices.set(symbol, latestClose);
              }
            } catch {
              // Skip symbols that fail pricing so we can still price the rest.
            }
          }),
        );

        if (cancelled) {
          return;
        }

        let nextNav = 0;
        let totalCostBasis = 0;
        let totalCurrentValue = 0;
        let pricedPositions = 0;

        for (const position of allPortfolioPositions) {
          const costBasis = position.average_price * position.quantity;
          const currentPrice = position.position_closed_price
            ?? symbolPrices.get(position.symbol);
          if (currentPrice === undefined || currentPrice === null) {
            continue;
          }

          const currentValue = currentPrice * position.quantity;
          totalCostBasis += costBasis;
          totalCurrentValue += currentValue;

          if (!position.position_closed_at) {
            nextNav += currentValue;
          }

          pricedPositions += 1;
        }

        setAllPortfolioNav(nextNav);
        setAllPortfolioGainAmount(totalCurrentValue - totalCostBasis);
        setAllPortfolioGainPercentage(
          totalCostBasis > 0
            ? ((totalCurrentValue - totalCostBasis) / totalCostBasis) * 100
            : 0,
        );
        setNavCoverage({
          priced: pricedPositions,
          total: allPortfolioPositions.length,
        });
      } finally {
        if (!cancelled) {
          setNavLoading(false);
        }
      }
    }

    void refreshAllPortfolioNav();
    const intervalId = window.setInterval(() => {
      void refreshAllPortfolioNav();
    }, 15000);

    return () => {
      cancelled = true;
      window.clearInterval(intervalId);
    };
  }, [allOpenPortfolioPositions, allPortfolioPositions]);

  function handleQuoteLookup(symbolOverride?: string) {
    const symbol = (symbolOverride ?? quoteSymbol).trim().toUpperCase();
    if (!symbol) {
      setErrorMessage("Enter a symbol before requesting stock data.");
      return;
    }
    setQuoteSymbol(symbol);
    navigate(`/market/${encodeURIComponent(symbol)}`);
  }

  async function handleCreateProject(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setErrorMessage("");

    try {
      const payload = toProjectPayload(
        { ...projectForm, id: makeEntityId("project") },
      );
      await deskApi.createProject(payload);
      selectedProjectIdRef.current = payload.id;
      setSelectedProjectId(payload.id);
      await syncData();
      setProjectModalOpen(false);
      setProjectForm(emptyProjectForm);
      setSummaryMessage(`Created strategy ${payload.name}.`);
    } catch (error) {
      const message =
        error instanceof Error ? error.message : "Failed to create project.";
      setErrorMessage(message);
    }
  }

  async function handleCreatePortfolio(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setErrorMessage("");

    try {
      const payload = toPortfolioPayload(
        { ...portfolioForm, id: makeEntityId("portfolio") },
      );
      await deskApi.createPortfolio(payload);
      selectedPortfolioIdRef.current = payload.id;
      setSelectedPortfolioId(payload.id);
      await syncData();
      setPortfolioModalOpen(false);
      setPortfolioForm(emptyPortfolioForm);
      setSummaryMessage(`Created portfolio ${payload.name}.`);
    } catch (error) {
      const message =
        error instanceof Error ? error.message : "Failed to create portfolio.";
      setErrorMessage(message);
    }
  }

  async function handleDeleteProject() {
    if (!selectedProject) {
      setErrorMessage("Select a project before deleting it.");
      return;
    }

    setErrorMessage("");

    try {
      const deletedName = selectedProject.name;
      await deskApi.deleteProject(selectedProject.id);
      await syncData();
      setSummaryMessage(`Deleted strategy ${deletedName}.`);
    } catch (error) {
      const message =
        error instanceof Error ? error.message : "Failed to delete project.";
      setErrorMessage(message);
    }
  }

  async function handleDeletePortfolio() {
    if (!selectedPortfolio) {
      setErrorMessage("Select a portfolio before deleting it.");
      return;
    }

    setErrorMessage("");

    try {
      const deletedName = selectedPortfolio.name;
      await deskApi.deletePortfolio(selectedPortfolio.id);
      await syncData();
      setSummaryMessage(`Deleted portfolio ${deletedName}.`);
    } catch (error) {
      const message =
        error instanceof Error ? error.message : "Failed to delete portfolio.";
      setErrorMessage(message);
    }
  }

  async function handleCreatePosition(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!selectedPortfolioId) {
      setErrorMessage("Choose a target portfolio before adding positions.");
      return;
    }

    setErrorMessage("");

    try {
      const payload = toPositionPayload(positionForm);
      await deskApi.createPosition(selectedPortfolioId, payload);
      await syncData();
      setPositionForm(emptyPositionForm);
      setSummaryMessage(`Created position ${payload.symbol}.`);
    } catch (error) {
      const message =
        error instanceof Error ? error.message : "Failed to create position.";
      setErrorMessage(message);
    }
  }

  async function handleDeletePosition(position: DisplayPosition) {
    setErrorMessage("");

    try {
      await deskApi.deletePosition(
        position.portfolioId,
        position.symbol,
        position.position_opened_at,
      );
      await syncData();
      setSummaryMessage(`Deleted position ${position.symbol} from ${position.portfolioName}.`);
    } catch (error) {
      const message =
        error instanceof Error ? error.message : "Failed to delete position.";
      setErrorMessage(message);
    }
  }

  function handleOpenClosePosition(position: DisplayPosition) {
    setPositionToClose(position);
    setClosePositionForm({
      positionClosedAt: makeTimestamp(),
      positionClosedPrice:
        position.position_closed_price !== null
          ? String(position.position_closed_price)
          : "",
    });
    setClosePositionModalOpen(true);
  }

  async function handleSubmitClosePosition(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!positionToClose) {
      setErrorMessage("Choose a position before closing it.");
      return;
    }

    setErrorMessage("");

    try {
      await deskApi.updatePosition(positionToClose.portfolioId, positionToClose.symbol, positionToClose.position_opened_at, {
        symbol: positionToClose.symbol,
        quantity: positionToClose.quantity,
        average_price: positionToClose.average_price,
        position_opened_at: positionToClose.position_opened_at,
        position_closed_at: closePositionForm.positionClosedAt.trim() || makeTimestamp(),
        position_closed_price: closePositionForm.positionClosedPrice.trim()
          ? Number(closePositionForm.positionClosedPrice)
          : null,
      });
      await syncData();
      setClosePositionModalOpen(false);
      setPositionToClose(null);
      setClosePositionForm(emptyClosePositionForm);
      setSummaryMessage(`Closed position ${positionToClose.symbol} in ${positionToClose.portfolioName}.`);
    } catch (error) {
      const message =
        error instanceof Error ? error.message : "Failed to close position.";
      setErrorMessage(message);
    }
  }

  return (
    <main className="app-page min-h-screen pt-16 pb-6">
      <Topbar
        onToggleSidebar={() => setSidebarOpen((open) => !open)}
        onToggleChat={() => setChatOpen((open) => !open)}
        sidebarOpen={sidebarOpen}
        chatOpen={chatOpen}
        quoteSymbol={quoteSymbol}
        onQuoteSymbolChange={setQuoteSymbol}
        onQuoteLookup={() => handleQuoteLookup()}
        quoteLoading={false}
      />
      <LeftSidebar
        open={sidebarOpen}
      />

      <section
        className={`space-y-6 p-6 transition-all duration-200 ${
          sidebarOpen ? "md:ml-64" : "md:ml-0"
        } ${chatOpen ? "lg:mr-96 xl:mr-[25vw]" : "lg:mr-0"}`}
      >
        <section className="grid gap-4">
          <article
            id="market"
            className="app-surface rounded-2xl p-6 shadow-sm"
          >
            <div className="flex flex-wrap items-start justify-between gap-4">
              <div>
                <p className="app-text-muted text-sm uppercase tracking-[0.25em]">
                  Manual Ops
                </p>
                <h2 className="mt-2 text-3xl font-semibold">
                  Portfolio control center
                </h2>
                <p className="app-text-muted mt-3 max-w-2xl text-sm">
                  {helloMessage} This screen is now calling the Poem OpenAPI server
                  for project, portfolio, position, and quote workflows.
                </p>
                <div className="mt-4 flex max-w-xl flex-col gap-4">
                  <label className="block">
                    <span className="mb-2 block text-sm font-medium">
                      Account
                    </span>
                    <select
                      value={`${selectedTradingAccount.kind}:${selectedTradingAccount.id}`}
                      onChange={(event) => {
                        const [kind, id] = event.target.value.split(":", 2) as [
                          TradingAccountKind,
                          string,
                        ];
                        setSelectedPortfolioAccount({ kind, id });
                        setSelectedPaperAccountId(kind === "paper" ? id : null);
                      }}
                      className="app-input w-full rounded-2xl px-4 py-3 text-sm transition"
                    >
                      <optgroup label="Manual Portfolio">
                        <option value={`manual:${MANUAL_ACCOUNT_ID}`}>
                          Manual Portfolio
                        </option>
                      </optgroup>
                      <optgroup label="Paper Accounts">
                        {accountOptions
                          .filter((option) => option.kind === "paper")
                          .map((option) => (
                            <option
                              key={`${option.kind}:${option.id}`}
                              value={`${option.kind}:${option.id}`}
                            >
                              {option.label}
                              {!option.is_active ? " (inactive)" : ""}
                            </option>
                          ))}
                        {!accountOptions.some((option) => option.kind === "paper") && (
                          <option value="" disabled>
                            {paperAccountsQuery.isLoading
                              ? "Loading paper accounts..."
                              : "No paper accounts available"}
                          </option>
                        )}
                      </optgroup>
                      <optgroup label="Live Accounts">
                        {accountOptions
                          .filter((option) => option.kind === "live")
                          .map((option) => (
                            <option
                              key={`${option.kind}:${option.id}`}
                              value={`${option.kind}:${option.id}`}
                            >
                              {option.label}
                              {!option.is_active ? " (inactive)" : ""}
                            </option>
                          ))}
                        {!accountOptions.some((option) => option.kind === "live") && (
                          <option value="" disabled>
                            {liveAccountsQuery.isLoading
                              ? "Loading live accounts..."
                              : "No live accounts connected"}
                          </option>
                        )}
                      </optgroup>
                    </select>
                  </label>

                  {selectedAccountOption && (
                    <div className="app-surface-muted rounded-2xl px-4 py-3 text-sm">
                      <p className="app-text-muted text-xs uppercase tracking-[0.18em]">
                        Active Account
                      </p>
                      <p className="mt-2 font-medium">
                        {selectedAccountOption.label}
                      </p>
                    </div>
                  )}

                  {isManualAccountSelected && (
                    <details className="app-surface-muted w-full rounded-2xl px-4 py-3">
                      <summary className="flex cursor-pointer list-none items-center justify-between gap-3">
                        <div>
                          <p className="app-text-muted text-xs uppercase tracking-[0.18em]">
                            Portfolio Scope
                          </p>
                          <p className="mt-1 text-sm font-medium">
                            {portfolioSelectionLabel}
                          </p>
                        </div>
                        <span className="app-text-muted text-sm">
                          {selectedPortfolioIds.length}/{portfolios.length || 0}
                        </span>
                      </summary>
                      <div className="mt-4 space-y-3">
                        <div className="flex flex-wrap gap-2">
                          <button
                            type="button"
                            onClick={() => {
                              portfolioSelectionInitializedRef.current = true;
                              setSelectedPortfolioIds(
                                portfolios.map((portfolio) => portfolio.id),
                              );
                            }}
                            className="app-button-secondary rounded-full px-3 py-1.5 text-xs font-medium transition"
                          >
                            Select all
                          </button>
                          <button
                            type="button"
                            onClick={() => {
                              portfolioSelectionInitializedRef.current = true;
                              setSelectedPortfolioIds([]);
                            }}
                            className="app-button-secondary rounded-full px-3 py-1.5 text-xs font-medium transition"
                          >
                            Clear
                          </button>
                        </div>
                        <div className="max-h-52 space-y-2 overflow-auto pr-1">
                          {portfolios.map((portfolio) => {
                            const checked = selectedPortfolioIds.includes(portfolio.id);
                            return (
                              <label
                                key={portfolio.id}
                                className="app-surface flex items-center gap-3 rounded-xl px-3 py-2 text-sm"
                              >
                                <input
                                  type="checkbox"
                                  checked={checked}
                                  onChange={(event) => {
                                    portfolioSelectionInitializedRef.current = true;
                                    setSelectedPortfolioIds((current) =>
                                      event.target.checked
                                        ? [...current, portfolio.id]
                                        : current.filter((id) => id !== portfolio.id),
                                    );
                                  }}
                                />
                                <span>{portfolio.name}</span>
                              </label>
                            );
                          })}
                          {!portfolios.length && (
                            <p className="app-text-muted text-sm">
                              Create a portfolio to start filtering manual ops.
                            </p>
                          )}
                        </div>
                      </div>
                    </details>
                  )}
                </div>
              </div>
              <div className="flex flex-wrap gap-3">
                <button
                  type="button"
                  onClick={() => setProjectModalOpen(true)}
                  className="app-button-primary rounded-full px-4 py-2 text-sm font-medium transition"
                >
                  Create strategy
                </button>
                <button
                  type="button"
                  onClick={() => setPortfolioModalOpen(true)}
                  className="app-button-secondary rounded-full px-4 py-2 text-sm font-medium transition"
                >
                  Create portfolio
                </button>
                <button
                  type="button"
                  className="app-pill-success rounded-full px-4 py-2 text-sm font-medium"
                >
                  {loading ? "Starting sync..." : "Auto-sync every second"}
                </button>
              </div>
            </div>

            <div className="mt-6 grid gap-4 md:grid-cols-2 xl:grid-cols-5">
              <MetricCard
                label="Strategies"
                value={String(projects.length)}
                detail="Tracked strategy workspaces"
              />
              <MetricCard
                label="Portfolios"
                value={String(portfolios.length)}
                detail="Manual books available"
              />
              <MetricCard
                label="Open Positions"
                value={String(totalOpenPositions)}
                detail={`${positions.length} loaded from selected portfolio`}
              />
              <MetricCard
                label="All Portfolio NAV"
                value={formatCurrency(allPortfolioNav)}
                detail={
                  navLoading
                    ? "Refreshing live prices across all portfolios..."
                    : navCoverage.total
                      ? `Priced ${navCoverage.priced} of ${navCoverage.total} total positions from live and closed values`
                      : "No open positions across portfolios"
                }
              />
              <MetricCard
                label="All Portfolio Gain"
                value={
                  <SignedValue value={allPortfolioGainAmount}>
                    {formatSignedCurrency(allPortfolioGainAmount)}
                  </SignedValue>
                }
                detail={
                  navCoverage.total ? (
                    <SignedValue value={allPortfolioGainPercentage}>
                      {formatPercent(allPortfolioGainPercentage)} versus cost basis
                    </SignedValue>
                  ) : (
                    "No positions available for gain calculation"
                  )
                }
              />
            </div>

            {(summaryMessage || errorMessage) && (
              <div
                className={`mt-4 rounded-xl px-4 py-3 text-sm ${
                  errorMessage ? "app-alert-error" : "app-alert-success"
                }`}
              >
                {errorMessage || summaryMessage}
              </div>
            )}

            {(paperAccountsQuery.error || liveAccountsQuery.error) && (
              <div className="mt-4">
                <ErrorInline
                  message={
                    paperAccountsQuery.error instanceof Error
                      ? paperAccountsQuery.error.message
                      : liveAccountsQuery.error instanceof Error
                        ? liveAccountsQuery.error.message
                        : "Failed to refresh account options."
                  }
                />
              </div>
            )}
          </article>
        </section>

        {isManualAccountSelected && (
          <Panel
            id="positions"
            title="Positions"
            subtitle="Calling nested /portfolios/:portfolio_id/positions routes for list, detail, create, update, and delete."
          >
          <div className="grid gap-6 xl:grid-cols-[1.15fr_0.85fr]">
            <div className="space-y-6">
              <div className="app-surface-muted rounded-2xl p-4">
                <div className="flex items-start justify-between gap-4">
                  <div>
                    <h4 className="text-lg font-semibold">
                      Allocation
                    </h4>
                    <p className="app-text-muted mt-1 text-sm">
                      Open positions by cost basis across the selected portfolio scope.
                    </p>
                  </div>
                  <p className="app-text-muted text-sm font-medium">
                    {formatCurrency(positionAllocations.totalValue)}
                  </p>
                </div>

                <div className="mt-4 flex flex-col items-center gap-4 sm:flex-row sm:items-start">
                  <div
                    aria-label="Position allocation donut chart"
                    className="relative h-44 w-44 shrink-0 rounded-full"
                    style={{
                      background: `conic-gradient(${positionAllocations.background})`,
                    }}
                  >
                    <div className="app-surface absolute inset-[22%] flex items-center justify-center rounded-full">
                      <div className="text-center">
                        <p className="app-text-muted text-xs uppercase tracking-[0.18em]">
                          Open
                        </p>
                        <p className="mt-1 text-2xl font-semibold">
                          {totalOpenPositions}
                        </p>
                      </div>
                    </div>
                  </div>

                  <div className="w-full space-y-2">
                    {positionAllocations.slices.length ? (
                      positionAllocations.slices.map((slice) => (
                        <div
                          key={slice.key}
                          className="app-surface rounded-xl px-3 py-2 text-sm"
                        >
                          <div className="flex items-center gap-3">
                            <span
                              className="h-3 w-3 rounded-full"
                              style={{ backgroundColor: slice.color }}
                            />
                            <span className="font-medium">
                              {slice.symbol}
                            </span>
                          </div>
                          <div className="text-right">
                            <p className="font-medium">
                              {slice.percentage.toFixed(1)}%
                            </p>
                            <p className="app-text-muted text-xs">
                              {formatCurrency(slice.value)}
                            </p>
                          </div>
                        </div>
                      ))
                    ) : (
                      <div className="app-text-muted rounded-xl border border-dashed px-4 py-6 text-center text-sm" style={{ borderColor: "var(--color-border)" }}>
                        Add an open position to see the allocation donut.
                      </div>
                    )}
                  </div>
                </div>
              </div>

              <div className="app-surface overflow-hidden rounded-2xl">
                <table className="min-w-full text-sm">
                  <thead className="app-surface-muted">
                    <tr className="app-text-muted text-left">
                      <th className="px-4 py-3 font-medium">Portfolio</th>
                      <th className="px-4 py-3 font-medium">Symbol</th>
                      <th className="px-4 py-3 font-medium">Qty</th>
                      <th className="px-4 py-3 font-medium">Avg Price</th>
                      <th className="px-4 py-3 font-medium">Opened</th>
                      <th className="px-4 py-3 font-medium">Status</th>
                      <th className="px-4 py-3 font-medium text-right">Actions</th>
                    </tr>
                  </thead>
                  <tbody style={{ background: "var(--color-surface-strong)" }}>
                    {positions.map((position) => {
                      const key = displayPositionKey(position);

                      return (
                        <tr
                          key={key}
                          className="transition"
                        >
                          <td className="px-4 py-3">{position.portfolioName}</td>
                          <td className="px-4 py-3 font-medium">
                            {position.symbol}
                          </td>
                          <td className="px-4 py-3">{position.quantity}</td>
                          <td className="px-4 py-3">{position.average_price}</td>
                          <td className="px-4 py-3">{formatTimestamp(position.position_opened_at)}</td>
                          <td className="px-4 py-3">
                            {position.position_closed_at ? "Closed" : "Open"}
                          </td>
                          <td className="px-4 py-3 text-right">
                            <details className="relative inline-block text-left">
                              <summary className="app-button-secondary inline-flex cursor-pointer list-none rounded-full px-3 py-1.5 text-xs font-medium transition">
                                Actions
                              </summary>
                              <div className="app-surface absolute right-0 top-10 z-10 flex min-w-36 flex-col rounded-xl p-2 shadow-lg">
                                {!position.position_closed_at && (
                                  <button
                                    type="button"
                                    onClick={() => handleOpenClosePosition(position)}
                                    className="app-nav-link rounded-lg px-3 py-2 text-left text-sm"
                                  >
                                    Close position
                                  </button>
                                )}
                                <button
                                  type="button"
                                  onClick={() => void handleDeletePosition(position)}
                                  className="app-button-danger rounded-lg px-3 py-2 text-left text-sm"
                                >
                                  Delete position
                                </button>
                              </div>
                            </details>
                          </td>
                        </tr>
                      );
                    })}
                    {!positions.length && (
                      <tr>
                        <td
                          colSpan={7}
                          className="app-text-muted px-4 py-8 text-center"
                        >
                          Select one or more portfolios to view positions.
                        </td>
                      </tr>
                    )}
                  </tbody>
                </table>
              </div>
            </div>

            <form className="space-y-3" onSubmit={handleCreatePosition}>
              <label className="block">
                <span className="mb-1 block text-sm font-medium">
                  Target Portfolio
                </span>
                <select
                  value={selectedPortfolioId}
                  onChange={(event) => setSelectedPortfolioId(event.target.value)}
                  className="app-input w-full rounded-xl px-3 py-2 text-sm transition"
                >
                  <option value="">Select a portfolio</option>
                  {portfolios.map((portfolio) => (
                    <option key={portfolio.id} value={portfolio.id}>
                      {portfolio.name} ({portfolio.id})
                    </option>
                  ))}
                </select>
              </label>
              <Input
                label="Symbol"
                value={positionForm.symbol}
                onChange={(value) =>
                  setPositionForm((current) => ({ ...current, symbol: value }))
                }
              />
              <div className="grid gap-3 sm:grid-cols-2">
                <Input
                  label="Quantity"
                  value={positionForm.quantity}
                  onChange={(value) =>
                    setPositionForm((current) => ({ ...current, quantity: value }))
                  }
                  placeholder="100"
                />
                <Input
                  label="Average Price"
                  value={positionForm.averagePrice}
                  onChange={(value) =>
                    setPositionForm((current) => ({
                      ...current,
                      averagePrice: value,
                    }))
                  }
                  placeholder="182.42"
                />
              </div>
              <Input
                label="Opened At"
                type="datetime-local"
                value={toDateTimeLocalValue(positionForm.positionOpenedAt)}
                onChange={(value) =>
                  setPositionForm((current) => ({
                    ...current,
                    positionOpenedAt: fromDateTimeLocalValue(value),
                  }))
                }
              />
              <div className="grid gap-3 sm:grid-cols-2">
                <Input
                  label="Closed At"
                  type="datetime-local"
                  value={toDateTimeLocalValue(positionForm.positionClosedAt)}
                  onChange={(value) =>
                    setPositionForm((current) => ({
                      ...current,
                      positionClosedAt: fromDateTimeLocalValue(value),
                    }))
                  }
                />
                <Input
                  label="Closed Price"
                  value={positionForm.positionClosedPrice}
                  onChange={(value) =>
                    setPositionForm((current) => ({
                      ...current,
                      positionClosedPrice: value,
                    }))
                  }
                  placeholder="Optional"
                />
              </div>
              <div className="flex flex-wrap gap-3 pt-2">
                <button
                  type="submit"
                  className="app-button-primary rounded-full px-4 py-2 text-sm font-medium transition"
                >
                  Create position
                </button>
                <button
                  type="button"
                  onClick={() => setPositionForm(emptyPositionForm)}
                  className="app-button-secondary rounded-full px-4 py-2 text-sm font-medium transition"
                >
                  Reset form
                </button>
              </div>
            </form>
          </div>
          </Panel>
        )}

        {selectedTradingAccount.kind === "paper" && (
          <Panel
            id="paper-account"
            title="Paper Account Portfolio"
            subtitle="Display-only paper trading summary backed by /paper/accounts endpoints."
          >
            <div className="space-y-6">
              {paperSummaryQuery.isLoading && (
                <LoadingInline message="Loading paper account summary..." />
              )}

              {paperSummaryQuery.error && (
                <ErrorInline
                  message={
                    paperSummaryQuery.error instanceof Error
                      ? paperSummaryQuery.error.message
                      : "Failed to load paper account summary."
                  }
                />
              )}

              {!paperSummaryQuery.isLoading && !paperSummaryQuery.error && !selectedPaperSummary && (
                <EmptyState message="Select a paper account to view its portfolio summary." />
              )}

              {selectedPaperSummary && selectedPaperAccount && (
                <>
                  <div className="flex justify-end">
                    <RefreshBadge
                      refreshing={paperSummaryQuery.isFetching}
                      updatedLabel="Paper summary synced"
                    />
                  </div>
                  <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-6">
                    <MetricCard
                      label="Account Name"
                      value={selectedPaperAccount.name}
                      detail={selectedPaperAccount.is_active ? "Active paper account" : "Inactive paper account"}
                    />
                    <MetricCard
                      label="Cash Balance"
                      value={formatCurrency(selectedPaperAccount.cash_balance)}
                      detail="Available paper cash balance"
                    />
                    <MetricCard
                      label="Estimated Equity"
                      value={formatCurrency(paperEquityEstimate)}
                      detail="Cash plus estimated open position value"
                    />
                    <MetricCard
                      label="Starting Cash"
                      value={formatCurrency(paperStartingCash)}
                      detail="Initial paper funding level"
                    />
                    <MetricCard
                      label="Total Return"
                      value={
                        <SignedValue value={paperTotalReturn}>
                          {formatSignedCurrency(paperTotalReturn)}
                        </SignedValue>
                      }
                      detail={
                        <SignedValue value={paperTotalReturnPercent}>
                          {formatPercent(paperTotalReturnPercent)}
                        </SignedValue>
                      }
                    />
                    <MetricCard
                      label="Unrealized Gain/Loss"
                      value={
                        <SignedValue value={selectedPaperMetrics?.unrealizedGain ?? 0}>
                          {formatSignedCurrency(
                            selectedPaperMetrics?.unrealizedGain ?? 0,
                          )}
                        </SignedValue>
                      }
                      detail={
                        <SignedValue value={selectedPaperMetrics?.unrealizedGainPercent ?? 0}>
                          {formatPercent(
                            selectedPaperMetrics?.unrealizedGainPercent ?? 0,
                          )}
                        </SignedValue>
                      }
                    />
                  </div>

                  <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-5">
                    <DetailRow label="Position Count" value={String(selectedPaperPositions.length)} />
                    <DetailRow label="Open Orders" value={String(selectedPaperSummary.open_orders.length)} />
                    <DetailRow label="Recent Fills" value={String(selectedPaperSummary.recent_fills.length)} />
                    <DetailRow
                      label="Realized P/L"
                      value={
                        <SignedValue
                          value={selectedPaperPositions.reduce(
                            (sum, position) => sum + position.realized_pnl,
                            0,
                          )}
                        >
                          {formatSignedCurrency(
                            selectedPaperPositions.reduce(
                              (sum, position) => sum + position.realized_pnl,
                              0,
                            ),
                          )}
                        </SignedValue>
                      }
                    />
                    <DetailRow
                      label="Market Value"
                      value={formatCurrency(selectedPaperMetrics?.marketValue ?? 0)}
                    />
                  </div>

                  <DataTable
                    title="Positions"
                    emptyMessage="No open positions in this paper account."
                    columns={[
                      "Symbol",
                      "Quantity",
                      "Average Price",
                      "Current Price",
                      "Market Value",
                      "Unrealized $",
                      "Unrealized %",
                      "Realized P/L",
                    ]}
                    rows={selectedPaperPositions.map((position) => {
                      const metrics = getPaperPositionMetrics(position);
                      return [
                        position.symbol,
                        position.quantity.toFixed(4),
                        formatCurrency(position.average_price),
                        position.current_price == null
                          ? `${formatCurrency(metrics.currentPrice)} (est.)`
                          : formatCurrency(metrics.currentPrice),
                        formatCurrency(metrics.marketValue),
                        <SignedValue value={metrics.unrealizedGain}>
                          {formatSignedCurrency(metrics.unrealizedGain)}
                        </SignedValue>,
                        <SignedValue value={metrics.unrealizedGainPercent}>
                          {formatPercent(metrics.unrealizedGainPercent)}
                        </SignedValue>,
                        <SignedValue value={position.realized_pnl}>
                          {formatSignedCurrency(position.realized_pnl)}
                        </SignedValue>,
                      ];
                    })}
                  />

                  <DataTable
                    title="Recent Fills"
                    emptyMessage="No recent fills yet."
                    columns={["Time", "Symbol", "Side", "Quantity", "Price"]}
                    rows={selectedPaperSummary.recent_fills.map((fill) => [
                      formatTimestamp(fill.created_at),
                      fill.symbol,
                      fill.side.toUpperCase(),
                      fill.quantity.toFixed(4),
                      formatCurrency(fill.price),
                    ])}
                  />
                </>
              )}
            </div>
          </Panel>
        )}

        {isLiveAccountSelected && (
          <Panel
            id="live-account"
            title="Live Account Portfolio"
            subtitle="Reserved for future display-only live account metadata."
          >
            <div className="space-y-4">
              <div className="app-surface-muted rounded-2xl px-4 py-4">
                <p className="text-sm font-medium">
                  Live account portfolio display is not connected yet.
                </p>
                <p className="app-text-muted mt-2 text-sm">
                  No real trading, broker credentials, or live broker integrations are enabled here.
                </p>
              </div>

              {selectedAccountOption && (
                <div className="grid gap-4 md:grid-cols-2">
                  <DetailRow label="Account Name" value={selectedAccountOption.name} />
                  <DetailRow
                    label="Status"
                    value={selectedAccountOption.is_active ? "Active" : "Inactive"}
                  />
                </div>
              )}

              {liveAccountsQuery.error && (
                <ErrorInline
                  message={
                    liveAccountsQuery.error instanceof Error
                      ? liveAccountsQuery.error.message
                      : "Failed to refresh live account metadata."
                  }
                />
              )}
            </div>
          </Panel>
        )}
      </section>

      <Modal
        open={projectModalOpen}
        title="Create strategy"
        description="Add a strategy workspace without keeping a full project card on the homepage."
        onClose={() => {
          setProjectModalOpen(false);
          setProjectForm(emptyProjectForm);
        }}
      >
        <form className="space-y-3" onSubmit={handleCreateProject}>
          <Input
            label="Name"
            value={projectForm.name}
            onChange={(value) =>
              setProjectForm((current) => ({ ...current, name: value }))
            }
          />
          <TextArea
            label="Description"
            value={projectForm.description}
            onChange={(value) =>
              setProjectForm((current) => ({ ...current, description: value }))
            }
          />
          <Input
            label="Symbols"
            value={projectForm.symbols}
            onChange={(value) =>
              setProjectForm((current) => ({ ...current, symbols: value }))
            }
            placeholder="AAPL, MSFT, NVDA"
          />
          <div className="grid gap-3 sm:grid-cols-2">
            <Input
              label="Interval"
              value={projectForm.interval}
              onChange={(value) =>
                setProjectForm((current) => ({ ...current, interval: value }))
              }
            />
            <Input
              label="Range"
              value={projectForm.range}
              onChange={(value) =>
                setProjectForm((current) => ({ ...current, range: value }))
              }
            />
          </div>
          <label className="app-surface rounded-xl px-3 py-2 text-sm">
            <input
              type="checkbox"
              checked={projectForm.prepost}
              onChange={(event) =>
                setProjectForm((current) => ({
                  ...current,
                  prepost: event.target.checked,
                }))
              }
            />
            Include pre/post market data
          </label>
          <ModalActions
            submitLabel="Create strategy"
            onCancel={() => {
              setProjectModalOpen(false);
              setProjectForm(emptyProjectForm);
            }}
          />
        </form>
      </Modal>

      <Modal
        open={portfolioModalOpen}
        title="Create portfolio"
        description="Spin up a new manual book from a focused modal instead of an inline dashboard card."
        onClose={() => {
          setPortfolioModalOpen(false);
          setPortfolioForm(emptyPortfolioForm);
        }}
      >
        <form className="space-y-3" onSubmit={handleCreatePortfolio}>
          <Input
            label="Name"
            value={portfolioForm.name}
            onChange={(value) =>
              setPortfolioForm((current) => ({ ...current, name: value }))
            }
          />
          <TextArea
            label="Description"
            value={portfolioForm.description}
            onChange={(value) =>
              setPortfolioForm((current) => ({ ...current, description: value }))
            }
          />
          <ModalActions
            submitLabel="Create portfolio"
            onCancel={() => {
              setPortfolioModalOpen(false);
              setPortfolioForm(emptyPortfolioForm);
            }}
          />
        </form>
      </Modal>

      <Modal
        open={closePositionModalOpen}
        title="Close position"
        description={
          positionToClose
            ? `Finalize ${positionToClose.symbol} in ${positionToClose.portfolioName}.`
            : "Finalize the selected open position."
        }
        onClose={() => {
          setClosePositionModalOpen(false);
          setPositionToClose(null);
          setClosePositionForm(emptyClosePositionForm);
        }}
      >
        <form className="space-y-3" onSubmit={handleSubmitClosePosition}>
          <Input
            label="Closed At"
            type="datetime-local"
            value={toDateTimeLocalValue(closePositionForm.positionClosedAt)}
            onChange={(value) =>
              setClosePositionForm((current) => ({
                ...current,
                positionClosedAt: fromDateTimeLocalValue(value),
              }))
            }
          />
          <Input
            label="Closed Price"
            value={closePositionForm.positionClosedPrice}
            onChange={(value) =>
              setClosePositionForm((current) => ({
                ...current,
                positionClosedPrice: value,
              }))
            }
            placeholder="Optional"
          />
          <ModalActions
            submitLabel="Close position"
            onCancel={() => {
              setClosePositionModalOpen(false);
              setPositionToClose(null);
              setClosePositionForm(emptyClosePositionForm);
            }}
          />
        </form>
      </Modal>

      <ChatPanel
        open={chatOpen}
        title="Portfolio assistant"
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

function MetricCard(props: {
  label: string;
  value: React.ReactNode;
  detail: React.ReactNode;
}) {
  return (
    <div className="app-surface-muted rounded-2xl p-4">
      <p className="app-text-muted text-sm">{props.label}</p>
      <p className="mt-3 text-3xl font-semibold">
        {props.value}
      </p>
      <p className="app-text-muted mt-2 text-sm">{props.detail}</p>
    </div>
  );
}

function DetailRow(props: { label: string; value: React.ReactNode }) {
  return (
    <div className="app-surface rounded-xl px-4 py-3">
      <dt className="app-text-muted text-xs uppercase tracking-[0.16em]">
        {props.label}
      </dt>
      <dd className="mt-2 text-sm font-medium">
        {props.value}
      </dd>
    </div>
  );
}

function DataTable(props: {
  title: string;
  columns: string[];
  rows: React.ReactNode[][];
  emptyMessage: string;
}) {
  return (
    <div className="app-surface overflow-hidden rounded-2xl">
      <div className="app-surface-muted flex items-center justify-between gap-3 px-4 py-3">
        <p className="text-sm font-medium">
          {props.title}
        </p>
        <p className="app-text-muted text-xs uppercase tracking-[0.18em]">
          {props.rows.length} rows
        </p>
      </div>

      {!props.rows.length ? (
        <div className="app-text-muted px-4 py-8 text-sm">
          {props.emptyMessage}
        </div>
      ) : (
        <div className="overflow-x-auto">
          <table className="min-w-full text-sm">
            <thead className="app-surface-muted">
              <tr className="app-text-muted text-left">
                {props.columns.map((column) => (
                  <th key={column} className="px-4 py-3 font-medium">
                    {column}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody style={{ background: "var(--color-surface-strong)" }}>
              {props.rows.map((row, rowIndex) => (
                <tr key={`${props.title}-${rowIndex}`}>
                  {row.map((cell, cellIndex) => (
                    <td key={`${props.title}-${rowIndex}-${cellIndex}`} className="px-4 py-3">
                      {cell}
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

function Panel(props: {
  id?: string;
  title: string;
  subtitle: string;
  children: React.ReactNode;
}) {
  return (
    <article
      id={props.id}
      className="app-surface rounded-2xl p-6 shadow-sm"
    >
      <div className="mb-5">
        <h3 className="text-xl font-semibold">
          {props.title}
        </h3>
        <p className="app-text-muted mt-1 text-sm">
          {props.subtitle}
        </p>
      </div>
      {props.children}
    </article>
  );
}

function Modal(props: {
  open: boolean;
  title: string;
  description: string;
  onClose: () => void;
  children: React.ReactNode;
}) {
  if (!props.open) {
    return null;
  }

  return (
    <div className="app-modal-backdrop fixed inset-0 z-40 flex items-center justify-center p-4">
      <div className="app-surface w-full max-w-2xl rounded-3xl p-6 shadow-2xl">
        <div className="mb-5 flex items-start justify-between gap-4">
          <div>
            <h3 className="text-xl font-semibold">
              {props.title}
            </h3>
            <p className="app-text-muted mt-1 text-sm">
              {props.description}
            </p>
          </div>
          <button
            type="button"
            onClick={props.onClose}
            className="app-button-secondary rounded-full px-3 py-1 text-sm transition"
          >
            Close
          </button>
        </div>
        {props.children}
      </div>
    </div>
  );
}

function ModalActions(props: { submitLabel: string; onCancel: () => void }) {
  return (
    <div className="flex flex-wrap gap-3 pt-2">
      <button
        type="submit"
        className="app-button-primary rounded-full px-4 py-2 text-sm font-medium transition"
      >
        {props.submitLabel}
      </button>
      <button
        type="button"
        onClick={props.onCancel}
        className="app-button-secondary rounded-full px-4 py-2 text-sm font-medium transition"
      >
        Cancel
      </button>
    </div>
  );
}

function Input(props: {
  label: string;
  value: string;
  onChange: (value: string) => void;
  placeholder?: string;
  type?: "text" | "datetime-local";
}) {
  return (
    <label className="block">
      <span className="mb-1 block text-sm font-medium">
        {props.label}
      </span>
      <input
        type={props.type ?? "text"}
        value={props.value}
        onChange={(event) => props.onChange(event.target.value)}
        placeholder={props.placeholder}
        className="app-input w-full rounded-xl px-3 py-2.5 text-sm transition"
      />
    </label>
  );
}

function TextArea(props: {
  label: string;
  value: string;
  onChange: (value: string) => void;
}) {
  return (
    <label className="block">
      <span className="mb-1 block text-sm font-medium">
        {props.label}
      </span>
      <textarea
        value={props.value}
        onChange={(event) => props.onChange(event.target.value)}
        rows={4}
        className="app-input w-full rounded-xl px-3 py-2.5 text-sm transition"
      />
    </label>
  );
}

function ActionRow(props: {
  onReset: () => void;
  onUpdate: () => void;
  onDelete: () => void;
  createLabel: string;
  canEdit: boolean;
}) {
  return (
    <div className="flex flex-wrap gap-3 pt-2">
      <button
        type="submit"
        className="app-button-primary rounded-full px-4 py-2 text-sm font-medium transition"
      >
        {props.createLabel}
      </button>
      <button
        type="button"
        disabled={!props.canEdit}
        onClick={props.onUpdate}
        className="app-button-secondary rounded-full px-4 py-2 text-sm font-medium transition disabled:cursor-not-allowed disabled:opacity-50"
      >
        Update selected
      </button>
      <button
        type="button"
        disabled={!props.canEdit}
        onClick={props.onDelete}
        className="app-button-danger rounded-full px-4 py-2 text-sm font-medium transition disabled:cursor-not-allowed disabled:opacity-50"
      >
        Delete selected
      </button>
      <button
        type="button"
        onClick={props.onReset}
        className="app-button-secondary rounded-full px-4 py-2 text-sm font-medium transition"
      >
        Reset form
      </button>
    </div>
  );
}
