import { createSignal } from "solid-js";
import { createStore } from "solid-js/store";

export type HeatScheme = "original" | "playground" | "v2";
export const [heatScheme, setHeatScheme] = createSignal<HeatScheme>("playground");
import { listLayouts, listLanguages, currentLanguage, getCharFrequencies, startupError } from "./api";
import type { DownloadProgress } from "./api";
import type { Layout } from "./types";

type AppStore = {
  layouts: Layout[];
  languages: string[];
  currentLanguage: string;
  /** char → frequency percent (0–100), for heatmap coloring */
  charFrequencies: Record<string, number>;
  loading: boolean;
  error: string | null;
  /** First-run data download progress; null when idle or complete. */
  downloadProgress: DownloadProgress | null;
};

export const [appStore, setAppStore] = createStore<AppStore>({
  layouts: [],
  languages: [],
  currentLanguage: "",
  charFrequencies: {},
  loading: false,
  error: null,
  downloadProgress: null,
});

export async function initStore(): Promise<void> {
  setAppStore("loading", true);
  setAppStore("error", null);

  // Checked before fetching: a startup failure means the commands below will fail too,
  // and the user needs the specific reason rather than a downstream command error.
  const startup = await startupError().catch(() => null);
  if (startup) {
    setAppStore({ error: startup, loading: false });
    return;
  }

  try {
    await fetchStore();
    setAppStore("loading", false);
  } catch (e) {
    setAppStore({ error: String(e), loading: false });
  }
}

/**
 * Refreshes store data without toggling `loading`, so mounted views keep
 * their local state (generation results, comparisons, …) across the refresh.
 */
export async function refreshStore(): Promise<void> {
  try {
    await fetchStore();
  } catch (e) {
    console.error("Store refresh failed:", e);
  }
}

async function fetchStore(): Promise<void> {
  const [layouts, languages, lang, freqList] = await Promise.all([
    listLayouts(),
    listLanguages(),
    currentLanguage(),
    getCharFrequencies(),
  ]);

  const charFrequencies: Record<string, number> = {};
  for (const { char, percent } of freqList) {
    charFrequencies[char] = percent;
  }

  setAppStore({
    layouts,
    languages,
    currentLanguage: lang,
    charFrequencies,
  });
}
