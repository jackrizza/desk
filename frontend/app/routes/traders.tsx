import { useEffect, useMemo, useState } from "react";
import type React from "react";
import { useNavigate, useParams } from "react-router";
import { EmptyState, ErrorInline, LoadingInline } from "../components/AppFeedback";
import { ChatPanel } from "../components/ChatPanel";
import { LeftSidebar } from "../components/LeftSidebar";
import { Topbar } from "../components/Topbar";
import {
  deskApi,
  type CreateTraderInfoSourceRequest,
  type Trader,
  type TraderDetail,
  type TraderFreedomLevel,
  type TraderSymbol,
  type TraderSymbolAssetType,
  type TraderSymbolSource,
  type TraderSymbolStatus,
  type TraderPortfolioProposalDetail,
  type TraderTradeProposal,
  type PaperAccount,
  type DataSource,
  type TraderMemory,
  type CreateTraderMemoryRequest,
} from "../lib/api";
import { CHAT_COMMAND_EXECUTED_EVENT, useDeskChat } from "../lib/chat";
import { CHAT_OPEN_STORAGE_KEY, usePersistentBoolean } from "../lib/ui-state";

type TraderFormState = {
  name: string;
  fundamental_perspective: string;
  freedom_level: TraderFreedomLevel;
  default_paper_account_id: string;
  openai_api_key: string;
  source_type: string;
  source_name: string;
};

type SymbolFormState = {
  symbol: string;
  asset_type: TraderSymbolAssetType;
  status: TraderSymbolStatus;
  thesis: string;
  notes: string;
};

type SymbolFilters = {
  status: TraderSymbolStatus | "";
  asset_type: TraderSymbolAssetType | "";
  source: TraderSymbolSource | "";
};

type PersonaFormState = {
  persona: string;
  tone: string;
  communication_style: string;
};

type MemoryFormState = {
  id: string;
  memory_type: string;
  topic: string;
  summary: string;
  importance: number;
  confidence: string;
  status: string;
};

const emptyForm: TraderFormState = {
  name: "",
  fundamental_perspective: "",
  freedom_level: "analyst",
  default_paper_account_id: "",
  openai_api_key: "",
  source_type: "placeholder",
  source_name: "",
};

const emptySymbolForm: SymbolFormState = {
  symbol: "",
  asset_type: "stock",
  status: "watching",
  thesis: "",
  notes: "",
};

const emptyPersonaForm: PersonaFormState = {
  persona: "",
  tone: "",
  communication_style: "",
};

const emptyMemoryForm: MemoryFormState = {
  id: "",
  memory_type: "review",
  topic: "",
  summary: "",
  importance: 3,
  confidence: "",
  status: "active",
};

export default function TradersRoute() {
  const params = useParams();
  const navigate = useNavigate();
  const [sidebarOpen, setSidebarOpen] = useState(true);
  const [traders, setTraders] = useState<Trader[]>([]);
  const [paperAccounts, setPaperAccounts] = useState<PaperAccount[]>([]);
  const [detail, setDetail] = useState<TraderDetail | null>(null);
  const [proposals, setProposals] = useState<TraderTradeProposal[]>([]);
  const [traderSymbols, setTraderSymbols] = useState<TraderSymbol[]>([]);
  const [portfolioProposals, setPortfolioProposals] = useState<TraderPortfolioProposalDetail[]>([]);
  const [dataSources, setDataSources] = useState<DataSource[]>([]);
  const [assignedDataSourceIds, setAssignedDataSourceIds] = useState<string[]>([]);
  const [memories, setMemories] = useState<TraderMemory[]>([]);
  const [memoryFilters, setMemoryFilters] = useState({ status: "active", memory_type: "", topic: "" });
  const [memoryForm, setMemoryForm] = useState<MemoryFormState>(emptyMemoryForm);
  const [memoryModalOpen, setMemoryModalOpen] = useState(false);
  const [memoryPending, setMemoryPending] = useState(false);
  const [form, setForm] = useState<TraderFormState>(emptyForm);
  const [editing, setEditing] = useState(false);
  const [apiKeySaved, setApiKeySaved] = useState(false);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  const [symbolForm, setSymbolForm] = useState<SymbolFormState>(emptySymbolForm);
  const [symbolFilters, setSymbolFilters] = useState<SymbolFilters>({
    status: "",
    asset_type: "",
    source: "",
  });
  const [suggestionForm, setSuggestionForm] = useState({
    max_symbols: 15,
    include_etfs: true,
    include_stocks: true,
    focus: "",
  });
  const [symbolsPending, setSymbolsPending] = useState(false);
  const [personaForm, setPersonaForm] = useState<PersonaFormState>(emptyPersonaForm);
  const [personaMessage, setPersonaMessage] = useState("");
  const [chatOpen, setChatOpen] = usePersistentBoolean(
    CHAT_OPEN_STORAGE_KEY,
    false,
  );

  const selectedTraderId = params.traderId ?? traders[0]?.id;
  const selectedTrader = detail?.trader;
  const chat = useDeskChat({
    page: "traders",
    traderCount: traders.length,
    runningCount: traders.filter((trader) => trader.status === "running").length,
    selectedTraderName: selectedTrader?.name ?? null,
    selectedTraderStatus: selectedTrader?.status ?? null,
  });

  async function refresh(nextTraderId = selectedTraderId) {
    setError("");
    const [nextTraders, accounts, allSources] = await Promise.all([
      deskApi.listTraders(),
      deskApi.listPaperAccounts().catch(() => []),
      deskApi.listDataSources().catch(() => []),
    ]);
    setTraders(nextTraders);
    setPaperAccounts(accounts);
    setDataSources(allSources);
    const id = nextTraderId ?? nextTraders[0]?.id;
    if (id) {
      const [nextDetail, nextProposals, nextPortfolioProposals, assignedSources, nextMemories] = await Promise.all([
        deskApi.getTrader(id),
        deskApi.getTraderTradeProposals(id).catch(() => ({ proposals: [] })),
        deskApi.listTraderProposals(id).catch(() => ({ proposals: [] })),
        deskApi.getTraderDataSources(id).catch(() => ({ data_sources: [] })),
        deskApi.listTraderMemories(id, memoryFilters).catch(() => []),
      ]);
      setDetail(nextDetail);
      setTraderSymbols(nextDetail.tracked_symbols ?? []);
      setProposals(nextProposals.proposals);
      setPortfolioProposals(nextPortfolioProposals.proposals);
      setAssignedDataSourceIds(assignedSources.data_sources.map((source) => source.id));
      setMemories(nextMemories);
      if (!params.traderId) {
        navigate(`/traders/${encodeURIComponent(id)}`, { replace: true });
      }
    } else {
      setDetail(null);
      setProposals([]);
      setTraderSymbols([]);
      setPortfolioProposals([]);
      setMemories([]);
    }
  }

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    refresh(params.traderId)
      .catch((err) => {
        if (!cancelled) {
          setError(err instanceof Error ? err.message : "Failed to load traders.");
        }
      })
      .finally(() => {
        if (!cancelled) {
          setLoading(false);
        }
      });
    const intervalId = window.setInterval(() => {
      void refresh(params.traderId).catch(() => undefined);
    }, 5000);
    return () => {
      cancelled = true;
      window.clearInterval(intervalId);
    };
  }, [params.traderId, memoryFilters.status, memoryFilters.memory_type, memoryFilters.topic]);

  useEffect(() => {
    const handleCommand = () => {
      void refresh(params.traderId).catch(() => undefined);
    };
    window.addEventListener(CHAT_COMMAND_EXECUTED_EVENT, handleCommand);
    return () => window.removeEventListener(CHAT_COMMAND_EXECUTED_EVENT, handleCommand);
  }, [params.traderId]);

  useEffect(() => {
    if (!selectedTrader || !editing) {
      return;
    }
    setForm({
      name: selectedTrader.name,
      fundamental_perspective: selectedTrader.fundamental_perspective,
      freedom_level: selectedTrader.freedom_level,
      default_paper_account_id: selectedTrader.default_paper_account_id ?? "",
      openai_api_key: "",
      source_type: detail?.info_sources[0]?.source_type ?? "placeholder",
      source_name: detail?.info_sources[0]?.name ?? "",
    });
    setApiKeySaved(true);
  }, [editing, selectedTrader?.id]);

  useEffect(() => {
    if (!selectedTrader) {
      setPersonaForm(emptyPersonaForm);
      return;
    }
    setPersonaForm({
      persona: selectedTrader.persona ?? "",
      tone: selectedTrader.tone ?? "",
      communication_style: selectedTrader.communication_style ?? "",
    });
    setPersonaMessage("");
  }, [selectedTrader?.id, selectedTrader?.persona, selectedTrader?.tone, selectedTrader?.communication_style]);

  const pendingProposals = useMemo(
    () => proposals.filter((proposal) => proposal.status === "pending_review"),
    [proposals],
  );

  async function handleSubmit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError("");
    const info_sources: CreateTraderInfoSourceRequest[] = form.source_name.trim()
      ? [
          {
            source_type: form.source_type.trim() || "placeholder",
            name: form.source_name.trim(),
            config_json: null,
            enabled: true,
          },
        ]
      : [];

    try {
      if (editing && selectedTrader) {
        const input = {
          name: form.name.trim(),
          fundamental_perspective: form.fundamental_perspective.trim(),
          freedom_level: form.freedom_level,
          default_paper_account_id: form.default_paper_account_id || null,
          info_sources,
          ...(form.openai_api_key.trim()
            ? { openai_api_key: form.openai_api_key.trim() }
            : {}),
        };
        await deskApi.updateTrader(selectedTrader.id, input);
        setApiKeySaved(true);
        setEditing(false);
        setForm(emptyForm);
        await refresh(selectedTrader.id);
      } else {
        const created = await deskApi.createTrader({
          name: form.name.trim(),
          fundamental_perspective: form.fundamental_perspective.trim(),
          freedom_level: form.freedom_level,
          default_paper_account_id: form.default_paper_account_id || null,
          openai_api_key: form.openai_api_key.trim(),
          info_sources,
        });
        setApiKeySaved(true);
        setForm(emptyForm);
        await refresh(created.id);
        navigate(`/traders/${encodeURIComponent(created.id)}`);
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to save trader.");
    }
  }

  async function mutateTrader(action: "start" | "stop" | "pause" | "delete") {
    if (!selectedTrader) {
      return;
    }
    setError("");
    try {
      if (action === "start") await deskApi.startTrader(selectedTrader.id);
      if (action === "stop") await deskApi.stopTrader(selectedTrader.id);
      if (action === "pause") await deskApi.pauseTrader(selectedTrader.id);
      if (action === "delete") await deskApi.deleteTrader(selectedTrader.id);
      await refresh(action === "delete" ? undefined : selectedTrader.id);
      if (action === "delete") navigate("/traders");
    } catch (err) {
      setError(err instanceof Error ? err.message : `Failed to ${action} trader.`);
    }
  }

  async function handleSavePersona(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!selectedTrader) return;
    setError("");
    setPersonaMessage("");
    try {
      await deskApi.updateTraderPersona(selectedTrader.id, {
        persona: personaForm.persona,
        tone: personaForm.tone,
        communication_style: personaForm.communication_style,
      });
      setPersonaMessage("Persona saved.");
      await refresh(selectedTrader.id);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to save persona.");
    }
  }

  function openMemoryModal(memory?: TraderMemory) {
    if (memory) {
      setMemoryForm({
        id: memory.id,
        memory_type: memory.memory_type,
        topic: memory.topic,
        summary: memory.summary,
        importance: memory.importance,
        confidence: memory.confidence == null ? "" : String(memory.confidence),
        status: memory.status,
      });
    } else {
      setMemoryForm(emptyMemoryForm);
    }
    setMemoryModalOpen(true);
  }

  async function saveMemory(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!selectedTrader) return;
    setError("");
    setMemoryPending(true);
    const payload: CreateTraderMemoryRequest = {
      memory_type: memoryForm.memory_type,
      topic: memoryForm.topic.trim(),
      summary: memoryForm.summary.trim(),
      importance: memoryForm.importance,
      confidence: memoryForm.confidence ? Number(memoryForm.confidence) : null,
    };
    try {
      if (memoryForm.id) {
        await deskApi.updateTraderMemory(selectedTrader.id, memoryForm.id, {
          ...payload,
          status: memoryForm.status,
        });
      } else {
        await deskApi.createTraderMemory(selectedTrader.id, payload);
      }
      setMemoryModalOpen(false);
      setMemoryForm(emptyMemoryForm);
      await refresh(selectedTrader.id);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to save memory.");
    } finally {
      setMemoryPending(false);
    }
  }

  async function setMemoryStatus(memory: TraderMemory, status: "active" | "archived") {
    if (!selectedTrader) return;
    setError("");
    try {
      if (status === "archived") {
        await deskApi.archiveTraderMemory(selectedTrader.id, memory.id);
      } else {
        await deskApi.updateTraderMemory(selectedTrader.id, memory.id, { status });
      }
      await refresh(selectedTrader.id);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to update memory.");
    }
  }

  async function reviewProposal(proposalId: string, action: "approve" | "reject") {
    if (!selectedTrader) return;
    setError("");
    try {
      if (action === "approve") {
        await deskApi.approveTraderTradeProposal(selectedTrader.id, proposalId);
      } else {
        await deskApi.rejectTraderTradeProposal(selectedTrader.id, proposalId);
      }
      await refresh(selectedTrader.id);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to review proposal.");
    }
  }

  async function saveTraderDataSources() {
    if (!selectedTrader) return;
    setError("");
    try {
      await deskApi.updateTraderDataSources(selectedTrader.id, assignedDataSourceIds);
      await refresh(selectedTrader.id);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to save trader data sources.");
    }
  }

  async function addTraderSymbol(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!selectedTrader) return;
    setError("");
    setSymbolsPending(true);
    try {
      await deskApi.createTraderSymbol(selectedTrader.id, {
        symbol: symbolForm.symbol,
        asset_type: symbolForm.asset_type,
        status: symbolForm.status,
        thesis: symbolForm.thesis || null,
        notes: symbolForm.notes || null,
        source: "manual",
      });
      setSymbolForm(emptySymbolForm);
      await refresh(selectedTrader.id);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to add tracked symbol.");
    } finally {
      setSymbolsPending(false);
    }
  }

  async function updateTraderSymbol(symbol: TraderSymbol, input: Partial<TraderSymbol>) {
    if (!selectedTrader) return;
    setError("");
    try {
      await deskApi.updateTraderSymbol(selectedTrader.id, symbol.id, {
        asset_type: input.asset_type,
        name: input.name,
        exchange: input.exchange,
        sector: input.sector,
        industry: input.industry,
        notes: input.notes,
        thesis: input.thesis,
        fit_score: input.fit_score,
        status: input.status,
      });
      await refresh(selectedTrader.id);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to update tracked symbol.");
    }
  }

  async function setTraderSymbolStatus(symbol: TraderSymbol, action: "activate" | "reject" | "archive") {
    if (!selectedTrader) return;
    setError("");
    try {
      if (action === "activate") await deskApi.activateTraderSymbol(selectedTrader.id, symbol.id);
      if (action === "reject") await deskApi.rejectTraderSymbol(selectedTrader.id, symbol.id);
      if (action === "archive") await deskApi.archiveTraderSymbol(selectedTrader.id, symbol.id);
      await refresh(selectedTrader.id);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to update tracked symbol.");
    }
  }

  async function suggestTraderSymbols(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!selectedTrader) return;
    setError("");
    setSymbolsPending(true);
    try {
      await deskApi.suggestTraderSymbols(selectedTrader.id, {
        max_symbols: suggestionForm.max_symbols,
        include_etfs: suggestionForm.include_etfs,
        include_stocks: suggestionForm.include_stocks,
        focus: suggestionForm.focus || null,
      });
      await refresh(selectedTrader.id);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to suggest symbols.");
    } finally {
      setSymbolsPending(false);
    }
  }

  async function reviewPortfolioProposal(proposalId: string, action: "accept" | "reject") {
    if (!selectedTrader) return;
    setError("");
    try {
      if (action === "accept") {
        await deskApi.acceptTraderProposal(selectedTrader.id, proposalId);
      } else {
        await deskApi.rejectTraderProposal(selectedTrader.id, proposalId);
      }
      await refresh(selectedTrader.id);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to review portfolio proposal.");
    }
  }

  return (
    <div className="min-h-screen app-bg">
      <Topbar
        sidebarOpen={sidebarOpen}
        onToggleSidebar={() => setSidebarOpen((open) => !open)}
        onToggleChat={() => setChatOpen((open) => !open)}
        chatOpen={chatOpen}
        quoteSymbol=""
        onQuoteSymbolChange={() => undefined}
        onQuoteLookup={() => undefined}
        quoteLoading={false}
      />
      <LeftSidebar open={sidebarOpen} />
      <main className={`pt-20 transition-all ${sidebarOpen ? "pl-64" : "pl-0"} ${chatOpen ? "lg:pr-96 xl:pr-[25vw]" : "pr-0"}`}>
        <div className="mx-auto flex max-w-7xl gap-6 px-6 pb-12">
          <aside className="w-72 shrink-0">
            <div className="app-surface rounded-lg border p-4">
              <div className="mb-4">
                <h1 className="text-xl font-semibold">Traders</h1>
                <p className="app-text-muted mt-1 text-sm">
                  Create traders from the left sidebar.
                </p>
              </div>
              {traders.map((trader) => (
                <button
                  key={trader.id}
                  className="app-nav-link mb-1 flex w-full items-center justify-between rounded-md px-3 py-2 text-left"
                  onClick={() => navigate(`/traders/${encodeURIComponent(trader.id)}`)}
                >
                  <span className="truncate">{trader.name}</span>
                  <TraderStatusBadge status={trader.status} />
                </button>
              ))}
              {!loading && traders.length === 0 ? <EmptyState message="No traders yet" /> : null}
            </div>
          </aside>

          <section className="min-w-0 flex-1 space-y-5">
            {loading ? <LoadingInline message="Loading traders" /> : null}
            {error ? <ErrorInline message={error} /> : null}

            {selectedTrader ? (
              <div className="app-surface rounded-lg border p-5">
                <div className="flex flex-wrap items-start justify-between gap-4">
                  <div>
                    <div className="flex items-center gap-3">
                      <h2 className="text-2xl font-semibold">{selectedTrader.name}</h2>
                      <TraderStatusBadge status={selectedTrader.status} />
                    </div>
                    <p className="app-text-muted mt-2 max-w-3xl text-sm leading-6">
                      {selectedTrader.fundamental_perspective}
                    </p>
                  </div>
                  <div className="flex flex-wrap gap-2">
                    <button className="app-button-primary rounded-full px-4 py-2 text-sm" onClick={() => mutateTrader("start")}>Start</button>
                    <button className="app-button-secondary rounded-full px-4 py-2 text-sm" onClick={() => mutateTrader("pause")}>Pause</button>
                    <button className="app-button-secondary rounded-full px-4 py-2 text-sm" onClick={() => mutateTrader("stop")}>Stop</button>
                    <button className="app-button-secondary rounded-full px-4 py-2 text-sm" onClick={() => setEditing(true)}>Edit</button>
                    <button className="app-button-secondary rounded-full px-4 py-2 text-sm" onClick={() => mutateTrader("delete")}>Delete</button>
                  </div>
                </div>
                <div className="mt-5 grid gap-4 md:grid-cols-3">
                  <Metric label="Freedom" value={formatFreedom(selectedTrader.freedom_level)} />
                  <Metric label="Paper account" value={accountName(paperAccounts, selectedTrader.default_paper_account_id)} />
                  <Metric label="API key" value={apiKeySaved ? "API key saved" : "Saved server-side"} />
                </div>
              </div>
            ) : null}

            {selectedTrader ? (
              <form className="app-surface rounded-lg border p-5" onSubmit={handleSavePersona}>
                <div className="flex flex-wrap items-center justify-between gap-3">
                  <h2 className="text-lg font-semibold">Persona & Tone</h2>
                  <button className="app-button-primary rounded-full px-4 py-2 text-sm" type="submit">
                    Save persona
                  </button>
                </div>
                <div className="mt-4 grid gap-4">
                  <Field label="Persona">
                    <textarea className="app-input w-full rounded-lg px-3 py-2" rows={3} value={personaForm.persona} onChange={(event) => setPersonaForm((current) => ({ ...current, persona: event.target.value }))} />
                  </Field>
                  <Field label="Tone">
                    <textarea className="app-input w-full rounded-lg px-3 py-2" rows={2} value={personaForm.tone} onChange={(event) => setPersonaForm((current) => ({ ...current, tone: event.target.value }))} />
                  </Field>
                  <Field label="Communication style">
                    <textarea className="app-input w-full rounded-lg px-3 py-2" rows={3} value={personaForm.communication_style} onChange={(event) => setPersonaForm((current) => ({ ...current, communication_style: event.target.value }))} />
                  </Field>
                </div>
                {personaMessage ? <p className="mt-3 text-sm" style={{ color: "var(--color-success)" }}>{personaMessage}</p> : null}
              </form>
            ) : null}

            {editing && selectedTrader ? (
              <TraderForm
                form={form}
                setForm={setForm}
                editing={editing}
                paperAccounts={paperAccounts}
                onSubmit={handleSubmit}
                onCancel={() => {
                  setEditing(false);
                  setForm(emptyForm);
                }}
              />
            ) : null}

            {detail ? (
              <div className="grid gap-5 lg:grid-cols-2">
                <Panel title="Runtime">
                  <RuntimePanel detail={detail} />
                </Panel>
                <Panel title="Pending Proposals">
                  <ProposalsPanel proposals={pendingProposals} onReview={reviewProposal} />
                </Panel>
                <div className="lg:col-span-2">
                  <Panel title="Proposals">
                    <PortfolioProposalsPanel
                      proposals={portfolioProposals}
                      onReview={reviewPortfolioProposal}
                    />
                  </Panel>
                </div>
                <Panel title="Recent Events">
                  <EventsPanel detail={detail} />
                </Panel>
                <Panel title="Information Sources">
                  <SourcesPanel detail={detail} />
                </Panel>
                <Panel title="Data Sources">
                  <TraderDataSourcesPanel
                    dataSources={dataSources}
                    assignedIds={assignedDataSourceIds}
                    setAssignedIds={setAssignedDataSourceIds}
                    onSave={saveTraderDataSources}
                  />
                </Panel>
                <div className="lg:col-span-2">
                  <Panel title="Memories">
                    <TraderMemoriesPanel
                      memories={memories}
                      filters={memoryFilters}
                      setFilters={setMemoryFilters}
                      onAdd={() => openMemoryModal()}
                      onEdit={openMemoryModal}
                      onArchive={(memory) => setMemoryStatus(memory, "archived")}
                      onRestore={(memory) => setMemoryStatus(memory, "active")}
                    />
                  </Panel>
                </div>
                <div className="lg:col-span-2">
                  <Panel title="Tracked Symbols">
                    <TraderSymbolsPanel
                      symbols={traderSymbols}
                      filters={symbolFilters}
                      setFilters={setSymbolFilters}
                      form={symbolForm}
                      setForm={setSymbolForm}
                      suggestionForm={suggestionForm}
                      setSuggestionForm={setSuggestionForm}
                      pending={symbolsPending}
                      onAdd={addTraderSymbol}
                      onUpdate={updateTraderSymbol}
                      onStatus={setTraderSymbolStatus}
                      onSuggest={suggestTraderSymbols}
                    />
                  </Panel>
                </div>
              </div>
            ) : null}
          </section>
        </div>
      </main>
      <ChatPanel
        open={chatOpen}
        title="Trader assistant"
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
      {memoryModalOpen ? (
        <div className="app-modal-backdrop fixed inset-0 z-40 flex items-center justify-center p-4">
          <form className="app-surface max-h-[90vh] w-full max-w-2xl overflow-auto rounded-lg p-5 shadow-lg" onSubmit={saveMemory}>
            <div className="mb-4 flex items-center justify-between gap-3">
              <h2 className="text-lg font-semibold">{memoryForm.id ? "Edit Memory" : "Add Memory"}</h2>
              <button type="button" className="app-nav-link rounded px-2 py-1" onClick={() => setMemoryModalOpen(false)}>x</button>
            </div>
            <div className="grid gap-4">
              <Field label="Topic">
                <input className="app-input w-full rounded-lg px-3 py-2 text-sm" value={memoryForm.topic} onChange={(event) => setMemoryForm((current) => ({ ...current, topic: event.target.value }))} required />
              </Field>
              <Field label="Type">
                <select className="app-input w-full rounded-lg px-3 py-2 text-sm" value={memoryForm.memory_type} onChange={(event) => setMemoryForm((current) => ({ ...current, memory_type: event.target.value }))}>
                  {memoryTypes.map((type) => <option key={type} value={type}>{type}</option>)}
                </select>
              </Field>
              <Field label="Summary">
                <textarea className="app-input w-full rounded-lg px-3 py-2 text-sm" rows={5} maxLength={1000} value={memoryForm.summary} onChange={(event) => setMemoryForm((current) => ({ ...current, summary: event.target.value }))} required />
              </Field>
              <div className="grid gap-4 sm:grid-cols-3">
                <Field label="Importance">
                  <input type="number" min={1} max={5} className="app-input w-full rounded-lg px-3 py-2 text-sm" value={memoryForm.importance} onChange={(event) => setMemoryForm((current) => ({ ...current, importance: Number(event.target.value) }))} />
                </Field>
                <Field label="Confidence">
                  <input type="number" min={0} max={1} step={0.01} className="app-input w-full rounded-lg px-3 py-2 text-sm" value={memoryForm.confidence} onChange={(event) => setMemoryForm((current) => ({ ...current, confidence: event.target.value }))} />
                </Field>
                <Field label="Status">
                  <select className="app-input w-full rounded-lg px-3 py-2 text-sm" value={memoryForm.status} onChange={(event) => setMemoryForm((current) => ({ ...current, status: event.target.value }))}>
                    <option value="active">active</option>
                    <option value="archived">archived</option>
                    <option value="superseded">superseded</option>
                  </select>
                </Field>
              </div>
            </div>
            <div className="mt-5 flex justify-end gap-2">
              <button type="button" className="app-button-secondary rounded-full px-4 py-2 text-sm" onClick={() => setMemoryModalOpen(false)}>Cancel</button>
              <button className="app-button-primary rounded-full px-4 py-2 text-sm" disabled={memoryPending}>{memoryPending ? "Saving..." : "Save memory"}</button>
            </div>
          </form>
        </div>
      ) : null}
    </div>
  );
}

function TraderForm({
  form,
  setForm,
  editing,
  paperAccounts,
  onSubmit,
  onCancel,
}: {
  form: TraderFormState;
  setForm: React.Dispatch<React.SetStateAction<TraderFormState>>;
  editing: boolean;
  paperAccounts: PaperAccount[];
  onSubmit: (event: React.FormEvent<HTMLFormElement>) => void;
  onCancel: () => void;
}) {
  return (
    <form className="app-surface rounded-lg border p-5" onSubmit={onSubmit}>
      <h2 className="text-lg font-semibold">{editing ? "Edit trader" : "Create trader"}</h2>
      <div className="mt-4 grid gap-4 md:grid-cols-2">
        <Field label="Name">
          <input className="app-input w-full rounded-lg px-3 py-2" value={form.name} onChange={(event) => setForm((current) => ({ ...current, name: event.target.value }))} required />
        </Field>
        <Field label="Freedom level">
          <select className="app-input w-full rounded-lg px-3 py-2" value={form.freedom_level} onChange={(event) => setForm((current) => ({ ...current, freedom_level: event.target.value as TraderFreedomLevel }))}>
            <option value="analyst">Analyst</option>
            <option value="junior_trader">Junior Trader</option>
            <option value="senior_trader">Senior Trader</option>
          </select>
        </Field>
        <Field label="Default paper account">
          <select className="app-input w-full rounded-lg px-3 py-2" value={form.default_paper_account_id} onChange={(event) => setForm((current) => ({ ...current, default_paper_account_id: event.target.value }))}>
            <option value="">None selected</option>
            {paperAccounts.map((account) => (
              <option key={account.id} value={account.id}>{account.name}</option>
            ))}
          </select>
        </Field>
        <Field label={editing ? "Replace OpenAI API key" : "OpenAI API key"}>
          <input className="app-input w-full rounded-lg px-3 py-2" type="password" value={form.openai_api_key} onChange={(event) => setForm((current) => ({ ...current, openai_api_key: event.target.value }))} required={!editing} placeholder={editing ? "Leave blank to keep saved key" : "sk-..."} />
        </Field>
        <Field label="Info source type">
          <input className="app-input w-full rounded-lg px-3 py-2" value={form.source_type} onChange={(event) => setForm((current) => ({ ...current, source_type: event.target.value }))} />
        </Field>
        <Field label="Info source name">
          <input className="app-input w-full rounded-lg px-3 py-2" value={form.source_name} onChange={(event) => setForm((current) => ({ ...current, source_name: event.target.value }))} placeholder="Future API placeholder" />
        </Field>
      </div>
      <Field label="Fundamental perspective">
        <textarea className="app-input mt-2 w-full rounded-lg px-3 py-2" rows={4} value={form.fundamental_perspective} onChange={(event) => setForm((current) => ({ ...current, fundamental_perspective: event.target.value }))} required />
      </Field>
      <p className="app-text-muted mt-3 text-xs">API keys are write-only after save and are not returned to the browser.</p>
      <div className="mt-4 flex justify-end gap-3">
        {editing ? <button type="button" className="app-button-secondary rounded-full px-4 py-2 text-sm" onClick={onCancel}>Cancel</button> : null}
        <button className="app-button-primary rounded-full px-4 py-2 text-sm" type="submit">{editing ? "Save trader" : "Create trader"}</button>
      </div>
    </form>
  );
}

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <label className="block">
      <span className="mb-2 block text-sm font-medium">{label}</span>
      {children}
    </label>
  );
}

function Panel({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div className="app-surface rounded-lg border p-5">
      <h3 className="text-lg font-semibold">{title}</h3>
      <div className="mt-4">{children}</div>
    </div>
  );
}

function Metric({ label, value }: { label: string; value: string }) {
  return (
    <div className="app-surface-muted rounded-lg p-3">
      <div className="app-text-muted text-xs uppercase">{label}</div>
      <div className="mt-1 font-medium">{value}</div>
    </div>
  );
}

function TraderStatusBadge({ status }: { status: string }) {
  return <span className="rounded-full px-2 py-1 text-xs app-surface-muted">{status}</span>;
}

function RuntimePanel({ detail }: { detail: TraderDetail }) {
  const state = detail.runtime_state;
  if (!state) return <p className="app-text-muted text-sm">No runtime state yet.</p>;
  return (
    <dl className="space-y-2 text-sm">
      <RuntimeRow label="Engine" value={state.engine_name ?? "Unknown"} />
      <RuntimeRow label="Heartbeat" value={state.last_heartbeat_at ?? "None"} />
      <RuntimeRow label="Last evaluation" value={state.last_evaluation_at ?? "None"} />
      <RuntimeRow label="Task" value={state.current_task ?? "None"} />
      <RuntimeRow label="Last error" value={state.last_error ?? "None"} />
    </dl>
  );
}

function RuntimeRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex justify-between gap-4">
      <dt className="app-text-muted">{label}</dt>
      <dd className="min-w-0 truncate text-right">{value}</dd>
    </div>
  );
}

function ProposalsPanel({ proposals, onReview }: { proposals: TraderTradeProposal[]; onReview: (id: string, action: "approve" | "reject") => void }) {
  if (proposals.length === 0) return <p className="app-text-muted text-sm">No pending proposals.</p>;
  return (
    <div className="space-y-3">
      {proposals.map((proposal) => (
        <div key={proposal.id} className="app-surface-muted rounded-lg p-3">
          <div className="flex items-center justify-between gap-3">
            <div className="font-medium">{proposal.side.toUpperCase()} {proposal.quantity} {proposal.symbol}</div>
            <div className="flex gap-2">
              <button className="app-button-primary rounded-full px-3 py-1.5 text-xs" onClick={() => onReview(proposal.id, "approve")}>Approve</button>
              <button className="app-button-secondary rounded-full px-3 py-1.5 text-xs" onClick={() => onReview(proposal.id, "reject")}>Reject</button>
            </div>
          </div>
          <p className="app-text-muted mt-2 text-sm">{proposal.reason}</p>
        </div>
      ))}
    </div>
  );
}

function PortfolioProposalsPanel({
  proposals,
  onReview,
}: {
  proposals: TraderPortfolioProposalDetail[];
  onReview: (id: string, action: "accept" | "reject") => void;
}) {
  if (proposals.length === 0) {
    return <p className="app-text-muted text-sm">No proposals yet. The trader will generate one while running.</p>;
  }
  const activePlan = proposals.find((detail) => detail.proposal.plan_state === "active");
  const proposedChange = proposals.find((detail) => detail.proposal.status === "proposed");
  const history = proposals.filter(
    (detail) =>
      detail.proposal.id !== activePlan?.proposal.id &&
      detail.proposal.id !== proposedChange?.proposal.id,
  );
  return (
    <div className="space-y-5">
      {activePlan ? (
        <PlanCard title="Active Plan" detail={activePlan} onReview={onReview} />
      ) : (
        <p className="app-text-muted text-sm">No active plan yet. Accept a proposal to make it durable.</p>
      )}
      {activePlan ? <ProposalActionsTable actions={activePlan.actions} /> : null}
      {proposedChange ? (
        <>
          <PlanCard title="Proposed Change" detail={proposedChange} onReview={onReview} />
          <ProposalActionsTable actions={proposedChange.actions} />
        </>
      ) : null}
      {history.length > 0 ? (
        <div>
          <h4 className="mb-3 font-semibold">History</h4>
          <div className="space-y-2">
            {history.map((detail) => (
              <div key={detail.proposal.id} className="app-surface-muted rounded-lg p-3 text-sm">
                <div className="flex flex-wrap items-center justify-between gap-3">
                  <span className="font-medium">{detail.proposal.title}</span>
                  <span className="app-text-muted text-xs">{detail.proposal.status} - {detail.proposal.created_at}</span>
                </div>
                <p className="app-text-muted mt-1">{detail.proposal.summary}</p>
              </div>
            ))}
          </div>
        </div>
      ) : null}
    </div>
  );
}

function PlanCard({
  title,
  detail,
  onReview,
}: {
  title: string;
  detail: TraderPortfolioProposalDetail;
  onReview: (id: string, action: "accept" | "reject") => void;
}) {
  const conditions = parseJsonList(detail.proposal.invalidation_conditions_json);
  return (
    <div className="app-surface-muted rounded-lg p-4">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <div className="app-text-muted text-xs uppercase">{title}</div>
          <h4 className="mt-1 text-lg font-semibold">{detail.proposal.title}</h4>
          <p className="app-text-muted mt-2 text-sm">{detail.proposal.summary}</p>
        </div>
        <span className="rounded-full px-3 py-1 text-xs app-surface">
          {detail.proposal.status} / {detail.proposal.plan_state}
        </span>
      </div>
      <p className="mt-3 text-sm leading-6">{detail.proposal.thesis}</p>
      {detail.proposal.replacement_reason ? (
        <p className="app-text-muted mt-2 text-sm">Replacement reason: {detail.proposal.replacement_reason}</p>
      ) : null}
      <div className="mt-3 grid gap-3 md:grid-cols-4">
        <Metric label="Confidence" value={formatConfidence(detail.proposal.confidence)} />
        <Metric label="Accepted" value={detail.proposal.accepted_at ?? "Not accepted"} />
        <Metric label="Duration" value={formatDuration(detail.proposal.expected_duration_seconds)} />
        <Metric label="Active until" value={detail.proposal.active_until ?? "Not set"} />
      </div>
      {conditions.length > 0 ? (
        <div className="mt-3">
          <div className="app-text-muted text-xs uppercase">Invalidation conditions</div>
          <ul className="mt-2 list-disc space-y-1 pl-5 text-sm">
            {conditions.map((condition) => (
              <li key={condition}>{condition}</li>
            ))}
          </ul>
        </div>
      ) : null}
      {detail.proposal.review_note ? (
        <p className="app-text-muted mt-3 text-sm">{detail.proposal.review_note}</p>
      ) : null}
      {detail.proposal.status === "proposed" ? (
        <div className="mt-4 flex gap-2">
          <button className="app-button-primary rounded-full px-4 py-2 text-sm" onClick={() => onReview(detail.proposal.id, "accept")}>Accept</button>
          <button className="app-button-secondary rounded-full px-4 py-2 text-sm" onClick={() => onReview(detail.proposal.id, "reject")}>Reject</button>
        </div>
      ) : null}
    </div>
  );
}

function ProposalActionsTable({ actions }: { actions: TraderPortfolioProposalDetail["actions"] }) {
  if (actions.length === 0) return <p className="app-text-muted text-sm">No actions recorded.</p>;
  return (
    <div className="overflow-x-auto">
      <table className="w-full min-w-[760px] text-left text-sm">
        <thead className="app-text-muted border-b text-xs uppercase">
          <tr>
            <th className="py-2 pr-3">Symbol</th>
            <th className="py-2 pr-3">Action</th>
            <th className="py-2 pr-3">Side</th>
            <th className="py-2 pr-3">Qty</th>
            <th className="py-2 pr-3">Entry</th>
            <th className="py-2 pr-3">Limit</th>
            <th className="py-2 pr-3">Exit</th>
            <th className="py-2 pr-3">Stop</th>
            <th className="py-2 pr-3">Current</th>
            <th className="py-2 pr-3">Enact by</th>
            <th className="py-2 pr-3">Risk</th>
            <th className="py-2 pr-3">Rationale</th>
          </tr>
        </thead>
        <tbody>
          {actions.map((action) => (
            <tr key={action.id} className="border-b align-top">
              <td className="py-3 pr-3 font-medium">{action.symbol ?? "-"}</td>
              <td className="py-3 pr-3">{action.action_type}</td>
              <td className="py-3 pr-3">{action.side ?? "-"}</td>
              <td className="py-3 pr-3">{action.quantity ?? "-"}</td>
              <td className="py-3 pr-3">{formatMoney(action.entry_price)}</td>
              <td className="py-3 pr-3">{formatMoney(action.limit_price)}</td>
              <td className="py-3 pr-3">{formatMoney(action.exit_price)}</td>
              <td className="py-3 pr-3">{formatMoney(action.stop_price)}</td>
              <td className="py-3 pr-3">{formatMoney(action.market_price_at_creation)}</td>
              <td className="py-3 pr-3">{action.enact_by ?? "-"}</td>
              <td className="py-3 pr-3">{action.risk_decision ?? "-"}</td>
              <td className="py-3 pr-3 app-text-muted">{action.rationale}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

function EventsPanel({ detail }: { detail: TraderDetail }) {
  if (detail.recent_events.length === 0) return <p className="app-text-muted text-sm">No events yet.</p>;
  return (
    <div className="max-h-80 space-y-3 overflow-auto">
      {detail.recent_events.map((event) => (
        <div key={event.id} className="border-b pb-3 text-sm">
          <div className="font-medium">{event.event_type}</div>
          <p className="app-text-muted mt-1">{event.message}</p>
          <div className="app-text-muted mt-1 text-xs">{event.created_at}</div>
        </div>
      ))}
    </div>
  );
}

function SourcesPanel({ detail }: { detail: TraderDetail }) {
  if (detail.info_sources.length === 0) return <p className="app-text-muted text-sm">No placeholder sources configured.</p>;
  return (
    <div className="space-y-2 text-sm">
      {detail.info_sources.map((source) => (
        <div key={source.id} className="app-surface-muted rounded-lg p-3">
          <div className="font-medium">{source.name}</div>
          <div className="app-text-muted text-xs">{source.source_type}</div>
        </div>
      ))}
    </div>
  );
}

function TraderDataSourcesPanel({
  dataSources,
  assignedIds,
  setAssignedIds,
  onSave,
}: {
  dataSources: DataSource[];
  assignedIds: string[];
  setAssignedIds: React.Dispatch<React.SetStateAction<string[]>>;
  onSave: () => void;
}) {
  if (dataSources.length === 0) return <p className="app-text-muted text-sm">No data sources available.</p>;
  return (
    <div className="space-y-3">
      {dataSources.map((source) => (
        <label key={source.id} className="app-surface-muted flex items-start gap-3 rounded-lg p-3 text-sm">
          <input
            type="checkbox"
            checked={assignedIds.includes(source.id)}
            onChange={(event) =>
              setAssignedIds((current) =>
                event.target.checked
                  ? Array.from(new Set([...current, source.id]))
                  : current.filter((id) => id !== source.id),
              )
            }
          />
          <span className="min-w-0 flex-1">
            <span className="block font-medium">{source.name}</span>
            <span className="app-text-muted block text-xs">
              {source.source_type} - {source.enabled ? "enabled" : "disabled"} - checked {source.last_checked_at ?? "never"}
            </span>
            {source.last_error ? <span className="app-text-muted block text-xs">{source.last_error}</span> : null}
          </span>
        </label>
      ))}
      <button className="app-button-primary rounded-full px-4 py-2 text-sm" onClick={onSave}>Save data sources</button>
    </div>
  );
}

const memoryTypes = [
  "answer",
  "review",
  "decision",
  "user_preference",
  "risk_note",
  "data_note",
  "proposal_note",
  "channel_resolution",
];

function TraderMemoriesPanel({
  memories,
  filters,
  setFilters,
  onAdd,
  onEdit,
  onArchive,
  onRestore,
}: {
  memories: TraderMemory[];
  filters: { status: string; memory_type: string; topic: string };
  setFilters: React.Dispatch<React.SetStateAction<{ status: string; memory_type: string; topic: string }>>;
  onAdd: () => void;
  onEdit: (memory: TraderMemory) => void;
  onArchive: (memory: TraderMemory) => void;
  onRestore: (memory: TraderMemory) => void;
}) {
  return (
    <div className="space-y-4">
      <div className="flex flex-wrap items-end justify-between gap-3">
        <div className="grid flex-1 gap-3 md:grid-cols-3">
          <FilterSelect label="Status" value={filters.status} onChange={(value) => setFilters((current) => ({ ...current, status: value }))} options={["active", "archived", "superseded"]} />
          <FilterSelect label="Type" value={filters.memory_type} onChange={(value) => setFilters((current) => ({ ...current, memory_type: value }))} options={memoryTypes} />
          <Field label="Search text">
            <input className="app-input w-full rounded-lg px-3 py-2 text-sm" value={filters.topic} onChange={(event) => setFilters((current) => ({ ...current, topic: event.target.value }))} placeholder="risk, sector, data..." />
          </Field>
        </div>
        <button type="button" className="app-button-primary rounded-full px-4 py-2 text-sm" onClick={onAdd}>Add Memory</button>
      </div>
      {memories.length === 0 ? (
        <p className="app-text-muted text-sm">No memories match the current filters.</p>
      ) : (
        <div className="overflow-x-auto">
          <table className="w-full min-w-[980px] text-left text-sm">
            <thead className="app-text-muted border-b text-xs uppercase">
              <tr>
                <th className="py-2 pr-3">Topic</th>
                <th className="py-2 pr-3">Type</th>
                <th className="py-2 pr-3">Summary</th>
                <th className="py-2 pr-3">Importance</th>
                <th className="py-2 pr-3">Confidence</th>
                <th className="py-2 pr-3">Last Used</th>
                <th className="py-2 pr-3">Created</th>
                <th className="py-2 pr-3">Status</th>
                <th className="py-2 pr-3">Actions</th>
              </tr>
            </thead>
            <tbody>
              {memories.map((memory) => (
                <tr key={memory.id} className="border-b align-top">
                  <td className="py-3 pr-3 font-medium">{memory.topic}</td>
                  <td className="py-3 pr-3">{memory.memory_type}</td>
                  <td className="py-3 pr-3 app-text-muted">{memory.summary}</td>
                  <td className="py-3 pr-3">{memory.importance}</td>
                  <td className="py-3 pr-3">{formatConfidence(memory.confidence)}</td>
                  <td className="py-3 pr-3 app-text-muted text-xs">{memory.last_used_at ?? "-"}</td>
                  <td className="py-3 pr-3 app-text-muted text-xs">{memory.created_at}</td>
                  <td className="py-3 pr-3">{memory.status}</td>
                  <td className="space-y-2 py-3 pr-3">
                    <button type="button" className="app-button-secondary rounded-full px-3 py-1 text-xs" onClick={() => onEdit(memory)}>Edit</button>
                    {memory.status === "archived" ? (
                      <button type="button" className="app-button-primary ml-2 rounded-full px-3 py-1 text-xs" onClick={() => onRestore(memory)}>Restore</button>
                    ) : (
                      <button type="button" className="app-button-secondary ml-2 rounded-full px-3 py-1 text-xs" onClick={() => onArchive(memory)}>Archive</button>
                    )}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}

function TraderSymbolsPanel({
  symbols,
  filters,
  setFilters,
  form,
  setForm,
  suggestionForm,
  setSuggestionForm,
  pending,
  onAdd,
  onUpdate,
  onStatus,
  onSuggest,
}: {
  symbols: TraderSymbol[];
  filters: SymbolFilters;
  setFilters: React.Dispatch<React.SetStateAction<SymbolFilters>>;
  form: SymbolFormState;
  setForm: React.Dispatch<React.SetStateAction<SymbolFormState>>;
  suggestionForm: {
    max_symbols: number;
    include_etfs: boolean;
    include_stocks: boolean;
    focus: string;
  };
  setSuggestionForm: React.Dispatch<React.SetStateAction<{
    max_symbols: number;
    include_etfs: boolean;
    include_stocks: boolean;
    focus: string;
  }>>;
  pending: boolean;
  onAdd: (event: React.FormEvent<HTMLFormElement>) => void;
  onUpdate: (symbol: TraderSymbol, input: Partial<TraderSymbol>) => void;
  onStatus: (symbol: TraderSymbol, action: "activate" | "reject" | "archive") => void;
  onSuggest: (event: React.FormEvent<HTMLFormElement>) => void;
}) {
  const visibleSymbols = symbols.filter((symbol) => {
    if (filters.status && symbol.status !== filters.status) return false;
    if (filters.asset_type && symbol.asset_type !== filters.asset_type) return false;
    if (filters.source && symbol.source !== filters.source) return false;
    return true;
  });

  return (
    <div className="space-y-5">
      <form className="grid gap-3 md:grid-cols-5" onSubmit={onAdd}>
        <Field label="Symbol">
          <input
            className="app-input w-full rounded-lg px-3 py-2 text-sm uppercase"
            value={form.symbol}
            onChange={(event) => setForm((current) => ({ ...current, symbol: event.target.value.toUpperCase() }))}
            placeholder="SPY"
            required
          />
        </Field>
        <Field label="Asset type">
          <select
            className="app-input w-full rounded-lg px-3 py-2 text-sm"
            value={form.asset_type}
            onChange={(event) => setForm((current) => ({ ...current, asset_type: event.target.value as TraderSymbolAssetType }))}
          >
            <option value="stock">Stock</option>
            <option value="etf">ETF</option>
            <option value="index">Index</option>
            <option value="crypto">Crypto</option>
            <option value="other">Other</option>
          </select>
        </Field>
        <Field label="Status">
          <select
            className="app-input w-full rounded-lg px-3 py-2 text-sm"
            value={form.status}
            onChange={(event) => setForm((current) => ({ ...current, status: event.target.value as TraderSymbolStatus }))}
          >
            <option value="watching">Watching</option>
            <option value="candidate">Candidate</option>
            <option value="active">Active</option>
          </select>
        </Field>
        <Field label="Thesis">
          <input
            className="app-input w-full rounded-lg px-3 py-2 text-sm"
            value={form.thesis}
            onChange={(event) => setForm((current) => ({ ...current, thesis: event.target.value }))}
            placeholder="Why it fits"
          />
        </Field>
        <div className="flex items-end">
          <button className="app-button-primary w-full rounded-full px-4 py-2 text-sm" disabled={pending}>
            Add
          </button>
        </div>
      </form>

      <form className="app-surface-muted rounded-lg p-4" onSubmit={onSuggest}>
        <div className="grid gap-3 md:grid-cols-5">
          <Field label="AI max">
            <input
              type="number"
              min={1}
              max={50}
              className="app-input w-full rounded-lg px-3 py-2 text-sm"
              value={suggestionForm.max_symbols}
              onChange={(event) =>
                setSuggestionForm((current) => ({
                  ...current,
                  max_symbols: Number(event.target.value),
                }))
              }
            />
          </Field>
          <label className="flex items-end gap-2 pb-2 text-sm">
            <input
              type="checkbox"
              checked={suggestionForm.include_etfs}
              onChange={(event) =>
                setSuggestionForm((current) => ({ ...current, include_etfs: event.target.checked }))
              }
            />
            ETFs
          </label>
          <label className="flex items-end gap-2 pb-2 text-sm">
            <input
              type="checkbox"
              checked={suggestionForm.include_stocks}
              onChange={(event) =>
                setSuggestionForm((current) => ({ ...current, include_stocks: event.target.checked }))
              }
            />
            Stocks
          </label>
          <Field label="Focus">
            <input
              className="app-input w-full rounded-lg px-3 py-2 text-sm"
              value={suggestionForm.focus}
              onChange={(event) =>
                setSuggestionForm((current) => ({ ...current, focus: event.target.value }))
              }
              placeholder="Retirement income"
            />
          </Field>
          <div className="flex items-end">
            <button className="app-button-secondary w-full rounded-full px-4 py-2 text-sm" disabled={pending}>
              {pending ? "Working..." : "AI suggest"}
            </button>
          </div>
        </div>
      </form>

      <div className="grid gap-3 md:grid-cols-3">
        <FilterSelect
          label="Status"
          value={filters.status}
          onChange={(value) => setFilters((current) => ({ ...current, status: value as TraderSymbolStatus | "" }))}
          options={["watching", "candidate", "active", "rejected", "archived"]}
        />
        <FilterSelect
          label="Asset"
          value={filters.asset_type}
          onChange={(value) => setFilters((current) => ({ ...current, asset_type: value as TraderSymbolAssetType | "" }))}
          options={["stock", "etf", "index", "crypto", "other"]}
        />
        <FilterSelect
          label="Source"
          value={filters.source}
          onChange={(value) => setFilters((current) => ({ ...current, source: value as TraderSymbolSource | "" }))}
          options={["manual", "ai", "import", "engine"]}
        />
      </div>

      {visibleSymbols.length === 0 ? (
        <p className="app-text-muted text-sm">No tracked symbols match the current filters.</p>
      ) : (
        <div className="overflow-x-auto">
          <table className="w-full min-w-[900px] text-left text-sm">
            <thead className="app-text-muted border-b text-xs uppercase">
              <tr>
                <th className="py-2 pr-3">Symbol</th>
                <th className="py-2 pr-3">Asset</th>
                <th className="py-2 pr-3">Status</th>
                <th className="py-2 pr-3">Fit</th>
                <th className="py-2 pr-3">Thesis</th>
                <th className="py-2 pr-3">Source</th>
                <th className="py-2 pr-3">Updated</th>
                <th className="py-2 pr-3">Actions</th>
              </tr>
            </thead>
            <tbody>
              {visibleSymbols.map((symbol) => (
                <TraderSymbolRow
                  key={symbol.id}
                  symbol={symbol}
                  onUpdate={onUpdate}
                  onStatus={onStatus}
                />
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}

function TraderSymbolRow({
  symbol,
  onUpdate,
  onStatus,
}: {
  symbol: TraderSymbol;
  onUpdate: (symbol: TraderSymbol, input: Partial<TraderSymbol>) => void;
  onStatus: (symbol: TraderSymbol, action: "activate" | "reject" | "archive") => void;
}) {
  const [status, setStatus] = useState<TraderSymbolStatus>(symbol.status);
  const [thesis, setThesis] = useState(symbol.thesis ?? "");

  useEffect(() => {
    setStatus(symbol.status);
    setThesis(symbol.thesis ?? "");
  }, [symbol.id, symbol.status, symbol.thesis]);

  return (
    <tr className="border-b align-top">
      <td className="py-3 pr-3 font-semibold">{symbol.symbol}</td>
      <td className="py-3 pr-3">{symbol.asset_type}</td>
      <td className="py-3 pr-3">
        <select
          className="app-input w-32 rounded-lg px-2 py-1 text-xs"
          value={status}
          onChange={(event) => setStatus(event.target.value as TraderSymbolStatus)}
        >
          <option value="watching">watching</option>
          <option value="candidate">candidate</option>
          <option value="active">active</option>
          <option value="rejected">rejected</option>
          <option value="archived">archived</option>
        </select>
      </td>
      <td className="py-3 pr-3">{symbol.fit_score == null ? "-" : `${Math.round(symbol.fit_score * 100)}%`}</td>
      <td className="py-3 pr-3">
        <textarea
          className="app-input min-h-16 w-80 rounded-lg px-2 py-1 text-xs"
          value={thesis}
          onChange={(event) => setThesis(event.target.value)}
        />
      </td>
      <td className="py-3 pr-3">{symbol.source}</td>
      <td className="py-3 pr-3 app-text-muted text-xs">{symbol.updated_at}</td>
      <td className="space-y-2 py-3 pr-3">
        <div className="flex flex-wrap gap-2">
          <button
            type="button"
            className="app-button-primary rounded-full px-3 py-1 text-xs"
            onClick={() => onUpdate(symbol, { status, thesis })}
          >
            Save
          </button>
          <button type="button" className="app-button-secondary rounded-full px-3 py-1 text-xs" onClick={() => onStatus(symbol, "activate")}>Activate</button>
          <button type="button" className="app-button-secondary rounded-full px-3 py-1 text-xs" onClick={() => onStatus(symbol, "reject")}>Reject</button>
          <button type="button" className="app-button-secondary rounded-full px-3 py-1 text-xs" onClick={() => onStatus(symbol, "archive")}>Archive</button>
        </div>
      </td>
    </tr>
  );
}

function FilterSelect({
  label,
  value,
  options,
  onChange,
}: {
  label: string;
  value: string;
  options: string[];
  onChange: (value: string) => void;
}) {
  return (
    <label className="block">
      <span className="mb-2 block text-sm font-medium">{label}</span>
      <select className="app-input w-full rounded-lg px-3 py-2 text-sm" value={value} onChange={(event) => onChange(event.target.value)}>
        <option value="">All</option>
        {options.map((option) => (
          <option key={option} value={option}>{option}</option>
        ))}
      </select>
    </label>
  );
}

function formatFreedom(level: string) {
  return level.replace("_", " ");
}

function formatConfidence(value?: number | null) {
  if (value == null) return "-";
  return `${Math.round(value * 100)}%`;
}

function formatMoney(value?: number | null) {
  if (value == null) return "-";
  return `$${value.toFixed(2)}`;
}

function formatDuration(value?: number | null) {
  if (!value) return "-";
  if (value < 3600) return `${Math.round(value / 60)}m`;
  if (value < 86400) return `${Math.round(value / 3600)}h`;
  return `${Math.round(value / 86400)}d`;
}

function parseJsonList(value?: string | null) {
  if (!value) return [];
  try {
    const parsed = JSON.parse(value);
    if (Array.isArray(parsed)) {
      return parsed.map(String);
    }
  } catch {
    return [];
  }
  return [];
}

function accountName(accounts: PaperAccount[], accountId?: string | null) {
  if (!accountId) return "None";
  return accounts.find((account) => account.id === accountId)?.name ?? accountId;
}
