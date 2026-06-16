#!/usr/bin/env python3
"""
Coverage planning and reporting tool for the HERMES Rust core.

For now:
  - Reports all Rust modules and whether they have #[cfg(test)] blocks
  - Tracks which modules have inline unit tests
  - Tracks integration tests in hermes-core/tests/
  - Reports planned coverage targets

Future enhancement: integrate with cargo-tarpaulin or grcov for actual % measurement.
"""

import os
import sys
import re
from pathlib import Path

PROJECT_ROOT = Path(__file__).resolve().parent.parent
CORE_SRC = PROJECT_ROOT / "hermes-core" / "src"
CORE_TESTS = PROJECT_ROOT / "hermes-core" / "tests"

# Coverage targets per module (as percentage)
COVERAGE_TARGETS = {
    "agent_registry": 80,
    "config": 85,
    "dispatcher": 75,
    "errors": 90,
    "intent_router": 80,
    "models": 70,
    "permissions": 85,
    "session": 85,
    "skill_loader": 80,
    "server": 60,
    "main": 50,
}

# Minimal coverage gate (project-wide)
MIN_COVERAGE_PCT = 60


def find_rust_modules():
    """Find all Rust module files and detect test blocks."""
    modules = []

    if not CORE_SRC.exists():
        print(f"❌ Core source directory not found: {CORE_SRC}")
        return modules

    for filepath in sorted(CORE_SRC.glob("*.rs")):
        with open(filepath, "r", encoding="utf-8") as f:
            content = f.read()

        # Detect #[cfg(test)] mod tests blocks
        has_unit_tests = bool(re.search(r"#\[cfg\(test\)\]\s*\n\s*(mod|fn)\s+tests", content))

        # Count test functions
        test_fn_count = len(re.findall(r"#\[test\]\s*\n\s*(?:async\s+)?fn\s+\w+", content))

        # Count public items (structs, fns, pub fns)
        pub_fn_count = len(re.findall(r"pub\s+(?:async\s+)?fn\s+\w+", content))
        pub_struct_count = len(re.findall(r"pub\s+struct\s+\w+", content))
        total_lines = content.count("\n") + 1

        module_name = filepath.stem
        target = COVERAGE_TARGETS.get(module_name, 60)

        # Build test status
        if has_unit_tests:
            test_status = f"✅ {test_fn_count} test(s)"
        else:
            test_status = "❌ No unit tests"

        modules.append({
            "name": module_name,
            "file": filepath.name,
            "lines": total_lines,
            "pub_fn": pub_fn_count,
            "pub_struct": pub_struct_count,
            "has_tests": has_unit_tests,
            "test_count": test_fn_count,
            "test_status": test_status,
            "target": target,
        })

    return modules


def find_integration_tests():
    """Find integration test files."""
    tests = []
    if CORE_TESTS.exists():
        for filepath in sorted(CORE_TESTS.glob("*.rs")):
            with open(filepath, "r", encoding="utf-8") as f:
                content = f.read()
            test_fn_count = len(re.findall(r"#\[test\]\s*\n\s*(?:async\s+)?fn\s+\w+", content))
            tokio_test_count = len(re.findall(r"#\[tokio::test\]", content))
            total = test_fn_count + tokio_test_count
            tests.append({
                "file": filepath.name,
                "count": total,
            })
    return tests


def main():
    modules = find_rust_modules()
    integration_tests = find_integration_tests()

    if not modules:
        print("❌ No Rust modules found. Run this script from the HERMES project root.")
        return 1

    # ── Module Coverage Table ──
    print("═" * 80)
    print("📊 HERMES Core — Coverage Report")
    print("═" * 80)
    print()

    print(f"{'Module':<22} {'Lines':>7} {'Pub Fn':>7} {'Pub St':>7} {'Tests':>6} {'Target':>8} {'Status'}")
    print("-" * 80)

    modules_with_tests = 0
    total_test_count = 0

    for m in modules:
        status = "✅" if m["has_tests"] else "❌"
        print(
            f"  {m['name']:<20} {m['lines']:>7} {m['pub_fn']:>7} {m['pub_struct']:>7} "
            f"{m['test_count']:>6} {m['target']:>7}% {status}"
        )
        if m["has_tests"]:
            modules_with_tests += 1
        total_test_count += m["test_count"]

    print("-" * 80)
    coverage_pct = (modules_with_tests / len(modules) * 100) if modules else 0
    print(
        f"  {'TOTAL':<20} {sum(m['lines'] for m in modules):>7} "
        f"{sum(m['pub_fn'] for m in modules):>7} "
        f"{sum(m['pub_struct'] for m in modules):>7} "
        f"{total_test_count:>6} {MIN_COVERAGE_PCT:>7}% "
        f"{'✅' if coverage_pct >= MIN_COVERAGE_PCT else '❌'}"
    )

    print()
    print(f"  Modules with tests: {modules_with_tests}/{len(modules)} ({coverage_pct:.0f}%)")
    print(f"  Total test functions: {total_test_count}")
    print(f"  Minimum coverage gate: {MIN_COVERAGE_PCT}%")

    if coverage_pct < MIN_COVERAGE_PCT:
        print(f"  ❌ FAILED: {coverage_pct:.0f}% < {MIN_COVERAGE_PCT}% minimum")
        print(f"     Modules needing tests:")
        for m in modules:
            if not m["has_tests"]:
                print(f"       - {m['name']} ({m['file']})")
    else:
        print(f"  ✅ PASSED: {coverage_pct:.0f}% >= {MIN_COVERAGE_PCT}% minimum")

    # ── Integration Tests ──
    if integration_tests:
        print()
        print("─" * 80)
        print("📁 Integration Tests")
        print("─" * 80)
        for t in integration_tests:
            print(f"  ✅ {t['file']} — {t['count']} test(s)")
    else:
        print()
        print("  ℹ️  No integration test files found")

    # ── Planned Coverage Matrix ──
    print()
    print("─" * 80)
    print("🎯 Target Coverage Matrix")
    print("─" * 80)
    print()
    print(f"{'Module':<22} {'Target':>8} {'Reached':>8} {'Gap':>8}")
    print("-" * 50)

    for m in modules:
        target = m["target"]
        reached = 100 if m["has_tests"] else 0
        gap = max(0, target - reached)
        status_icon = "✅" if reached >= target else "⚠️"
        print(f"  {m['name']:<20} {target:>7}% {reached:>7}% {gap:>7}% {status_icon}")

    print()
    passing = all(m["has_tests"] for m in modules)
    if passing:
        print("✅ All modules have tests. Aim for 80%+ line coverage with cargo-tarpaulin.")
    else:
        print("⚠️  Some modules still lack tests. Add #[cfg(test)] blocks to reach 100% coverage.")

    print()
    print("💡 Run `cargo test` in hermes-core/ to execute all tests.")
    print("💡 For actual line coverage: `cargo tarpaulin --out Html`")

    return 0 if coverage_pct >= MIN_COVERAGE_PCT else 1


if __name__ == "__main__":
    sys.exit(main())
