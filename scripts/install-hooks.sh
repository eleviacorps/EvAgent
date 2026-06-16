#!/usr/bin/env bash
# ──────────────────────────────────────────────────────────────────────────────
# Install Hermes pre-commit hooks and set up the development environment.
# ──────────────────────────────────────────────────────────────────────────────
set -euo pipefail

PROJECT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
HOOKS_DIR="$PROJECT_ROOT/.git/hooks"
SCRIPT_DIR="$PROJECT_ROOT/scripts"

echo "🔧 Installing HERMES development environment..."
echo

# ── Install pre-commit hook ───────────────────────────────────────────────────
echo "── [Pre-commit Hook] ─────────────────────────────────"
if [ -f "$SCRIPT_DIR/pre-commit.sh" ]; then
    cp "$SCRIPT_DIR/pre-commit.sh" "$HOOKS_DIR/pre-commit"
    chmod +x "$HOOKS_DIR/pre-commit"
    echo -e "  ✅ Installed pre-commit hook from scripts/pre-commit.sh"
elif [ -f "$HOOKS_DIR/pre-commit" ]; then
    echo -e "  ℹ️  Pre-commit hook already exists"
else
    echo -e "  ⚠️  No pre-commit.sh found in scripts/. Skipping hook installation."
fi
echo

# ── Check Python dependencies ─────────────────────────────────────────────────
echo "── [Python Dependencies] ──────────────────────────────"
if command -v python3 &>/dev/null; then
    # Check PyYAML
    if python3 -c "import yaml" 2>/dev/null; then
        echo -e "  ✅ PyYAML is installed"
    else
        echo -e "  ⚠️  PyYAML not found. Installing..."
        pip3 install pyyaml 2>/dev/null && echo -e "  ✅ Installed PyYAML" || echo -e "  ⚠️  Could not install PyYAML"
    fi
else
    echo -e "  ⚠️  Python3 not found. Validation scripts require Python 3."
fi
echo

# ── Check Rust toolchain ──────────────────────────────────────────────────────
echo "── [Rust Toolchain] ───────────────────────────────────"
if command -v cargo &>/dev/null; then
    echo -e "  ✅ Cargo found: $(cargo --version 2>/dev/null | head -1)"
    if [ -f "$PROJECT_ROOT/hermes-core/Cargo.toml" ]; then
        echo "  Checking Rust dependencies..."
        cd "$PROJECT_ROOT/hermes-core" && cargo fetch 2>/dev/null && echo -e "  ✅ Dependencies fetched" || echo -e "  ⚠️  Could not fetch dependencies"
        cd "$PROJECT_ROOT"
    fi
else
    echo -e "  ⚠️  Cargo not found. Install Rust: https://rustup.rs/"
fi
echo

# ── Make scripts executable ────────────────────────────────────────────────────
echo "── [Script Permissions] ──────────────────────────────"
for script in "$SCRIPT_DIR"/*.py "$SCRIPT_DIR"/*.sh; do
    if [ -f "$script" ]; then
        chmod +x "$script"
        echo -e "  ✅ Made executable: $(basename "$script")"
    fi
done
echo

# ── Summary ────────────────────────────────────────────────────────────────────
echo "═"*60
echo -e "✅ HERMES development environment setup complete!"
echo
echo "Available commands:"
echo "  python scripts/validate_agents.py    — Validate all agent YAML files"
echo "  python scripts/validate_skills.py    — Validate all skill SKILL.md files"
echo "  python scripts/check_line_limits.py  — Enforce 500-line limit"
echo "  python scripts/coverage_gate.py      — Check test coverage"
echo "  python scripts/security_audit.py     — Run security audit"
echo "  cd hermes-core && cargo test         — Run Rust tests"
echo "  .git/hooks/pre-commit                — Run pre-commit checks manually"
echo
echo "CI pipeline: scripts/ci.sh or .github/workflows/ci.yml"
