# Start the desktop with its local AI assistant (release mode for inference).
dev:
    #!/usr/bin/env bash
    set -euo pipefail
    export OCTOSENSE_HOME="${OCTOSENSE_HOME:-${MAKEOS_HOME:-$HOME/.octosense}}"
    export MAKEPAD_AI_CHAT_MODEL="${MAKEPAD_AI_CHAT_MODEL:-$OCTOSENSE_HOME/weights/Qwen3.5-9B-UD-Q4_K_XL.gguf}"
    if [[ ! -f "$MAKEPAD_AI_CHAT_MODEL" ]]; then
        printf 'Model missing: %s\nInstall the weights using docs/local-ai.md, or set MAKEPAD_AI_CHAT_MODEL.\n' "$MAKEPAD_AI_CHAT_MODEL" >&2
        exit 1
    fi
    printf 'Assistant model: %s\n' "$MAKEPAD_AI_CHAT_MODEL"
    exec cargo run --release --locked -- --assistant
