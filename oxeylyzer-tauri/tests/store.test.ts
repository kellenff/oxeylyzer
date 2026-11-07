import { mock, test, expect } from "bun:test";
import type { StoreSnapshot } from "../src/api";

const pending: Array<{
  resolve: (value: StoreSnapshot) => void;
  reject: (error: Error) => void;
}> = [];
mock.module("../src/api", () => ({
  getStoreSnapshot: () =>
    new Promise<StoreSnapshot>((resolve, reject) => pending.push({ resolve, reject })),
}));
const { appStore, initStore, refreshStore } = await import("../src/store");
const snapshot: StoreSnapshot = {
  layouts: [],
  languages: ["english"],
  currentLanguage: "english",
  charFrequencies: {},
};

test("a successful refresh recovers from an earlier startup failure", async () => {
  const startup = initStore();
  const refresh = refreshStore();
  pending.shift()!.reject(new Error("startup failed"));
  await startup;
  pending.shift()!.resolve(snapshot);
  await refresh;
  expect(appStore.error).toBeNull();
  expect(appStore.currentLanguage).toBe("english");
});

test("a startup failure cannot replace an already successful refresh", async () => {
  const startup = initStore();
  const refresh = refreshStore();
  const older = pending.shift()!;
  pending.shift()!.resolve(snapshot);
  await refresh;
  older.reject(new Error("startup failed"));
  await startup;
  expect(appStore.error).toBeNull();
  expect(appStore.currentLanguage).toBe("english");
});
