import { useEffect, useMemo, useRef, useState } from "react";
import {
  getStoredOpenAIApiKey,
  OPENAI_DEFAULT_MODEL,
  OPENAI_SETTINGS_CHANGE_EVENT,
} from "./openai";

export type ChatMessage = {
  id: string;
  role: "assistant" | "user";
  content: string;
  createdAt: string;
};

export type ChatContext =
  | {
      page: "home";
      projectCount: number;
      portfolioCount: number;
      portfolioScopeLabel: string;
      nav: number;
      gainPercentage: number;
      totalPositions: number;
      openPositions: number;
      selectedProjectName: string | null;
      topPositions: Array<{
        symbol: string;
        portfolioName: string;
        quantity: number;
        averagePrice: number;
        status: "open" | "closed";
      }>;
    }
  | {
      page: "market";
      symbol: string;
      interval: string;
      range: string;
      lastClose: number | null;
      barCount: number;
      lastRefreshed: string | null;
      dayHigh: number | null;
      dayLow: number | null;
    }
  | {
      page: "project";
      projectId: string;
      projectName: string | null;
      description: string | null;
      symbols: string[];
      interval: string | null;
      range: string | null;
      prepost: boolean | null;
    };

function createId() {
  return `msg-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 8)}`;
}

function createMessage(
  role: ChatMessage["role"],
  content: string,
  createdAt = new Date().toISOString(),
): ChatMessage {
  return {
    id: createId(),
    role,
    content,
    createdAt,
  };
}

function createWelcomeMessage(page: ChatContext["page"]) {
  return createMessage("assistant", getWelcomeMessage(page), "");
}

function getWelcomeMessage(page: ChatContext["page"]) {
  return page === "home"
    ? "Portfolio assistant is live. Ask about NAV, gain, portfolio scope, strategies, or positions on this dashboard."
    : page === "market"
      ? "Market assistant is live. Ask about the active symbol, range, interval, last close, or chart state on this screen."
      : "Strategy assistant is live. Describe the trading pattern you want and I will help turn it into a rules-based idea for this project.";
}

function formatCurrency(value: number) {
  return new Intl.NumberFormat("en-US", {
    style: "currency",
    currency: "USD",
    maximumFractionDigits: 2,
  }).format(value);
}

function formatSignedPercent(value: number) {
  return `${value >= 0 ? "+" : ""}${value.toFixed(2)}%`;
}

function buildHomeReply(input: string, context: Extract<ChatContext, { page: "home" }>) {
  const normalized = input.toLowerCase();

  if (/(hello|hi|hey)/.test(normalized)) {
    return `Watching ${context.portfolioScopeLabel.toLowerCase()} right now. NAV is ${formatCurrency(context.nav)} with aggregate gain ${formatSignedPercent(context.gainPercentage)}.`;
  }

  if (normalized.includes("nav") || normalized.includes("asset value")) {
    return `Scoped NAV is ${formatCurrency(context.nav)} across ${context.portfolioScopeLabel.toLowerCase()}.`;
  }

  if (normalized.includes("gain") || normalized.includes("performance") || normalized.includes("pnl")) {
    return `Scoped gain is ${formatSignedPercent(context.gainPercentage)} versus cost basis.`;
  }

  if (normalized.includes("scope") || normalized.includes("portfolio")) {
    return `Current portfolio scope: ${context.portfolioScopeLabel}. It covers ${context.portfolioCount} total portfolios in the workspace and ${context.totalPositions} scoped positions.`;
  }

  if (normalized.includes("project")) {
    return context.selectedProjectName
      ? `The selected strategy is ${context.selectedProjectName}. There are ${context.projectCount} tracked strategies overall.`
      : `There are ${context.projectCount} tracked strategies. No strategy is actively selected right now.`;
  }

  if (normalized.includes("position")) {
    if (!context.topPositions.length) {
      return `There are no positions in the current scope yet.`;
    }

    const preview = context.topPositions
      .slice(0, 3)
      .map(
        (position) =>
          `${position.symbol} in ${position.portfolioName} (${position.status}, ${position.quantity} @ ${position.averagePrice})`,
      )
      .join("; ");

    return `There are ${context.totalPositions} positions in scope, ${context.openPositions} open. Top rows: ${preview}.`;
  }

  return `I can summarize the current dashboard state, including scoped NAV (${formatCurrency(context.nav)}), gain (${formatSignedPercent(context.gainPercentage)}), strategies, portfolio scope, and positions. For true open-ended ChatGPT reasoning, the next step would be wiring this panel to a real chat backend endpoint.`;
}

function buildMarketReply(input: string, context: Extract<ChatContext, { page: "market" }>) {
  const normalized = input.toLowerCase();

  if (/(hello|hi|hey)/.test(normalized)) {
    return `Watching ${context.symbol} on the market screen. Last close is ${context.lastClose !== null ? formatCurrency(context.lastClose) : "unavailable"} with ${context.barCount} bars loaded.`;
  }

  if (normalized.includes("price") || normalized.includes("close")) {
    return context.lastClose !== null
      ? `${context.symbol} last close is ${formatCurrency(context.lastClose)}.`
      : `Last close for ${context.symbol} is not available from the current dataset.`;
  }

  if (normalized.includes("range") || normalized.includes("interval")) {
    return `The chart is currently showing ${context.symbol} on range ${context.range} and interval ${context.interval}.`;
  }

  if (normalized.includes("high") || normalized.includes("low")) {
    return `Visible chart range for ${context.symbol}: high ${context.dayHigh !== null ? formatCurrency(context.dayHigh) : "--"}, low ${context.dayLow !== null ? formatCurrency(context.dayLow) : "--"}.`;
  }

  if (normalized.includes("refresh") || normalized.includes("updated")) {
    return context.lastRefreshed
      ? `Market data for ${context.symbol} was last refreshed at ${context.lastRefreshed}.`
      : `Refresh metadata for ${context.symbol} is not available yet.`;
  }

  return `I can summarize the active market view for ${context.symbol}: last close, range, interval, bar count, and visible highs/lows. For broader ChatGPT-style reasoning, this chat rail is ready for a real model-backed endpoint when you add one.`;
}

function buildProjectReply(input: string, context: Extract<ChatContext, { page: "project" }>) {
  const normalized = input.toLowerCase();
  const projectName = context.projectName ?? "this project";
  const symbolList = context.symbols.length ? context.symbols.join(", ") : "no symbols yet";

  if (/(hello|hi|hey)/.test(normalized)) {
    return `Working on ${projectName}. The current symbol set is ${symbolList}. Tell me the trading behavior you want and I can help shape it into entries, exits, filters, and risk rules.`;
  }

  if (normalized.includes("symbols") || normalized.includes("universe")) {
    return `${projectName} currently tracks ${context.symbols.length} symbols: ${symbolList}.`;
  }

  if (normalized.includes("timeframe") || normalized.includes("interval") || normalized.includes("range")) {
    return `${projectName} is configured for interval ${context.interval ?? "--"} and range ${context.range ?? "--"}${context.prepost !== null ? ` with pre/post market ${context.prepost ? "enabled" : "disabled"}` : ""}.`;
  }

  if (normalized.includes("strategy") || normalized.includes("pattern") || normalized.includes("idea")) {
    return `I can help turn your idea for ${projectName} into a strategy spec with market regime, setup conditions, entry trigger, exits, and risk management. Start with something like: "mean reversion after 3 red candles" or "breakout on volume expansion".`;
  }

  return `I can help design an algorithmic pattern for ${projectName} using ${symbolList}. Describe the setup you want, and I can turn it into structured rules for signals, filters, entries, exits, and risk.`;
}

function buildProjectOutlineLocally(
  rawReply: string,
  context: Extract<ChatContext, { page: "project" }>,
) {
  const projectName = context.projectName ?? "this project";
  return [
    `Strategy Outline: ${projectName}`,
    `Universe: ${context.symbols.length ? context.symbols.join(", ") : "Define project symbols"}`,
    `Timeframe: ${context.interval ?? "--"} interval, ${context.range ?? "--"} range`,
    `Setup: ${rawReply}`,
    "Entry: Define one primary trigger that confirms the setup before entering.",
    "Exit: Use one profit-taking condition and one invalidation stop.",
    "Risk: Size positions consistently and cap loss per trade.",
  ].join("\n");
}

async function buildAssistantReply(input: string, context: ChatContext) {
  await new Promise((resolve) => window.setTimeout(resolve, 350));
  if (context.page === "home") {
    return buildHomeReply(input, context);
  }

  if (context.page === "market") {
    return buildMarketReply(input, context);
  }

  return buildProjectReply(input, context);
}

async function buildProjectOutlineReply(
  rawReply: string,
  context: Extract<ChatContext, { page: "project" }>,
  apiKey?: string,
) {
  if (!apiKey) {
    return buildProjectOutlineLocally(rawReply, context);
  }

  const response = await fetch("https://api.openai.com/v1/responses", {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      Authorization: `Bearer ${apiKey}`,
    },
    body: JSON.stringify({
      model: OPENAI_DEFAULT_MODEL,
      input: [
        "You are a trading strategy editor.",
        "Rewrite the provided draft into a tight, concise, accurate strategy outline.",
        "Do not add fluff, disclaimers, or long explanations.",
        "Return only the outline using these labels in this order:",
        "Strategy, Universe, Setup, Entry, Exit, Risk.",
        "Each label should have one short, information-dense line.",
        `Project name: ${context.projectName ?? "Unknown project"}.`,
        `Project symbols: ${context.symbols.length ? context.symbols.join(", ") : "none"}.`,
        `Project interval: ${context.interval ?? "unknown"}.`,
        `Project range: ${context.range ?? "unknown"}.`,
        "",
        "Draft to refine:",
        rawReply,
      ].join("\n"),
    }),
  });

  if (!response.ok) {
    let details = `OpenAI request failed with status ${response.status}.`;

    try {
      const errorBody = (await response.json()) as {
        error?: { message?: string };
      };
      if (errorBody.error?.message) {
        details = errorBody.error.message;
      }
    } catch {
      // keep fallback error
    }

    throw new Error(details);
  }

  const data = (await response.json()) as unknown;
  const text = extractResponseText(data);
  if (!text) {
    throw new Error("OpenAI returned a strategy outline without text output.");
  }

  return text;
}

function buildDeveloperInstruction(context: ChatContext) {
  if (context.page === "home") {
    return [
      "You are Desk, a portfolio assistant embedded in a trading dashboard.",
      "Be concise, practical, and grounded in the current UI state.",
      `Current dashboard state: scoped NAV ${formatCurrency(context.nav)}, scoped gain ${formatSignedPercent(context.gainPercentage)}, total positions ${context.totalPositions}, open positions ${context.openPositions}, selected project ${context.selectedProjectName ?? "none"}, portfolio scope ${context.portfolioScopeLabel}.`,
      `Top visible positions: ${context.topPositions.length ? context.topPositions.map((position) => `${position.symbol} in ${position.portfolioName} (${position.status}, ${position.quantity} @ ${position.averagePrice})`).join("; ") : "none"}.`,
      "If the user asks for data you do not have, say so plainly and suggest the next app action.",
    ].join(" ");
  }

  if (context.page === "project") {
    return [
      "You are Desk, a strategy-building assistant embedded in a project workspace for algorithmic trading.",
      "Help the user turn freeform ideas into concrete trading rules.",
      "Be practical and structured. First produce the best raw strategy draft you can from the user request.",
      `Project context: name ${context.projectName ?? "unknown"}, id ${context.projectId}, symbols ${context.symbols.length ? context.symbols.join(", ") : "none"}, interval ${context.interval ?? "unknown"}, range ${context.range ?? "unknown"}, pre/post market ${context.prepost === null ? "unknown" : context.prepost ? "enabled" : "disabled"}.`,
      `Project description: ${context.description ?? "none"}.`,
      "Do not claim backtesting results you do not have. If the user asks for code, produce concise pseudocode or implementation-ready rule logic.",
    ].join(" ");
  }

  return [
    "You are Desk, a market assistant embedded in a stock chart view.",
    "Be concise, practical, and grounded in the visible chart state.",
    `Current market state: symbol ${context.symbol}, range ${context.range}, interval ${context.interval}, last close ${context.lastClose !== null ? formatCurrency(context.lastClose) : "unavailable"}, bars loaded ${context.barCount}, visible high ${context.dayHigh !== null ? formatCurrency(context.dayHigh) : "unavailable"}, visible low ${context.dayLow !== null ? formatCurrency(context.dayLow) : "unavailable"}, last refreshed ${context.lastRefreshed ?? "unavailable"}.`,
    "If the user asks for data you do not have, say so plainly and suggest the next app action.",
  ].join(" ");
}

function extractResponseText(response: unknown): string {
  if (!response || typeof response !== "object") {
    return "";
  }

  const candidate = response as {
    output_text?: string;
    output?: Array<{
      type?: string;
      content?: Array<{
        type?: string;
        text?: string;
      }>;
    }>;
  };

  if (typeof candidate.output_text === "string" && candidate.output_text.trim()) {
    return candidate.output_text.trim();
  }

  const textFromOutput =
    candidate.output
      ?.flatMap((item) => item.content ?? [])
      .filter((item) => item.type === "output_text" && typeof item.text === "string")
      .map((item) => item.text?.trim() ?? "")
      .filter(Boolean)
      .join("\n\n") ?? "";

  return textFromOutput.trim();
}

async function buildOpenAIReply(
  input: string,
  context: ChatContext,
  history: ChatMessage[],
  apiKey: string,
) {
  const transcript = history
    .map((message) => `${message.role === "user" ? "User" : "Assistant"}: ${message.content}`)
    .join("\n\n");

  const response = await fetch("https://api.openai.com/v1/responses", {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      Authorization: `Bearer ${apiKey}`,
    },
    body: JSON.stringify({
      model: OPENAI_DEFAULT_MODEL,
      input: `${buildDeveloperInstruction(context)}\n\nConversation so far:\n${transcript || "No prior messages."}\n\nUser: ${input}\nAssistant:`,
    }),
  });

  if (!response.ok) {
    let details = `OpenAI request failed with status ${response.status}.`;

    try {
      const errorBody = (await response.json()) as {
        error?: { message?: string };
      };
      if (errorBody.error?.message) {
        details = errorBody.error.message;
      }
    } catch {
      // keep fallback error
    }

    throw new Error(details);
  }

  const data = (await response.json()) as unknown;
  const text = extractResponseText(data);
  if (!text) {
    throw new Error("OpenAI returned a response without text output.");
  }

  return text;
}

function getStorageKey(context: ChatContext) {
  if (context.page === "project") {
    return `desk-chat-history-project-${context.projectId}`;
  }

  return context.page === "market"
    ? "desk-chat-history-market"
    : "desk-chat-history-home";
}

function loadMessages(context: ChatContext) {
  if (typeof window === "undefined") {
    return [createWelcomeMessage(context.page)];
  }

  try {
    const raw = window.localStorage.getItem(getStorageKey(context));
    if (!raw) {
      return [createWelcomeMessage(context.page)];
    }

    const parsed = JSON.parse(raw) as ChatMessage[];
    return parsed.length ? parsed : [createWelcomeMessage(context.page)];
  } catch {
    return [createWelcomeMessage(context.page)];
  }
}

export function useDeskChat(context: ChatContext) {
  const storageKey = useMemo(() => getStorageKey(context), [context]);
  const [messages, setMessages] = useState<ChatMessage[]>(() => [
    createWelcomeMessage(context.page),
  ]);
  const [pending, setPending] = useState(false);
  const [apiKeyAvailable, setApiKeyAvailable] = useState(Boolean(getStoredOpenAIApiKey()));
  const contextRef = useRef(context);

  useEffect(() => {
    contextRef.current = context;
  }, [context]);

  useEffect(() => {
    setMessages(loadMessages(context));
  }, [storageKey]);

  useEffect(() => {
    window.localStorage.setItem(storageKey, JSON.stringify(messages));
  }, [messages, storageKey]);

  useEffect(() => {
    const syncApiKey = () => {
      setApiKeyAvailable(Boolean(getStoredOpenAIApiKey()));
    };

    syncApiKey();
    window.addEventListener("storage", syncApiKey);
    window.addEventListener(OPENAI_SETTINGS_CHANGE_EVENT, syncApiKey as EventListener);

    return () => {
      window.removeEventListener("storage", syncApiKey);
      window.removeEventListener(
        OPENAI_SETTINGS_CHANGE_EVENT,
        syncApiKey as EventListener,
      );
    };
  }, []);

  const suggestions = useMemo(
    () =>
      context.page === "home"
        ? [
            "What is the scoped NAV?",
            "How many open positions do I have?",
            "What portfolios are in scope?",
          ]
        : context.page === "market"
          ? [
            "What is the last close?",
            "What range and interval am I using?",
            "What are the visible highs and lows?",
          ]
          : [
            "Build a mean reversion strategy for this project",
            "Turn this idea into entry and exit rules",
            "What filters should I use for these symbols?",
          ],
    [context.page],
  );

  async function sendMessage(content: string) {
    const trimmed = content.trim();
    if (!trimmed || pending) {
      return;
    }

    const userMessage = createMessage("user", trimmed);
    const nextHistory = [...messages, userMessage];
    setMessages((current) => [...current, userMessage]);
    setPending(true);

    try {
      const apiKey = getStoredOpenAIApiKey();
      const rawReply = apiKey
        ? await buildOpenAIReply(trimmed, contextRef.current, nextHistory, apiKey)
        : await buildAssistantReply(trimmed, contextRef.current);

      const reply =
        contextRef.current.page === "project"
          ? await buildProjectOutlineReply(rawReply, contextRef.current, apiKey || undefined)
          : rawReply;

      setMessages((current) => [...current, createMessage("assistant", reply)]);
    } catch (error) {
      const message =
        error instanceof Error
          ? error.message
          : "Chat failed. Check your OpenAI API key and try again.";
      setMessages((current) => [...current, createMessage("assistant", message)]);
    } finally {
      setPending(false);
    }
  }

  function clearMessages() {
    const welcome = createWelcomeMessage(context.page);
    setMessages([welcome]);
  }

  return {
    messages,
    pending,
    suggestions,
    sendMessage,
    clearMessages,
    usingOpenAI: apiKeyAvailable,
  };
}
