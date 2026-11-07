import assert from "node:assert/strict";
import { test } from "node:test";
import { generationResultsAfterEvent } from "../src/generationResults.ts";
import type { GenerationResults } from "../src/generationResults.ts";
import type { Layout } from "../src/types.ts";

const layout: Layout = {
  name: "partial-layout",
  keys: "abcdefghijklmnopqrstuvwxyz,.;/",
  board: "generated",
  stats: {
    sfb: 1,
    dsfb: 2,
    fspeed: 3,
    finger_speed: Array(10).fill(1),
    finger_usage: Array(10).fill(10),
    stretches: 4,
    scissors: 5,
    lsbs: 6,
    pinky_ring: 7,
    score: 8,
    inrolls: 9,
    outrolls: 10,
    onehands: 11,
    alternates: 12,
    alternates_sfs: 13,
    redirects: 14,
    redirects_sfs: 15,
    bad_redirects: 16,
    bad_redirects_sfs: 17,
    bad_sfbs: 18,
    sfts: 19,
  },
  keyboard: [[0, 0, 1, 1]],
  shape: [10, 10, 10],
};

const completed: GenerationResults = {
  revision: 1,
  results: [layout],
  cancelled: false,
  reason: null,
};

test("engine replacement clears completed cards and explains why", () => {
  const next = generationResultsAfterEvent(completed, { type: "engine-changed", revision: 2 });
  assert.deepEqual(next.results, []);
  assert.ok(next.reason);
  assert.equal(next.revision, 2);
});

test("completion delivered after its engine was replaced cannot restore cards", () => {
  const invalidated = generationResultsAfterEvent(completed, {
    type: "engine-changed",
    revision: 2,
  });
  const next = generationResultsAfterEvent(invalidated, { type: "completed", payload: completed });
  assert.deepEqual(next.results, []);
  assert.ok(next.reason);
});

test("delayed invalidation cannot clear a newer engine's completed results", () => {
  const next = generationResultsAfterEvent(completed, { type: "engine-changed", revision: 1 });
  assert.deepEqual(next.results, [layout]);
  assert.equal(next.reason, null);
});

test("ordinary cancellation preserves nonempty partial cards", () => {
  const next = generationResultsAfterEvent(
    { ...completed, results: [] },
    { type: "completed", payload: { ...completed, cancelled: true } },
  );
  assert.deepEqual(next.results, [layout]);
  assert.equal(next.cancelled, true);
  assert.equal(next.reason, null);
});

test("late stale-worker completion cannot clear newer completed cards", () => {
  const next = generationResultsAfterEvent(completed, {
    type: "completed",
    payload: { revision: 0, results: [], cancelled: true, reason: "Engine changed." },
  });
  assert.deepEqual(next.results, [layout]);
  assert.equal(next.reason, null);
});

test("replacement during a run explains a delayed obsolete completion", () => {
  const running = { ...completed, results: [] };
  const invalidated = generationResultsAfterEvent(running, { type: "engine-changed", revision: 2 });
  const next = generationResultsAfterEvent(invalidated, { type: "completed", payload: completed });
  assert.deepEqual(next.results, []);
  assert.ok(next.reason);
  assert.equal(next.cancelled, true);
});

test("delayed invalidation preserves the stale worker's cancellation reason", () => {
  const stopped = { ...completed, results: [], cancelled: true, reason: "Engine changed." };
  const next = generationResultsAfterEvent(stopped, { type: "engine-changed", revision: 2 });
  assert.equal(next.cancelled, true);
  assert.equal(next.reason, "Engine changed.");
});
