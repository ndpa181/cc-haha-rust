#!/usr/bin/env bash
# Setup claude code model-switching launcher scripts on a remote machine.
# Usage: bash setup-claude-launchers.sh [--binary PATH]
#
# Options:
#   --binary PATH    Path to claude binary (default: first found in PATH,
#                    fallback to /opt/homebrew/bin/claude)
set -euo pipefail

MODELS="mimo qwen minimax kimi deepseek opus glm haha local"

# --- Determine claude binary ---
CLAUDE_BIN=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --binary) CLAUDE_BIN="$2"; shift 2 ;;
    *) shift ;;
  esac
done

if [[ -z "$CLAUDE_BIN" ]]; then
  # Search common locations in order
  for p in "$HOME/.local/bin/claude" \
           "/opt/homebrew/bin/claude" \
           "/usr/local/bin/claude" \
           "/usr/bin/claude"; do
    if [[ -x "$p" ]]; then
      CLAUDE_BIN="$p"
      break
    fi
  done
  # Fallback to PATH lookup
  if [[ -z "$CLAUDE_BIN" ]]; then
    CLAUDE_BIN="$(command -v claude 2>/dev/null || true)"
  fi
fi

if [[ -z "$CLAUDE_BIN" ]]; then
  echo "Error: claude binary not found. Install claude first, or use --binary PATH" >&2
  exit 1
fi

# --- Create ~/bin ---
mkdir -p ~/bin

# --- Write cc-* scripts ---
for model in $MODELS; do
  cat > "$HOME/bin/cc-${model}" <<SCRIPT
#!/usr/bin/env bash
# Launch claude with ~/.claude/settings.${model}.json
set -euo pipefail

CLAUDE_BIN="${CLAUDE_BIN}"
SETTINGS_FILE="\$HOME/.claude/settings.${model}.json"

if [[ ! -f "\$CLAUDE_BIN" ]]; then
  echo "Error: \$CLAUDE_BIN not found" >&2
  exit 1
fi

if [[ ! -f "\$SETTINGS_FILE" ]]; then
  echo "Error: \$SETTINGS_FILE not found" >&2
  exit 1
fi

exec "\$CLAUDE_BIN" --settings "\$SETTINGS_FILE" "\$@"
SCRIPT
  chmod +x "$HOME/bin/cc-${model}"
done

# --- Write cc-mini (uses ~/.local/bin/claude with mimo settings) ---
cat > "$HOME/bin/cc-mini" <<'SCRIPT'
#!/usr/bin/env bash
# Launch local claude with ~/.claude/settings.mimo.json
set -euo pipefail

CLAUDE_BIN="$HOME/.local/bin/claude"
SETTINGS_FILE="$HOME/.claude/settings.mimo.json"

if [[ ! -f "$CLAUDE_BIN" ]]; then
  echo "Error: $CLAUDE_BIN not found" >&2
  exit 1
fi

if [[ ! -f "$SETTINGS_FILE" ]]; then
  echo "Error: $SETTINGS_FILE not found" >&2
  exit 1
fi

exec "$CLAUDE_BIN" --settings "$SETTINGS_FILE" "$@"
SCRIPT
chmod +x "$HOME/bin/cc-mini"

# --- Ensure ~/bin is in PATH ---
if ! echo "$PATH" | tr ':' '\n' | grep -q "^${HOME}/bin$"; then
  shell_rc="$HOME/.zshrc"
  if [[ ! -f "$shell_rc" ]]; then
    shell_rc="$HOME/.bashrc"
    touch "$shell_rc"
  fi
  echo 'export PATH="$HOME/bin:$PATH"' >> "$shell_rc"
  export PATH="$HOME/bin:$PATH"
fi

# --- Verify ---
echo ""
echo "Installed launcher scripts:"
ls -1 ~/bin/cc-*
echo ""
echo "Claude binary: $CLAUDE_BIN"
if command -v claude >/dev/null 2>&1; then
  echo "claude version: $(claude --version 2>&1 || echo "unknown")"
fi
echo ""
echo "Next steps:"
echo "  1. Create ~/.claude/settings.<model>.json for each model"
echo "  2. Run cc-<model> to launch with that model"
