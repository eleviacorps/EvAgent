#!/usr/bin/env python3
"""
Validate all agent YAML files in the HERMES project.

Scans:
  - agents/*.yaml
  - domains/*/agents/*.yaml

Validates every agent has:
  - name (non-empty)
  - description (non-empty)
  - domain (non-empty)
  - tool_scope (list)
  - model_preference (optional but checked if present)
  - permission_profile (non-empty, defaults to "default")

Reports:
  - Missing required fields
  - Duplicate agent names
  - YAML parse errors
"""

import os
import sys
import yaml
from pathlib import Path
from collections import defaultdict

REQUIRED_FIELDS = ["name", "description", "domain", "tool_scope", "permission_profile"]
PROJECT_ROOT = Path(__file__).resolve().parent.parent


def find_agent_files():
    """Return list of (domain_label, path) for all agent YAML files."""
    files = []

    # agents/*.yaml (root level)
    agents_dir = PROJECT_ROOT / "agents"
    if agents_dir.exists():
        for f in sorted(agents_dir.glob("*.yaml")):
            files.append(("(root)", f))
        for f in sorted(agents_dir.glob("*.yml")):
            files.append(("(root)", f))

    # domains/*/agents/*.yaml
    domains_dir = PROJECT_ROOT / "domains"
    if domains_dir.exists():
        for domain_dir in sorted(domains_dir.iterdir()):
            if domain_dir.is_dir():
                domain_agents = domain_dir / "agents"
                if domain_agents.exists():
                    for f in sorted(domain_agents.glob("*.yaml")):
                        files.append((domain_dir.name, f))
                    for f in sorted(domain_agents.glob("*.yml")):
                        files.append((domain_dir.name, f))

    return files


def validate_agent(data, path, domain_label):
    """Validate a single agent's data. Returns list of error messages."""
    errors = []
    if not isinstance(data, dict):
        errors.append(f"  ❌ Not a valid YAML mapping (expected dict, got {type(data).__name__})")
        return errors

    for field in REQUIRED_FIELDS:
        if field not in data or data[field] is None:
            errors.append(f"  ❌ Missing required field: '{field}'")
        elif isinstance(data[field], str) and not data[field].strip():
            errors.append(f"  ❌ Required field '{field}' is empty")
        elif field == "tool_scope" and not isinstance(data[field], list):
            errors.append(f"  ❌ Field 'tool_scope' must be a list (got {type(data[field]).__name__})")

    # Check name for duplicates later; just check it exists
    if "name" in data and isinstance(data["name"], str):
        name = data["name"].strip()
        if not name:
            errors.append("  ❌ 'name' is empty or whitespace-only")

    # Check optional fields
    if "model_preference" in data and data["model_preference"] is not None:
        valid_prefs = ["powerful", "balanced", "fast", "economical"]
        if data["model_preference"] not in valid_prefs:
            errors.append(
                f"  ⚠️  'model_preference' should be one of {valid_prefs}, "
                f"got '{data['model_preference']}'"
            )

    return errors


def main():
    agent_files = find_agent_files()
    print(f"🔍 Scanning for agent YAML files...")
    print(f"   Found {len(agent_files)} agent file(s)")
    print()

    all_errors = 0
    all_warnings = 0
    name_counts = defaultdict(list)
    parsed_count = 0

    for domain_label, path in agent_files:
        try:
            with open(path, "r", encoding="utf-8") as f:
                data = yaml.safe_load(f)
        except yaml.YAMLError as e:
            print(f"  🚫 YAML parse error in {path.relative_to(PROJECT_ROOT)}: {e}")
            all_errors += 1
            continue
        except Exception as e:
            print(f"  🚫 Error reading {path.relative_to(PROJECT_ROOT)}: {e}")
            all_errors += 1
            continue

        parsed_count += 1
        print(f"📄 {domain_label}/agents/{path.name}")

        errors = validate_agent(data, path, domain_label)
        for err in errors:
            if "⚠️" in err:
                all_warnings += 1
            else:
                all_errors += 1
            print(err)

        if not errors:
            print(f"  ✅ Valid")

        # Track name for duplicate detection
        if isinstance(data, dict) and "name" in data and isinstance(data["name"], str):
            name_counts[data["name"].strip()].append(path)

        print()

    # Check for duplicate names
    duplicates_found = False
    for name, paths in name_counts.items():
        if len(paths) > 1:
            duplicates_found = True
            all_errors += 1
            print(f"  🚫 Duplicate agent name '{name}' found in:")
            for p in paths:
                print(f"       {p.relative_to(PROJECT_ROOT)}")

    if duplicates_found:
        print()

    # Summary
    print("═" * 60)
    print(f"📊 Summary: {parsed_count} agent(s) parsed")
    if all_errors > 0:
        print(f"   ❌ {all_errors} error(s) found")
    if all_warnings > 0:
        print(f"   ⚠️  {all_warnings} warning(s)")
    if all_errors == 0:
        print(f"   ✅ All agents valid!")

    return 1 if all_errors > 0 else 0


if __name__ == "__main__":
    sys.exit(main())
