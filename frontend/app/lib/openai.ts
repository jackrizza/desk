export const OPENAI_API_KEY_STORAGE_KEY = "desk-openai-api-key";
export const OPENAI_SETTINGS_CHANGE_EVENT = "desk-openai-settings-change";
export const OPENAI_DEFAULT_MODEL = "gpt-5";

export function getStoredOpenAIApiKey() {
  if (typeof window === "undefined") {
    return "";
  }

  return window.localStorage.getItem(OPENAI_API_KEY_STORAGE_KEY) ?? "";
}

export function saveOpenAIApiKey(apiKey: string) {
  if (typeof window === "undefined") {
    return;
  }

  const trimmed = apiKey.trim();
  if (trimmed) {
    window.localStorage.setItem(OPENAI_API_KEY_STORAGE_KEY, trimmed);
  } else {
    window.localStorage.removeItem(OPENAI_API_KEY_STORAGE_KEY);
  }

  window.dispatchEvent(
    new CustomEvent(OPENAI_SETTINGS_CHANGE_EVENT, {
      detail: trimmed ? "saved" : "cleared",
    }),
  );
}
