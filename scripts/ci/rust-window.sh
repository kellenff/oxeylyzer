#!/usr/bin/env bash
# Compute the MSRV floor: the oldest of the three newest stable Rust releases.
# Feed truth is https://static.rust-lang.org/dist/channel-rust-stable.toml; the
# `rust-version` in the root Cargo.toml is only a mirror of what this prints.
# Fail-closed (FR-001): a network problem must turn the build red, never green.
set -euo pipefail

FEED="https://static.rust-lang.org/dist/channel-rust-stable.toml"

body=$(curl -fsSL --max-time 30 "$FEED" 2>/dev/null) || {
  echo "dependency gate unavailable (network); build not advisory-verified" >&2
  exit 1
}

# `version = "1.99.0 (abc...)"` under [pkg.rust] -> "1.99.0", then "1.99".
latest=$(printf '%s\n' "$body" \
  | sed -n '/^\[pkg\.rust\]/,/^\[/p' \
  | sed -n 's/^version = "\([0-9]*\.[0-9]*\)\.[0-9]* .*/\1/p' \
  | head -1)

if [[ -z "$latest" ]]; then
  echo "dependency gate unavailable (network); build not advisory-verified" >&2
  exit 1
fi

# The window is three stable releases; the floor is the oldest of them.
# shortcut: the feed only names the current stable, so the two prior releases are
# latest-1 and latest-2. Breaks if Rust ever ships a 2.0 major; revisit then.
floor_minor=$(( ${latest##*.} - 2 ))
if (( floor_minor < 0 )); then
  echo "dependency gate unavailable (network); build not advisory-verified" >&2
  exit 1
fi

echo "${latest%%.*}.${floor_minor}"
