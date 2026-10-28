#!/bin/bash

set -euo pipefail
cd "$(dirname "$0")/.."

# Temporary exemptions. Remove each entry when its owning goal is implemented.
readonly -a TEMPORARY_ALLOWLIST=(
    "app/src/ai/observability/cost/calculator.rs" # Goal 7: cost calculation
    "app/src/ai/observability/usage/categories.rs" # Goal 7: usage categorization
)

is_temporarily_allowed() {
    local file=$1
    local allowed
    for allowed in "${TEMPORARY_ALLOWLIST[@]}"; do
        [[ "$file" == "$allowed" ]] && return 0
    done
    return 1
}

fail=0
while IFS= read -r file; do
    [[ -s "$file" ]] && continue
    if is_temporarily_allowed "$file"; then
        echo "ALLOWLISTED EMPTY: $file"
        continue
    fi
    echo "EMPTY: $file"
    fail=1
done < <(git ls-files '*.rs')

exit "$fail"
