#!/usr/bin/env bash
# Batch-create Ferro user accounts via the admin API.
# Usage: ./batch_create_users.sh <users.csv> <admin_token>
# CSV format: username,display_name,email (one per line, no header)
set -euo pipefail
CSV_FILE="${1:?Usage: batch_create_users.sh <users.csv> <admin_token>}"
TOKEN="${2:?Usage: batch_create_users.sh <users.csv> <admin_token>}"
FERRO_URL="${FERRO_URL:-https://ferro.wyattau.com}"
CREATED=0; SKIPPED=0; FAILED=0
while IFS=',' read -r username display_name email; do
    [[ -z "$username" || "$username" == \#* ]] && continue
    response=$(curl -sk --max-time 15 -X POST \
        -H "Authorization: Bearer $TOKEN" \
        -H "Content-Type: application/json" \
        -d "{\"username\":\"$username\",\"display_name\":\"$display_name\",\"email\":\"$email\",\"password\":\"$(openssl rand -base64 16)\",\"role\":\"user\"}" \
        "$FERRO_URL/api/admin/users" 2>&1)
    if echo "$response" | grep -q "already exists"; then
        echo "SKIP: $username"; ((SKIPPED++))
    else
        echo "OK: $username"; ((CREATED++))
    fi
done < "$CSV_FILE"
echo ""; echo "Done: $CREATED created, $SKIPPED skipped, $FAILED failed"
echo "Send users the Keycloak password reset link: https://auth.wyattau.com/realms/company-realm/login-actions/reset-credentials"
