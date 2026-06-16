#!/usr/bin/env python3
"""
Validate all skill SKILL.md files in the HERMES project.

Scans domains/*/skills/*/SKILL.md files.

Validates every skill has YAML frontmatter with:
  - name (non-empty)
  - domain (non-empty)
  - version (positive integer)
  - trigger_patterns (list)
  - applicable_agents (list)

Reports:
  - Missing frontmatter
  - Missing required fields in frontmatter
  - Duplicate skill names (across all domains)
  - YAML parse errors
"""

import os
import sys
import yaml
from pathlib import Path
from collections import defaultdict

REQUIRED_FIELDS = ["name", "domain", "version", "trigger_patterns", "applicable_agents"]
PROJECT_ROOT = Path(__file__).resolve().parent.parent


def find_skill_files():
    """Return list of (domain_label, path) for all SKILL.md files."""
    files = []

    # skills/*/SKILL.md (root level)
    skills_dir = PROJECT_ROOT / "skills"
    if skills_dir.exists():
        for skill_dir in sorted(skills_dir.iterdir()):
            if skill_dir.is_dir():
                skill_file = skill_dir / "SKILL.md"
                if skill_file.exists():
                    files.append(("(root)", skill_file))

    # domains/*/skills/*/SKILL.md
    domains_dir = PROJECT_ROOT / "domains"
    if domains_dir.exists():
        for domain_dir in sorted(domains_dir.iterdir()):
            if domain_dir.is_dir():
                domain_skills = domain_dir / "skills"
                if domain_skills.exists():
                    for skill_dir in sorted(domain_skills.iterdir()):
                        if skill_dir.is_dir():
                            skill_file = skill_dir / "SKILL.md"
                            if skill_file.exists():
                                files.append((domain_dir.name, skill_file))

    return files


def extract_frontmatter(content, path):
    """
    Extract and parse YAML frontmatter from a markdown file.
    Returns (frontmatter_data, remaining_content) or raises ValueError.
    """
    stripped = content.strip()
    if not stripped.startswith("---"):
        raise ValueError("Missing YAML frontmatter: file must start with '---'")

    # Remove the opening ---
    after_first = stripped[3:].lstrip("\n").lstrip("\r\n")

    # Find closing ---
    end_idx = after_first.find("\n---")
    if end_idx == -1:
        end_idx = after_first.find("\r\n---")
    if end_idx == -1:
        raise ValueError("Unclosed YAML frontmatter: missing closing '---'")

    yaml_str = after_first[:end_idx].strip()
    if not yaml_str:
        return {}, after_first[end_idx + 4:].strip()

    try:
        data = yaml.safe_load(yaml_str)
    except yaml.YAMLError as e:
        raise ValueError(f"YAML parse error in frontmatter: {e}")

    if data is None:
        data = {}

    return data, after_first[end_idx + 4:].strip()


def validate_skill(data, path, domain_label):
    """Validate a single skill's frontmatter data. Returns list of error messages."""
    errors = []
    if not isinstance(data, dict):
        errors.append(f"  ❌ Frontmatter is not a valid mapping")
        return errors

    for field in REQUIRED_FIELDS:
        if field not in data or data[field] is None:
            errors.append(f"  ❌ Missing required field: '{field}'")
        elif isinstance(data[field], str) and not data[field].strip():
            errors.append(f"  ❌ Required field '{field}' is empty")
        elif field == "version":
            if not isinstance(data[field], int):
                errors.append(f"  ❌ 'version' must be an integer (got {type(data[field]).__name__})")
            elif data[field] < 1:
                errors.append(f"  ❌ 'version' must be >= 1 (got {data[field]})")
        elif field in ("trigger_patterns", "applicable_agents"):
            if not isinstance(data[field], list):
                errors.append(f"  ❌ '{field}' must be a list (got {type(data[field]).__name__})")

    return errors


def main():
    skill_files = find_skill_files()
    print(f"🔍 Scanning for SKILL.md files...")
    print(f"   Found {len(skill_files)} skill file(s)")
    print()

    all_errors = 0
    all_warnings = 0
    name_counts = defaultdict(list)
    parsed_count = 0

    for domain_label, path in skill_files:
        try:
            with open(path, "r", encoding="utf-8") as f:
                content = f.read()
        except Exception as e:
            print(f"  🚫 Error reading {path.relative_to(PROJECT_ROOT)}: {e}")
            all_errors += 1
            continue

        parsed_count += 1
        print(f"📄 {domain_label}/{path.parent.name}/SKILL.md")

        try:
            data, _ = extract_frontmatter(content, path)
        except ValueError as e:
            print(f"  ❌ {e}")
            all_errors += 1
            print()
            continue

        errors = validate_skill(data, path, domain_label)
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
            print(f"  🚫 Duplicate skill name '{name}' found in:")
            for p in paths:
                print(f"       {p.relative_to(PROJECT_ROOT)}")

    if duplicates_found:
        print()

    # Summary
    print("═" * 60)
    print(f"📊 Summary: {parsed_count} skill(s) parsed")
    if all_errors > 0:
        print(f"   ❌ {all_errors} error(s) found")
    if all_warnings > 0:
        print(f"   ⚠️  {all_warnings} warning(s)")
    if all_errors == 0:
        print(f"   ✅ All skills valid!")

    return 1 if all_errors > 0 else 0


if __name__ == "__main__":
    sys.exit(main())
