export type Project = {
  id: string;
  name: string;
  description: string;
  strategy: string;
  created_at: string;
  updated_at: string;
  symbols: string[];
  interval: string;
  range: string;
  prepost: boolean;
};

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
};
