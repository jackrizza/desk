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

type ProjectFormState = {
    name: string;
    description: string;
    symbols: string;
    interval: string;
    range: string;
    prepost: boolean;
};

const emptyProjectForm: ProjectFormState = {
    name: "",
    description: "",
    symbols: "",
    interval: "1d",
    range: "1mo",
    prepost: false,
};

function makeEntityId() {
    const randomPart = Math.random().toString(36).slice(2, 8);
    return `project-${Date.now().toString(36)}-${randomPart}`;
}

function makeTimestamp() {
    return new Date().toISOString();
}

function toProjectPayload(form: ProjectFormState): Project {
    const timestamp = makeTimestamp();

    return {
        id: makeEntityId(),
        name: form.name.trim(),
        description: form.description.trim(),
        strategy: "",
        created_at: timestamp,
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

export function LeftSidebar({
    open,
}: {
    open: boolean;
}) {
    const [projectsOpen, setProjectsOpen] = useState(false);
    const [projects, setProjects] = useState<Project[]>([]);
    const [projectModalOpen, setProjectModalOpen] = useState(false);
    const [projectForm, setProjectForm] = useState<ProjectFormState>(emptyProjectForm);
    const [projectError, setProjectError] = useState("");
    const [projectPending, setProjectPending] = useState(false);
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

    async function handleCreateProject(event: React.FormEvent<HTMLFormElement>) {
        event.preventDefault();
        setProjectError("");
        setProjectPending(true);

        try {
            await deskApi.createProject(toProjectPayload(projectForm));
            const nextProjects = await deskApi.listProjects();
            setProjects(nextProjects);
            setProjectModalOpen(false);
            setProjectForm(emptyProjectForm);
        } catch (error) {
            setProjectError(
                error instanceof Error ? error.message : "Failed to create project.",
            );
        } finally {
            setProjectPending(false);
        }
    }

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
                                    onClick={() => {
                                        setProjectError("");
                                        setProjectModalOpen(true);
                                    }}
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

            {projectModalOpen && (
                <div className="app-modal-backdrop fixed inset-0 z-30 flex items-center justify-center p-4">
                    <div className="app-surface w-full max-w-xl rounded-3xl p-5 shadow-lg">
                        <div className="mb-4 flex items-center justify-between gap-3">
                            <div>
                                <p className="app-text-muted text-xs uppercase tracking-[0.2em]">Projects</p>
                                <h2 className="mt-2 text-xl font-semibold">Create project</h2>
                            </div>
                            <button
                                type="button"
                                onClick={() => {
                                    setProjectModalOpen(false);
                                    setProjectForm(emptyProjectForm);
                                    setProjectError("");
                                }}
                                className="app-nav-link app-text-muted rounded px-2 py-1"
                                aria-label="Close project modal"
                            >
                                x
                            </button>
                        </div>

                        {projectError ? (
                            <div className="app-alert-error mb-4 rounded-2xl px-4 py-3 text-sm">
                                {projectError}
                            </div>
                        ) : null}

                        <form className="space-y-3" onSubmit={handleCreateProject}>
                            <label className="block">
                                <span className="mb-2 block text-sm font-medium">Name</span>
                                <input
                                    value={projectForm.name}
                                    onChange={(event) =>
                                        setProjectForm((current) => ({ ...current, name: event.target.value }))
                                    }
                                    className="app-input w-full rounded-2xl px-3 py-2.5 text-sm"
                                    placeholder="Momentum swing workspace"
                                    required
                                />
                            </label>

                            <label className="block">
                                <span className="mb-2 block text-sm font-medium">Description</span>
                                <textarea
                                    value={projectForm.description}
                                    onChange={(event) =>
                                        setProjectForm((current) => ({ ...current, description: event.target.value }))
                                    }
                                    rows={3}
                                    className="app-input w-full rounded-2xl px-3 py-2.5 text-sm"
                                    placeholder="Describe the market behavior or setup this project is for."
                                    required
                                />
                            </label>

                            <label className="block">
                                <span className="mb-2 block text-sm font-medium">Symbols</span>
                                <input
                                    value={projectForm.symbols}
                                    onChange={(event) =>
                                        setProjectForm((current) => ({ ...current, symbols: event.target.value }))
                                    }
                                    className="app-input w-full rounded-2xl px-3 py-2.5 text-sm"
                                    placeholder="AAPL, MSFT, NVDA"
                                />
                            </label>

                            <div className="grid gap-3 sm:grid-cols-2">
                                <label className="block">
                                    <span className="mb-2 block text-sm font-medium">Interval</span>
                                    <select
                                        value={projectForm.interval}
                                        onChange={(event) =>
                                            setProjectForm((current) => ({ ...current, interval: event.target.value }))
                                        }
                                        className="app-input w-full rounded-2xl px-3 py-2.5 text-sm"
                                    >
                                        <option value="1m">1m</option>
                                        <option value="5m">5m</option>
                                        <option value="15m">15m</option>
                                        <option value="30m">30m</option>
                                        <option value="1h">1h</option>
                                        <option value="1d">1d</option>
                                        <option value="1wk">1wk</option>
                                    </select>
                                </label>

                                <label className="block">
                                    <span className="mb-2 block text-sm font-medium">Range</span>
                                    <select
                                        value={projectForm.range}
                                        onChange={(event) =>
                                            setProjectForm((current) => ({ ...current, range: event.target.value }))
                                        }
                                        className="app-input w-full rounded-2xl px-3 py-2.5 text-sm"
                                    >
                                        <option value="1d">1d</option>
                                        <option value="5d">5d</option>
                                        <option value="1mo">1mo</option>
                                        <option value="3mo">3mo</option>
                                        <option value="6mo">6mo</option>
                                        <option value="1y">1y</option>
                                        <option value="2y">2y</option>
                                    </select>
                                </label>
                            </div>

                            <label className="flex items-center gap-3 rounded-2xl px-1 py-2 text-sm">
                                <input
                                    type="checkbox"
                                    checked={projectForm.prepost}
                                    onChange={(event) =>
                                        setProjectForm((current) => ({ ...current, prepost: event.target.checked }))
                                    }
                                />
                                Include pre/post market data
                            </label>

                            <div className="flex justify-end gap-3 pt-2">
                                <button
                                    type="button"
                                    onClick={() => {
                                        setProjectModalOpen(false);
                                        setProjectForm(emptyProjectForm);
                                        setProjectError("");
                                    }}
                                    className="app-button-secondary rounded-full px-4 py-2 text-sm font-medium"
                                >
                                    Cancel
                                </button>
                                <button
                                    type="submit"
                                    disabled={projectPending}
                                    className="app-button-primary rounded-full px-4 py-2 text-sm font-medium disabled:cursor-not-allowed disabled:opacity-60"
                                >
                                    {projectPending ? "Creating..." : "Create project"}
                                </button>
                            </div>
                        </form>
                    </div>
                </div>
            )}
        </>
    );
}
