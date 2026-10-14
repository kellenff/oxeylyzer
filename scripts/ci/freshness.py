#!/usr/bin/env python3
"""Freshness gate: direct-dep drift vs msrv-holds.toml (FR-005, FR-006, FR-002a).

Input is the JSON of `cargo outdated --workspace --format json` (so no manual
registry query is ever needed, US1-3). A direct dependency whose latest stable
is a MAJOR ahead of what resolves is an unrecorded gap -> exit 1. A crate named
in msrv-holds.toml is reported "held (recorded)" instead. A hold whose recorded
`required_rust` now fits inside the MSRV window is stale -> exit 1 until removed.
Minor/patch gaps and dev-dependency drift are reported but do not fail (the
contract gates majors).

Fail-closed: unreadable/invalid input exits non-zero with the gate-unavailable
message; it never passes silently.

Usage:
  freshness.py --json outdated.json --holds msrv-holds.toml --floor 1.97 \
               [--summary step_summary.md]
"""
import argparse
import json
import re
import sys
import tomllib
from pathlib import Path
from typing import NoReturn

GATE_DOWN = "dependency gate unavailable (network); build not advisory-verified"


def die(msg: str) -> "NoReturn":
    print(msg, file=sys.stderr)
    sys.exit(1)


def major(version: str) -> int:
    m = re.match(r"(\d+)", version.strip())
    return int(m.group(1)) if m else -1


def rust_minor(value: str) -> tuple[int, int]:
    m = re.match(r"(\d+)\.(\d+)", value.strip())
    if m is None:
        die(f"unparseable rust version: {value!r}")
    return int(m[1]), int(m[2])


def declared_deps(manifest: Path, root: Path) -> dict[str, str]:
    """Dependency names declared by one member manifest, workspace values resolved."""
    table = tomllib.loads(manifest.read_text())
    names: dict[str, str] = {}
    for kind in ("dependencies", "dev-dependencies", "build-dependencies"):
        for key, spec in (table.get(kind) or {}).items():
            if isinstance(spec, dict) and spec.get("workspace"):
                wdeps = tomllib.loads(root.read_text()).get("workspace", {}).get("dependencies", {})
                wspec = wdeps.get(key, {})
                version = wspec.get("version", "workspace") if isinstance(wspec, dict) else str(wspec)
            else:
                version = spec.get("version", "") if isinstance(spec, dict) else str(spec)
            names[key] = version
    return names


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--json", required=True, help="cargo outdated --format json output")
    ap.add_argument("--holds", default="msrv-holds.toml")
    ap.add_argument("--floor", required=True, help="MSRV window floor, e.g. 1.97")
    ap.add_argument("--summary", help="append a markdown table here (GitHub job summary)")
    ap.add_argument("--root", default=".", help="repo root holding the member Cargo.toml files")
    args = ap.parse_args()

    try:
        # `cargo outdated --format json` emits JSONL: one crate object per line.
        data = [json.loads(line) for line in Path(args.json).read_text().splitlines() if line.strip()]
        if not all(isinstance(c, dict) and "dependencies" in c for c in data):
            raise ValueError("expected JSONL crate objects with a 'dependencies' field")
    except (OSError, ValueError) as err:
        die(f"{GATE_DOWN}: cannot read outdated report ({err})")

    holds: dict[str, dict] = {}
    holds_path = Path(args.holds)
    if holds_path.exists():
        try:
            for hold in tomllib.loads(holds_path.read_text()).get("hold", []):
                holds[hold["crate"]] = hold
        except (OSError, ValueError, KeyError) as err:
            die(f"{GATE_DOWN}: cannot read {holds_path} ({err})")

    root = Path(args.root)
    direct = set(declared_deps(root / "Cargo.toml", root / "Cargo.toml"))
    for member in ("oxeylyzer-core", "oxeylyzer-repl", "oxeylyzer-resources", "oxeylyzer-bench"):
        direct |= set(declared_deps(root / member / "Cargo.toml", root / "Cargo.toml"))
    tauri = root / "oxeylyzer-tauri" / "src-tauri" / "Cargo.toml"
    if tauri.exists():
        direct |= set(declared_deps(tauri, root / "Cargo.toml"))

    floor = rust_minor(args.floor)
    rows = []
    failures = []
    seen: set[str] = set()
    for crate in data:
        member = crate.get("crate_name", "?")
        for dep in crate.get("dependencies", []):
            name, project, latest = dep["name"], dep["project"], dep["latest"]
            if name not in direct or "->" in name or latest in ("---", "Removed"):
                continue
            seen.add(name)
            behind_major = major(latest) > major(project) >= 0
            if name in holds:
                hold = holds[name]
                status = "held (recorded)"
                # FR-002a: a hold whose latest now fits the window is stale.
                if rust_minor(hold.get("required_rust", "99.99")) <= floor:
                    failures.append(
                        f"{name}: recorded hold is stale — latest {hold['latest']} "
                        f"needs rust {hold.get('required_rust')}, within floor {args.floor}; remove the hold"
                    )
                    status = "HOLD STALE"
            elif behind_major:
                status = "BEHIND (unrecorded major)"
                failures.append(
                    f"{name}: {project} -> {latest} is an unrecorded major gap "
                    f"(bump it, or record a hold in {args.holds} with a reason)"
                )
            else:
                status = "behind (minor/compat)"
            rows.append((name, member, project, dep.get("compat", "---"), latest, status))

    width = max((len(r[0]) for r in rows), default=4)
    for name, member, project, compat, latest, status in sorted(rows):
        print(f"{name:<{width}}  {member:<22}  {project:>10}  compat {compat:>10}  latest {latest:>10}  {status}")
    for crate in sorted(direct - seen):
        print(f"{crate:<{width}}  (up to date)")

    if args.summary:
        lines = ["## Dependency freshness (current vs available)", "",
                 "| Crate | Manifest | Resolved | Compat | Latest | Status |",
                 "|---|---|---|---|---|---|"]
        lines += ["| " + " | ".join(r) + " |" for r in sorted(rows)]
        try:
            Path(args.summary).write_text("\n".join(lines) + "\n", encoding="utf-8")
        except OSError as err:
            print(f"warning: could not write job summary: {err}", file=sys.stderr)

    if failures:
        print("\n".join(failures), file=sys.stderr)
        return 1
    print("freshness: no unrecorded major gaps; no stale holds")
    return 0


if __name__ == "__main__":
    sys.exit(main())
