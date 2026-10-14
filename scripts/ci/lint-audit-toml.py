#!/usr/bin/env python3
"""Lint the audit.toml allowance list (FR-004, contracts/ci-gate.md step 7).

Rule: every id in [advisories] ignore must be preceded by a `# reason:` comment,
so an accepted advisory is always discoverable in the repo, not just in CI logs.
One id per line; that is the format this lint (and audit.toml's own comment) enforces.

Usage: lint-audit-toml.py [path]   (default: audit.toml at repo root)
"""
import re
import sys


def main() -> int:
    config = sys.argv[1] if len(sys.argv) > 1 else "audit.toml"
    try:
        with open(config, encoding="utf-8") as handle:
            lines = handle.read().splitlines()
    except OSError as err:
        print(f"cannot read {config}: {err}", file=sys.stderr)
        return 1

    ignore_re = re.compile(r'^\s*"(?P<id>[^"]+)"')
    in_ignore = False
    problems = []
    counted = 0
    for num, line in enumerate(lines):
        stripped = line.strip()
        if stripped.startswith("ignore"):
            # Multi-line array opens here; an inline non-empty array is a lint
            # violation because the reason rule needs one id per line.
            in_ignore = "]" not in stripped
            inline = re.search(r"ignore\s*=\s*\[\s*(\".+?)\s*\]\s*$", stripped)
            if inline:
                problems.append(
                    f"{config}:{num + 1}: ids must be one per line with a preceding '# reason:' comment: {stripped!r}"
                )
            continue
        if not in_ignore:
            continue
        if stripped.startswith("]"):
            in_ignore = False
            continue
        if not stripped or stripped.startswith("#"):
            continue
        match = ignore_re.match(line)
        if match:
            counted += 1
            prev = lines[num - 1].strip() if num else ""
            if not prev.startswith("# reason:"):
                problems.append(f'{config}:{num + 1}: id "{match.group("id")}" has no "# reason:" comment on the line above')
        else:
            problems.append(f"{config}:{num + 1}: not an ignore id line: {stripped!r}")

    if problems:
        print("\n".join(problems), file=sys.stderr)
        return 1
    print(f"{config}: allowance list conforms ({counted} ignored ids, each with a reason)")
    return 0

if __name__ == "__main__":
    sys.exit(main())
