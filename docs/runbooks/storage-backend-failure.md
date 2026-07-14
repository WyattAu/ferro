# Runbook: Storage Backend Failure

## Symptoms

- `503 Storage Unavailable` responses
- Upload/download timeouts
- `Connection refused` or `Request timed out` errors for S3/GCS/Azure endpoints
- Local filesystem: `No space left on device` or `Permission denied`

## Diagnosis

1. **Check backend connectivity**
   ```bash
   # S3
   aws s3 ls s3://<bucket-name>/ --endpoint-url <endpoint>

   # GCS
   gsutil ls gs://<bucket-name>/

   # Azure
   az storage blob list --container-name <name> --account-name <account>

   # Local FS
   ls -la /var/lib/ferro/storage/
   ```

2. **Check credentials/secrets**
   ```bash
   # Verify env vars or mounted secrets exist
   env | grep -i "AWS_\|GCS_\|AZURE_\|FERRO_STORAGE_"
   ```

3. **Check network (for cloud backends)**
   ```bash
   ping <storage-endpoint-host>
   traceroute <storage-endpoint-host>
   ```

4. **Check Ferro storage config**
   ```bash
   cat /etc/ferro/ferro.toml | grep -A 10 "\[storage\]"
   ```

## Fallback Behavior

- Ferro uses a fallback chain defined in `[storage.fallback]` in `ferro.toml`.
- If the primary backend is unreachable, Ferro retries with exponential backoff (up to 3 retries).
- If all retries fail and a fallback is configured, traffic routes to the fallback.
- If no fallback exists, requests return `503 Storage Unavailable`.

## Recovery

1. **Local FS**: Free disk space or fix permissions, then restart Ferro.
2. **S3/GCS/Azure**:
   - Verify IAM credentials are not expired.
   - Check bucket policies and network ACLs.
   - If the bucket was deleted/recreated, update `ferro.toml` and restart.
3. **After backend is restored**, trigger a storage consistency check:
   ```bash
   ferro-admin storage verify --backend <backend-name>
   ```

## Escalation

- If the issue persists after credential refresh, open a support ticket with the cloud provider.
- If data loss is suspected, activate the disaster recovery plan.
