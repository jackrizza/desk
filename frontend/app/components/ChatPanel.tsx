import { useEffect, useRef, useState } from "react";
import type { ChatMessage } from "../lib/chat";

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
  onClose,
  onSubmit,
  onClear,
}: {
  open: boolean;
  title: string;
  messages: ChatMessage[];
  pending: boolean;
  suggestions: string[];
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
                {message.role === "user" ? "You" : "Desk"}
              </p>
              <p className="app-text-muted text-xs">{formatTimestamp(message.createdAt)}</p>
            </div>
            <p className="whitespace-pre-wrap text-sm leading-6">{message.content}</p>
          </article>
        ))}

        {pending ? (
          <div className="app-surface-muted max-w-[90%] rounded-2xl px-4 py-3">
            <p className="app-text-muted text-sm">Desk is thinking...</p>
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
