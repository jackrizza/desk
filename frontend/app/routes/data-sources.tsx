import type React from "react";
import { useEffect, useRef, useState } from "react";
import Editor from "@monaco-editor/react";
import { ErrorInline, LoadingInline } from "../components/AppFeedback";
import { ChatPanel } from "../components/ChatPanel";
import { LeftSidebar } from "../components/LeftSidebar";
import { Topbar } from "../components/Topbar";
import { useSearchParams } from "react-router";
import {
  deskApi,
  type DataSource,
  type DataSourceEvent,
  type DataSourceItem,
  type DataSourceScript,
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
  const [script, setScript] = useState<DataSourceScript | null>(null);
  const [scriptText, setScriptText] = useState("");
  const [saveStatus, setSaveStatus] = useState<"idle" | "saved" | "saving" | "unsaved" | "failed">("idle");
  const [buildStatus, setBuildStatus] = useState<"idle" | "building" | "success" | "failed">("idle");
  const [buildOutput, setBuildOutput] = useState("");
  const [consoleOpen, setConsoleOpen] = useState(false);
  const [buildTimestamp, setBuildTimestamp] = useState("");
  const saveTimerRef = useRef<number | null>(null);
  const latestScriptRef = useRef("");
  const [form, setForm] = useState<FormState>(emptyForm);
  const [editing, setEditing] = useState<DataSource | null>(null);
  const [editModalOpen, setEditModalOpen] = useState(false);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  const [searchParams, setSearchParams] = useSearchParams();
  const selectedSourceId = searchParams.get("source") ?? undefined;
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

  async function refresh(sourceId = selectedSourceId ?? selected?.id) {
    const nextSources = await deskApi.listDataSources();
    setSources(nextSources);
    const nextSelected = nextSources.find((source) => source.id === sourceId) ?? nextSources[0] ?? null;
    setSelected(nextSelected);
    if (!sourceId && nextSelected) {
      setSearchParams({ source: nextSelected.id }, { replace: true });
    }
    if (nextSelected) {
      const [nextItems, nextEvents, nextScript] = await Promise.all([
        deskApi.getDataSourceItems(nextSelected.id).catch(() => ({ items: [] })),
        deskApi.getDataSourceEvents(nextSelected.id).catch(() => ({ events: [] })),
        nextSelected.source_type === "python_script"
          ? deskApi.getDataSourceScript(nextSelected.id).catch(() => null)
          : Promise.resolve(null),
      ]);
      setItems(nextItems.items);
      setEvents(nextEvents.events);
      setScript(nextScript);
      setScriptText(nextScript?.script_text ?? "");
      latestScriptRef.current = nextScript?.script_text ?? "";
      setSaveStatus(nextScript ? "saved" : "idle");
    } else {
      setItems([]);
      setEvents([]);
      setScript(null);
      setScriptText("");
      latestScriptRef.current = "";
      setSaveStatus("idle");
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
    if (!selectedSourceId) return;
    void refresh(selectedSourceId).catch((err) => {
      setError(err instanceof Error ? err.message : "Failed to load data source.");
    });
  }, [selectedSourceId]);

  useEffect(() => {
    const handleCommand = () => {
      void refresh().catch(() => undefined);
    };
    window.addEventListener(CHAT_COMMAND_EXECUTED_EVENT, handleCommand);
    return () => window.removeEventListener(CHAT_COMMAND_EXECUTED_EVENT, handleCommand);
  }, [selected?.id]);

  function editSource(source: DataSource) {
    setEditing(source);
    setEditModalOpen(true);
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
        setEditModalOpen(false);
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

  useEffect(() => {
    if (!selected || selected.source_type !== "python_script") return;
    if (!script) return;
    if (scriptText === script.script_text) return;
    latestScriptRef.current = scriptText;
    setSaveStatus("unsaved");
    if (saveTimerRef.current) window.clearTimeout(saveTimerRef.current);
    saveTimerRef.current = window.setTimeout(() => {
      void saveScript(scriptText);
    }, 1000);
    return () => {
      if (saveTimerRef.current) window.clearTimeout(saveTimerRef.current);
    };
  }, [scriptText, selected?.id]);

  async function saveScript(text = latestScriptRef.current) {
    if (!selected || selected.source_type !== "python_script") return null;
    setSaveStatus("saving");
    try {
      const saved = await deskApi.updateDataSourceScript(selected.id, { script_text: text });
      setScript(saved);
      setScriptText(saved.script_text);
      latestScriptRef.current = saved.script_text;
      setSaveStatus("saved");
      return saved;
    } catch (err) {
      setSaveStatus("failed");
      setBuildOutput(err instanceof Error ? err.message : "Save failed.");
      return null;
    }
  }

  async function buildScript() {
    if (!selected || selected.source_type !== "python_script") return;
    if (saveTimerRef.current) window.clearTimeout(saveTimerRef.current);
    setConsoleOpen(true);
    setBuildStatus("building");
    const saved = await saveScript(latestScriptRef.current || scriptText);
    if (!saved) {
      setBuildStatus("failed");
      setBuildTimestamp(new Date().toLocaleString());
      return;
    }
    try {
      const result = await deskApi.buildDataSourceScript(selected.id);
      setBuildStatus(result.success ? "success" : "failed");
      setBuildOutput(result.output);
      setBuildTimestamp(new Date().toLocaleString());
      await refresh(selected.id);
    } catch (err) {
      setBuildStatus("failed");
      setBuildOutput(err instanceof Error ? err.message : "Build failed.");
      setBuildTimestamp(new Date().toLocaleString());
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
        <div className="mx-auto max-w-[96rem] px-6 pb-12">
          <section className="space-y-5">
            {loading ? <LoadingInline message="Loading data sources" /> : null}
            {error ? <ErrorInline message={error} /> : null}

            {selected ? (
              <div className={selected.source_type === "python_script" ? "grid min-h-[42rem] gap-5 xl:grid-cols-[minmax(22rem,0.9fr)_minmax(34rem,1.4fr)]" : "grid gap-5 lg:grid-cols-2"}>
                <div className="grid gap-5">
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
                {selected.source_type === "python_script" ? (
                  <PythonScriptEditor
                    script={script}
                    scriptText={scriptText}
                    saveStatus={saveStatus}
                    buildStatus={buildStatus}
                    buildOutput={buildOutput}
                    consoleOpen={consoleOpen}
                    buildTimestamp={buildTimestamp}
                    onChange={(value) => setScriptText(value ?? "")}
                    onBuild={buildScript}
                    onToggleConsole={() => setConsoleOpen((open) => !open)}
                  />
                ) : null}
              </div>
            ) : !loading ? (
              <Panel title="Data Sources">
                <p className="app-text-muted text-sm">Choose a data source from the left sidebar or create a new one there.</p>
              </Panel>
            ) : null}
          </section>
        </div>
      </main>
      {editModalOpen && editing ? (
        <div className="app-modal-backdrop fixed inset-0 z-30 flex items-center justify-center p-4">
          <div className="app-surface max-h-[90vh] w-full max-w-2xl overflow-auto rounded-3xl p-5 shadow-lg">
            <div className="mb-4 flex items-center justify-between gap-3">
              <div>
                <p className="app-text-muted text-xs uppercase tracking-[0.2em]">Data Sources</p>
                <h2 className="mt-2 text-xl font-semibold">Edit data source</h2>
              </div>
              <button
                type="button"
                className="app-nav-link app-text-muted rounded px-2 py-1"
                aria-label="Close data source modal"
                onClick={() => {
                  setEditModalOpen(false);
                  setEditing(null);
                  setForm(emptyForm);
                }}
              >
                x
              </button>
            </div>
            <form className="space-y-3" onSubmit={submit}>
              <div className="grid gap-3 sm:grid-cols-2">
                <Field label="Name"><input className="app-input w-full rounded-2xl px-3 py-2.5 text-sm" value={form.name} onChange={(e) => setForm((c) => ({ ...c, name: e.target.value }))} required /></Field>
                <Field label="Type">
                  <select className="app-input w-full rounded-2xl px-3 py-2.5 text-sm" value={form.source_type} onChange={(e) => setForm((c) => ({ ...c, source_type: e.target.value as DataSourceType }))}>
                    <option value="rss">RSS</option>
                    <option value="web_page">Web Page</option>
                    <option value="manual_note">Manual Note</option>
                    <option value="placeholder_api">Placeholder API</option>
                    <option value="python_script">Python Script</option>
                  </select>
                </Field>
              </div>
              <div className="grid gap-3 sm:grid-cols-2">
                <Field label="URL"><input className="app-input w-full rounded-2xl px-3 py-2.5 text-sm" value={form.url} onChange={(e) => setForm((c) => ({ ...c, url: e.target.value }))} placeholder="https://..." /></Field>
                <Field label="Poll interval seconds"><input className="app-input w-full rounded-2xl px-3 py-2.5 text-sm" value={form.poll_interval_seconds} onChange={(e) => setForm((c) => ({ ...c, poll_interval_seconds: e.target.value }))} /></Field>
              </div>
              <label className="flex items-center gap-3 rounded-2xl px-1 py-2 text-sm"><input type="checkbox" checked={form.enabled} onChange={(e) => setForm((c) => ({ ...c, enabled: e.target.checked }))} /> Enabled</label>
              <Field label="Config JSON"><textarea className="app-input w-full rounded-2xl px-3 py-2.5 text-sm" rows={4} value={form.config_json} onChange={(e) => setForm((c) => ({ ...c, config_json: e.target.value }))} /></Field>
              <div className="flex justify-end gap-3 pt-2">
                <button type="button" className="app-button-secondary rounded-full px-4 py-2 text-sm" onClick={() => { setEditModalOpen(false); setEditing(null); setForm(emptyForm); }}>Cancel</button>
                <button className="app-button-primary rounded-full px-4 py-2 text-sm">Save data source</button>
              </div>
            </form>
          </div>
        </div>
      ) : null}
      <ChatPanel
        open={chatOpen}
        title="Data Source assistant"
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
    </div>
  );
}

function PythonScriptEditor({
  script,
  scriptText,
  saveStatus,
  buildStatus,
  buildOutput,
  consoleOpen,
  buildTimestamp,
  onChange,
  onBuild,
  onToggleConsole,
}: {
  script: DataSourceScript | null;
  scriptText: string;
  saveStatus: "idle" | "saved" | "saving" | "unsaved" | "failed";
  buildStatus: "idle" | "building" | "success" | "failed";
  buildOutput: string;
  consoleOpen: boolean;
  buildTimestamp: string;
  onChange: (value: string | undefined) => void;
  onBuild: () => void;
  onToggleConsole: () => void;
}) {
  return (
    <section className="app-surface flex min-h-[42rem] flex-col rounded-lg border">
      <div className="flex flex-wrap items-center justify-between gap-3 border-b px-4 py-3">
        <div>
          <h3 className="text-base font-semibold">Python Script</h3>
          <p className="app-text-muted text-xs">
            {script?.last_build_status ? `Last build: ${script.last_build_status}` : "Not built"}
          </p>
        </div>
        <div className="flex items-center gap-3">
          <span className="app-text-muted text-xs">{saveLabel(saveStatus)}</span>
          <button
            type="button"
            className="app-button-secondary rounded-full px-3 py-1.5 text-sm"
            onClick={onToggleConsole}
          >
            Console
          </button>
          <button
            type="button"
            className="app-button-primary rounded-full px-4 py-1.5 text-sm"
            onClick={onBuild}
            disabled={buildStatus === "building" || !script}
          >
            {buildStatus === "building" ? "Building..." : "Build"}
          </button>
        </div>
      </div>
      <div className="min-h-0 flex-1">
        {script ? (
          <Editor
            height="100%"
            defaultLanguage="python"
            language="python"
            theme="vs-dark"
            value={scriptText}
            onChange={onChange}
            options={{
              minimap: { enabled: false },
              fontSize: 14,
              scrollBeyondLastLine: false,
              wordWrap: "on",
              automaticLayout: true,
            }}
          />
        ) : (
          <div className="flex h-full items-center justify-center p-6 text-sm app-text-muted">
            Loading script...
          </div>
        )}
      </div>
      {consoleOpen ? (
        <div className="max-h-56 border-t p-4">
          <div className="mb-2 flex items-center justify-between gap-4">
            <span className="text-sm font-medium">{buildStatusLabel(buildStatus)}</span>
            {buildTimestamp ? <span className="app-text-muted text-xs">{buildTimestamp}</span> : null}
          </div>
          <pre className="app-surface-muted max-h-36 overflow-auto rounded-md p-3 text-xs whitespace-pre-wrap">
            {buildOutput || script?.last_build_output || "No build output yet."}
          </pre>
        </div>
      ) : null}
    </section>
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

function saveLabel(status: "idle" | "saved" | "saving" | "unsaved" | "failed") {
  if (status === "saving") return "Saving...";
  if (status === "unsaved") return "Unsaved changes";
  if (status === "failed") return "Save failed";
  if (status === "saved") return "Saved";
  return "";
}

function buildStatusLabel(status: "idle" | "building" | "success" | "failed") {
  if (status === "building") return "Build running";
  if (status === "success") return "Build successful";
  if (status === "failed") return "Build failed";
  return "Build console";
}
