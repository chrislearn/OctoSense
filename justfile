# Start the desktop with its local AI assistant and AppCard module.
dev:
    #!/usr/bin/env bash
    set -euo pipefail
    export OCTOSENSE_HOME="${OCTOSENSE_HOME:-${MAKEOS_HOME:-$HOME/.octosense}}"
    export MAKEPAD_AI_CHAT_MODEL="${MAKEPAD_AI_CHAT_MODEL:-$OCTOSENSE_HOME/weights/Qwen3.5-9B-UD-Q4_K_XL.gguf}"
    if [[ ! -f "$MAKEPAD_AI_CHAT_MODEL" ]]; then
        printf 'Model missing: %s\nInstall the weights using docs/local-ai.md, or set MAKEPAD_AI_CHAT_MODEL.\n' "$MAKEPAD_AI_CHAT_MODEL" >&2
        exit 1
    fi
    export OCTOS_APP_CORE_BIN="${OCTOS_APP_CORE_BIN:-$(command -v octos || true)}"
    if [[ ! -x "$OCTOS_APP_CORE_BIN" ]]; then
        printf 'Octos executable missing: %s\n' "$OCTOS_APP_CORE_BIN" >&2
        exit 1
    fi
    if [[ -z "${OCTOS_APP_CORE_DIR:-}" ]]; then
        export OCTOS_APP_CORE_DIR="$OCTOSENSE_HOME/appcard/core"
        python3 tools/prepare-appcard-core.py \
            --source-home "${OCTOSENSE_APPCARD_SOURCE_HOME:-${OCTOS_HOME:-$HOME/.octos}}" \
            --profile "${OCTOSENSE_APPCARD_PROFILE:-octos}" \
            --output "$OCTOS_APP_CORE_DIR"
    fi
    printf 'Assistant model: %s\n' "$MAKEPAD_AI_CHAT_MODEL"
    exec cargo run --release --locked --features app-appcard -- --module appcard --assistant --test-action launch-appcard
