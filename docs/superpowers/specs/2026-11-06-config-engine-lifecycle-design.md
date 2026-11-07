# Configuration and engine lifecycle

## Goal and scope

Resolve GitHub issues #4, #11, #12, and #18. Preserve the last working configuration on failed saves, maintain the session language, prevent stale generation results, and reload the latest external edits after a burst. Generation state across navigation (#13) is outside this batch.

## Approaches

Patch each replacement path independently, or coordinate changes through a shared state transition. The shared transition is recommended: it gives configuration, engine, layouts, and generated results one consistent ordering and gives generation publication a reliable current-engine check. Independent patches would duplicate concurrency rules across three paths.

## Behavior

- Switching languages changes the active corpus for this session only. It does not change the persisted corpus path.
- Saving unrelated settings preserves the active session language and persists the configured corpus path, not the session selection. A failed preparation or save leaves both the previously saved configuration and active state intact.
- An external edit that changes the persisted corpus path selects that new corpus. External edits that leave that path unchanged preserve the session language. The app's own save must not be mistaken for an external corpus selection.
- An engine change invalidates existing generated results and requests cancellation of a run based on the old engine. The worker stops at the next batch boundary, discards its results, releases the running state, and reports why it stopped. A stale worker must never publish results or make them available for saving, including when engine replacement races with completion.
- File events are debounced with a trailing reload. A burst can contain both config and layout changes; neither category is lost. A failed read does not prevent a subsequent valid write from being loaded. Atomic config replacement must not detach future config monitoring.

## Design constraints

Prepare the replacement engine and layouts before committing a change. Write a temporary file in the configuration directory and atomically replace the saved file before publishing the prepared state; handle write and replacement failures without publishing it. Coordinate concurrent reloads, saves, language changes, generation startup, and generation completion so they cannot publish mismatched state. Do not hold a shared lock during expensive generation. Keep external filesystem changes observable and avoid watcher feedback turning an internal settings save into a language change.

The existing generation completion event should finish the UI lifecycle for an invalidated run and communicate its reason; ordinary cancellation retains its existing partial-result behavior. Preserve existing error reporting and managed paths.

## Verification

Use deterministic focused tests for a failed save preserving file and active state; session language across settings save and self-generated watcher events; external changed and unchanged corpus paths; stale generation completion racing an engine transition; and trailing watcher bursts containing config and layout events, including an invalid write followed by a valid one. Exercise atomic replacement and subsequent file monitoring with a filesystem test where feasible. Run relevant format and test checks and report whether native desktop behavior was exercised.
