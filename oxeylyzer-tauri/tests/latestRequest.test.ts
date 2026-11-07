import assert from "node:assert/strict";
import { test } from "node:test";
import { latestRequest } from "../src/latestRequest.ts";

test("an older response cannot overwrite a newer response", async () => {
  const pending: Array<(value: string) => void> = [];
  const published: string[] = [];
  const refresh = latestRequest(
    () => new Promise<string>((resolve) => pending.push(resolve)),
    (value) => published.push(value),
  );
  const older = refresh();
  const newer = refresh();
  pending[1]("new");
  await newer;
  pending[0]("old");
  await older;
  assert.deepEqual(published, ["new"]);
});

test("an older successful response is published when the newer request fails", async () => {
  const pending: Array<{ resolve: (value: string) => void; reject: (error: Error) => void }> = [];
  const published: string[] = [];
  const refresh = latestRequest(
    () => new Promise<string>((resolve, reject) => pending.push({ resolve, reject })),
    (value) => published.push(value),
  );
  const older = refresh();
  const newer = refresh();
  pending[1].reject(new Error("offline"));
  await assert.rejects(newer, /offline/);
  pending[0].resolve("available");
  await older;
  assert.deepEqual(published, ["available"]);
});
