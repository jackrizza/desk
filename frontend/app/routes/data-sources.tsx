import type React from "react";
import { useEffect, useState } from "react";
import { EmptyState, ErrorInline, LoadingInline } from "../components/AppFeedback";
import { ChatPanel } from "../components/ChatPanel";
import { LeftSidebar } from "../components/LeftSidebar";
import { Topbar } from "../components/Topbar";
import {
  deskApi,
  type DataSource,
  type DataSourceEvent,
  type DataSourceItem,
  type DataSourceType,
} from "../lib/api";
import { CHAT_COMMAND_EXECUTED_EVENT, useDeskChat } from "../lib/chat";
import { CHAT_OPEN_STORAGE_KEY, usePersistentBoolean } from "../lib/ui-state";

type FormState = {
  name: string;
  source_type: DataSourceType;
  url: string;
  enabled: boolean;
  poll_interval_seconds: string;
  config_json: string;
};

const emptyForm: FormState = {
  name: "",
  source_type: "rss",
  url: "",
  enabled: true,
  poll_interval_seconds: "30",
  config_json: "",
};

export default function DataSourcesRoute() {
  const [sidebarOpen, setSidebarOpen] = useState(true);
  const [sources, setSources] = useState<DataSource[]>([]);
  const [selected, setSelected] = useState<DataSource | null>(null);
  const [items, setItems] = useState<DataSourceItem[]>([]);
  const [events, setEvents] = useState<DataSourceEvent[]>([]);
  const [form, setForm] = useState<FormState>(emptyForm);
  const [editing, setEditing] = useState<DataSource | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  const [chatOpen, setChatOpen] = usePersistentBoolean(
    CHAT_OPEN_STORAGE_KEY,
    false,
  );
  const chat = useDeskChat({
    page: "data_sources",
    sourceCount: sources.length,
    enabledCount: sources.filter((source) => source.enabled).length,
    selectedSourceName: selected?.name ?? null,
    selectedSourceType: selected?.source_type ?? null,
  });

  async function refresh(sourceId = selected?.id) {
    const nextSources = await deskApi.listDataSources();
    setSources(nextSources);
    const nextSelected = nextSources.find((source) => source.id === sourceId) ?? nextSources[0] ?? null;
    setSelected(nextSelected);
    if (nextSelected) {
      const [nextItems, nextEvents] = await Promise.all([
        deskApi.getDataSourceItems(nextSelected.id).catch(() => ({ items: [] })),
        deskApi.getDataSourceEvents(nextSelected.id).catch(() => ({ events: [] })),
      ]);
      setItems(nextItems.items);
      setEvents(nextEvents.events);
    } else {
      setItems([]);
      setEvents([]);
    }
  }

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    refresh()
      .catch((err) => {
        if (!cancelled) setError(err instanceof Error ? err.message : "Failed to load data sources.");
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    const interval = window.setInterval(() => {
      void refresh().catch(() => undefined);
    }, 30000);
    return () => {
      cancelled = true;
      window.clearInterval(interval);
    };
  }, []);

  useEffect(() => {
    const handleCommand = () => {
      void refresh().catch(() => undefined);
    };
    window.addEventListener(CHAT_COMMAND_EXECUTED_EVENT, handleCommand);
    return () => window.removeEventListener(CHAT_COMMAND_EXECUTED_EVENT, handleCommand);
  }, [selected?.id]);

  function editSource(source: DataSource) {
    setEditing(source);
    setForm({
      name: source.name,
      source_type: source.source_type,
      url: source.url ?? "",
      enabled: source.enabled,
      poll_interval_seconds: String(source.poll_interval_seconds),
      config_json: source.config_json ?? "",
    });
  }

  async function submit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError("");
    const payload = {
      name: form.name.trim(),
      source_type: form.source_type,
      url: form.url.trim() || null,
      enabled: form.enabled,
      poll_interval_seconds: Number(form.poll_interval_seconds) || 30,
      config_json: form.config_json.trim() || null,
    };
    try {
      if (editing) {
        await deskApi.updateDataSource(editing.id, payload);
        setEditing(null);
      } else {
        await deskApi.createDataSource(payload);
      }
      setForm(emptyForm);
      await refresh(editing?.id);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to save data source.");
    }
  }

  async function disable(sourceId: string) {
    setError("");
    try {
      await deskApi.deleteDataSource(sourceId);
      await refresh(sourceId);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to disable data source.");
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
        <div className="mx-auto grid max-w-7xl gap-6 px-6 pb-12 lg:grid-cols-[22rem_1fr]">
          <aside className="app-surface rounded-lg border p-4">
            <h1 className="mb-4 text-xl font-semibold">Data Sources</h1>
            {sources.map((source) => (
              <button
                key={source.id}
                className="app-nav-link mb-1 flex w-full items-center justify-between rounded-md px-3 py-2 text-left"
                onClick={() => {
                  setSelected(source);
                  void refresh(source.id);
                }}
              >
                <span className="truncate">{source.name}</span>
                <StatusBadge enabled={source.enabled} />
              </button>
            ))}
            {!loading && sources.length === 0 ? <EmptyState message="No data sources yet" /> : null}
          </aside>

          <section className="space-y-5">
            {loading ? <LoadingInline message="Loading data sources" /> : null}
            {error ? <ErrorInline message={error} /> : null}
            <form className="app-surface rounded-lg border p-5" onSubmit={submit}>
              <h2 className="text-lg font-semibold">{editing ? "Edit data source" : "Create data source"}</h2>
              <div className="mt-4 grid gap-4 md:grid-cols-2">
                <Field label="Name"><input className="app-input w-full rounded-lg px-3 py-2" value={form.name} onChange={(e) => setForm((c) => ({ ...c, name: e.target.value }))} required /></Field>
                <Field label="Type">
                  <select className="app-input w-full rounded-lg px-3 py-2" value={form.source_type} onChange={(e) => setForm((c) => ({ ...c, source_type: e.target.value as DataSourceType }))}>
                    <option value="rss">RSS</option>
                    <option value="web_page">Web Page</option>
                    <option value="manual_note">Manual Note</option>
                    <option value="placeholder_api">Placeholder API</option>
                  </select>
                </Field>
                <Field label="URL"><input className="app-input w-full rounded-lg px-3 py-2" value={form.url} onChange={(e) => setForm((c) => ({ ...c, url: e.target.value }))} placeholder="https://..." /></Field>
                <Field label="Poll interval seconds"><input className="app-input w-full rounded-lg px-3 py-2" value={form.poll_interval_seconds} onChange={(e) => setForm((c) => ({ ...c, poll_interval_seconds: e.target.value }))} /></Field>
              </div>
              <label className="mt-4 flex items-center gap-3 text-sm"><input type="checkbox" checked={form.enabled} onChange={(e) => setForm((c) => ({ ...c, enabled: e.target.checked }))} /> Enabled</label>
              <Field label="Config JSON"><textarea className="app-input mt-2 w-full rounded-lg px-3 py-2" rows={4} value={form.config_json} onChange={(e) => setForm((c) => ({ ...c, config_json: e.target.value }))} /></Field>
              <div className="mt-4 flex justify-end gap-3">
                {editing ? <button type="button" className="app-button-secondary rounded-full px-4 py-2 text-sm" onClick={() => { setEditing(null); setForm(emptyForm); }}>Cancel</button> : null}
                <button className="app-button-primary rounded-full px-4 py-2 text-sm">{editing ? "Save" : "Create"}</button>
              </div>
            </form>

            {selected ? (
              <div className="grid gap-5 lg:grid-cols-2">
                <Panel title={selected.name}>
                  <div className="space-y-2 text-sm">
                    <Row label="Type" value={selected.source_type} />
                    <Row label="Last checked" value={selected.last_checked_at ?? "Never"} />
                    <Row label="Last success" value={selected.last_success_at ?? "Never"} />
                    <Row label="Last error" value={selected.last_error ?? "None"} />
                  </div>
                  <div className="mt-4 flex gap-2">
                    <button className="app-button-secondary rounded-full px-3 py-1.5 text-sm" onClick={() => editSource(selected)}>Edit</button>
                    <button className="app-button-secondary rounded-full px-3 py-1.5 text-sm" onClick={() => disable(selected.id)}>Disable</button>
                  </div>
                </Panel>
                <Panel title="Recent Items">
                  <div className="max-h-80 space-y-3 overflow-auto text-sm">
                    {items.map((item) => <div key={item.id} className="border-b pb-3"><div className="font-medium">{item.title}</div><div className="app-text-muted text-xs">{item.discovered_at}</div></div>)}
                    {items.length === 0 ? <p className="app-text-muted text-sm">No items yet.</p> : null}
                  </div>
                </Panel>
                <Panel title="Events">
                  <div className="max-h-80 space-y-3 overflow-auto text-sm">
                    {events.map((event) => <div key={event.id} className="border-b pb-3"><div className="font-medium">{event.event_type}</div><p className="app-text-muted">{event.message}</p></div>)}
                    {events.length === 0 ? <p className="app-text-muted text-sm">No events yet.</p> : null}
                  </div>
                </Panel>
              </div>
            ) : null}
          </section>
        </div>
      </main>
      <ChatPanel
        open={chatOpen}
        title="Data Source assistant"
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

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return <label className="block"><span className="mb-2 block text-sm font-medium">{label}</span>{children}</label>;
}

function Panel({ title, children }: { title: string; children: React.ReactNode }) {
  return <div className="app-surface rounded-lg border p-5"><h3 className="text-lg font-semibold">{title}</h3><div className="mt-4">{children}</div></div>;
}

function Row({ label, value }: { label: string; value: string }) {
  return <div className="flex justify-between gap-4"><span className="app-text-muted">{label}</span><span className="truncate text-right">{value}</span></div>;
}

function StatusBadge({ enabled }: { enabled: boolean }) {
  return <span className="app-surface-muted rounded-full px-2 py-1 text-xs">{enabled ? "enabled" : "disabled"}</span>;
}
