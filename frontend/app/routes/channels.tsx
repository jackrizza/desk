import { useEffect, useMemo, useRef, useState } from "react";
import type React from "react";
import ReactMarkdown from "react-markdown";
import { Link, useNavigate, useParams } from "react-router";
import remarkGfm from "remark-gfm";
import { EmptyState, ErrorInline, LoadingInline } from "../components/AppFeedback";
import { ChatPanel } from "../components/ChatPanel";
import { LeftSidebar } from "../components/LeftSidebar";
import { Topbar } from "../components/Topbar";
import { deskApi, type Channel, type ChannelMessage } from "../lib/api";
import { CHAT_OPEN_STORAGE_KEY, usePersistentBoolean } from "../lib/ui-state";

export default function ChannelsRoute() {
  const [sidebarOpen, setSidebarOpen] = useState(true);
  const [channels, setChannels] = useState<Channel[]>([]);
  const [messages, setMessages] = useState<ChannelMessage[]>([]);
  const [composer, setComposer] = useState("");
  const [loading, setLoading] = useState(true);
  const [posting, setPosting] = useState(false);
  const [error, setError] = useState("");
  const [autoScroll, setAutoScroll] = useState(true);
  const [composerHeight, setComposerHeight] = useState(176);
  const [chatOpen, setChatOpen] = usePersistentBoolean(CHAT_OPEN_STORAGE_KEY, false);
  const navigate = useNavigate();
  const { channel: channelParam } = useParams();
  const messagesPaneRef = useRef<HTMLDivElement | null>(null);
  const messagesEndRef = useRef<HTMLDivElement | null>(null);
  const composerRef = useRef<HTMLFormElement | null>(null);

  const selectedChannel = useMemo(
    () => channels.find((channel) => channel.name === channelParam || channel.id === channelParam) ?? channels[0],
    [channels, channelParam],
  );

  async function loadChannels() {
    const nextChannels = await deskApi.listChannels();
    setChannels(nextChannels);
  }

  async function loadMessages(channelId = selectedChannel?.id) {
    if (!channelId) {
      setMessages([]);
      return;
    }
    const response = await deskApi.listChannelMessages(channelId, { limit: 250 });
    setMessages(response.messages);
  }

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    loadChannels()
      .catch((err) => {
        if (!cancelled) {
          setError(err instanceof Error ? err.message : "Failed to load channels.");
        }
      })
      .finally(() => {
        if (!cancelled) {
          setLoading(false);
        }
      });
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    if (!selectedChannel?.id) {
      return;
    }
    let cancelled = false;
    setAutoScroll(true);
    loadMessages(selectedChannel.id).catch((err) => {
      if (!cancelled) {
        setError(err instanceof Error ? err.message : "Failed to load channel messages.");
      }
    });
    const intervalId = window.setInterval(() => {
      void loadMessages(selectedChannel.id).catch(() => undefined);
    }, 5000);
    return () => {
      cancelled = true;
      window.clearInterval(intervalId);
    };
  }, [selectedChannel?.id]);

  useEffect(() => {
    if (loading || channels.length === 0) {
      return;
    }
    if (!channelParam) {
      navigate("/channels/general", { replace: true });
      return;
    }
    const matched = channels.some((channel) => channel.name === channelParam || channel.id === channelParam);
    if (!matched) {
      navigate(`/channels/${channels[0].name}`, { replace: true });
    }
  }, [channelParam, channels, loading, navigate]);

  useEffect(() => {
    if (!autoScroll) {
      return;
    }
    messagesEndRef.current?.scrollIntoView({ block: "end" });
  }, [autoScroll, composerHeight, messages.length, selectedChannel?.id]);

  useEffect(() => {
    const composerElement = composerRef.current;
    if (!composerElement) {
      return;
    }

    const updateComposerHeight = () => {
      setComposerHeight(Math.ceil(composerElement.getBoundingClientRect().height));
    };

    updateComposerHeight();
    const resizeObserver = new ResizeObserver(updateComposerHeight);
    resizeObserver.observe(composerElement);
    window.addEventListener("resize", updateComposerHeight);

    return () => {
      resizeObserver.disconnect();
      window.removeEventListener("resize", updateComposerHeight);
    };
  }, []);

  function handleMessagesScroll() {
    const pane = messagesPaneRef.current;
    if (!pane) {
      return;
    }
    const distanceFromBottom = pane.scrollHeight - pane.scrollTop - pane.clientHeight;
    const nearBottom = distanceFromBottom < 80;
    setAutoScroll(nearBottom);
  }

  function handleAutoScrollChange(checked: boolean) {
    setAutoScroll(checked);
    if (checked) {
      window.requestAnimationFrame(() => {
        messagesEndRef.current?.scrollIntoView({ block: "end" });
      });
    }
  }

  async function handleSubmit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!selectedChannel || !composer.trim()) {
      return;
    }
    setPosting(true);
    setError("");
    try {
      const created = await deskApi.postChannelMessage(selectedChannel.id, composer.trim());
      setMessages((current) => [...current, created]);
      setComposer("");
      void loadMessages(selectedChannel.id).catch(() => undefined);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to send message.");
    } finally {
      setPosting(false);
    }
  }

  return (
    <div className="min-h-screen app-bg app-text">
      <Topbar
        sidebarOpen={sidebarOpen}
        setSidebarOpen={setSidebarOpen}
        chatOpen={chatOpen}
        setChatOpen={setChatOpen}
      />
      <LeftSidebar open={sidebarOpen} />
      <main className={`min-h-screen pt-16 transition-all duration-200 ${sidebarOpen ? "pl-64" : "pl-0"}`}>
        <div className="flex h-[calc(100vh-4rem)]">
          <aside className="app-surface w-64 shrink-0 border-r p-4">
            <h1 className="mb-4 text-lg font-semibold">Channels</h1>
            <div className="space-y-1">
              {channels.map((channel) => (
                <Link
                  key={channel.id}
                  className={`app-nav-link w-full rounded-md px-3 py-2 text-left ${selectedChannel?.id === channel.id ? "font-semibold" : ""}`}
                  to={`/channels/${channel.name}`}
                >
                  <span className="block truncate">{channel.display_name}</span>
                  {channel.description ? (
                    <span className="app-text-muted block truncate text-xs">{channel.description}</span>
                  ) : null}
                </Link>
              ))}
            </div>
          </aside>

          <section className="flex min-w-0 flex-1 flex-col">
            <header className="app-surface flex items-start justify-between gap-4 border-b px-6 py-4">
              <div className="min-w-0">
                <h2 className="text-xl font-semibold">{selectedChannel?.display_name ?? "Channels"}</h2>
                {selectedChannel?.description ? (
                  <p className="app-text-muted mt-1 text-sm">{selectedChannel.description}</p>
                ) : null}
              </div>
              <label className="app-text-muted flex shrink-0 items-center gap-2 text-sm">
                <input
                  type="checkbox"
                  checked={autoScroll}
                  onChange={(event) => handleAutoScrollChange(event.target.checked)}
                />
                Auto-scroll
              </label>
            </header>

            {error ? <div className="px-6 pt-4"><ErrorInline message={error} /></div> : null}
            <div
              ref={messagesPaneRef}
              onScroll={handleMessagesScroll}
              className="flex-1 overflow-y-auto px-6 pt-5"
            >
              {loading ? (
                <LoadingInline message="Loading channels..." />
              ) : messages.length === 0 ? (
                <EmptyState title="No messages yet" description="Start the channel discussion." />
              ) : (
                <div className="space-y-3">
                {messages.map((message) => (
                  <MessageBubble key={message.id} message={message} />
                ))}
                </div>
              )}
              <div style={{ height: composerHeight + 24 }} />
              <div ref={messagesEndRef} />
            </div>

            <form
              ref={composerRef}
              className="app-surface fixed bottom-0 right-0 z-20 border-t p-4"
              style={{ left: sidebarOpen ? "32rem" : "16rem" }}
              onSubmit={handleSubmit}
            >
              <textarea
                value={composer}
                onChange={(event) => setComposer(event.target.value)}
                rows={3}
                className="app-input w-full rounded-2xl px-4 py-3 text-sm"
                placeholder={`Message ${selectedChannel?.display_name ?? "channel"}`}
              />
              <div className="mt-3 flex justify-end">
                <button
                  type="submit"
                  disabled={posting || !composer.trim()}
                  className="app-button-primary rounded-full px-4 py-2 text-sm font-medium disabled:cursor-not-allowed disabled:opacity-60"
                >
                  {posting ? "Sending..." : "Send"}
                </button>
              </div>
            </form>
          </section>
        </div>
      </main>
      <ChatPanel open={chatOpen} onClose={() => setChatOpen(false)} />
    </div>
  );
}

function MessageBubble({ message }: { message: ChannelMessage }) {
  const renderedMarkdown = enhanceChannelMarkdown(message.content_markdown);

  return (
    <article className={`channel-message channel-message--${message.author_type} rounded-lg p-4`}>
      <div className="mb-2 flex flex-wrap items-center gap-2">
        <span className="font-semibold">{message.author_name}</span>
        <span className="channel-badge rounded-full px-2 py-0.5 text-xs uppercase">{message.author_type}</span>
        <span className="app-text-muted text-xs">{formatTimestamp(message.created_at)}</span>
        {message.role !== "message" ? (
          <span className="app-text-muted text-xs">{message.role}</span>
        ) : null}
      </div>
      <div className="prose prose-sm max-w-none">
        <ReactMarkdown
          remarkPlugins={[remarkGfm]}
          components={{
            a: ({ href, children }) => {
              if (href?.startsWith("#mention-")) {
                return <span className="font-medium text-blue-600 dark:text-blue-400">{children}</span>;
              }
              if (href?.startsWith("/market/")) {
                return (
                  <Link className="font-medium text-blue-600 underline-offset-2 hover:underline dark:text-blue-400" to={href}>
                    {children}
                  </Link>
                );
              }
              return (
                <a href={href} className="underline-offset-2 hover:underline">
                  {children}
                </a>
              );
            },
          }}
        >
          {renderedMarkdown}
        </ReactMarkdown>
      </div>
    </article>
  );
}

function formatTimestamp(value: string) {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return value;
  }
  return date.toLocaleString();
}

const COMMON_UPPERCASE_WORDS = new Set([
  "AI",
  "API",
  "CEO",
  "CFO",
  "COO",
  "CSV",
  "ETF",
  "HTTP",
  "JSON",
  "LLM",
  "MD",
  "NYSE",
  "NASDAQ",
  "PCT",
  "Q",
  "QOQ",
  "SEC",
  "UI",
  "URL",
  "USD",
  "USER",
  "YOY",
]);

function enhanceChannelMarkdown(markdown: string) {
  const lines = markdown.split("\n");
  let inFence = false;

  return lines
    .map((line) => {
      if (line.trimStart().startsWith("```")) {
        inFence = !inFence;
        return line;
      }
      if (inFence) {
        return line;
      }
      return enhanceInlineChannelMarkdown(line);
    })
    .join("\n");
}

function enhanceInlineChannelMarkdown(markdown: string) {
  const protectedParts: string[] = [];
  const protect = (match: string) => {
    const token = `\u0000${protectedParts.length}\u0000`;
    protectedParts.push(match);
    return token;
  };

  let next = markdown
    .replace(/`[^`]*`/g, protect)
    .replace(/!?\[[^\]]+\]\([^)]+\)/g, protect);

  next = next
    .replace(/(^|[^\w/])@\[([^\]\n]{1,40})\]/g, (_match, prefix: string, name: string) => {
      const handle = channelHandle(name);
      return `${prefix}[@${name}](#mention-${handle})`;
    })
    .replace(/(^|[^\w/])@([A-Za-z][A-Za-z0-9_-]{0,39})\b/g, (_match, prefix: string, handle: string) => {
      return `${prefix}[@${handle}](#mention-${channelHandle(handle)})`;
    })
    .replace(
      /(^|[^\w$@/])(\$?)([A-Z]{1,5})(?=\b)/g,
      (match: string, prefix: string, marker: string, symbol: string) => {
        if (!marker && symbol.length < 2) {
          return match;
        }
        if (COMMON_UPPERCASE_WORDS.has(symbol)) {
          return match;
        }
        return `${prefix}[${marker}${symbol}](/market/${encodeURIComponent(symbol)})`;
      },
    );

  return next.replace(/\u0000(\d+)\u0000/g, (_match, index: string) => protectedParts[Number(index)] ?? "");
}

function channelHandle(value: string) {
  return value.replace(/[^A-Za-z0-9_-]/g, "") || "mention";
}
