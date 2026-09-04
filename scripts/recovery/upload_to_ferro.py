#!/usr/bin/env python3
"""Upload the recovered OCIS tree into Ferro via WebDAV with auto token refresh."""
import os, sys, time, requests, threading

STAGE = "/mnt/pool_HDD_x2/tank/datasources/sis/appdata/ferro/data/ocis-restore-staging"
BASE = "http://127.0.0.1:8081"
AUTH = ("https://auth.wyattau.com/realms/company-realm/protocol/openid-connect/token",
        "wyatt", "temporal-fix-2026")

tok = {"v": None, "ts": 0}
lock = threading.Lock()

def refresh(force=False):
    with lock:
        if not force and tok["v"] and time.time() - tok["ts"] < 240:
            return
        r = requests.post(AUTH[0], data={
            "grant_type": "password", "client_id": "ferro",
            "client_secret": "ferro-secret-2026",
            "username": AUTH[1], "password": AUTH[2], "scope": "openid"}, timeout=20)
        tok["v"] = r.json()["access_token"]
        tok["ts"] = time.time()

refresh(True)

def H():
    return {"Authorization": f"Bearer {tok['v']}"}

files, dirs = [], []
for root, dnames, fnames in os.walk(STAGE):
    rel = os.path.relpath(root, STAGE)
    if rel != ".":
        dirs.append(rel)
    for f in fnames:
        files.append(os.path.relpath(os.path.join(root, f), STAGE))
files.sort()
print(f"uploading {len(files)} files, {len(dirs)} dirs", flush=True)

fail = []
t0 = time.time()
done = 0
for rel in files:
    for attempt in range(3):
        try:
            if time.time() - tok["ts"] > 240:
                refresh(True)
            with open(os.path.join(STAGE, rel), "rb") as f:
                r = requests.put(f"{BASE}/{rel}", data=f, headers={**H(), "Content-Type": "application/octet-stream"}, timeout=120)
            if r.status_code in (200, 201, 204):
                done += 1
                if done % 250 == 0:
                    print(f"{done} files ({(time.time()-t0):.0f}s)", flush=True)
                break
            if r.status_code == 401:
                refresh(True); continue
            fail.append(f"{rel}: {r.status_code} {r.text[:80]}")
            break
        except Exception as e:
            if attempt == 2:
                fail.append(f"{rel}: {e}")
            time.sleep(1)
print(f"DONE {done}/{len(files)} in {(time.time()-t0):.0f}s, failures: {len(fail)}", flush=True)
with open("/tmp/upload_failures.txt", "w") as f:
    f.write("\n".join(fail))
print("DONE_MARKER", flush=True)
