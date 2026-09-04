#!/usr/bin/env python3
import msgpack, os, json
from pathlib import Path

SPACE = Path("/mnt/pool_HDD_x2/tank/datasources/sis/appdata/storage/ocis-data/storage/users/spaces/22/2dad5a-0d33-41ab-b903-96733621bc56")
STAGE = Path("/mnt/pool_HDD_x2/tank/datasources/sis/appdata/ferro/data/ocis-restore-staging")
ROOT_ID = "222dad5a-0d33-41ab-b903-96733621bc56"
NODES = SPACE / "nodes"

def norm(v):
    return v.decode("utf-8","replace") if isinstance(v,bytes) else v

def parse_meta(p: Path):
    """Parse a file as node metadata; return (id_from_meta, meta) or (None, meta)."""
    try:
        d = msgpack.unpackb(p.read_bytes(), raw=False)
    except Exception:
        return None, None
    if not isinstance(d, dict) or "user.ocis.name" not in d:
        return None, None
    return norm(d.get("user.ocis.id")), d

nodes = {}
dup = 0
for p in NODES.rglob("*"):
    if not p.is_file():
        continue
    if p.suffix == ".mlock":
        continue
    id_from_meta, meta = parse_meta(p)
    if meta is None:
        continue
    if id_from_meta:
        nid = id_from_meta
    elif p.suffix == ".mpk":
        # bucket-path layout: id from path (strip .mpk, concat buckets + filename)
        rel = p.relative_to(NODES)
        parts = rel.parts[:-1]
        nid = "".join(parts) + rel.parts[-1][:-4]
    else:
        # nested named metadata file without id — derive from parent dir node id + name
        # parent node dir: .../-<uuid-rest>; parent full id unknown here — use dir path
        parent_dir = p.parent.name
        nid = "NESTED:" + str(p.relative_to(NODES))
        meta["user.ocis.name"] = p.name
        meta["_nested_under"] = parent_dir
        nodes[nid] = meta
        continue
    if nid in nodes:
        dup += 1
    meta["user.ocis.id"] = nid
    meta["user.ocis.parentid"] = norm(meta.get("user.ocis.parentid"))
    meta["user.ocis.name"] = norm(meta.get("user.ocis.name"))
    meta["user.ocis.blobid"] = norm(meta.get("user.ocis.blobid"))
    meta["user.ocis.type"] = norm(meta.get("user.ocis.type"))
    nodes[nid] = meta

print(f"nodes: {len(nodes)}, dups: {dup}", flush=True)

children = {}
for nid, d in nodes.items():
    children.setdefault(d.get("user.ocis.parentid"), []).append(nid)

missing = {p: len(c) for p, c in children.items() if p and p not in nodes}
print(f"missing parents: {len(missing)}", flush=True)
for pid, c in sorted(missing.items(), key=lambda x: -x[1])[:8]:
    nm = nodes[children[pid][0]].get("user.ocis.name")
    print(f"  {pid} children={c} sampleChild={nm}", flush=True)
print(f"root children: {len(children.get(ROOT_ID, []))}", flush=True)
print(f"root names: {[nodes[c].get('user.ocis.name') for c in children.get(ROOT_ID, [])][:12]}", flush=True)

def sanitize(n): return n.replace("/", "_").replace("\x00", "_") or "_"

def blob_path(blobid):
    u = blobid.replace("-", "")
    return SPACE / "blobs" / u[0:2] / u[2:4] / u[4:6] / u[6:8] / ("-" + blobid[9:])

files_written = bytes_written = dirs_written = 0
errors = []
os.makedirs(STAGE, exist_ok=True)

def walk(nid, rel):
    global files_written, bytes_written, dirs_written
    for cid in children.get(nid, []):
        d = nodes[cid]
        name = sanitize(d.get("user.ocis.name", ""))
        child_rel = os.path.join(rel, name)
        if d.get("user.ocis.type") == "2":
            dirs_written += 1
            walk(cid, child_rel)
        else:
            blobid = d.get("user.ocis.blobid")
            if not blobid:
                errors.append(f"no-blobid: {child_rel}")
                continue
            src = blob_path(blobid)
            if not src.exists():
                errors.append(f"missing-blob: {child_rel}")
                continue
            dst = STAGE / child_rel
            dst.parent.mkdir(parents=True, exist_ok=True)
            try:
                with open(src, "rb") as f, open(dst, "wb") as o:
                    while True:
                        chunk = f.read(1 << 20)
                        if not chunk: break
                        o.write(chunk)
                files_written += 1
                bytes_written += int(d.get("user.ocis.blobsize", "0"))
            except OSError as e:
                errors.append(f"copy-fail: {child_rel}: {e}")

walk(ROOT_ID, ".")
print(f"dirs: {dirs_written}, files: {files_written}, bytes: {bytes_written/1e9:.2f} GB", flush=True)
print(f"errors: {len(errors)}", flush=True)
Path("/tmp/ocis_recover_errors.json").write_text(json.dumps(errors))
for e in errors[:10]: print("  ", e, flush=True)
print("DONE", flush=True)
