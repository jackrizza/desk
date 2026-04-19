import { useEffect, useState } from "react";

export const CHAT_OPEN_STORAGE_KEY = "desk-chat-open";
export const UI_STATE_CHANGE_EVENT = "desk-ui-state-change";

function getStoredBoolean(key: string, fallback: boolean) {
  if (typeof window === "undefined") {
    return fallback;
  }

  const raw = window.localStorage.getItem(key);
  if (raw === "true") {
    return true;
  }
  if (raw === "false") {
    return false;
  }

  return fallback;
}

function setStoredBoolean(key: string, value: boolean) {
  if (typeof window === "undefined") {
    return;
  }

  window.localStorage.setItem(key, String(value));
  window.dispatchEvent(
    new CustomEvent(UI_STATE_CHANGE_EVENT, {
      detail: { key, value },
    }),
  );
}

export function usePersistentBoolean(
  key: string,
  fallback: boolean,
): [boolean, (value: boolean | ((current: boolean) => boolean)) => void] {
  const [value, setValue] = useState<boolean>(() => getStoredBoolean(key, fallback));

  useEffect(() => {
    const sync = () => {
      setValue(getStoredBoolean(key, fallback));
    };

    const handleStorage = (event: StorageEvent) => {
      if (event.key === key) {
        sync();
      }
    };

    const handleCustomState = (event: Event) => {
      const customEvent = event as CustomEvent<{ key?: string }>;
      if (customEvent.detail?.key === key) {
        sync();
      }
    };

    window.addEventListener("storage", handleStorage);
    window.addEventListener(UI_STATE_CHANGE_EVENT, handleCustomState as EventListener);

    return () => {
      window.removeEventListener("storage", handleStorage);
      window.removeEventListener(
        UI_STATE_CHANGE_EVENT,
        handleCustomState as EventListener,
      );
    };
  }, [fallback, key]);

  function update(next: boolean | ((current: boolean) => boolean)) {
    const resolved =
      typeof next === "function"
        ? (next as (current: boolean) => boolean)(getStoredBoolean(key, fallback))
        : next;

    setValue(resolved);
    setStoredBoolean(key, resolved);
  }

  return [value, update];
}
