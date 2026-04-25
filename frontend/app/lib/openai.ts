export const OPENAI_API_KEY_STORAGE_KEY = "desk-openai-api-key";
export const OPENAI_SETTINGS_CHANGE_EVENT = "desk-openai-settings-change";
export const OPENAI_DEFAULT_MODEL = "gpt-5";

export function getStoredOpenAIApiKey() {
  if (typeof window === "undefined") {
    return "";
  }

  window.localStorage.removeItem(OPENAI_API_KEY_STORAGE_KEY);
  return window.sessionStorage.getItem(OPENAI_API_KEY_STORAGE_KEY) ?? "";
}

export function saveOpenAIApiKey(apiKey: string) {
  if (typeof window === "undefined") {
    return;
  }

  const trimmed = apiKey.trim();
  window.localStorage.removeItem(OPENAI_API_KEY_STORAGE_KEY);
  if (trimmed) {
    window.sessionStorage.setItem(OPENAI_API_KEY_STORAGE_KEY, trimmed);
  } else {
    window.sessionStorage.removeItem(OPENAI_API_KEY_STORAGE_KEY);
  }

  window.dispatchEvent(
    new CustomEvent(OPENAI_SETTINGS_CHANGE_EVENT, {
      detail: trimmed ? "saved" : "cleared",
    }),
  );
}
