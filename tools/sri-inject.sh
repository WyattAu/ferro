#!/usr/bin/env bash
# AX-006: Compute and inject Subresource Integrity (SRI) hashes into index.html.
# Usage: ./tools/sri-inject.sh [path/to/dist]
# Must be run AFTER trunk build.

set -euo pipefail

DIST_DIR="${1:-crates/web/dist}"
INDEX="$DIST_DIR/index.html"

if [ ! -f "$INDEX" ]; then
    echo "ERROR: $INDEX not found. Run 'trunk build' first." >&2
    exit 1
fi

# For each <link> or <script> with a local href/src, compute SRI and inject integrity= attribute.
# We use sha384 as per W3C recommendation.

TMPFILE=$(mktemp)
trap 'rm -f "$TMPFILE"' EXIT

cp "$INDEX" "$TMPFILE"

# Process <script src="..."> tags
while IFS= read -r -d '' match; do
    SRC=$(echo "$match" | grep -oP 'src="\K[^"]+')
    if [ -z "$SRC" ] || echo "$SRC" | grep -qE '^(https?:|//)'; then
        continue
    fi
    FILE="$DIST_DIR/$SRC"
    if [ ! -f "$FILE" ]; then
        echo "WARNING: $FILE not found, skipping SRI" >&2
        continue
    fi
    HASH=$(openssl dgst -sha384 -binary "$FILE" | openssl base64 -A)
    INTEGRITY="sha384-$HASH"
    # Replace the tag in the temp file
    sed -i "s|src=\"$SRC\"|src=\"$SRC\" integrity=\"$INTEGRITY\"|g" "$TMPFILE"
done < <(grep -zoP '<script[^>]*src="[^"]*"[^>]*>' "$INDEX" || true)

# Process <link rel="stylesheet" href="..."> tags
while IFS= read -r -d '' match; do
    HREF=$(echo "$match" | grep -oP 'href="\K[^"]+')
    if [ -z "$HREF" ] || echo "$HREF" | grep -qE '^(https?:|//)'; then
        continue
    fi
    FILE="$DIST_DIR/$HREF"
    if [ ! -f "$FILE" ]; then
        echo "WARNING: $FILE not found, skipping SRI" >&2
        continue
    fi
    HASH=$(openssl dgst -sha384 -binary "$FILE" | openssl base64 -A)
    INTEGRITY="sha384-$HASH"
    sed -i "s|href=\"$HREF\"|href=\"$HREF\" integrity=\"$INTEGRITY\"|g" "$TMPFILE"
done < <(grep -zoP '<link[^>]*rel="stylesheet"[^>]*href="[^"]*"[^>]*>' "$INDEX" || true)

mv "$TMPFILE" "$INDEX"
echo "SRI hashes injected into $INDEX"
