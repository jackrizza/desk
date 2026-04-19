import { useEffect, useState } from "react";
import { Link, useNavigate } from "react-router";
import { deskApi, type Project } from "../lib/api";
import {
    applyTheme,
    getResolvedTheme,
    getStoredTheme,
    THEME_CHANGE_EVENT,
    type Theme,
} from "../lib/theme";
import {
    getStoredOpenAIApiKey,
    saveOpenAIApiKey,
} from "../lib/openai";

export function LeftSidebar({
    open,
}: {
    open: boolean;
}) {
    const [projectsOpen, setProjectsOpen] = useState(false);
    const [projects, setProjects] = useState<Project[]>([]);
    const [settingsOpen, setSettingsOpen] = useState(false);
    const [theme, setTheme] = useState<Theme>("light");
    const [openAIApiKey, setOpenAIApiKey] = useState("");
    const [apiKeySaved, setApiKeySaved] = useState(false);
    const navigate = useNavigate();

    useEffect(() => {
        setTheme(getResolvedTheme());

        const handleThemeChange = (event: Event) => {
            const customEvent = event as CustomEvent<Theme>;
            if (customEvent.detail === "light" || customEvent.detail === "dark") {
                setTheme(customEvent.detail);
                return;
            }

            const storedTheme = getStoredTheme();
            if (storedTheme) {
                setTheme(storedTheme);
            }
        };

        const handleStorage = (event: StorageEvent) => {
            if (event.key !== "desk-theme") {
                return;
            }

            const storedTheme = getStoredTheme();
            if (storedTheme) {
                setTheme(storedTheme);
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
        applyTheme(theme);
    }, [theme]);

    useEffect(() => {
        setOpenAIApiKey(getStoredOpenAIApiKey());
    }, [settingsOpen]);

    useEffect(() => {
        let cancelled = false;

        async function loadProjects() {
            try {
                const nextProjects = await deskApi.listProjects();
                if (!cancelled) {
                    setProjects(nextProjects);
                }
            } catch {
                if (!cancelled) {
                    setProjects([]);
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

    return (
        <>
            <div className={`app-surface fixed top-0 left-0 z-10 h-screen w-64 border-r pt-16 transition-transform duration-200 ${open ? "translate-x-0" : "-translate-x-full"}`}>
                <nav className="flex flex-col space-y-1 p-4">
                    <Link to="/market/AAPL" className="app-nav-link rounded-md px-3 py-2">Market</Link>
                    <Link to="/" className="app-nav-link rounded-md px-3 py-2">Portfolio</Link>

                    <div>
                        <button
                            onClick={() => setProjectsOpen((o) => !o)}
                            className="app-nav-link flex w-full items-center justify-between rounded-md px-3 py-2"
                        >
                            <span>Projects</span>
                            <svg
                                className={`w-4 h-4 transition-transform duration-200 ${projectsOpen ? "rotate-180" : ""}`}
                                fill="none" stroke="currentColor" viewBox="0 0 24 24"
                            >
                                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 9l-7 7-7-7" />
                            </svg>
                        </button>
                        {projectsOpen && (
                            <div className="flex flex-col mt-1 ml-4 space-y-1">
                                <button
                                    type="button"
                                    onClick={() => navigate("/?createProject=1")}
                                    className="app-nav-link app-text-muted rounded-md px-3 py-2 text-left"
                                >
                                    Create project
                                </button>
                                {projects.map((project) => (
                                    <button
                                        key={project.id}
                                        type="button"
                                        onClick={() => navigate(`/projects/${encodeURIComponent(project.id)}`)}
                                        className="app-nav-link app-text-muted rounded-md px-3 py-2 text-left"
                                        title={project.name}
                                    >
                                        {project.name}
                                    </button>
                                ))}
                                {projects.length === 0 ? (
                                    <div className="app-text-muted rounded-md px-3 py-2 text-sm">
                                        No projects yet
                                    </div>
                                ) : null}
                            </div>
                        )}
                    </div>

                    <button
                        onClick={() => setSettingsOpen(true)}
                        className="app-nav-link rounded-md px-3 py-2 text-left"
                    >
                        Settings
                    </button>
                </nav>
            </div>

            {settingsOpen && (
                <div className="app-modal-backdrop fixed inset-0 z-30 flex items-center justify-center p-4">
                    <div className="app-surface w-full max-w-md rounded-lg p-5 shadow-lg">
                        <div className="mb-4 flex items-center justify-between">
                            <h2 className="text-lg font-semibold">Settings</h2>
                            <button
                                onClick={() => setSettingsOpen(false)}
                                className="app-nav-link app-text-muted rounded px-2 py-1"
                                aria-label="Close settings"
                            >
                                x
                            </button>
                        </div>

                        <div className="app-surface-muted flex items-center justify-between rounded-md p-3">
                            <span className="text-sm font-medium">
                                {theme === "dark" ? "Dark mode" : "Light mode"}
                            </span>
                            <button
                                type="button"
                                role="switch"
                                aria-checked={theme === "dark"}
                                onClick={() => setTheme((current) => current === "dark" ? "light" : "dark")}
                                className="relative inline-flex h-6 w-11 items-center rounded-full transition-colors"
                                style={{
                                    background: theme === "dark" ? "var(--color-primary)" : "color-mix(in srgb, var(--color-primary) 36%, var(--color-surface))",
                                }}
                            >
                                <span
                                    className={`inline-block h-4 w-4 transform rounded-full transition-transform ${theme === "dark" ? "translate-x-6" : "translate-x-1"}`}
                                    style={{ background: "var(--color-background)" }}
                                />
                            </button>
                        </div>

                        <div className="app-surface-muted mt-4 rounded-md p-3">
                            <label className="block">
                                <span className="mb-2 block text-sm font-medium">
                                    OpenAI API key
                                </span>
                                <input
                                    type="password"
                                    value={openAIApiKey}
                                    onChange={(event) => {
                                        setOpenAIApiKey(event.target.value);
                                        setApiKeySaved(false);
                                    }}
                                    placeholder="sk-..."
                                    className="app-input w-full rounded-xl px-3 py-2.5 text-sm transition"
                                />
                            </label>
                            <p className="app-text-muted mt-2 text-xs leading-5">
                                Saved locally in this browser so the chat rail can call OpenAI directly from the app.
                            </p>
                            <div className="mt-3 flex items-center justify-between gap-3">
                                <button
                                    type="button"
                                    onClick={() => {
                                        saveOpenAIApiKey(openAIApiKey);
                                        setOpenAIApiKey(getStoredOpenAIApiKey());
                                        setApiKeySaved(Boolean(getStoredOpenAIApiKey()));
                                    }}
                                    className="app-button-primary rounded-full px-4 py-2 text-sm font-medium transition"
                                >
                                    Save API key
                                </button>
                                <button
                                    type="button"
                                    onClick={() => {
                                        saveOpenAIApiKey("");
                                        setOpenAIApiKey("");
                                        setApiKeySaved(false);
                                    }}
                                    className="app-button-secondary rounded-full px-4 py-2 text-sm font-medium transition"
                                >
                                    Clear
                                </button>
                            </div>
                            {apiKeySaved ? (
                                <p className="mt-2 text-xs" style={{ color: "var(--color-success)" }}>
                                    API key saved locally.
                                </p>
                            ) : null}
                        </div>
                    </div>
                </div>
            )}
        </>
    );
}
