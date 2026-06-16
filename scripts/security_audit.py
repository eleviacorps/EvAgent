#!/usr/bin/env python3
"""
Security audit script for the HERMES Rust core.

Checks:
  1. shell=True equivalents (Command::new("sh") / Command::new("cmd") with -c flags)
     — these are checked but may be valid if bounded (spawning sub-agents).
  2. unwrap() usage in production code (allowed in #[cfg(test)] blocks)
  3. Unbounded recursion (recursive fn calls without explicit depth limits)
  4. Filesystem walk patterns — verify they respect max_walk_depth
  5. schema-validated env parsing (config.rs already does this)
  6. Any hardcoded secrets, tokens, or credentials

Exits with code >0 on any HIGH severity finding.
"""

import os
import sys
import re
from pathlib import Path

PROJECT_ROOT = Path(__file__).resolve().parent.parent
CORE_SRC = PROJECT_ROOT / "hermes-core" / "src"

SEVERITY_ORDER = {"LOW": 1, "MEDIUM": 2, "HIGH": 3, "CRITICAL": 4}


class Finding:
    def __init__(self, severity, filepath, line_num, message, snippet=""):
        self.severity = severity
        self.filepath = filepath
        self.line_num = line_num
        self.message = message
        self.snippet = snippet

    def __repr__(self):
        return f"[{self.severity}] {self.filepath}:{self.line_num} — {self.message}"


def scan_file(filepath, relative_to):
    """Scan a single Rust file for security issues."""
    findings = []
    rel_path = filepath.relative_to(relative_to)

    with open(filepath, "r", encoding="utf-8", errors="replace") as f:
        lines = f.readlines()

    in_test_block = False
    in_production_block = True  # flip when entering #[cfg(test)]

    for i, line in enumerate(lines, 1):
        stripped = line.strip()
        line_lower = stripped.lower()

        # Track whether we're inside a #[cfg(test)] block
        if re.search(r"#\[cfg\(test\)\]", stripped):
            in_test_block = True
            in_production_block = False
            continue
        if stripped.startswith("fn ") and in_test_block:
            # Still inside test module
            pass
        if stripped == "}" and in_test_block:
            # End of test module (simplistic — assumes no nested mods)
            in_test_block = False
            in_production_block = True

        # ── 1. Check for unwrap() in production code ──
        if in_production_block and ".unwrap()" in stripped:
            # Allow unwrap in test modules and in lines inside #[cfg(test)]
            findings.append(Finding(
                "MEDIUM",
                rel_path, i,
                "unwrapped Result/Option in production code (use ? operator or proper error handling)",
                stripped[:120],
            ))

        # ── 2. Check shell spawning patterns ──
        if "Command::new(\"sh\")" in stripped and "-c" in line_lower:
            findings.append(Finding(
                "HIGH",
                rel_path, i,
                "Shell command execution via sh -c — ensure input is sanitized and not user-controlled",
                stripped[:120],
            ))
        if "Command::new(\"cmd\")" in stripped and "/C" in line_lower:
            findings.append(Finding(
                "HIGH",
                rel_path, i,
                "Shell command execution via cmd /C — ensure input is sanitized and not user-controlled",
                stripped[:120],
            ))

        # ── 3. Check for unbounded recursion ──
        # Look for functions that call themselves (recursive)
        # We look for the function name appearing in its own body
        fn_match = re.match(r"^\s*(?:pub\s+)?(?:async\s+)?fn\s+(\w+)", stripped)
        if fn_match:
            fn_name = fn_match.group(1)
            # Scan next lines for self-call
            for j in range(i, min(i + 40, len(lines))):
                next_line = lines[j]
                if fn_name in next_line and "fn " not in next_line:
                    # Check if recursion is bounded by a depth parameter
                    # Simple check: look for "depth" in nearby context
                    context = "".join(lines[max(0, i - 3):min(len(lines), i + 5)])
                    if "depth" not in context.lower() and "max_" not in context.lower():
                        findings.append(Finding(
                            "MEDIUM",
                            rel_path, j + 1,
                            f"Potential unbounded recursion in '{fn_name}' — no depth limit detected nearby",
                            next_line.strip()[:120],
                        ))
                    break

        # ── 4. Check for hardcoded secrets / tokens / passwords ──
        secret_patterns = [
            r'(?i)(?:password|passwd|pwd|secret|token|api[_-]?key|auth[_-]?key)\s*[:=]\s*["\'](?!<)[^"\']+["\']',
        ]
        for pattern in secret_patterns:
            if re.search(pattern, stripped):
                findings.append(Finding(
                    "CRITICAL",
                    rel_path, i,
                    "Possible hardcoded credential detected",
                    stripped[:120],
                ))

        # ── 5. Check for std::process::Command without validation ──
        if "std::process::Command::new" in stripped or "Command::new" in stripped:
            if "sanitize" not in line_lower and "validate" not in line_lower:
                # Allow if it's clearly using a hardcoded string, flag if using variables
                if re.search(r'Command::new\(\s*["\']', stripped):
                    pass  # Hardcoded command is fine
                elif re.search(r'Command::new\(\s*[^"\']', stripped):
                    findings.append(Finding(
                        "MEDIUM",
                        rel_path, i,
                        "Dynamic command name — ensure input cannot inject arbitrary commands",
                        stripped[:120],
                    ))

    return findings


def check_fs_walk_patterns():
    """Verify that filesystem walks have depth limits."""
    findings = []
    if not CORE_SRC.exists():
        return findings

    for filepath in CORE_SRC.glob("*.rs"):
        rel_path = filepath.relative_to(PROJECT_ROOT)
        with open(filepath, "r", encoding="utf-8") as f:
            content = f.read()

        # Check for read_dir usage
        if "read_dir" in content:
            # Verify there's a depth parameter check nearby
            if "max_walk_depth" not in content and "depth" not in content.lower():
                findings.append(Finding(
                    "HIGH",
                    rel_path, 0,
                    "Filesystem walk (read_dir) without depth limit — potential DoS / infinite recursion",
                ))

            # Count recursive directory walkers
            walkers = re.findall(r"fn\s+\w+.*dir.*depth", content)
            if not walkers:
                # Check for the scan_directory pattern
                scan_fns = re.findall(r"fn\s+scan_\w+\([^)]*dir[^)]*\)", content)
                for fn_sig in scan_fns:
                    if "depth" not in fn_sig:
                        findings.append(Finding(
                            "HIGH",
                            rel_path, 0,
                            f"Directory scanning function '{fn_sig}' without depth parameter",
                        ))

    return findings


def check_env_parsing():
    """Verify environment variable parsing uses validated parsing."""
    findings = []
    if not CORE_SRC.exists():
        return findings

    config_file = CORE_SRC / "config.rs"
    if config_file.exists():
        with open(config_file, "r", encoding="utf-8") as f:
            content = f.read()

        # Check for env::var patterns with proper parsing
        env_vars = re.findall(r'env::var\("([^"]+)"\)', content)
        for var in env_vars:
            # Verify there's a .parse() or similar validation
            # Find the context around this env::var call
            idx = content.index(f'env::var("{var}")')
            snippet = content[idx:idx + 200]
            if ".parse::<" not in snippet and ".parse(" not in snippet:
                # Check if it's just a string assignment (string-based en vars don't need parse)
                if "eq_ignore_ascii_case" not in snippet:
                    findings.append(Finding(
                        "MEDIUM",
                        config_file.relative_to(PROJECT_ROOT), 0,
                        f"Environment variable '{var}' may not be type-validated after parsing",
                    ))

        if not env_vars:
            findings.append(Finding(
                "LOW",
                config_file.relative_to(PROJECT_ROOT), 0,
                "No environment variable overrides found (expected in config.rs)",
            ))

    return findings


def main():
    print("═" * 80)
    print("🔒 HERMES Security Audit")
    print("═" * 80)
    print()

    all_findings = []

    # Scan all Rust source files
    if CORE_SRC.exists():
        for filepath in sorted(CORE_SRC.glob("*.rs")):
            findings = scan_file(filepath, PROJECT_ROOT)
            all_findings.extend(findings)

    # Check FS walk patterns
    all_findings.extend(check_fs_walk_patterns())

    # Check env parsing
    all_findings.extend(check_env_parsing())

    # Sort findings by severity
    all_findings.sort(key=lambda f: SEVERITY_ORDER.get(f.severity, 0), reverse=True)

    # Group and display
    if not all_findings:
        print("  ✅ No security findings!")
        print()
        print("═" * 80)
        return 0

    by_severity = {}
    for f in all_findings:
        by_severity.setdefault(f.severity, []).append(f)

    for severity in ["CRITICAL", "HIGH", "MEDIUM", "LOW"]:
        if severity in by_severity:
            print(f"── [{severity}] ──" + "─" * 65)
            for f in by_severity[severity]:
                print(f"  📍 {f.filepath}:{f.line_num}")
                print(f"     {f.message}")
                if f.snippet:
                    print(f"     Code: {f.snippet}")
                print()

    # Summary
    print("═" * 80)
    print("📊 Audit Summary")
    print("─" * 80)
    severity_counts = {}
    for f in all_findings:
        severity_counts[f.severity] = severity_counts.get(f.severity, 0) + 1

    for sev in ["CRITICAL", "HIGH", "MEDIUM", "LOW"]:
        count = severity_counts.get(sev, 0)
        icon = {"CRITICAL": "🚨", "HIGH": "🔴", "MEDIUM": "🟡", "LOW": "🔵"}.get(sev, "⚪")
        print(f"  {icon} {sev}: {count}")

    print(f"  Total: {len(all_findings)} finding(s)")

    # Exit code: CRITICAL or HIGH findings cause non-zero exit
    high_severity_count = sum(
        count for sev, count in severity_counts.items()
        if sev in ("CRITICAL", "HIGH")
    )
    if high_severity_count > 0:
        print(f"\n  ❌ FAILED: {high_severity_count} HIGH/CRITICAL finding(s) need review")
        return 2
    else:
        print(f"\n  ✅ PASSED: No high-severity findings")
        return 0


if __name__ == "__main__":
    sys.exit(main())
