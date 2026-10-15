#!/usr/bin/env python3
"""Freshness gate: direct-dep drift vs the holds file (FR-005, FR-006, FR-002a).

Rust side: input is the JSON of `cargo outdated --workspace --format json` (so no
manual registry query is ever needed, US1-3). A direct dependency whose latest
stable is a MAJOR ahead of what resolves is an unrecorded gap -> exit 1. A crate
named in msrv-holds.toml is reported "held (recorded)" instead. A hold whose
recorded `required_rust` now fits inside the MSRV window is stale -> exit 1 until
removed. Minor/patch gaps and dev-dependency drift are reported but do not fail
(the contract gates majors).

Frontend side: `oxeylyzer-tauri/package.json` is checked against the npm registry
so FR-006's "Tauri frontend" half is gated too, and the T056 recorded-hold
fallback has somewhere to be surfaced. Same major-gates/minor-reports rule; holds
are read from the same holds file, keyed by package name.

Fail-closed: unreadable/invalid input, or any registry error, exits non-zero with
the gate-unavailable message; it never passes silently.

Usage:
  freshness.py --json outdated.json --holds msrv-holds.toml --floor 1.97 \
               [--frontend oxeylyzer-tauri/package.json] [--summary step_summary.md]
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
                wdeps = (
                    tomllib.loads(root.read_text())
                    .get("workspace", {})
                    .get("dependencies", {})
                )
                wspec = wdeps.get(key, {})
                version = (
                    wspec.get("version", "workspace")
                    if isinstance(wspec, dict)
                    else str(wspec)
                )
            else:
                version = (
                    spec.get("version", "") if isinstance(spec, dict) else str(spec)
                )
            names[key] = version
    return names


def npm_requested(range_: str) -> str:
    """The lowest version a npm range admits, e.g. '^4.2.2' -> '4.2.2'."""
    m = re.search(r"\d+(?:\.\d+)*(?:-[0-9A-Za-z.-]+)?", range_)
    return m.group(0) if m else ""


def npm_satisfies(range_: str, version: str) -> bool | None:
    """Does a fresh install of `range_` resolve to `version` (the registry latest)?

    True means the declared range already admits latest, so the dependency is
    current even though the range's floor is an older number. None means the range
    is a form this gate does not reason about (git:, link:, *, workspace protocols)
    and must be reported rather than guessed at.
    """
    floor = npm_requested(range_)
    if not floor:
        return None
    pinned, got = _triple(floor), _triple(version)
    if got < pinned:
        return False
    if range_.startswith("^"):
        # caret: any non-breaking release above the floor; a 0.x floor pins the minor
        return got[0] == pinned[0] if pinned[0] else got[:2] == pinned[:2]
    if range_.startswith("~"):
        return got[:2] == pinned[:2]
    if range_.startswith(">="):
        return True
    if range_ == floor or range_.startswith("="):
        return got == pinned  # exact pin
    return None  # * , ranges with upper bounds, git:/link: specs: report, don't guess


def _triple(version: str) -> tuple[int, int, int]:
    nums = [int(n) for n in re.findall(r"\d+", version)[:3]]
    nums += [0] * (3 - len(nums))
    return (nums[0], nums[1], nums[2])


def npm_latest(name: str, timeout: float) -> str:
    """Latest stable for one npm package, straight from the registry.

    curl, not urllib: it uses the system trust store, where a framework python on
    macOS has no CA bundle and would fail every query closed. Same tool the MSRV
    floor helper uses (scripts/ci/rust-window.sh).
    """
    import subprocess
    import urllib.parse

    url = f"https://registry.npmjs.org/{urllib.parse.quote(name, safe='')}/latest"
    out = subprocess.run(
        ["curl", "-fsSL", "--max-time", str(int(timeout)), url],
        capture_output=True,
        text=True,
        check=True,
    ).stdout
    version = json.loads(out)["version"]
    if not isinstance(version, str):
        raise ValueError(f"{name}: registry returned a non-string version")
    return version


def frontend_freshness(
    package_json: Path, holds: dict[str, dict], timeout: float
) -> tuple[list[tuple[str, str, str, str]], list[str]]:
    """Rows and gate failures for every direct registry dependency of the frontend.

    A row is (name, requested, latest, status). Fail-closed: any registry error
    stops the gate with the standard message — an unreachable registry must never
    read as "up to date" (FR-005).
    """
    try:
        pkg = json.loads(package_json.read_text())
    except (OSError, ValueError) as err:
        die(f"{GATE_DOWN}: cannot read {package_json} ({err})")
    declared: dict[str, str] = {}
    for section in ("dependencies", "devDependencies"):
        declared.update(pkg.get(section, {}))

    rows: list[tuple[str, str, str, str]] = []
    failures: list[str] = []
    for name, range_ in sorted(declared.items()):
        requested = npm_requested(range_)
        if not requested:
            continue  # git:/file:/link: specs carry no registry version to compare
        try:
            latest = npm_latest(name, timeout)
        except Exception as err:  # noqa: BLE001 - any registry failure is gate-down
            die(f"{GATE_DOWN}: cannot query the npm registry for {name} ({err})")
        # A range that already admits latest means a fresh install IS current, even
        # when the range's floor is an older number (^8.0.0 with 8.3.4 published).
        admits = npm_satisfies(range_, latest)
        if name in holds:
            status = "held (recorded)"
            if admits:
                failures.append(
                    f"{name} (frontend): recorded hold is stale — latest {latest} fits the "
                    f"declared range {range_}; remove the hold or bump it"
                )
                status = "HOLD STALE"
        elif admits:
            status = "up to date"
        elif admits is None:
            status = f"check by hand ({range_} is not a range this gate parses)"
        elif major(latest) > major(requested):
            status = "BEHIND (unrecorded major)"
            failures.append(
                f"{name} (frontend): {requested} -> {latest} is an unrecorded major gap "
                f"(bump {package_json}, or record a hold with a reason)"
            )
        else:
            status = "behind (minor/patch)"
        rows.append((name, requested, latest, status))
    return rows, failures


def _self_check() -> int:
    """Runnable check for the npm range logic, which the gate's verdicts depend on."""
    cases = [
        # (range, latest, expected)
        ("^8.0.0", "8.3.4", True),  # caret admits newer minor/patch -> current
        ("^8.0.0", "9.0.0", False),  # new major -> not admitted
        ("^6.0.3", "8.3.4", False),  # the vite-6 hold shape
        ("^0.34.3", "0.34.5", True),  # 0.x caret still pins the minor
        ("^0.34.3", "0.35.0", False),
        ("~5.6.2", "5.6.9", True),  # tilde: same minor only
        ("~5.6.2", "5.7.0", False),
        ("7.0.0", "7.0.2", False),  # exact pin
        ("7.0.2", "7.0.2", True),
        ("^2", "2.11.14", True),  # bare major caret
        ("^2", "3.0.0", False),
        ("*", "1.2.3", None),  # not a range this gate reasons about
        ("github:a/b", "1.0.0", None),
    ]
    failed = 0
    for range_, latest, want in cases:
        got = npm_satisfies(range_, latest)
        if got != want:
            print(
                f"self-check FAIL: npm_satisfies({range_!r}, {latest!r}) = {got}, want {want}",
                file=sys.stderr,
            )
            failed += 1
    for range_, want in (("^8.0.0", "8.0.0"), ("~5.6.2", "5.6.2"), ("github:a/b", "")):
        got = npm_requested(range_)
        if got != want:
            print(
                f"self-check FAIL: npm_requested({range_!r}) = {got!r}, want {want!r}",
                file=sys.stderr,
            )
            failed += 1
    if not failed:
        print("freshness: npm range self-check passed (%d cases)" % len(cases))
    return 1 if failed else 0


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--json", required=True, help="cargo outdated --format json output")
    ap.add_argument("--holds", default="msrv-holds.toml")
    ap.add_argument("--floor", required=True, help="MSRV window floor, e.g. 1.97")
    ap.add_argument(
        "--summary", help="append a markdown table here (GitHub job summary)"
    )
    ap.add_argument(
        "--root", default=".", help="repo root holding the member Cargo.toml files"
    )
    ap.add_argument(
        "--frontend",
        default="oxeylyzer-tauri/package.json",
        help="frontend manifest to check against the npm registry (empty to skip)",
    )
    ap.add_argument(
        "--timeout", type=float, default=15.0, help="registry timeout, seconds"
    )
    ap.add_argument(
        "--self-check", action="store_true", help="run the npm range check and exit"
    )
    args = ap.parse_args()
    if args.self_check:
        return _self_check()

    try:
        # `cargo outdated --format json` emits JSONL: one crate object per line.
        data = [
            json.loads(line)
            for line in Path(args.json).read_text().splitlines()
            if line.strip()
        ]
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
    for member in (
        "oxeylyzer-core",
        "oxeylyzer-repl",
        "oxeylyzer-resources",
        "oxeylyzer-bench",
    ):
        direct |= set(declared_deps(root / member / "Cargo.toml", root / "Cargo.toml"))
    tauri = root / "oxeylyzer-tauri" / "src-tauri" / "Cargo.toml"
    if tauri.exists():
        direct |= set(declared_deps(tauri, root / "Cargo.toml"))

    # Versions actually present in the lockfile, per name. cargo-outdated emits a
    # row per *resolved copy* of a duplicated crate: a 1.9.3 transitive copy shows
    # up even though the direct requirement resolves to 2.14.2. Only a name with no
    # copy at `compat` anywhere in the lock is real drift of the direct requirement.
    lock_versions: dict[str, list[str]] = {}
    try:
        lock = tomllib.loads((root / "Cargo.lock").read_text())
        for pkg in lock.get("package", []):
            lock_versions.setdefault(pkg["name"], []).append(pkg["version"])
    except (OSError, ValueError) as err:
        die(f"{GATE_DOWN}: cannot read Cargo.lock ({err})")

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
            compat = dep.get("compat", "---")
            if compat not in ("---", "Removed") and compat in lock_versions.get(
                name, []
            ):
                continue  # direct requirement resolves at compat; stale transitive copy, not our drift
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
            rows.append((name, member, project, compat, latest, status))

    width = max((len(r[0]) for r in rows), default=4)
    for name, member, project, compat, latest, status in sorted(rows):
        print(
            f"{name:<{width}}  {member:<22}  {project:>10}  compat {compat:>10}  latest {latest:>10}  {status}"
        )
    for crate in sorted(direct - seen):
        print(f"{crate:<{width}}  (up to date)")

    # Frontend (FR-006 names the Tauri frontend too, and SC-008 forbids a manual
    # registry query): the same current-vs-available answer for package.json.
    fe_rows: list[tuple[str, str, str, str]] = []
    if args.frontend:
        manifest = root / args.frontend
        if manifest.exists():
            fe_rows, fe_failures = frontend_freshness(manifest, holds, args.timeout)
            failures += fe_failures
            fe_width = max((len(r[0]) for r in fe_rows), default=4)
            print()
            for name, requested, latest, status in sorted(fe_rows):
                print(
                    f"{name:<{fe_width}}  {'frontend':<22}  {requested:>10}  latest {latest:>10}  {status}"
                )

    if args.summary:
        lines = [
            "## Dependency freshness (current vs available)",
            "",
            "| Crate | Manifest | Resolved | Compat | Latest | Status |",
            "|---|---|---|---|---|---|",
        ]
        lines += ["| " + " | ".join(r) + " |" for r in sorted(rows)]
        lines += [
            "| " + " | ".join((r[0], "frontend", r[1], "---", r[2], r[3])) + " |"
            for r in sorted(fe_rows)
        ]
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
