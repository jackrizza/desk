import { useEffect, useRef, useState } from "react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import type { ChatMessage, ChatTarget } from "../lib/chat";
import type { Trader } from "../lib/api";

function formatTimestamp(value: string) {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return "";
  }

  return date.toLocaleTimeString([], {
    hour: "numeric",
    minute: "2-digit",
  });
}

export function ChatPanel({
  open,
  title,
  messages,
  pending,
  suggestions,
  chatTarget,
  chatTraders = [],
  onChatTargetChange,
  onClose,
  onSubmit,
  onClear,
}: {
  open: boolean;
  title: string;
  messages: ChatMessage[];
  pending: boolean;
  suggestions: string[];
  chatTarget?: ChatTarget;
  chatTraders?: Trader[];
  onChatTargetChange?: (target: ChatTarget) => void;
  onClose: () => void;
  onSubmit: (value: string) => Promise<void> | void;
  onClear: () => void;
}) {
  const [draft, setDraft] = useState("");
  const scrollRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    if (!open || !scrollRef.current) {
      return;
    }

    scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
  }, [messages, pending, open]);

  if (!open) {
    return null;
  }

  return (
    <aside className="app-surface fixed right-0 top-[calc(4rem-1px)] z-30 flex h-[calc(100vh-4rem+1px)] w-full flex-col border-l shadow-2xl md:w-96 xl:w-[25vw]">
      <div className="app-surface-muted flex items-center justify-between border-b px-4 py-3">
        <div>
          <p className="app-text-muted text-xs uppercase tracking-[0.18em]">Desk Chat</p>
          <h2 className="mt-1 text-base font-semibold">{title}</h2>
        </div>
        <div className="flex items-center gap-2">
          <button
            type="button"
            onClick={onClear}
            className="app-button-secondary rounded-full px-3 py-1.5 text-xs font-medium transition"
          >
            Clear
          </button>
          <button
            type="button"
            onClick={onClose}
            className="app-button-secondary rounded-full px-3 py-1.5 text-xs font-medium transition"
          >
            Close
          </button>
        </div>
      </div>

      {chatTarget && onChatTargetChange ? (
        <div className="border-b px-4 py-3" style={{ borderColor: "var(--color-border)" }}>
          <label className="flex items-center gap-3 text-sm">
            <span className="app-text-muted shrink-0">Chat with:</span>
            <select
              className="app-input min-w-0 flex-1 rounded-xl px-3 py-2 text-sm"
              value={
                chatTarget.kind === "desk"
                  ? "desk"
                  : chatTarget.kind === "md"
                    ? "md"
                    : chatTarget.kind === "data_scientist"
                      ? "data_scientist"
                  : `trader:${chatTarget.trader_id}`
              }
              onChange={(event) => {
                if (event.target.value === "desk") {
                  onChatTargetChange({ kind: "desk" });
                  return;
                }
                if (event.target.value === "md") {
                  onChatTargetChange({ kind: "md" });
                  return;
                }
                if (event.target.value === "data_scientist") {
                  onChatTargetChange({ kind: "data_scientist" });
                  return;
                }
                const traderId = event.target.value.replace(/^trader:/, "");
                const trader = chatTraders.find((candidate) => candidate.id === traderId);
                if (trader) {
                  onChatTargetChange({
                    kind: "trader",
                    trader_id: trader.id,
                    trader_name: trader.name,
                  });
                }
              }}
            >
              <option value="desk">Desk</option>
              <option value="md">MD</option>
              <option value="data_scientist">Data Scientist</option>
              {chatTraders.map((trader) => (
                <option key={trader.id} value={`trader:${trader.id}`}>
                  {`Trader: ${trader.name} - ${formatFreedom(trader.freedom_level)} - ${trader.status}`}
                </option>
              ))}
            </select>
          </label>
        </div>
      ) : null}

      <div ref={scrollRef} className="flex-1 space-y-4 overflow-y-auto px-4 py-4">
        {messages.map((message) => (
          <article
            key={message.id}
            className={`max-w-[90%] rounded-2xl px-4 py-3 ${
              message.role === "user"
                ? "ml-auto app-button-primary"
                : "app-surface-muted"
            }`}
          >
            <div className="mb-2 flex items-center justify-between gap-3">
              <p className="text-xs font-semibold uppercase tracking-[0.16em]">
                {message.role === "user"
                  ? "You"
                  : chatTarget?.kind === "md"
                    ? "MD"
                    : chatTarget?.kind === "data_scientist"
                      ? "Data Scientist"
                  : chatTarget?.kind === "trader"
                    ? chatTarget.trader_name
                    : "Desk"}
              </p>
              <p className="app-text-muted text-xs">{formatTimestamp(message.createdAt)}</p>
            </div>
            {message.role === "assistant" ? (
              <>
                <MarkdownBubble content={message.content} />
                {message.dataSourceResult ? (
                  <DataSourceResultCard result={message.dataSourceResult} />
                ) : null}
              </>
            ) : (
              <p className="whitespace-pre-wrap text-sm leading-6">{message.content}</p>
            )}
          </article>
        ))}

        {pending ? (
          <div className="app-surface-muted max-w-[90%] rounded-2xl px-4 py-3">
            <p className="app-text-muted text-sm">{thinkingLabel(chatTarget)} is thinking...</p>
          </div>
        ) : null}
      </div>

      <div className="border-t px-4 py-4" style={{ borderColor: "var(--color-border)" }}>
        <div className="mb-3 flex flex-wrap gap-2">
          {suggestions.map((suggestion) => (
            <button
              key={suggestion}
              type="button"
              onClick={() => {
                setDraft(suggestion);
              }}
              className="app-button-secondary rounded-full px-3 py-1.5 text-xs font-medium transition"
            >
              {suggestion}
            </button>
          ))}
        </div>

        <form
          className="space-y-3"
          onSubmit={async (event) => {
            event.preventDefault();
            const nextDraft = draft.trim();
            if (!nextDraft) {
              return;
            }

            setDraft("");
            await onSubmit(nextDraft);
          }}
        >
          <textarea
            value={draft}
            onChange={(event) => setDraft(event.target.value)}
            rows={4}
            placeholder="Ask about your dashboard or market view..."
            className="app-input w-full rounded-2xl px-3 py-3 text-sm transition"
          />
          <div className="flex justify-end">
            <button
              type="submit"
              disabled={pending}
              className="app-button-primary rounded-full px-4 py-2 text-sm font-medium transition disabled:cursor-not-allowed disabled:opacity-60"
            >
              Send
            </button>
          </div>
        </form>
      </div>
    </aside>
  );
}

function thinkingLabel(target?: ChatTarget) {
  if (target?.kind === "md") return "MD";
  if (target?.kind === "data_scientist") return "Data Scientist";
  if (target?.kind === "trader") return target.trader_name;
  return "Desk";
}

function formatFreedom(level: string) {
  return level.replace("_", " ");
}

function DataSourceResultCard({
  result,
}: {
  result: NonNullable<ChatMessage["dataSourceResult"]>;
}) {
  return (
    <div className="app-surface mt-3 rounded-lg border p-3 text-sm" style={{ borderColor: "var(--color-border)" }}>
      <p className="text-xs font-semibold uppercase tracking-[0.16em]">Created Data Source</p>
      <div className="mt-2 space-y-1">
        <ResultRow label="Name" value={result.name ?? "Unknown"} />
        <ResultRow label="Type" value={result.source_type === "python_script" ? "Python Script" : result.source_type ?? "Unknown"} />
        <ResultRow label="URL" value={result.url ?? "None"} />
        <ResultRow label="Build" value={result.build_status === "success" ? "Success" : result.build_status === "failed" ? "Failed" : result.build_status ?? "Unknown"} />
      </div>
    </div>
  );
}

function ResultRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="grid grid-cols-[4rem_1fr] gap-2">
      <span className="app-text-muted">{label}</span>
      <span className="break-words">{value}</span>
    </div>
  );
}

function MarkdownBubble({ content }: { content: string }) {
  return (
    <div className="text-sm leading-6">
      <ReactMarkdown
        remarkPlugins={[remarkGfm]}
        components={{
          p: ({ children }) => <p className="mb-3 last:mb-0">{children}</p>,
          h1: ({ children }) => (
            <h1 className="mb-2 mt-4 text-lg font-semibold first:mt-0">{children}</h1>
          ),
          h2: ({ children }) => (
            <h2 className="mb-2 mt-4 text-base font-semibold first:mt-0">{children}</h2>
          ),
          h3: ({ children }) => (
            <h3 className="mb-2 mt-3 text-sm font-semibold first:mt-0">{children}</h3>
          ),
          ul: ({ children }) => (
            <ul className="mb-3 list-disc space-y-1 pl-5 last:mb-0">{children}</ul>
          ),
          ol: ({ children }) => (
            <ol className="mb-3 list-decimal space-y-1 pl-5 last:mb-0">{children}</ol>
          ),
          li: ({ children }) => <li className="pl-1">{children}</li>,
          a: ({ children, href }) => (
            <a
              href={href}
              target="_blank"
              rel="noreferrer"
              className="font-medium underline underline-offset-2"
              style={{ color: "var(--color-primary)" }}
            >
              {children}
            </a>
          ),
          blockquote: ({ children }) => (
            <blockquote
              className="my-3 border-l-2 pl-3 italic app-text-muted"
              style={{ borderColor: "var(--color-border)" }}
            >
              {children}
            </blockquote>
          ),
          code: ({ children, className }) => {
            const inline = !className;
            return inline ? (
              <code className="app-surface rounded px-1.5 py-0.5 text-[0.85em]">
                {children}
              </code>
            ) : (
              <code className={className}>{children}</code>
            );
          },
          pre: ({ children }) => (
            <pre className="app-surface my-3 overflow-x-auto rounded-xl p-3 text-xs leading-5">
              {children}
            </pre>
          ),
          table: ({ children }) => (
            <div className="my-3 overflow-x-auto">
              <table className="min-w-full text-left text-xs">{children}</table>
            </div>
          ),
          th: ({ children }) => (
            <th className="border-b px-2 py-1 font-semibold" style={{ borderColor: "var(--color-border)" }}>
              {children}
            </th>
          ),
          td: ({ children }) => (
            <td className="border-b px-2 py-1 align-top" style={{ borderColor: "var(--color-border)" }}>
              {children}
            </td>
          ),
        }}
      >
        {content}
      </ReactMarkdown>
    </div>
  );
}
