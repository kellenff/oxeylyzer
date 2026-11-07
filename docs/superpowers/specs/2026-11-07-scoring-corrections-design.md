# Scoring corrections for issues #5–#8

## Intent

Make generation and displayed statistics reflect the configured scoring weights, including zero weights, while preserving established nonzero-weight behavior.

## Scope and behavior

- **#5:** Order generation trigrams by descending frequency, with a deterministic character-based tie break, before applying precision limits. Full statistics continue to include all trigrams.
- **#6:** Make full-precision `score()` use the same components as the displayed score, including stretch. Preserve the configured precision used by the optimizer and its cache.
- **#7:** Calculate the Pinky-Ring percentage from unweighted pair counts. Its descriptive value remains independent of the scoring weight.
- **#8:** Preserve the existing formula when SFB weight is nonzero. When SFB is zero, apply the configured skipgram weight equally to same-finger skipgrams at all three distances. For stretch scoring in that case, count direct bigrams and, when the skipgram weight is nonzero, all three skipgram distances equally before applying the configured stretch weight. A zero skipgram weight excludes skipgrams. This explicitly defines the formerly undefined case; it is discontinuous at zero SFB.

The changes do not redesign the general scoring model or change the meaning of other settings. Rankings can change where the existing behavior was incorrect.

## Approaches

For zero SFB, we considered a special-case rule and a broader independently defined decay model. The user chose the special-case rule because it preserves working nonzero-SFB configurations. The other three issues use the existing scoring and statistics paths rather than introducing a new model.

## Verification

Add focused regressions for frequency ordering and deterministic ties, agreement between full scores and displayed scores including a stretch-sensitive example, a finite weight-independent Pinky-Ring statistic, and zero SFB with enabled and disabled skipgrams for both same-finger and stretch calculations. Check that nonzero-weight results retain existing behavior and that cached scores agree with precision-limited scoring. Run the relevant core tests and formatting checks; assess GUI/CLI integration through their existing score callers.
