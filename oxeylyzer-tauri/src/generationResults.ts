import type { Layout } from "./types";

export type GenerationResults = {
  revision: number;
  results: Layout[];
  cancelled: boolean;
  reason: string | null;
};

export type GenerationEvent =
  { type: "completed"; payload: GenerationResults } | { type: "engine-changed"; revision: number };

const INVALIDATION_REASON =
  "Generated results were cleared because the language or configuration changed. Please generate again.";

export function generationResultsAfterEvent(
  current: GenerationResults,
  event: GenerationEvent,
): GenerationResults {
  if (event.type === "completed") {
    if (event.payload.revision >= current.revision) return event.payload;
    return current.results.length > 0
      ? current
      : { ...current, cancelled: true, reason: current.reason ?? INVALIDATION_REASON };
  }
  if (event.revision <= current.revision) return current;
  return {
    ...current,
    revision: event.revision,
    results: [],
    reason: current.results.length > 0 ? INVALIDATION_REASON : current.reason,
  };
}
