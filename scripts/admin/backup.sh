#!/bin/bash
# SIS Infrastructure Backup Script v3
# Uses restic for deduplicated, encrypted backups
# Includes offsite sync to Backblaze B2
set -uo pipefail

RESTIC_REPO="/mnt/pool_HDD_x2/tank/datasources/sis/backups/restic-repo-new"
RESTIC_PASSWORD_FILE="/mnt/pool_HDD_x2/tank/datasources/sis/backups/.restic-password"
DATA_BASE="/mnt/pool_HDD_x2/tank/datasources/sis/appdata"
STACKS_BASE="/mnt/pool_HDD_x2/infra/stacks/stacks"
LOG_FILE="/mnt/pool_HDD_x2/tank/datasources/sis/backups/backup-$(date +%Y%m%d-%H%M%S).log"
DUMP_DIR="/mnt/pool_HDD_x2/tank/datasources/sis/backups/db-dumps-$(date +%Y%m%d-%H%M%S)"

# Offsite (Backblaze B2) configuration
B2_REPO="s3:https://s3.eu-central-003.backblazeb2.com/SisInfraBackup/repo"
# Creds come from env (cron-trigger) or the root-only file (host cron)
if [ -z "${OFFSITE_AWS_KEY:-}" ] && [ -f /mnt/pool_HDD_x2/tank/datasources/sis/backups/.offsite-credentials ]; then
    . /mnt/pool_HDD_x2/tank/datasources/sis/backups/.offsite-credentials
fi
B2_KEY="${OFFSITE_AWS_KEY:?OFFSITE_AWS_KEY not set}"
B2_SECRET="${OFFSITE_AWS_SECRET:?OFFSITE_AWS_SECRET not set}"

# Textfile collector for Prometheus metrics
TEXTFILE_DIR="/mnt/pool_HDD_x2/tank/datasources/sis/appdata/monitoring/textfile-collector"

restic_cmd() { restic -r "$RESTIC_REPO" --password-file "$RESTIC_PASSWORD_FILE" "$@"; }
log() { echo "[$(date '+%Y-%m-%d %H:%M:%S')] $*" | tee -a "$LOG_FILE"; }

BACKUP_START=$(date +%s)
mkdir -p "$DUMP_DIR"
log "=== Starting SIS Infrastructure Backup ==="

# === Database dumps ===
log "Dumping databases..."
docker exec headscale-postgres pg_dumpall -U headscale > "$DUMP_DIR/headscale.sql" 2>/dev/null || true
docker exec iam-postgres pg_dumpall -U keycloak > "$DUMP_DIR/keycloak.sql" 2>/dev/null || true
docker exec operations-postgres-forgejo pg_dumpall -U forgejo > "$DUMP_DIR/forgejo.sql" 2>/dev/null || true
docker exec erpnext-mariadb mariadb-dump -u root -perpnext123 --all-databases > "$DUMP_DIR/erpnext.sql" 2>/dev/null || true
docker exec documents-postgres pg_dumpall -U paperless > "$DUMP_DIR/paperless.sql" 2>/dev/null || true
docker exec photos-postgres pg_dumpall -U immich > "$DUMP_DIR/immich.sql" 2>/dev/null || true
docker exec collaboration-postgres pg_dumpall -U synapse > "$DUMP_DIR/synapse.sql" 2>/dev/null || true
cp "$DATA_BASE/vaultwarden/db.sqlite3" "$DUMP_DIR/vaultwarden.sqlite3" 2>/dev/null || true
log "Database dumps complete"

# === Restic backups (split by service to limit memory) ===
log "Backing up configs..."
restic_cmd backup --tag configs "$STACKS_BASE" >> "$LOG_FILE" 2>&1 || log "WARN: configs backup had issues"

log "Backing up db-dumps..."
restic_cmd backup --tag db-dumps "$DUMP_DIR" >> "$LOG_FILE" 2>&1 || log "WARN: db-dumps backup had issues"

log "Backing up iam..."
restic_cmd backup --tag appdata --tag iam "$DATA_BASE/iam" >> "$LOG_FILE" 2>&1 || true

log "Backing up operations..."
restic_cmd backup --tag appdata --tag operations "$DATA_BASE/operations" >> "$LOG_FILE" 2>&1 || true

log "Backing up vaultwarden..."
restic_cmd backup --tag appdata --tag vaultwarden "$DATA_BASE/vaultwarden" >> "$LOG_FILE" 2>&1 || true

log "Backing up proxy..."
restic_cmd backup --tag appdata --tag proxy "$DATA_BASE/proxy" >> "$LOG_FILE" 2>&1 || true

log "Backing up monitoring..."
restic_cmd backup --tag appdata --tag monitoring "$DATA_BASE/monitoring" >> "$LOG_FILE" 2>&1 || true

log "Backing up storage..."
restic_cmd backup --tag appdata --tag storage "$DATA_BASE/storage" >> "$LOG_FILE" 2>&1 || true

log "Backing up documents..."
restic_cmd backup --tag appdata --tag documents "$DATA_BASE/documents" >> "$LOG_FILE" 2>&1 || true

log "Backing up photos..."
restic_cmd backup --tag appdata --tag photos "$DATA_BASE/photos" >> "$LOG_FILE" 2>&1 || true

log "Backing up remaining services..."
restic_cmd backup --tag appdata \
  "$DATA_BASE/security" "$DATA_BASE/vpn" "$DATA_BASE/utility" \
  "$DATA_BASE/books" "$DATA_BASE/collaboration" "$DATA_BASE/rss" \
  >> "$LOG_FILE" 2>&1 || true

# === Retention ===
log "Applying retention..."
restic_cmd forget --keep-daily 30 --keep-weekly 12 --keep-monthly 12 --prune >> "$LOG_FILE" 2>&1 || true

log "Verifying integrity..."
restic_cmd check >> "$LOG_FILE" 2>&1 || true

# === Offsite sync to Backblaze B2 ===
# Uses backup-restic container which has network_mode: bridge (working DNS)
# The container has /restic-repo-new mounted
log "Syncing to offsite repository (B2)..."

# Write password to container temp file for restic copy
RESTIC_PASS=$(cat "$RESTIC_PASSWORD_FILE")
docker exec backup-restic sh -c "echo -n '${RESTIC_PASS}' > /tmp/restic-pass && chmod 600 /tmp/restic-pass" 2>/dev/null

# Clear stale locks on BOTH repos (OOM-kills during copy leave exclusive
# locks that block every subsequent sync - Sep 2025 incident)
docker exec \
  -e AWS_ACCESS_KEY_ID="$B2_KEY" \
  -e AWS_SECRET_ACCESS_KEY="$B2_SECRET" \
  backup-restic restic \
    --password-file /tmp/restic-pass \
    -r "$B2_REPO" unlock --remove-all >> "$LOG_FILE" 2>&1 || true
docker exec backup-restic restic \
  -r /restic-repo-new --password-file /tmp/restic-pass unlock --remove-all >> "$LOG_FILE" 2>&1 || true

# Copy all local snapshots to B2
if docker exec \
  -e AWS_ACCESS_KEY_ID="$B2_KEY" \
  -e AWS_SECRET_ACCESS_KEY="$B2_SECRET" \
  backup-restic restic \
    --password-file /tmp/restic-pass \
    -r "$B2_REPO" \
    copy --from-repo /restic-repo-new --from-password-file /tmp/restic-pass \
    >> "$LOG_FILE" 2>&1; then
    OFFSITE_SUCCESS=$(date +%s)
    log "Offsite sync completed successfully."
else
    OFFSITE_SUCCESS=0
    log "WARN: Offsite sync had issues."
fi

# Cleanup temp password
docker exec backup-restic rm -f /tmp/restic-pass 2>/dev/null

# === Update monitoring metrics ===
BACKUP_END=$(date +%s)
BACKUP_DURATION=$((BACKUP_END - BACKUP_START))
mkdir -p "$TEXTFILE_DIR"
cat > "$TEXTFILE_DIR/backup.prom" <<EOF
# HELP sis_backup_last_success Unix timestamp of last successful backup
# TYPE sis_backup_last_success gauge
sis_backup_last_success $(date +%s)
# HELP sis_backup_last_run Unix timestamp of last backup attempt
# TYPE sis_backup_last_run gauge
sis_backup_last_run $(date +%s)
# HELP sis_backup_duration_seconds Duration of last backup run in seconds
# TYPE sis_backup_duration_seconds gauge
sis_backup_duration_seconds ${BACKUP_DURATION}
# HELP sis_backup_offsite_last_success Unix timestamp of last successful offsite sync
# TYPE sis_backup_offsite_last_success gauge
sis_backup_offsite_last_success ${OFFSITE_SUCCESS:-0}
EOF

log "=== Backup Complete ==="

# Cleanup
rm -rf "$DUMP_DIR"
find /mnt/pool_HDD_x2/tank/datasources/sis/backups/ -maxdepth 1 -name "backup-*.log" -type f -mtime +30 -delete 2>/dev/null || true
