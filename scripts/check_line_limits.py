#!/usr/bin/env python3
"""
Enforce a maximum line limit across source files in the HERMES project.

Scans:
  - evagent-core/src/**/*.rs (Rust source files)
  - scripts/*.py (Python validation/CI scripts)
  - tui/src/**/*.ts, *.tsx (TypeScript source files)

Exits with code 1 if any file exceeds MAX_LINES (default: 500).
"""

import os
import sys
from pathlib import Path

MAX_LINES = 500
PROJECT_ROOT = Path(__file__).resolve().parent.parent

# Patterns: (root_dir, glob_pattern, label)
SCAN_PATTERNS = [
    ("evagent-core/src", "**/*.rs", "Rust source"),
    ("scripts", "*.py", "Python scripts"),
    ("tui/src", "**/*.ts", "TypeScript source"),
    ("tui/src", "**/*.tsx", "TypeScript React"),
]


def scan_files():
    """Scan all matching files and return violations."""
    violations = []
    total_files = 0

    for rel_dir, glob_pattern, label in SCAN_PATTERNS:
        scan_dir = PROJECT_ROOT / rel_dir
        if not scan_dir.exists():
            print(f"  ℹ️  Directory '{rel_dir}' does not exist, skipping {label} scan")
            continue

        for filepath in sorted(scan_dir.glob(glob_pattern)):
            # Skip node_modules
            if "node_modules" in filepath.parts:
                continue

            total_files += 1
            try:
                with open(filepath, "r", encoding="utf-8", errors="replace") as f:
                    line_count = sum(1 for _ in f)
            except Exception as e:
                print(f"  ⚠️  Could not read {filepath.relative_to(PROJECT_ROOT)}: {e}")
                continue

            rel_path = filepath.relative_to(PROJECT_ROOT)
            if line_count > MAX_LINES:
                violations.append((rel_path, line_count, label))
                print(f"  ❌ {rel_path} ({line_count} lines, exceeds {MAX_LINES})")
            else:
                print(f"  ✅ {rel_path} ({line_count} lines)")

    return violations, total_files


def main():
    print(f"🔍 Checking line limits (max: {MAX_LINES} lines per file)...")
    print()

    violations, total_files = scan_files()

    print()
    print("═" * 60)
    print(f"📊 Summary: {total_files} file(s) scanned")

    if violations:
        print(f"   ❌ {len(violations)} file(s) exceed {MAX_LINES} lines:")
        for path, count, label in violations:
            print(f"       - {path} ({count} lines, {label})")
        print()
        print("💡 Tip: Consider refactoring these files into smaller modules.")
        return 1
    else:
        print(f"   ✅ All files within {MAX_LINES}-line limit!")
        return 0


if __name__ == "__main__":
    sys.exit(main())
