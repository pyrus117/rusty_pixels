#!/usr/bin/env bash
# Run one headless Claude Code task on MiMo (Xiaomi Token Plan) inside the repo.
# Usage: tools/mimo.sh <prompt-file> [log-file]
# Recipe from Helm's pilots.rs: clean env (inherited CLAUDE_CODE_* vars 401 the request),
# override model AND fallback, tool search on (MiMo 400s on the full MCP schema set).
set -euo pipefail

prompt_file=${1:?usage: tools/mimo.sh <prompt-file> [log-file]}
log_file=${2:-/dev/stdout}
# MIMO_REPO points a worker at a worktree while the launcher itself comes from main.
repo=${MIMO_REPO:-$(cd "$(dirname "$0")/.." && pwd)}
key_file="$HOME/.config/helm/keys/mimo"

[[ -r $prompt_file ]] || { echo "mimo.sh: cannot read prompt file $prompt_file" >&2; exit 2; }
[[ -r $key_file ]] || { echo "mimo.sh: missing MiMo key at $key_file" >&2; exit 2; }

model=mimo-v2.6-pro
small=mimo-v2.6-flash
# Extra allowed tools for one task, e.g. MIMO_EXTRA_TOOLS='Bash(/path/to/aseprite:*)'.
read -r -a extra_tools <<< "${MIMO_EXTRA_TOOLS:-}"

cd "$repo"
env -i \
  HOME="$HOME" USER="$USER" LANG="${LANG:-C.UTF-8}" TERM=dumb \
  XDG_RUNTIME_DIR="${XDG_RUNTIME_DIR:-}" \
  PATH="$HOME/.local/bin:$HOME/.cargo/bin:/usr/lib/rustup/bin:/usr/local/bin:/usr/bin:/bin" \
  ANTHROPIC_BASE_URL="https://token-plan-sgp.xiaomimimo.com/anthropic" \
  ANTHROPIC_AUTH_TOKEN="$(tr -d '[:space:]' < "$key_file")" \
  ANTHROPIC_MODEL=$model ANTHROPIC_DEFAULT_OPUS_MODEL=$model ANTHROPIC_DEFAULT_SONNET_MODEL=$model \
  ANTHROPIC_DEFAULT_FABLE_MODEL=$model CLAUDE_CODE_SUBAGENT_MODEL=$model CLAUDE_CODE_AUTO_MODE_MODEL=$model \
  ANTHROPIC_DEFAULT_HAIKU_MODEL=$small ANTHROPIC_SMALL_FAST_MODEL=$small \
  CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC=1 ENABLE_TOOL_SEARCH=true \
  CLAUDE_CODE_MAX_OUTPUT_TOKENS="${MIMO_MAX_OUTPUT_TOKENS:-64000}" \
  claude -p "$(cat "$prompt_file")" \
    --model "$model" --fallback-model "$small" \
    --permission-mode acceptEdits \
    --allowedTools "Bash(cargo:*)" "Bash(git add:*)" "Bash(git commit:*)" "Bash(git status:*)" "Bash(git diff:*)" "Bash(rg:*)" "Bash(find:*)" "Bash(ls:*)" "Bash(mkdir:*)" "Bash(bash .claude/verify.sh)" "${extra_tools[@]}" \
    --disallowedTools "Agent" "Bash(tools/mimo.sh:*)" "Bash(bash tools/mimo.sh:*)" "Bash(claude:*)" \
    --output-format json \
  > "$log_file"
