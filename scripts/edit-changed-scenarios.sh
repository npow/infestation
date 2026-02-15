#!/bin/bash
# Edit all new or modified scenario files in the scenario editor

set -e
cd "$(dirname "$0")/.."

# Get list of changed scenario files (_1.csv, _2.csv, _i.json) from git status
# This handles both tracked modifications and untracked files
scenarios=""

# Check for modified tracked files (any of _1.csv, _2.csv, _i.json)
for f in $(git status --porcelain | grep -E '(_1\.csv|_2\.csv|_i\.json)$' | sed 's/^...//'); do
    name=$(basename "$f" | sed -E 's/_(1\.csv|2\.csv|i\.json)$//')
    scenarios="$scenarios $name"
done

# Check for untracked scenario directories
for dir in $(git status --porcelain | grep '^??' | sed 's/^?? //' | grep 'scenarios/$'); do
    for f in "$dir"*_i.json; do
        if [ -f "$f" ]; then
            name=$(basename "$f" _i.json)
            scenarios="$scenarios $name"
        fi
    done
done

# Deduplicate and trim whitespace
scenarios=$(echo $scenarios | tr ' ' '\n' | sort -u | xargs)

if [ -z "$scenarios" ]; then
    echo "No new or modified scenarios found"
    exit 0
fi

echo "Found changed scenarios:"
echo "$scenarios" | tr ' ' '\n'
echo ""

# Run the scenario editor with the list of scenarios
exec cargo run -r --bin scenario_editor -- $scenarios
