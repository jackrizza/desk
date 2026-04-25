import { useEffect, useState } from "react";
import type React from "react";
import { NavLink, useNavigate } from "react-router";
import {
    deskApi,
    type CreateTraderInfoSourceRequest,
    type DataSource,
    type DataScientistProfile,
    type DataSourceType,
    type MdProfile,
    type PaperAccount,
    type Project,
    type Trader,
    type TraderFreedomLevel,
    type UserInvestorProfile,
} from "../lib/api";
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

type TraderFormState = {
    name: string;
    fundamental_perspective: string;
    freedom_level: TraderFreedomLevel;
    default_paper_account_id: string;
    openai_api_key: string;
    source_type: string;
    source_name: string;
};

type DataSourceFormState = {
    name: string;
    source_type: DataSourceType;
    url: string;
    enabled: boolean;
    poll_interval_seconds: string;
    config_json: string;
};

type InvestorProfileFormState = Omit<UserInvestorProfile, "id" | "created_at" | "updated_at">;
type MdProfileFormState = Pick<MdProfile, "name" | "persona" | "tone" | "communication_style">;
type DataScientistProfileFormState = Pick<DataScientistProfile, "name" | "persona" | "tone" | "communication_style">;

const emptyProjectForm: ProjectFormState = {
    name: "",
    description: "",
    symbols: "",
    interval: "1d",
    range: "1mo",
    prepost: false,
};

const emptyTraderForm: TraderFormState = {
    name: "",
    fundamental_perspective: "",
    freedom_level: "analyst",
    default_paper_account_id: "",
    openai_api_key: "",
    source_type: "placeholder",
    source_name: "",
};

const emptyDataSourceForm: DataSourceFormState = {
    name: "",
    source_type: "rss",
    url: "",
    enabled: true,
    poll_interval_seconds: "30",
    config_json: "",
};

const emptyInvestorProfileForm: InvestorProfileFormState = {
    name: "",
    age: null,
    about: "",
    investment_goals: "",
    risk_tolerance: "",
    time_horizon: "",
    liquidity_needs: "",
    income_needs: "",
    investment_experience: "",
    restrictions: "",
    preferred_sectors: "",
    avoided_sectors: "",
    notes: "",
};

const emptyMdProfileForm: MdProfileFormState = {
    name: "MD",
    persona: "",
    tone: "",
    communication_style: "",
};

const emptyDataScientistProfileForm: DataScientistProfileFormState = {
    name: "Data Scientist",
    persona: "",
    tone: "",
    communication_style: "",
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
        strategy_json: "{}",
        strategy_status: "draft",
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
    const [tradersOpen, setTradersOpen] = useState(false);
    const [traders, setTraders] = useState<Trader[]>([]);
    const [paperAccounts, setPaperAccounts] = useState<PaperAccount[]>([]);
    const [traderModalOpen, setTraderModalOpen] = useState(false);
    const [traderForm, setTraderForm] = useState<TraderFormState>(emptyTraderForm);
    const [traderError, setTraderError] = useState("");
    const [traderPending, setTraderPending] = useState(false);
    const [dataSourcesOpen, setDataSourcesOpen] = useState(false);
    const [dataSources, setDataSources] = useState<DataSource[]>([]);
    const [dataSourceModalOpen, setDataSourceModalOpen] = useState(false);
    const [dataSourceForm, setDataSourceForm] = useState<DataSourceFormState>(emptyDataSourceForm);
    const [dataSourceError, setDataSourceError] = useState("");
    const [dataSourcePending, setDataSourcePending] = useState(false);
    const [settingsOpen, setSettingsOpen] = useState(false);
    const [theme, setTheme] = useState<Theme>("light");
    const [openAIApiKey, setOpenAIApiKey] = useState("");
    const [apiKeySaved, setApiKeySaved] = useState(false);
    const [investorProfileForm, setInvestorProfileForm] = useState<InvestorProfileFormState>(emptyInvestorProfileForm);
    const [mdProfileForm, setMdProfileForm] = useState<MdProfileFormState>(emptyMdProfileForm);
    const [dataScientistProfileForm, setDataScientistProfileForm] = useState<DataScientistProfileFormState>(emptyDataScientistProfileForm);
    const [settingsMessage, setSettingsMessage] = useState("");
    const [settingsError, setSettingsError] = useState("");
    const [settingsPending, setSettingsPending] = useState(false);
    const [mdOpenAIApiKey, setMdOpenAIApiKey] = useState("");
    const [dataScientistOpenAIApiKey, setDataScientistOpenAIApiKey] = useState("");
    const [clearChannelsPending, setClearChannelsPending] = useState(false);
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
        if (!settingsOpen) {
            return;
        }
        setSettingsError("");
        setSettingsMessage("");
        Promise.all([
            deskApi.getInvestorProfile(),
            deskApi.getMdProfile(),
            deskApi.getDataScientistProfile(),
        ])
            .then(([profile, mdProfile, dataScientistProfile]) => {
                setInvestorProfileForm({
                    name: profile.name ?? "",
                    age: profile.age ?? null,
                    about: profile.about ?? "",
                    investment_goals: profile.investment_goals ?? "",
                    risk_tolerance: profile.risk_tolerance ?? "",
                    time_horizon: profile.time_horizon ?? "",
                    liquidity_needs: profile.liquidity_needs ?? "",
                    income_needs: profile.income_needs ?? "",
                    investment_experience: profile.investment_experience ?? "",
                    restrictions: profile.restrictions ?? "",
                    preferred_sectors: profile.preferred_sectors ?? "",
                    avoided_sectors: profile.avoided_sectors ?? "",
                    notes: profile.notes ?? "",
                });
                setMdProfileForm({
                    name: mdProfile.name,
                    persona: mdProfile.persona,
                    tone: mdProfile.tone,
                    communication_style: mdProfile.communication_style,
                });
                setMdOpenAIApiKey("");
                setDataScientistProfileForm({
                    name: dataScientistProfile.name,
                    persona: dataScientistProfile.persona,
                    tone: dataScientistProfile.tone,
                    communication_style: dataScientistProfile.communication_style,
                });
                setDataScientistOpenAIApiKey("");
            })
            .catch((error) => {
                setSettingsError(error instanceof Error ? error.message : "Failed to load settings.");
            });
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

    useEffect(() => {
        let cancelled = false;

        async function loadDataSources() {
            try {
                const nextDataSources = await deskApi.listDataSources();
                if (!cancelled) {
                    setDataSources(nextDataSources);
                }
            } catch {
                if (!cancelled) {
                    setDataSources([]);
                }
            }
        }

        void loadDataSources();
        const intervalId = window.setInterval(() => {
            void loadDataSources();
        }, 5000);

        return () => {
            cancelled = true;
            window.clearInterval(intervalId);
        };
    }, []);

    useEffect(() => {
        let cancelled = false;

        async function loadTraders() {
            try {
                const [nextTraders, nextPaperAccounts] = await Promise.all([
                    deskApi.listTraders(),
                    deskApi.listPaperAccounts().catch(() => []),
                ]);
                if (!cancelled) {
                    setTraders(nextTraders);
                    setPaperAccounts(nextPaperAccounts);
                }
            } catch {
                if (!cancelled) {
                    setTraders([]);
                    setPaperAccounts([]);
                }
            }
        }

        void loadTraders();
        const intervalId = window.setInterval(() => {
            void loadTraders();
        }, 5000);

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

    async function handleCreateTrader(event: React.FormEvent<HTMLFormElement>) {
        event.preventDefault();
        setTraderError("");
        setTraderPending(true);

        const infoSources: CreateTraderInfoSourceRequest[] = traderForm.source_name.trim()
            ? [
                {
                    source_type: traderForm.source_type.trim() || "placeholder",
                    name: traderForm.source_name.trim(),
                    config_json: null,
                    enabled: true,
                },
            ]
            : [];

        try {
            const created = await deskApi.createTrader({
                name: traderForm.name.trim(),
                fundamental_perspective: traderForm.fundamental_perspective.trim(),
                freedom_level: traderForm.freedom_level,
                default_paper_account_id: traderForm.default_paper_account_id || null,
                openai_api_key: traderForm.openai_api_key.trim(),
                info_sources: infoSources,
            });
            const nextTraders = await deskApi.listTraders();
            setTraders(nextTraders);
            setTraderModalOpen(false);
            setTraderForm(emptyTraderForm);
            navigate(`/traders/${encodeURIComponent(created.id)}`);
        } catch (error) {
            setTraderError(
                error instanceof Error ? error.message : "Failed to create trader.",
            );
        } finally {
            setTraderPending(false);
        }
    }

    async function handleCreateDataSource(event: React.FormEvent<HTMLFormElement>) {
        event.preventDefault();
        setDataSourceError("");
        setDataSourcePending(true);

        try {
            const created = await deskApi.createDataSource({
                name: dataSourceForm.name.trim(),
                source_type: dataSourceForm.source_type,
                url: dataSourceForm.url.trim() || null,
                enabled: dataSourceForm.enabled,
                poll_interval_seconds: Number(dataSourceForm.poll_interval_seconds) || 30,
                config_json: dataSourceForm.config_json.trim() || null,
            });
            const nextDataSources = await deskApi.listDataSources();
            setDataSources(nextDataSources);
            setDataSourceModalOpen(false);
            setDataSourceForm(emptyDataSourceForm);
            navigate(`/data-sources?source=${encodeURIComponent(created.id)}`);
        } catch (error) {
            setDataSourceError(
                error instanceof Error ? error.message : "Failed to create data source.",
            );
        } finally {
            setDataSourcePending(false);
        }
    }

    async function handleSaveProfiles() {
        setSettingsPending(true);
        setSettingsError("");
        setSettingsMessage("");
        try {
            await Promise.all([
                deskApi.updateInvestorProfile({
                    ...investorProfileForm,
                    age: investorProfileForm.age ? Number(investorProfileForm.age) : null,
                }),
                deskApi.updateMdProfile({
                    ...mdProfileForm,
                    ...(mdOpenAIApiKey.trim() ? { openai_api_key: mdOpenAIApiKey.trim() } : {}),
                }),
                deskApi.updateDataScientistProfile({
                    ...dataScientistProfileForm,
                    ...(dataScientistOpenAIApiKey.trim()
                        ? { openai_api_key: dataScientistOpenAIApiKey.trim() }
                        : {}),
                }),
            ]);
            setMdOpenAIApiKey("");
            setDataScientistOpenAIApiKey("");
            setSettingsMessage("Profiles saved.");
        } catch (error) {
            setSettingsError(error instanceof Error ? error.message : "Failed to save profiles.");
        } finally {
            setSettingsPending(false);
        }
    }

    async function handleClearChannels() {
        const confirmed = window.confirm(
            "Clear all channel messages? This keeps the channels, but permanently removes the chat history.",
        );
        if (!confirmed) {
            return;
        }
        setClearChannelsPending(true);
        setSettingsError("");
        setSettingsMessage("");
        try {
            await deskApi.clearChannelMessages();
            setSettingsMessage("Channel messages cleared.");
        } catch (error) {
            setSettingsError(error instanceof Error ? error.message : "Failed to clear channel messages.");
        } finally {
            setClearChannelsPending(false);
        }
    }

    return (
        <>
            <div className={`app-surface fixed top-0 left-0 z-10 h-screen w-64 border-r pt-16 transition-transform duration-200 ${open ? "translate-x-0" : "-translate-x-full"}`}>
                <nav className="flex flex-col space-y-1 p-4">
                    <NavLink
                        to="/market/AAPL"
                        className={({ isActive }) =>
                            `app-nav-link rounded-md px-3 py-2 ${isActive ? "font-medium" : ""}`
                        }
                    >
                        Market
                    </NavLink>
                    <NavLink
                        to="/"
                        className={({ isActive }) =>
                            `app-nav-link rounded-md px-3 py-2 ${isActive ? "font-medium" : ""}`
                        }
                    >
                        Portfolio
                    </NavLink>
                    <NavLink
                        to="/channels"
                        className={({ isActive }) =>
                            `app-nav-link rounded-md px-3 py-2 ${isActive ? "font-medium" : ""}`
                        }
                    >
                        Channels
                    </NavLink>
                    <div>
                        <button
                            onClick={() => setTradersOpen((openNow) => !openNow)}
                            className="app-nav-link flex w-full items-center justify-between rounded-md px-3 py-2"
                        >
                            <span>Traders</span>
                            <svg
                                className={`w-4 h-4 transition-transform duration-200 ${tradersOpen ? "rotate-180" : ""}`}
                                fill="none" stroke="currentColor" viewBox="0 0 24 24"
                            >
                                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 9l-7 7-7-7" />
                            </svg>
                        </button>
                        {tradersOpen && (
                            <div className="flex flex-col mt-1 ml-4 space-y-1">
                                <button
                                    type="button"
                                    onClick={() => {
                                        setTraderError("");
                                        setTraderForm(emptyTraderForm);
                                        setTraderModalOpen(true);
                                    }}
                                    className="app-nav-link app-subnav-create rounded-md px-3 py-2 text-left"
                                >
                                    Create trader
                                </button>
                                {traders.map((trader) => (
                                    <button
                                        key={trader.id}
                                        type="button"
                                        onClick={() => navigate(`/traders/${encodeURIComponent(trader.id)}`)}
                                        className="app-nav-link app-text-muted rounded-md px-3 py-2 text-left"
                                        title={`${trader.name} - ${formatFreedomLevel(trader.freedom_level)} - ${trader.status}`}
                                    >
                                        <span className="block truncate">{trader.name}</span>
                                        <span className="app-text-muted block truncate text-xs">
                                            {formatFreedomLevel(trader.freedom_level)} - {trader.status}
                                        </span>
                                    </button>
                                ))}
                                {traders.length === 0 ? (
                                    <div className="app-text-muted rounded-md px-3 py-2 text-sm">
                                        No traders yet
                                    </div>
                                ) : null}
                            </div>
                        )}
                    </div>

                    <div>
                        <button
                            onClick={() => setDataSourcesOpen((openNow) => !openNow)}
                            className="app-nav-link flex w-full items-center justify-between rounded-md px-3 py-2"
                        >
                            <span>Data Sources</span>
                            <svg
                                className={`w-4 h-4 transition-transform duration-200 ${dataSourcesOpen ? "rotate-180" : ""}`}
                                fill="none" stroke="currentColor" viewBox="0 0 24 24"
                            >
                                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 9l-7 7-7-7" />
                            </svg>
                        </button>
                        {dataSourcesOpen && (
                            <div className="flex flex-col mt-1 ml-4 space-y-1">
                                <button
                                    type="button"
                                    onClick={() => {
                                        setDataSourceError("");
                                        setDataSourceForm(emptyDataSourceForm);
                                        setDataSourceModalOpen(true);
                                    }}
                                    className="app-nav-link app-subnav-create rounded-md px-3 py-2 text-left"
                                >
                                    Create data source
                                </button>
                                {dataSources.map((source) => (
                                    <button
                                        key={source.id}
                                        type="button"
                                        onClick={() => navigate(`/data-sources?source=${encodeURIComponent(source.id)}`)}
                                        className="app-nav-link app-text-muted rounded-md px-3 py-2 text-left"
                                        title={`${source.name} - ${formatDataSourceType(source.source_type)}`}
                                    >
                                        <span className="block truncate">{source.name}</span>
                                        <span className="app-text-muted block truncate text-xs">
                                            {formatDataSourceType(source.source_type)}
                                        </span>
                                    </button>
                                ))}
                                {dataSources.length === 0 ? (
                                    <div className="app-text-muted rounded-md px-3 py-2 text-sm">
                                        No data sources yet
                                    </div>
                                ) : null}
                            </div>
                        )}
                    </div>

                    <div>
                        <button
                            onClick={() => setProjectsOpen((o) => !o)}
                            className="app-nav-link flex w-full items-center justify-between rounded-md px-3 py-2"
                        >
                            <span>Strategies</span>
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
                                    className="app-nav-link app-subnav-create rounded-md px-3 py-2 text-left"
                                >
                                    Create strategy
                                </button>
                                {projects.map((project) => (
                                    <button
                                        key={project.id}
                                        type="button"
                                        onClick={() => navigate(`/strategies/${encodeURIComponent(project.id)}`)}
                                        className="app-nav-link app-text-muted rounded-md px-3 py-2 text-left"
                                        title={project.name}
                                    >
                                        {project.name}
                                    </button>
                                ))}
                                {projects.length === 0 ? (
                                    <div className="app-text-muted rounded-md px-3 py-2 text-sm">
                                        No strategies yet
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
                    <div className="app-surface max-h-[90vh] w-full max-w-3xl overflow-auto rounded-lg p-5 shadow-lg">
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
                                Kept only for this browser session so Desk chat can call OpenAI directly from the app.
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

                        <div className="app-surface-muted mt-4 rounded-md p-3">
                            <h3 className="mb-3 text-sm font-semibold">Investor Profile</h3>
                            <div className="grid gap-3 sm:grid-cols-2">
                                <SettingsInput label="Name" value={investorProfileForm.name ?? ""} onChange={(value) => setInvestorProfileForm((current) => ({ ...current, name: value }))} />
                                <SettingsInput label="Age" value={investorProfileForm.age?.toString() ?? ""} onChange={(value) => setInvestorProfileForm((current) => ({ ...current, age: value ? Number(value) : null }))} />
                                <SettingsInput label="Risk tolerance" value={investorProfileForm.risk_tolerance ?? ""} onChange={(value) => setInvestorProfileForm((current) => ({ ...current, risk_tolerance: value }))} />
                                <SettingsInput label="Time horizon" value={investorProfileForm.time_horizon ?? ""} onChange={(value) => setInvestorProfileForm((current) => ({ ...current, time_horizon: value }))} />
                                <SettingsInput label="Liquidity needs" value={investorProfileForm.liquidity_needs ?? ""} onChange={(value) => setInvestorProfileForm((current) => ({ ...current, liquidity_needs: value }))} />
                                <SettingsInput label="Income needs" value={investorProfileForm.income_needs ?? ""} onChange={(value) => setInvestorProfileForm((current) => ({ ...current, income_needs: value }))} />
                                <SettingsInput label="Investment experience" value={investorProfileForm.investment_experience ?? ""} onChange={(value) => setInvestorProfileForm((current) => ({ ...current, investment_experience: value }))} />
                                <SettingsInput label="Preferred sectors" value={investorProfileForm.preferred_sectors ?? ""} onChange={(value) => setInvestorProfileForm((current) => ({ ...current, preferred_sectors: value }))} />
                            </div>
                            <div className="mt-3 grid gap-3">
                                <SettingsTextarea label="About" value={investorProfileForm.about ?? ""} onChange={(value) => setInvestorProfileForm((current) => ({ ...current, about: value }))} />
                                <SettingsTextarea label="Investment goals" value={investorProfileForm.investment_goals ?? ""} onChange={(value) => setInvestorProfileForm((current) => ({ ...current, investment_goals: value }))} />
                                <SettingsTextarea label="Restrictions" value={investorProfileForm.restrictions ?? ""} onChange={(value) => setInvestorProfileForm((current) => ({ ...current, restrictions: value }))} />
                                <SettingsTextarea label="Avoided sectors" value={investorProfileForm.avoided_sectors ?? ""} onChange={(value) => setInvestorProfileForm((current) => ({ ...current, avoided_sectors: value }))} />
                                <SettingsTextarea label="Notes" value={investorProfileForm.notes ?? ""} onChange={(value) => setInvestorProfileForm((current) => ({ ...current, notes: value }))} />
                            </div>
                        </div>

                        <div className="app-surface-muted mt-4 rounded-md p-3">
                            <h3 className="mb-3 text-sm font-semibold">MD Profile</h3>
                            <div className="grid gap-3">
                                <SettingsInput label="MD name" value={mdProfileForm.name} onChange={(value) => setMdProfileForm((current) => ({ ...current, name: value }))} />
                                <SettingsTextarea label="Persona" value={mdProfileForm.persona} onChange={(value) => setMdProfileForm((current) => ({ ...current, persona: value }))} />
                                <SettingsTextarea label="Tone" value={mdProfileForm.tone} onChange={(value) => setMdProfileForm((current) => ({ ...current, tone: value }))} />
                                <SettingsTextarea label="Communication style" value={mdProfileForm.communication_style} onChange={(value) => setMdProfileForm((current) => ({ ...current, communication_style: value }))} />
                                <SettingsInput label="OpenAI API key" type="password" value={mdOpenAIApiKey} onChange={setMdOpenAIApiKey} placeholder="Leave blank to keep saved key" />
                            </div>
                        </div>

                        <div className="app-surface-muted mt-4 rounded-md p-3">
                            <h3 className="mb-3 text-sm font-semibold">Data Scientist Profile</h3>
                            <div className="grid gap-3">
                                <SettingsInput label="Name" value={dataScientistProfileForm.name} onChange={(value) => setDataScientistProfileForm((current) => ({ ...current, name: value }))} />
                                <SettingsTextarea label="Persona" value={dataScientistProfileForm.persona} onChange={(value) => setDataScientistProfileForm((current) => ({ ...current, persona: value }))} />
                                <SettingsTextarea label="Tone" value={dataScientistProfileForm.tone} onChange={(value) => setDataScientistProfileForm((current) => ({ ...current, tone: value }))} />
                                <SettingsTextarea label="Communication style" value={dataScientistProfileForm.communication_style} onChange={(value) => setDataScientistProfileForm((current) => ({ ...current, communication_style: value }))} />
                                <SettingsInput label="OpenAI API key" type="password" value={dataScientistOpenAIApiKey} onChange={setDataScientistOpenAIApiKey} placeholder="Leave blank to keep saved key" />
                            </div>
                        </div>

                        <div className="app-surface-muted mt-4 rounded-md p-3">
                            <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
                                <div>
                                    <h3 className="text-sm font-semibold">Danger Zone</h3>
                                    <p className="app-text-muted mt-1 text-xs leading-5">
                                        Permanently remove all channel messages while keeping #general, #data_analysis, and #trading.
                                    </p>
                                </div>
                                <button
                                    type="button"
                                    onClick={handleClearChannels}
                                    disabled={clearChannelsPending}
                                    className="app-button-danger rounded-full px-4 py-2 text-sm font-medium disabled:cursor-not-allowed disabled:opacity-60"
                                >
                                    {clearChannelsPending ? "Clearing..." : "Clear all channels"}
                                </button>
                            </div>
                        </div>

                        {settingsError ? <div className="app-alert-error mt-4 rounded-2xl px-4 py-3 text-sm">{settingsError}</div> : null}
                        {settingsMessage ? <div className="app-alert-success mt-4 rounded-2xl px-4 py-3 text-sm">{settingsMessage}</div> : null}
                        <div className="mt-4 flex justify-end">
                            <button
                                type="button"
                                onClick={handleSaveProfiles}
                                disabled={settingsPending}
                                className="app-button-primary rounded-full px-4 py-2 text-sm font-medium disabled:cursor-not-allowed disabled:opacity-60"
                            >
                                {settingsPending ? "Saving..." : "Save profiles"}
                            </button>
                        </div>
                    </div>
                </div>
            )}

            {traderModalOpen && (
                <div className="app-modal-backdrop fixed inset-0 z-30 flex items-center justify-center p-4">
                    <div className="app-surface max-h-[90vh] w-full max-w-2xl overflow-auto rounded-3xl p-5 shadow-lg">
                        <div className="mb-4 flex items-center justify-between gap-3">
                            <div>
                                <p className="app-text-muted text-xs uppercase tracking-[0.2em]">Traders</p>
                                <h2 className="mt-2 text-xl font-semibold">Create trader</h2>
                            </div>
                            <button
                                type="button"
                                onClick={() => {
                                    setTraderModalOpen(false);
                                    setTraderForm(emptyTraderForm);
                                    setTraderError("");
                                }}
                                className="app-nav-link app-text-muted rounded px-2 py-1"
                                aria-label="Close trader modal"
                            >
                                x
                            </button>
                        </div>

                        {traderError ? (
                            <div className="app-alert-error mb-4 rounded-2xl px-4 py-3 text-sm">
                                {traderError}
                            </div>
                        ) : null}

                        <form className="space-y-3" onSubmit={handleCreateTrader}>
                            <div className="grid gap-3 sm:grid-cols-2">
                                <label className="block">
                                    <span className="mb-2 block text-sm font-medium">Name</span>
                                    <input
                                        value={traderForm.name}
                                        onChange={(event) =>
                                            setTraderForm((current) => ({ ...current, name: event.target.value }))
                                        }
                                        className="app-input w-full rounded-2xl px-3 py-2.5 text-sm"
                                        placeholder="Macro Scout"
                                        required
                                    />
                                </label>

                                <label className="block">
                                    <span className="mb-2 block text-sm font-medium">Freedom level</span>
                                    <select
                                        value={traderForm.freedom_level}
                                        onChange={(event) =>
                                            setTraderForm((current) => ({
                                                ...current,
                                                freedom_level: event.target.value as TraderFreedomLevel,
                                            }))
                                        }
                                        className="app-input w-full rounded-2xl px-3 py-2.5 text-sm"
                                    >
                                        <option value="analyst">Analyst</option>
                                        <option value="junior_trader">Junior Trader</option>
                                        <option value="senior_trader">Senior Trader</option>
                                    </select>
                                </label>
                            </div>

                            <label className="block">
                                <span className="mb-2 block text-sm font-medium">Fundamental perspective</span>
                                <textarea
                                    value={traderForm.fundamental_perspective}
                                    onChange={(event) =>
                                        setTraderForm((current) => ({
                                            ...current,
                                            fundamental_perspective: event.target.value,
                                        }))
                                    }
                                    rows={4}
                                    className="app-input w-full rounded-2xl px-3 py-2.5 text-sm"
                                    placeholder="Describe the worldview, constraints, and decision style this trader should use."
                                    required
                                />
                            </label>

                            <div className="grid gap-3 sm:grid-cols-2">
                                <label className="block">
                                    <span className="mb-2 block text-sm font-medium">Default paper account</span>
                                    <select
                                        value={traderForm.default_paper_account_id}
                                        onChange={(event) =>
                                            setTraderForm((current) => ({
                                                ...current,
                                                default_paper_account_id: event.target.value,
                                            }))
                                        }
                                        className="app-input w-full rounded-2xl px-3 py-2.5 text-sm"
                                    >
                                        <option value="">None selected</option>
                                        {paperAccounts.map((account) => (
                                            <option key={account.id} value={account.id}>{account.name}</option>
                                        ))}
                                    </select>
                                </label>

                                <label className="block">
                                    <span className="mb-2 block text-sm font-medium">OpenAI API key</span>
                                    <input
                                        type="password"
                                        value={traderForm.openai_api_key}
                                        onChange={(event) =>
                                            setTraderForm((current) => ({
                                                ...current,
                                                openai_api_key: event.target.value,
                                            }))
                                        }
                                        className="app-input w-full rounded-2xl px-3 py-2.5 text-sm"
                                        placeholder="sk-..."
                                        required
                                    />
                                </label>
                            </div>

                            <div className="grid gap-3 sm:grid-cols-2">
                                <label className="block">
                                    <span className="mb-2 block text-sm font-medium">Info source type</span>
                                    <input
                                        value={traderForm.source_type}
                                        onChange={(event) =>
                                            setTraderForm((current) => ({ ...current, source_type: event.target.value }))
                                        }
                                        className="app-input w-full rounded-2xl px-3 py-2.5 text-sm"
                                    />
                                </label>

                                <label className="block">
                                    <span className="mb-2 block text-sm font-medium">Info source name</span>
                                    <input
                                        value={traderForm.source_name}
                                        onChange={(event) =>
                                            setTraderForm((current) => ({ ...current, source_name: event.target.value }))
                                        }
                                        className="app-input w-full rounded-2xl px-3 py-2.5 text-sm"
                                        placeholder="Future API placeholder"
                                    />
                                </label>
                            </div>

                            <p className="app-text-muted text-xs leading-5">
                                API keys are sent to OpenAPI once and are not returned to the browser after saving.
                            </p>

                            <div className="flex justify-end gap-3 pt-2">
                                <button
                                    type="button"
                                    onClick={() => {
                                        setTraderModalOpen(false);
                                        setTraderForm(emptyTraderForm);
                                        setTraderError("");
                                    }}
                                    className="app-button-secondary rounded-full px-4 py-2 text-sm font-medium"
                                >
                                    Cancel
                                </button>
                                <button
                                    type="submit"
                                    disabled={traderPending}
                                    className="app-button-primary rounded-full px-4 py-2 text-sm font-medium disabled:cursor-not-allowed disabled:opacity-60"
                                >
                                    {traderPending ? "Creating..." : "Create trader"}
                                </button>
                            </div>
                        </form>
                    </div>
                </div>
            )}

            {dataSourceModalOpen && (
                <div className="app-modal-backdrop fixed inset-0 z-30 flex items-center justify-center p-4">
                    <div className="app-surface max-h-[90vh] w-full max-w-2xl overflow-auto rounded-3xl p-5 shadow-lg">
                        <div className="mb-4 flex items-center justify-between gap-3">
                            <div>
                                <p className="app-text-muted text-xs uppercase tracking-[0.2em]">Data Sources</p>
                                <h2 className="mt-2 text-xl font-semibold">Create data source</h2>
                            </div>
                            <button
                                type="button"
                                onClick={() => {
                                    setDataSourceModalOpen(false);
                                    setDataSourceForm(emptyDataSourceForm);
                                    setDataSourceError("");
                                }}
                                className="app-nav-link app-text-muted rounded px-2 py-1"
                                aria-label="Close data source modal"
                            >
                                x
                            </button>
                        </div>

                        {dataSourceError ? (
                            <div className="app-alert-error mb-4 rounded-2xl px-4 py-3 text-sm">
                                {dataSourceError}
                            </div>
                        ) : null}

                        <DataSourceForm
                            form={dataSourceForm}
                            setForm={setDataSourceForm}
                            onSubmit={handleCreateDataSource}
                            onCancel={() => {
                                setDataSourceModalOpen(false);
                                setDataSourceForm(emptyDataSourceForm);
                                setDataSourceError("");
                            }}
                            submitLabel={dataSourcePending ? "Creating..." : "Create data source"}
                            pending={dataSourcePending}
                        />
                    </div>
                </div>
            )}

            {projectModalOpen && (
                <div className="app-modal-backdrop fixed inset-0 z-30 flex items-center justify-center p-4">
                    <div className="app-surface w-full max-w-xl rounded-3xl p-5 shadow-lg">
                        <div className="mb-4 flex items-center justify-between gap-3">
                            <div>
                                <p className="app-text-muted text-xs uppercase tracking-[0.2em]">Strategies</p>
                                <h2 className="mt-2 text-xl font-semibold">Create strategy</h2>
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
                                    {projectPending ? "Creating..." : "Create strategy"}
                                </button>
                            </div>
                        </form>
                    </div>
                </div>
            )}
        </>
    );
}

function formatFreedomLevel(level: string) {
    return level.replace("_", " ");
}

function formatDataSourceType(sourceType: string) {
    return sourceType.replace("_", " ");
}

function SettingsInput({
    label,
    value,
    onChange,
    placeholder,
    type = "text",
}: {
    label: string;
    value: string;
    onChange: (value: string) => void;
    placeholder?: string;
    type?: string;
}) {
    return (
        <label className="block">
            <span className="mb-2 block text-sm font-medium">{label}</span>
            <input
                type={type}
                value={value}
                onChange={(event) => onChange(event.target.value)}
                placeholder={placeholder}
                className="app-input w-full rounded-xl px-3 py-2.5 text-sm"
            />
        </label>
    );
}

function SettingsTextarea({
    label,
    value,
    onChange,
}: {
    label: string;
    value: string;
    onChange: (value: string) => void;
}) {
    return (
        <label className="block">
            <span className="mb-2 block text-sm font-medium">{label}</span>
            <textarea
                value={value}
                onChange={(event) => onChange(event.target.value)}
                rows={3}
                className="app-input w-full rounded-xl px-3 py-2.5 text-sm"
            />
        </label>
    );
}

function DataSourceForm({
    form,
    setForm,
    onSubmit,
    onCancel,
    submitLabel,
    pending,
}: {
    form: DataSourceFormState;
    setForm: React.Dispatch<React.SetStateAction<DataSourceFormState>>;
    onSubmit: (event: React.FormEvent<HTMLFormElement>) => void;
    onCancel: () => void;
    submitLabel: string;
    pending: boolean;
}) {
    return (
        <form className="space-y-3" onSubmit={onSubmit}>
            <div className="grid gap-3 sm:grid-cols-2">
                <label className="block">
                    <span className="mb-2 block text-sm font-medium">Name</span>
                    <input
                        value={form.name}
                        onChange={(event) =>
                            setForm((current) => ({ ...current, name: event.target.value }))
                        }
                        className="app-input w-full rounded-2xl px-3 py-2.5 text-sm"
                        required
                    />
                </label>

                <label className="block">
                    <span className="mb-2 block text-sm font-medium">Type</span>
                    <select
                        value={form.source_type}
                        onChange={(event) =>
                            setForm((current) => ({
                                ...current,
                                source_type: event.target.value as DataSourceType,
                            }))
                        }
                        className="app-input w-full rounded-2xl px-3 py-2.5 text-sm"
                    >
                        <option value="rss">RSS</option>
                        <option value="web_page">Web Page</option>
                        <option value="manual_note">Manual Note</option>
                        <option value="placeholder_api">Placeholder API</option>
                        <option value="python_script">Python Script</option>
                    </select>
                </label>
            </div>

            <div className="grid gap-3 sm:grid-cols-2">
                <label className="block">
                    <span className="mb-2 block text-sm font-medium">URL</span>
                    <input
                        value={form.url}
                        onChange={(event) =>
                            setForm((current) => ({ ...current, url: event.target.value }))
                        }
                        className="app-input w-full rounded-2xl px-3 py-2.5 text-sm"
                        placeholder="https://..."
                    />
                </label>

                <label className="block">
                    <span className="mb-2 block text-sm font-medium">Poll interval seconds</span>
                    <input
                        value={form.poll_interval_seconds}
                        onChange={(event) =>
                            setForm((current) => ({
                                ...current,
                                poll_interval_seconds: event.target.value,
                            }))
                        }
                        className="app-input w-full rounded-2xl px-3 py-2.5 text-sm"
                    />
                </label>
            </div>

            <label className="flex items-center gap-3 rounded-2xl px-1 py-2 text-sm">
                <input
                    type="checkbox"
                    checked={form.enabled}
                    onChange={(event) =>
                        setForm((current) => ({ ...current, enabled: event.target.checked }))
                    }
                />
                Enabled
            </label>

            <label className="block">
                <span className="mb-2 block text-sm font-medium">Config JSON</span>
                <textarea
                    value={form.config_json}
                    onChange={(event) =>
                        setForm((current) => ({ ...current, config_json: event.target.value }))
                    }
                    rows={4}
                    className="app-input w-full rounded-2xl px-3 py-2.5 text-sm"
                />
            </label>

            <div className="flex justify-end gap-3 pt-2">
                <button
                    type="button"
                    onClick={onCancel}
                    className="app-button-secondary rounded-full px-4 py-2 text-sm font-medium"
                >
                    Cancel
                </button>
                <button
                    type="submit"
                    disabled={pending}
                    className="app-button-primary rounded-full px-4 py-2 text-sm font-medium disabled:cursor-not-allowed disabled:opacity-60"
                >
                    {submitLabel}
                </button>
            </div>
        </form>
    );
}
