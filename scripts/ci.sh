#!/usr/bin/env bash
# ──────────────────────────────────────────────────────────────────────────────
# CI Pipeline for HERMES (bash version, git-bash compatible)
#
# Steps:
#   1. Validate agent YAML files
#   2. Validate skill SKILL.md files
#   3. Check file line limits (500 max)
#   4. Run security audit
#   5. Run Rust tests (cargo test)
#   6. Build TUI (npm run build) — optional, skipped if no npm
#   7. Report coverage status
#   8. Summary table
# ──────────────────────────────────────────────────────────────────────────────
set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color
BOLD='\033[1m'

PROJECT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$PROJECT_ROOT"

echo -e "${BOLD}${BLUE}════════════════════════════════════════════════════════════════════════${NC}"
echo -e "${BOLD}${BLUE}  HERMES CI Pipeline${NC}"
echo -e "${BOLD}${BLUE}  $(date -u '+%Y-%m-%d %H:%M:%S UTC')${NC}"
echo -e "${BOLD}${BLUE}════════════════════════════════════════════════════════════════════════${NC}"
echo

# Results table
RESULTS=()

run_step() {
    local step_name="$1"
    shift
    echo -e "${BOLD}[$step_name]${NC}"
    echo "─"*60

    if "$@" 2>&1; then
        echo
        echo -e "${GREEN}  ✅ $step_name PASSED${NC}"
        RESULTS+=("✅|$step_name|PASSED")
    else
        local exit_code=$?
        echo
        echo -e "${RED}  ❌ $step_name FAILED (exit code: $exit_code)${NC}"
        RESULTS+=("❌|$step_name|FAILED (code $exit_code)")
    fi
    echo
}

# ── Step 1: Validate Agents ────────────────────────────────────────────────────
run_step "Agent Validation" python3 scripts/validate_agents.py

# ── Step 2: Validate Skills ────────────────────────────────────────────────────
run_step "Skill Validation" python3 scripts/validate_skills.py

# ── Step 3: Line Limits ────────────────────────────────────────────────────────
run_step "Line Limit Check" python3 scripts/check_line_limits.py

# ── Step 4: Security Audit ─────────────────────────────────────────────────────
# Note: security_audit may return 2 for HIGH findings — we report but don't fail
run_step "Security Audit" python3 scripts/security_audit.py

# ── Step 5: Rust Tests ─────────────────────────────────────────────────────────
if [ -d "hermes-core" ] && [ -f "hermes-core/Cargo.toml" ]; then
    echo -e "${BOLD}[Rust Tests]${NC}"
    echo "─"*60
    cd hermes-core
    if cargo test 2>&1; then
        echo
        echo -e "${GREEN}  ✅ Rust Tests PASSED${NC}"
        RESULTS+=("✅|Rust Tests|PASSED")
    else
        echo
        echo -e "${RED}  ❌ Rust Tests FAILED${NC}"
        RESULTS+=("❌|Rust Tests|FAILED")
    fi
    cd "$PROJECT_ROOT"
    echo
else
    echo -e "${YELLOW}  ⚠️  hermes-core/ not found, skipping Rust tests${NC}"
    RESULTS+=("⚠️|Rust Tests|SKIPPED")
fi

# ── Step 6: TUI Build (optional) ───────────────────────────────────────────────
if [ -d "tui" ] && [ -f "tui/package.json" ] && command -v npm &>/dev/null; then
    echo -e "${BOLD}[TUI Build]${NC}"
    echo "─"*60
    cd tui
    if npm run build 2>&1; then
        echo
        echo -e "${GREEN}  ✅ TUI Build PASSED${NC}"
        RESULTS+=("✅|TUI Build|PASSED")
    else
        echo
        echo -e "${RED}  ❌ TUI Build FAILED${NC}"
        RESULTS+=("❌|TUI Build|FAILED")
    fi
    cd "$PROJECT_ROOT"
    echo
else
    echo -e "${YELLOW}  ⚠️  tui/ not found or npm not available, skipping TUI build${NC}"
    RESULTS+=("⚠️|TUI Build|SKIPPED")
fi

# ── Step 7: Coverage Gate ──────────────────────────────────────────────────────
run_step "Coverage Gate" python3 scripts/coverage_gate.py

# ── Summary Table ──────────────────────────────────────────────────────────────
echo -e "${BOLD}${BLUE}════════════════════════════════════════════════════════════════════════${NC}"
echo -e "${BOLD}  CI Pipeline Summary${NC}"
echo -e "${BOLD}${BLUE}════════════════════════════════════════════════════════════════════════${NC}"
echo

printf "${BOLD}%-6s %-22s %s${NC}\n" "Status" "Step" "Result"
echo "─"*60

TOTAL=0
PASSED=0
FAILED=0
SKIPPED=0

for result in "${RESULTS[@]}"; do
    IFS='|' read -r status step result_detail <<< "$result"
    TOTAL=$((TOTAL + 1))
    case "$status" in
        "✅") PASSED=$((PASSED + 1)) ;;
        "❌") FAILED=$((FAILED + 1)) ;;
        "⚠️") SKIPPED=$((SKIPPED + 1)) ;;
    esac
    printf "%-6s %-22s %s\n" "$status" "$step" "$result_detail"
done

echo "─"*60
echo

echo "  Total:  $TOTAL"
echo -e "  ${GREEN}Passed: $PASSED${NC}"
if [ "$FAILED" -gt 0 ]; then
    echo -e "  ${RED}Failed: $FAILED${NC}"
else
    echo "  Failed: $FAILED"
fi
echo "  Skipped: $SKIPPED"
echo

if [ "$FAILED" -gt 0 ]; then
    echo -e "${RED}❌ CI Pipeline FAILED — some checks did not pass${NC}"
    exit 1
elif [ "$TOTAL" -eq 0 ]; then
    echo -e "${YELLOW}⚠️  No CI steps executed${NC}"
    exit 1
else
    echo -e "${GREEN}✅ CI Pipeline PASSED${NC}"
    exit 0
fi
