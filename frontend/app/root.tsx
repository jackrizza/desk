import {
  isRouteErrorResponse,
  Links,
  Meta,
  Outlet,
  Scripts,
  ScrollRestoration,
  useNavigation,
} from "react-router";

import type { Route } from "./+types/root";
import "./app.css";

export const links: Route.LinksFunction = () => [
  { rel: "preconnect", href: "https://fonts.googleapis.com" },
  {
    rel: "preconnect",
    href: "https://fonts.gstatic.com",
    crossOrigin: "anonymous",
  },
  {
    rel: "stylesheet",
    href: "https://fonts.googleapis.com/css2?family=Inter:ital,opsz,wght@0,14..32,100..900;1,14..32,100..900&display=swap",
  },
];

export function Layout({ children }: { children: React.ReactNode }) {
  const themeBootstrapScript = `
    (function() {
      try {
        var key = "desk-theme";
        var saved = localStorage.getItem(key);
        var prefersDark = window.matchMedia("(prefers-color-scheme: dark)").matches;
        var useDark = saved ? saved === "dark" : prefersDark;
        document.documentElement.classList.toggle("dark", useDark);
      } catch (e) {
        // no-op
      }
    })();
  `;

  return (
    <html lang="en">
      <head>
        <meta charSet="utf-8" />
        <meta name="viewport" content="width=device-width, initial-scale=1" />
        <Meta />
        <Links />
        <script dangerouslySetInnerHTML={{ __html: themeBootstrapScript }} />
      </head>
      <body>
        {children}
        <ScrollRestoration />
        <Scripts />
      </body>
    </html>
  );
}

export default function App() {
  const navigation = useNavigation();
  const isNavigating =
    navigation.state !== "idle" && navigation.location != null;

  return (
    <>
      <Outlet />
      {isNavigating ? <RouteLoadingOverlay /> : null}
    </>
  );
}

function RouteLoadingOverlay() {
  return (
    <div className="app-route-loading fixed inset-0 z-50 flex items-center justify-center px-6">
      <div className="app-route-loading-panel flex w-full max-w-md flex-col items-center gap-4 rounded-3xl px-8 py-10 text-center shadow-xl">
        <div className="app-route-loading-spinner h-12 w-12 rounded-full" />
        <div className="space-y-2">
          <p className="text-xs font-semibold uppercase tracking-[0.24em] app-text-muted">
            Loading Route
          </p>
          <h2 className="text-2xl font-semibold">Opening workspace</h2>
          <p className="app-text-muted text-sm leading-6">
            Pulling together the next view.
          </p>
        </div>
      </div>
    </div>
  );
}

export function ErrorBoundary({ error }: Route.ErrorBoundaryProps) {
  let message = "Oops!";
  let details = "An unexpected error occurred.";
  let stack: string | undefined;

  if (isRouteErrorResponse(error)) {
    message = error.status === 404 ? "404" : "Error";
    details =
      error.status === 404
        ? "The requested page could not be found."
        : error.statusText || details;
  } else if (import.meta.env.DEV && error && error instanceof Error) {
    details = error.message;
    stack = error.stack;
  }

  return (
    <main className="pt-16 p-4 container mx-auto">
      <h1>{message}</h1>
      <p>{details}</p>
      {stack && (
        <pre className="w-full p-4 overflow-x-auto">
          <code>{stack}</code>
        </pre>
      )}
    </main>
  );
}
