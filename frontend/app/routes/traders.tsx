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
  type TraderTradeProposal,
  type PaperAccount,
  type DataSource,
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

const emptyForm: TraderFormState = {
  name: "",
  fundamental_perspective: "",
  freedom_level: "analyst",
  default_paper_account_id: "",
  openai_api_key: "",
  source_type: "placeholder",
  source_name: "",
};

export default function TradersRoute() {
  const params = useParams();
  const navigate = useNavigate();
  const [sidebarOpen, setSidebarOpen] = useState(true);
  const [traders, setTraders] = useState<Trader[]>([]);
  const [paperAccounts, setPaperAccounts] = useState<PaperAccount[]>([]);
  const [detail, setDetail] = useState<TraderDetail | null>(null);
  const [proposals, setProposals] = useState<TraderTradeProposal[]>([]);
  const [dataSources, setDataSources] = useState<DataSource[]>([]);
  const [assignedDataSourceIds, setAssignedDataSourceIds] = useState<string[]>([]);
  const [form, setForm] = useState<TraderFormState>(emptyForm);
  const [editing, setEditing] = useState(false);
  const [apiKeySaved, setApiKeySaved] = useState(false);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
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
      const [nextDetail, nextProposals, assignedSources] = await Promise.all([
        deskApi.getTrader(id),
        deskApi.getTraderTradeProposals(id).catch(() => ({ proposals: [] })),
        deskApi.getTraderDataSources(id).catch(() => ({ data_sources: [] })),
      ]);
      setDetail(nextDetail);
      setProposals(nextProposals.proposals);
      setAssignedDataSourceIds(assignedSources.data_sources.map((source) => source.id));
      if (!params.traderId) {
        navigate(`/traders/${encodeURIComponent(id)}`, { replace: true });
      }
    } else {
      setDetail(null);
      setProposals([]);
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
  }, [params.traderId]);

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
              <div className="mb-4 flex items-center justify-between">
                <h1 className="text-xl font-semibold">Traders</h1>
                <button
                  className="app-button-secondary rounded-full px-3 py-1.5 text-sm"
                  onClick={() => {
                    setEditing(false);
                    setForm(emptyForm);
                    setDetail(null);
                    navigate("/traders");
                  }}
                >
                  New
                </button>
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

            {detail ? (
              <div className="grid gap-5 lg:grid-cols-2">
                <Panel title="Runtime">
                  <RuntimePanel detail={detail} />
                </Panel>
                <Panel title="Pending Proposals">
                  <ProposalsPanel proposals={pendingProposals} onReview={reviewProposal} />
                </Panel>
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
        onClose={() => setChatOpen(false)}
        onSubmit={chat.sendMessage}
        onClear={chat.clearMessages}
      />
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

function formatFreedom(level: string) {
  return level.replace("_", " ");
}

function accountName(accounts: PaperAccount[], accountId?: string | null) {
  if (!accountId) return "None";
  return accounts.find((account) => account.id === accountId)?.name ?? accountId;
}
