export function LoadingInline(props: { message: string }) {
  return (
    <div className="app-surface-muted rounded-2xl px-4 py-3 text-sm">
      {props.message}
    </div>
  );
}

export function ErrorInline(props: { message: string }) {
  return (
    <div className="app-alert-error rounded-2xl px-4 py-3 text-sm">
      {props.message}
    </div>
  );
}

export function EmptyState(props: { message: string }) {
  return (
    <div
      className="app-text-muted rounded-2xl border border-dashed px-4 py-6 text-center text-sm"
      style={{ borderColor: "var(--color-border)" }}
    >
      {props.message}
    </div>
  );
}

export function RefreshBadge(props: { refreshing: boolean; updatedLabel?: string }) {
  return (
    <span className="app-text-muted text-xs">
      {props.refreshing ? "Refreshing..." : props.updatedLabel ?? "Up to date"}
    </span>
  );
}
