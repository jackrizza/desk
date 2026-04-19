import { Link } from "react-router";

function SearchBar({
    quoteSymbol,
    onQuoteSymbolChange,
    onQuoteLookup,
    quoteLoading,
}: {
    quoteSymbol: string;
    onQuoteSymbolChange: (value: string) => void;
    onQuoteLookup: () => void;
    quoteLoading: boolean;
}) {
    return (
        <div className="app-surface-muted flex w-full items-center gap-2 rounded-2xl px-4 py-2.5">
            <input
                type="text"
                value={quoteSymbol}
                onChange={(event) => onQuoteSymbolChange(event.target.value)}
                onKeyDown={(event) => {
                    if (event.key === "Enter") {
                        event.preventDefault();
                        onQuoteLookup();
                    }
                }}
                placeholder="Lookup symbol"
                className="app-text-muted w-full bg-transparent focus:outline-none"
            />
            <button
                type="button"
                onClick={onQuoteLookup}
                className="app-button-primary rounded-lg px-3 py-1.5 text-xs font-medium transition"
            >
                {quoteLoading ? "..." : "Go"}
            </button>
        </div>
    );
}

export function Topbar({
    onToggleSidebar,
    onToggleChat,
    sidebarOpen,
    quoteSymbol,
    onQuoteSymbolChange,
    onQuoteLookup,
    quoteLoading,
    showMenuButton,
    chatOpen,
}: {
    onToggleSidebar: () => void;
    onToggleChat?: () => void;
    sidebarOpen?: boolean;
    quoteSymbol: string;
    onQuoteSymbolChange: (value: string) => void;
    onQuoteLookup: () => void;
    quoteLoading: boolean;
    showMenuButton?: boolean;
    chatOpen?: boolean;
}) {
    return (
        <div className="app-surface fixed top-0 z-20 h-16 w-full border-b px-4">
            <div className="relative flex h-full items-center justify-between">
                <div className="left flex items-center gap-3">
                    {showMenuButton !== false && (
                        <button
                            className="app-nav-link hamburger flex h-8 w-8 items-center justify-center rounded"
                            onClick={onToggleSidebar}
                            style={
                                sidebarOpen
                                    ? {
                                          background:
                                              "color-mix(in srgb, var(--color-primary) 14%, transparent)",
                                      }
                                    : undefined
                            }
                        >
                            <svg className="h-6 w-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 6h16M4 12h16M4 18h16" />
                            </svg>
                        </button>
                    )}
                    <Link to="/" className="text-3xl font-bold leading-none" style={{ color: "var(--color-primary)" }}>Desk</Link>
                </div>

                <div className="absolute left-1/2 hidden w-[33vw] min-w-[22rem] max-w-[34rem] -translate-x-1/2 md:block">
                    <SearchBar
                        quoteSymbol={quoteSymbol}
                        onQuoteSymbolChange={onQuoteSymbolChange}
                        onQuoteLookup={onQuoteLookup}
                        quoteLoading={quoteLoading}
                    />
                </div>

                <div className="right flex items-center gap-3">
                    <div className="w-40 sm:hidden">
                        <SearchBar
                            quoteSymbol={quoteSymbol}
                            onQuoteSymbolChange={onQuoteSymbolChange}
                            onQuoteLookup={onQuoteLookup}
                            quoteLoading={quoteLoading}
                        />
                    </div>
                    {onToggleChat && (
                        <button
                            type="button"
                            aria-label="Toggle chat"
                            onClick={onToggleChat}
                            className="app-nav-link flex h-8 w-8 items-center justify-center rounded"
                            style={
                                chatOpen
                                    ? {
                                          background:
                                              "color-mix(in srgb, var(--color-primary) 14%, transparent)",
                                      }
                                    : undefined
                            }
                        >
                            <svg className="h-6 w-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path
                                    strokeLinecap="round"
                                    strokeLinejoin="round"
                                    strokeWidth={2}
                                    d="M8 10h8M8 14h5m7-2a9 9 0 11-18 0 9 9 0 0118 0z"
                                />
                            </svg>
                        </button>
                    )}
                </div>
            </div>
        </div>
    );
}
