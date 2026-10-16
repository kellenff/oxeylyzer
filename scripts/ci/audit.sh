#!/usr/bin/env bash
# Advisory gate (FR-004, contracts/ci-gate.md step 6), identical locally and in CI.
#
# Why this script exists: cargo-audit 0.22 has no `--config` flag. It reads
# `./.cargo/audit.toml`, which this repo gitignores, so a bare `cargo audit` on a
# fresh checkout audits with NO allowance list (exit 1 on the recorded transitive
# advisories) while the same tree passes on a machine where someone happens to have
# the staged file left over. The allowance list is only authoritative if the gate
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

if [ ! -f "$config" ]; then
	echo "cannot read the allowance list $config" >&2
	exit 1
fi

# Preserve anything already at the stage path: a personal .cargo/audit.toml stays a
# personal one, restored after the run rather than quietly replaced by this script.
if [ -f "$stage" ]; then
	mv "$stage" "$tmp/original"
fi

cleanup() {
	rm -f "$stage"
	if [ -f "$tmp/original" ]; then
		mv "$tmp/original" "$stage"
	fi
	rm -rf "$tmp"
}
trap cleanup EXIT

mkdir -p .cargo
cp "$config" "$stage"
cargo audit --deny warnings
