import { expect, mock, test } from "bun:test";
import { createRoot } from "solid-js";
import { createGenerationStore } from "../src/generationStore";
import type { GenerationResults } from "../src/generationResults";
import type { Layout } from "../src/types";

const candidate = { name: "candidate" } as Layout;
const completed: GenerationResults = {
  revision: 1,
  results: [candidate],
  cancelled: false,
  reason: null,
};

function fixture() {
  const handlers = new Map<string, (event: { payload: unknown }) => void>();
  const unlisten = mock(() => {});
  const startGenerate = mock(async () => {});
  const cancelGenerate = mock(async () => {});
  const saveGenerated = mock(async (_index: number, _name?: string) => candidate);
  const deps: Parameters<typeof createGenerationStore>[0] = {
    listen: async (name, callback) => {
      handlers.set(name, callback as (event: { payload: unknown }) => void);
      return () => {
        handlers.delete(name);
        unlisten();
      };
    },
    startGenerate,
    cancelGenerate,
    saveGenerated,
  };
  const store = createGenerationStore(deps);
  return {
    store,
    deps,
    handlers,
    unlisten,
    startGenerate,
    cancelGenerate,
    saveGenerated,
    emit: (event: string, payload: unknown) => handlers.get(event)?.({ payload }),
    mountView: () => createRoot((dispose) => ({ state: store.state, unmount: dispose })),
  };
}

test("navigation during generation preserves running state and receives progress and completion", async () => {
  const f = fixture();
  await f.store.init();
  await f.store.init();
  expect(f.handlers.size).toBe(3);
  const firstView = f.mountView();
  await f.store.start("qwerty", 100, "ae", "hill");
  firstView.unmount();
  f.emit("generate-progress", { done: 40, total: 100 });
  const returningView = f.mountView();
  expect(returningView.state.running).toBe(true);
  expect(returningView.state.progress).toEqual({ done: 40, total: 100 });
  returningView.unmount();
  f.emit("generate-done", completed);
  const completedView = f.mountView();
  expect(completedView.state.running).toBe(false);
  expect(completedView.state.progress).toBeNull();
  expect(completedView.state.generationResults.results).toEqual([candidate]);
  expect(completedView.state.saveStates).toEqual([{ name: "candidate", saved: false }]);
  expect(f.unlisten).not.toHaveBeenCalled();
  expect(f.startGenerate).toHaveBeenCalledWith("qwerty", 100, "ae", "hill");
  completedView.unmount();
  f.store.dispose();
  expect(f.unlisten).toHaveBeenCalledTimes(3);
});

test("navigation after completion preserves save names and saved status", async () => {
  const f = fixture();
  await f.store.start("qwerty", 1, "", "hill");
  f.emit("generate-done", completed);
  const view = f.mountView();
  f.store.updateSaveName(0, "my-layout");
  await f.store.save(0);
  view.unmount();
  const returned = f.mountView();
  expect(returned.state.saveStates[0]).toEqual({ name: "my-layout", saved: true });
  expect(f.saveGenerated).toHaveBeenCalledWith(0, "my-layout");
  returned.unmount();
  f.store.dispose();
});

test("engine changes while Generate is absent invalidate results and reject stale completion", async () => {
  const f = fixture();
  await f.store.start("qwerty", 1, "", "hill");
  f.emit("generate-done", completed);
  f.emit("engine-changed", 2);
  f.emit("generate-done", completed);
  expect(f.store.state.generationResults.results).toEqual([]);
  expect(f.store.state.generationResults.reason).toBeTruthy();
  expect(f.store.state.saveStates).toEqual([]);
  f.store.dispose();
});

test("cancellation remains active until done and keeps partial results", async () => {
  const f = fixture();
  await f.store.start("qwerty", 100, "", "hill");
  await f.store.cancel();
  expect(f.store.state.running).toBe(true);
  expect(f.store.state.cancelling).toBe(true);
  f.emit("generate-done", { ...completed, cancelled: true });
  expect(f.store.state.cancelling).toBe(false);
  expect(f.store.state.generationResults.results).toEqual([candidate]);
  expect(f.store.state.generationResults.cancelled).toBe(true);
  f.store.dispose();
});

test("failed listener registration cleans up and permits a retry", async () => {
  const f = fixture();
  const listen = f.deps.listen;
  let calls = 0;
  f.deps.listen = async (...args) => {
    if (++calls === 2) throw new Error("listen failed");
    return listen(...args);
  };
  await f.store.start("qwerty", 1, "", "hill");
  expect(f.store.state.running).toBe(false);
  expect(f.store.state.error).toContain("listen failed");
  expect(f.unlisten).toHaveBeenCalledTimes(1);
  expect(f.startGenerate).not.toHaveBeenCalled();
  await f.store.start("qwerty", 1, "", "hill");
  expect(f.store.state.running).toBe(true);
  expect(f.handlers.size).toBe(3);
  f.store.dispose();
});

test("a save completing after a newer run cannot mark that run's cards saved", async () => {
  const f = fixture();
  await f.store.start("qwerty", 1, "", "hill");
  f.emit("generate-done", completed);
  let finishSave!: (layout: Layout) => void;
  f.saveGenerated.mockImplementationOnce(
    () =>
      new Promise((resolve) => {
        finishSave = resolve;
      }),
  );
  const saving = f.store.save(0);
  await f.store.start("qwerty", 1, "", "hill");
  f.emit("generate-done", completed);
  finishSave(candidate);
  await saving;
  expect(f.store.state.saveStates[0].saved).toBe(false);
  f.store.dispose();
});

test("a start failure after engine invalidation still clears running state", async () => {
  const f = fixture();
  await f.store.init();
  let rejectStart!: (reason: Error) => void;
  f.startGenerate.mockImplementationOnce(
    () =>
      new Promise((_, reject) => {
        rejectStart = reject;
      }),
  );
  const starting = f.store.start("missing", 1, "", "hill");
  await Promise.resolve();
  f.emit("engine-changed", 2);
  rejectStart(new Error("Layout not found"));
  await starting;
  expect(f.store.state.running).toBe(false);
  expect(f.store.state.error).toContain("Layout not found");
  f.store.dispose();
});
