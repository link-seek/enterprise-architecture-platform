#!/bin/bash
# on_stop.sh — OpenHands agent finish gate
#
# When the agent calls FinishTool, this hook runs:
#   1. pre-commit (local, fast)
#   2. git push (triggers PR CI)
#   3. Poll PR CI (review-ai + E2E) — wait up to 30min
#
# If any step fails: BLOCK with failure details → agent must fix → retry
# If all pass: ALLOW stop
#
# Exit codes:
#   0 + {"decision": "allow"}  → agent may finish
#   2 + {"decision": "deny"}   → agent must fix and retry

set -o pipefail

PROJECT_DIR="${OPENHANDS_PROJECT_DIR:-$(pwd)}"
cd "$PROJECT_DIR" || exit 1

ISSUES=""
BLOCK_STOP=false

log_issue() {
    ISSUES="${ISSUES}${1}\n"
    BLOCK_STOP=true
}

>&2 echo "=== on_stop.sh Hook ==="
>&2 echo "Project: $PROJECT_DIR"
>&2 echo ""

# ──────────────────────────────────────────────
# Step 1: pre-commit (local, fast)
# ──────────────────────────────────────────────
if command -v pre-commit &>/dev/null; then
    >&2 echo "=== Step 1: pre-commit ==="
    PRECOMMIT_OUTPUT=$(pre-commit run --all-files 2>&1)
    PRECOMMIT_EXIT=$?
    >&2 echo "$PRECOMMIT_OUTPUT"
    if [ $PRECOMMIT_EXIT -ne 0 ]; then
        >&2 echo "pre-commit failed (exit: $PRECOMMIT_EXIT)"
        log_issue "## Pre-commit Failed\n\n\`\`\`\n${PRECOMMIT_OUTPUT}\n\`\`\`"
    else
        >&2 echo "pre-commit passed"
    fi
    >&2 echo ""
fi

# ──────────────────────────────────────────────
# Step 2: git push (triggers PR CI)
# ──────────────────────────────────────────────
>&2 echo "=== Step 2: git push ==="

# Check if there are uncommitted changes
if ! git diff --cached --quiet 2>/dev/null || ! git diff --quiet 2>/dev/null || [ -n "$(git ls-files --others --exclude-standard 2>/dev/null)" ]; then
    >&2 echo "Uncommitted changes detected, committing..."
    git add -A
    COMMIT_MSG="auto-fix: $(date -u +%Y%m%d%H%M%S)"
    git commit -m "$COMMIT_MSG" || true
fi

# Get branch and push
BRANCH=$(git rev-parse --abbrev-ref HEAD 2>/dev/null)
>&2 echo "Branch: $BRANCH"

LOCAL_SHA=$(git rev-parse HEAD 2>/dev/null)
REMOTE_SHA=$(git ls-remote origin "$BRANCH" 2>/dev/null | awk '{print $1}')

if [ "$LOCAL_SHA" = "$REMOTE_SHA" ]; then
    >&2 echo "Already pushed (${LOCAL_SHA:0:8}), skipping push"
else
    >&2 echo "Pushing ${LOCAL_SHA:0:8} to origin/$BRANCH..."
    PUSH_OUTPUT=$(git push origin HEAD:"$BRANCH" 2>&1)
    PUSH_EXIT=$?
    >&2 echo "$PUSH_OUTPUT"
    if [ $PUSH_EXIT -ne 0 ]; then
        log_issue "## Git Push Failed\n\n\`\`\`\n${PUSH_OUTPUT}\n\`\`\`"
    else
        >&2 echo "Push successful"
        # Refresh SHA after push
        LOCAL_SHA=$(git rev-parse HEAD 2>/dev/null)
    fi
fi
>&2 echo ""

# ──────────────────────────────────────────────
# Step 3: Poll PR CI (review-ai + E2E)
# ──────────────────────────────────────────────
if [ "$BLOCK_STOP" = true ]; then
    >&2 echo "Skipping CI poll (earlier step failed)"
else
    >&2 echo "=== Step 3: Poll PR CI ==="

    # Extract repo from remote
    GITHUB_REMOTE=$(git remote -v 2>/dev/null | grep -E "(github\.com.*push)" | head -1)
    REPO=$(echo "$GITHUB_REMOTE" | sed -E 's|.*github\.com[:/]([^/]+/[^/.]+).*|\1|')

    if [ -z "$REPO" ]; then
        >&2 echo "No GitHub remote found, skipping CI poll"
    elif [ -z "$GITHUB_TOKEN" ] && ! command -v gh &>/dev/null; then
        >&2 echo "No gh CLI or GITHUB_TOKEN, skipping CI poll"
    else
        >&2 echo "Repo: $REPO"
        >&2 echo "SHA: ${LOCAL_SHA:0:8}"

        MAX_WAIT=1800   # 30 minutes
        WAIT_INTERVAL=30
        TOTAL_WAITED=0

        while [ $TOTAL_WAITED -lt $MAX_WAIT ]; do
            # Get check runs for this commit
            if command -v gh &>/dev/null; then
                CHECKS=$(gh api "repos/$REPO/commits/$LOCAL_SHA/check-runs" \
                    --jq '.check_runs | map({name: .name, status: .status, conclusion: .conclusion})' 2>/dev/null || echo "[]")
            else
                CHECKS=$(curl -sf -H "Authorization: token $GITHUB_TOKEN" \
                    "https://api.github.com/repos/$REPO/commits/$LOCAL_SHA/check-runs" \
                    | jq '.check_runs | map({name: .name, status: .status, conclusion: .conclusion})' 2>/dev/null || echo "[]")
            fi

            TOTAL=$(echo "$CHECKS" | jq 'length')
            IN_PROGRESS=$(echo "$CHECKS" | jq '[.[] | select(.status != "completed")] | length')
            FAILED=$(echo "$CHECKS" | jq '[.[] | select(.conclusion == "failure" or .conclusion == "timed_out" or .conclusion == "cancelled")] | length')

            >&2 echo "CI: $TOTAL checks, $IN_PROGRESS in progress, $FAILED failed (${TOTAL_WAITED}s / ${MAX_WAIT}s)"

            if [ "$IN_PROGRESS" -eq 0 ] || [ "$FAILED" -gt 0 ]; then
                break
            fi

            sleep $WAIT_INTERVAL
            TOTAL_WAITED=$((TOTAL_WAITED + WAIT_INTERVAL))
        done

        # Check for failures
        if [ "$FAILED" -gt 0 ]; then
            >&2 echo "CI FAILED: $FAILED check(s) failed"

            FAILED_DETAILS=$(echo "$CHECKS" | jq -r '.[] | select(.conclusion == "failure" or .conclusion == "timed_out" or .conclusion == "cancelled") | "- \(.name): \(.conclusion)"')

            FAILURE_MSG="## CI Failed\n\n$FAILED_DETAILS\n"

            # Download playwright artifacts for E2E failure details
            if command -v gh &>/dev/null; then
                RUN_ID=$(gh api "repos/$REPO/actions/runs?head_sha=$LOCAL_SHA&per_page=10" \
                    --jq '[.workflow_runs[] | select(.conclusion == "failure")] | first | .id' 2>/dev/null || echo "")

                if [ -n "$RUN_ID" ] && [ "$RUN_ID" != "null" ]; then
                    ARTIFACT_ID=$(gh api "repos/$REPO/actions/runs/$RUN_ID/artifacts" \
                        --jq '[.artifacts[] | select(.name == "playwright-results")] | first | .id' 2>/dev/null || echo "")

                    if [ -n "$ARTIFACT_ID" ] && [ "$ARTIFACT_ID" != "null" ]; then
                        >&2 echo "Downloading playwright-results artifact..."
                        ARTIFACTS_DIR=$(mktemp -d)
                        if gh api "repos/$REPO/actions/runs/$RUN_ID/artifacts/$ARTIFACT_ID/zip" > "$ARTIFACTS_DIR/pw.zip" 2>/dev/null; then
                            if unzip -t -q "$ARTIFACTS_DIR/pw.zip" 2>/dev/null; then
                                (cd "$ARTIFACTS_DIR" && unzip -o -q pw.zip 2>/dev/null || true)
                                ERROR_CONTEXT=$(find "$ARTIFACTS_DIR" -name 'error-context.md' -exec cat {} \; 2>/dev/null | head -150 || echo "")
                                if [ -n "$ERROR_CONTEXT" ]; then
                                    FAILURE_MSG="${FAILURE_MSG}\n### Playwright Error Context\n\`\`\`markdown\n${ERROR_CONTEXT}\n\`\`\`"
                                fi
                            fi
                        fi
                        rm -rf "$ARTIFACTS_DIR"
                    fi
                fi

                # Get failed job step details
                JOBS_OUTPUT=$(gh api "repos/$REPO/actions/runs/$RUN_ID/jobs" \
                    --jq '.jobs[] | select(.conclusion == "failure") | "### \(.name)\n" + (.steps[] | select(.conclusion == "failure") | "- \(.name): \(.conclusion)")' 2>/dev/null | head -50 || echo "")
                if [ -n "$JOBS_OUTPUT" ]; then
                    FAILURE_MSG="${FAILURE_MSG}\n### Failed Steps\n\`\`\`\n${JOBS_OUTPUT}\n\`\`\`"
                fi
            fi

            log_issue "$FAILURE_MSG"
        elif [ "$IN_PROGRESS" -gt 0 ]; then
            >&2 echo "CI still running after ${MAX_WAIT}s timeout"
            log_issue "## CI Still Running\n\nCI checks still in progress after ${MAX_WAIT}s. The agent may continue or wait for the next iteration."
        else
            >&2 echo "All CI checks passed!"
        fi
    fi
fi
>&2 echo ""

# ──────────────────────────────────────────────
# Final decision
# ──────────────────────────────────────────────
if [ "$BLOCK_STOP" = true ]; then
    >&2 echo "=== BLOCKING STOP ==="
    ESCAPED_ISSUES=$(echo -e "$ISSUES" | jq -Rs .)
    echo "{\"decision\": \"deny\", \"reason\": \"Checks failed\", \"additionalContext\": $ESCAPED_ISSUES}"
    exit 2
fi

>&2 echo "=== All checks passed, allowing stop ==="
echo '{"decision": "allow"}'
exit 0
