export function safeParseJson<T>(value: string | null, fallback: T): T {
  if (!value) {
    return fallback;
  }

  try {
    return JSON.parse(value) as T;
  } catch {
    return fallback;
  }
}

export function readStoredJson<T>(key: string, fallback: T): T {
  if (typeof window === "undefined") {
    return fallback;
  }

  return safeParseJson(window.localStorage.getItem(key), fallback);
}

export function writeStoredJson<T>(key: string, value: T) {
  if (typeof window === "undefined") {
    return;
  }

  const nextValue = JSON.stringify(value);
  if (window.localStorage.getItem(key) === nextValue) {
    return;
  }

  window.localStorage.setItem(key, nextValue);
}

export function subscribeStoredJson<T>(
  key: string,
  onChange: (value: T) => void,
  fallback: T,
) {
  if (typeof window === "undefined") {
    return () => undefined;
  }

  const sync = () => {
    onChange(readStoredJson(key, fallback));
  };

  const handleStorage = (event: StorageEvent) => {
    if (event.key === key) {
      sync();
    }
  };

  window.addEventListener("storage", handleStorage);

  return () => {
    window.removeEventListener("storage", handleStorage);
  };
}
