#!/usr/bin/env bash
#
# Local security review of the current diff vs origin/main with the Claude Code CLI.
# Zero cost: it uses the locally authenticated `claude` session (Pro/Max
# subscription), no API key needed.
#
# Usage:
#   bash scripts/review-diff.sh                     # review diff vs origin/main
#   bash scripts/review-diff.sh feat/my-branch      # review specific branch vs main
#   BASE=develop bash scripts/review-diff.sh        # custom base branch
#   MAX_DIFF_LINES=10000 bash scripts/review-diff.sh
#
# Skip conditions (exit 0 without review):
# - `claude` CLI not installed
# - current branch == main (nothing to review)
# - empty diff
# - diff > MAX_DIFF_LINES (default 8000): local review too slow/expensive;
#   the skip is printed as a clearly visible warning, not as just another line
#
# The diff leaves out what is not attack surface (specs, bmad artifacts,
# documentation, Gherkin scenarios and their steps in tests/bdd, fixtures, Cargo.lock):
# this keeps large stories under the cap. The other tests stay in the diff, because
# the domain purity tests (purity.rs) and the security tests enforce the principles.
#
# Output: markdown review printed on stdout. The exit code is ALWAYS 0: the
# review is advisory, you decide whether to cancel the push after reading it.

set -uo pipefail

BASE="${BASE:-origin/main}"
MAX_DIFF_LINES="${MAX_DIFF_LINES:-8000}"

# Skip if the claude CLI is not installed
if ! command -v claude >/dev/null 2>&1; then
  echo "[review-diff] \`claude\` CLI not installed: skip. See https://docs.claude.com/cli to install it." >&2
  exit 0
fi

# Current branch
current_branch="$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo "")"
if [ "$current_branch" = "main" ] || [ "$current_branch" = "HEAD" ]; then
  echo "[review-diff] on main/detached, nothing to review" >&2
  exit 0
fi

# Check that BASE exists
if ! git rev-parse --verify "$BASE" >/dev/null 2>&1; then
  echo "[review-diff] base ref \`$BASE\` not found: run \`git fetch\` or set BASE=<ref>" >&2
  exit 0
fi

# The pathspecs below are relative to the root, whatever folder the script is run from
cd "$(git rev-parse --show-toplevel)" || exit 0

# Compute the diff (code, migrations and scripts only: see the header)
scope=(-- . ':(exclude)specs' ':(exclude)_bmad-output' ':(exclude)docs' ':(exclude)features'
  ':(exclude)**/tests/bdd/**' ':(exclude)**/fixtures/**' ':(exclude)Cargo.lock')
diff_content="$(git diff "$BASE"...HEAD "${scope[@]}" 2>/dev/null)"
if [ -z "$diff_content" ]; then
  echo "[review-diff] nothing to review (no diff vs $BASE)" >&2
  exit 0
fi

# Cap on the diff size: a local review of huge refactors is slow
diff_lines=$(echo "$diff_content" | wc -l | tr -d ' ')
if [ "$diff_lines" -gt "$MAX_DIFF_LINES" ]; then
  {
    echo ""
    echo "⚠️  [review-diff] SECURITY REVIEW NOT RUN: code diff of $diff_lines lines > MAX_DIFF_LINES $MAX_DIFF_LINES"
    echo "⚠️  Run it by hand: MAX_DIFF_LINES=$((diff_lines + 1)) bash scripts/review-diff.sh"
    echo ""
  } >&2
  exit 0
fi

# List of changed files, for context
files_changed="$(git diff --name-only "$BASE"...HEAD "${scope[@]}" 2>/dev/null)"

echo "[review-diff] Claude security review: $current_branch vs $BASE ($diff_lines diff lines)" >&2
echo "" >&2

# Quoted heredoc: apostrophes and $ in the instructions are taken literally.
instructions="$(cat <<'EOF'
You are a security reviewer for topictoast (workflow orchestrator in Rust: API, scheduler and workers that run the tasks users define in the Open Workflow Specification DSL, web dashboard).

Analyse the diff below and report ONLY high-confidence findings, each with:
- exact file:line
- attack repro or logical repro
- concrete fix (1-3 lines)

Coverage: auth of the API and the dashboard, command and container execution of run tasks (command injection, escape), SSRF in call http tasks, evaluation of DSL runtime expressions (injection, unbounded resource use), secrets in logs and outputs, duplicate runs and idempotency of the scheduler, unvalidated workflow documents, OWASP Top 10, race conditions, logic bugs.

Severity: 🔴 CRITICAL | 🟡 HIGH | 🟠 MEDIUM | 🔵 LOW.

No generic best practices. If the diff is clean: "✅ No findings."

Project rules: CLAUDE.md and .specify/memory/constitution.md (pure hexagonal domain, Gateway vs Repository, tight boundaries, typed data).
EOF
)"

prompt="$instructions

Files changed:
$files_changed

Diff:
\`\`\`
$diff_content
\`\`\`"

# Run claude in print mode (non-interactive), without saving the session to disk.
# Read-only tools and without the hook's GIT_* variables: a review once created a test commit
# that emptied the branch (GIT_DIR inherited from the pre-push hook). `--tools` only filters the
# built-ins: `--strict-mcp-config` with an empty config also removes the MCP servers (plugins and
# connectors, with shells and paid actions) and `--permission-mode default` denies in --print
# any tool that asks for permission, even if the user's settings are in `auto`.
echo "$prompt" | env -u GIT_DIR -u GIT_INDEX_FILE -u GIT_WORK_TREE \
  claude --print --no-session-persistence --tools "Read,Grep,Glob" \
  --strict-mcp-config --mcp-config '{"mcpServers":{}}' --permission-mode default 2>&1 || {
  echo "[review-diff] claude exit $?: review failed, continuing" >&2
}

exit 0
