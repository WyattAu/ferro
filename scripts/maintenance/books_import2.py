"""Run ON TrueNAS: parallel, resumable calibre-library import into Ferro."""
import os, sys, urllib.request, urllib.parse, json, time
from concurrent.futures import ThreadPoolExecutor

HOST = "http://127.0.0.1:8081"
SUB = "92823fc5-cc56-434e-96dd-4285edad0c27"
LIB = "/mnt/pool_HDD_x2/tank/datasources/sis/appdata/books/calibre-library"
DONEFILE = "/tmp/books_done.txt"
KC = "https://auth.wyattau.com/realms/company-realm/protocol/openid-connect/token"

def token():
    data = urllib.parse.urlencode({
        "grant_type": "password", "client_id": "ferro", "client_secret": "ferro-secret-2026",
        "username": "wyatt", "password": "temporal-fix-2026", "scope": "openid"}).encode()
    req = urllib.request.Request(KC, data=data, headers={"Content-Type": "application/x-www-form-urlencoded"})
    with urllib.request.urlopen(req, timeout=30) as r:
        return json.load(r)["access_token"]

TOK = token()
def enc(rel):
    return "/".join(urllib.parse.quote(p) for p in rel.split("/"))

def call(method, rel, data=None):
    url = f"{HOST}/users/{SUB}/Books/{enc(rel)}" if rel else f"{HOST}/users/{SUB}/Books"
    h = {"Authorization": f"Bearer {TOK}"}
    last = None
    for a in range(3):
        try:
            rq = urllib.request.Request(url, data=data, headers=h, method=method)
            with urllib.request.urlopen(rq, timeout=180) as resp:
                return resp.status
        except urllib.error.HTTPError as e:
            return e.code
        except Exception as e:
            last = e; time.sleep(2 * (a + 1))
    raise last

import urllib.error
done = set()
if os.path.exists(DONEFILE):
    done = set(x.strip() for x in open(DONEFILE) if x.strip())

mkd = set()
def ensure_parents(rel):
    parts = [p for p in rel.split("/") if p][:-1]
    for i in range(1, len(parts) + 1):
        d = "/".join(parts[:i])
        if d not in mkd:
            call("MKCOL", d)
            mkd.add(d)

def one(args):
    full, rel = args
    try:
        ensure_parents(rel)
        with open(full, "rb") as fh:
            st = call("PUT", rel, data=fh.read())
        ok = st in (200, 201, 204)
        return (rel, ok, st)
    except Exception as e:
        return (rel, False, repr(e))

files = []
for root, _, fs in os.walk(LIB):
    for f in fs:
        full = os.path.join(root, f)
        rel = os.path.relpath(full, LIB)
        if rel not in done:
            files.append((full, rel))
files.sort(key=lambda x: x[1])
print(f"TOTAL {len(files)} remaining ({len(done)} already done)", flush=True)

ok = fail = 0
with open(DONEFILE, "a") as log:
    with ThreadPoolExecutor(max_workers=4) as ex:
        for rel, good, st in ex.map(one, files):
            if good:
                ok += 1; log.write(rel + "\n")
            else:
                fail += 1; print(f"FAIL {st} {rel}", flush=True)
            n = ok + fail
            if n % 25 == 0:
                log.flush(); print(f"progress ok={ok} fail={fail}", flush=True)
print(f"DONE ok={ok} fail={fail}")
