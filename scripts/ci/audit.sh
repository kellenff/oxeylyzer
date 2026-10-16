#!/usr/bin/env bash
# Advisory gate (FR-004, contracts/ci-gate.md step 6), identical locally and in CI.
#
# Why this script exists: cargo-audit 0.22 has no `--config` flag. It reads
# `./.cargo/audit.toml`, which this repo gitignores, so a bare `cargo audit` on a
# fresh clone audits with NO allowance list (exit 1 on the recorded transitive
# advisories) while the same tree passes on a machine where someone happens to have
# remembered the copy step. The allowance list is only authoritative if the gate
# actually reads it; staging it here makes local and CI agree by construction.
#
# `--deny warnings` fails on every advisory category, not just unsound: FR-004 says
# an unrecorded advisory MUST fail the build, and an *unmaintained* notice is an
# advisory. What is accepted is recorded in audit.toml with a reason, not silenced.
#
# Usage: scripts/ci/audit.sh [allowance-list]   (default: audit.toml at repo root)
set -euo pipefail

config="${1:-audit.toml}"
stage=".cargo/audit.toml"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

if [ ! -f "$config" ]; then
	echo "cannot read the allowance list $config" >&2
	exit 1
fi

# Preserve anything already at the stage path: a personal .cargo/audit.toml stays a
# personal one, restored after the run rather than quietly replaced by this script.
[ -f "$stage" ] && mv "$stage" "$tmp/original"
cleanup() {
	rm -f "$stage"
	[ -f "$tmp/original" ] && mv "$tmp/original" "$stage"
}
trap 'cleanup; rm -rf "$tmp"' EXIT

mkdir -p .cargo
cp "$config" "$stage"
cargo audit --deny warnings
