import { createStore } from "solid-js/store";
import type { listen } from "@tauri-apps/api/event";
import type { cancelGenerate, saveGenerated, startGenerate } from "./api";
import { generationResultsAfterEvent } from "./generationResults";
import type { GenerationEvent, GenerationResults } from "./generationResults";

type Progress = { done: number; total: number };
type SaveState = { name: string; saved: boolean };
type Dependencies = {
  listen: typeof listen;
  startGenerate: typeof startGenerate;
  cancelGenerate: typeof cancelGenerate;
  saveGenerated: typeof saveGenerated;
};

/** Own generation for the app lifetime, independently of the mounted view. */
export function createGenerationStore(deps: Dependencies) {
  const [state, setState] = createStore({
    running: false,
    cancelling: false,
    progress: null as Progress | null,
    generationResults: {
      revision: 0,
      results: [],
      cancelled: false,
      reason: null,
    } as GenerationResults,
    saveStates: [] as SaveState[],
    error: "",
  });
  let unlisteners: (() => void)[] = [];
  let ready: Promise<void> | undefined;
  let lifetime = 0;
  let resultsVersion = 0;
  let requestVersion = 0;

  function applyEvent(event: GenerationEvent) {
    const current = state.generationResults;
    const next = generationResultsAfterEvent(current, event);
    if (next !== current) {
      resultsVersion++;
      setState({
        generationResults: next,
        saveStates: next.results.map((layout) => ({ name: layout.name, saved: false })),
      });
    }
  }

  function init(): Promise<void> {
    if (ready) return ready;
    const currentLifetime = lifetime;
    ready = (async () => {
      const registered: (() => void)[] = [];
      try {
        registered.push(
          await deps.listen<number>("engine-changed", (event) => {
            applyEvent({ type: "engine-changed", revision: event.payload });
          }),
        );
        registered.push(
          await deps.listen<Progress>("generate-progress", (event) => {
            setState("progress", event.payload);
          }),
        );
        registered.push(
          await deps.listen<GenerationResults>("generate-done", (event) => {
            applyEvent({ type: "completed", payload: event.payload });
            setState({ running: false, cancelling: false, progress: null });
          }),
        );
        if (currentLifetime !== lifetime) {
          registered.forEach((unlisten) => unlisten());
        } else {
          unlisteners = registered;
        }
      } catch (error) {
        registered.forEach((unlisten) => unlisten());
        ready = undefined;
        throw error;
      }
    })();
    return ready;
  }

  function dispose() {
    lifetime++;
    unlisteners.forEach((unlisten) => unlisten());
    unlisteners = [];
    ready = undefined;
  }

  async function start(base: string, count: number, pins: string, algorithm: string) {
    if (state.running) return;
    const request = ++requestVersion;
    resultsVersion++;
    setState({
      running: true,
      cancelling: false,
      generationResults: {
        revision: state.generationResults.revision,
        results: [],
        cancelled: false,
        reason: null,
      },
      saveStates: [],
      progress: { done: 0, total: count },
      error: "",
    });
    try {
      await init();
      await deps.startGenerate(base, count, pins, algorithm);
    } catch (error) {
      if (request === requestVersion && state.running) {
        setState({ error: String(error), running: false, cancelling: false, progress: null });
      }
    }
  }

  async function cancel() {
    if (!state.running || state.cancelling) return;
    setState("cancelling", true);
    try {
      await deps.cancelGenerate();
    } catch (error) {
      setState({ error: String(error), cancelling: false });
    }
  }

  function updateSaveName(index: number, name: string) {
    if (state.saveStates[index]) setState("saveStates", index, "name", name);
  }

  async function save(index: number) {
    const saveState = state.saveStates[index];
    if (!saveState) return;
    const version = resultsVersion;
    try {
      await deps.saveGenerated(index, saveState.name || undefined);
      if (version === resultsVersion) setState("saveStates", index, "saved", true);
    } catch (error) {
      if (version === resultsVersion) setState("error", String(error));
    }
  }

  return { state, init, dispose, start, cancel, save, updateSaveName };
}
