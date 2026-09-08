"""Run ON TrueNAS with server STOPPED. Fix double-encoded storage keys.

1. Wipes {SUB}/Books subtree on disk + metadata (clean re-import follows).
2. Renames all other on-disk names containing '%' to fully-decoded form,
   deepest-first; conflicts get .ferro-conflict-N suffix (logged).
3. Applies the same mapping to file_metadata.path (prefer file rows over
   empty collection rows on collision).
4. Removes now-empty literal dirs and prunes orphan collection rows.
"""
import os, sys, sqlite3, urllib.parse

DATA = "/mnt/pool_HDD_x2/tank/datasources/sis/appdata/ferro/data"
FILES = os.path.join(DATA, "files")
DB = os.path.join(DATA, "ferro.db")
SUB = "92823fc5-cc56-434e-96dd-4285edad0c27"
BOOKS = os.path.join(FILES, "users", SUB, "Books")

def dec(s):
    prev = None
    while prev != s:
        prev = s
        s = urllib.parse.unquote(prev)
    return s

log = open("/tmp/migrate_keys.log", "w")
def L(m):
    print(m, flush=True); log.write(m + "\n")

renames = []  # (old_rel, new_rel) relative to FILES

# --- 1. wipe Books subtree on disk ---
import shutil
if os.path.isdir(BOOKS):
    shutil.rmtree(BOOKS)
    L("WIPED disk Books/")

# --- 2. collect renames (skip Books, already wiped) ---
dirs, files = [], []
for root, ds, fs in os.walk(FILES):
    for d in ds:
        p = os.path.join(root, d)
        if p.startswith(BOOKS): continue
        if "%" in d: dirs.append(p)
    for f in fs:
        p = os.path.join(root, f)
        if p.startswith(BOOKS): continue
        if "%" in f: files.append(p)

def target_for(p):
    rel = os.path.relpath(p, FILES)
    parts = [dec(seg) for seg in rel.split(os.sep)]
    return os.path.join(FILES, *parts)

# files first, then dirs deepest-first
for p in files:
    t = target_for(p)
    if t == p: continue
    if os.path.exists(t):
        base, ext = os.path.splitext(t)
        n = 1
        while os.path.exists(f"{base}.ferro-conflict-{n}{ext}"): n += 1
        t = f"{base}.ferro-conflict-{n}{ext}"
        L(f"CONFLICT file kept both: {os.path.relpath(p, FILES)} -> {os.path.relpath(t, FILES)}")
    os.makedirs(os.path.dirname(t), exist_ok=True)
    os.rename(p, t)
    renames.append((os.path.relpath(p, FILES), os.path.relpath(t, FILES)))
for p in sorted(dirs, key=len, reverse=True):
    t = target_for(p)
    if t == p or not os.path.isdir(p): continue
    if os.path.exists(t):
        # merge: move children up if target is a dir, else conflict-suffix
        if os.path.isdir(t):
            for child in os.listdir(p):
                src, dst = os.path.join(p, child), os.path.join(t, child)
                if os.path.exists(dst):
                    L(f"CONFLICT dir-child kept: {os.path.relpath(src, FILES)}")
                    continue
                os.rename(src, dst)
                renames.append((os.path.relpath(src, FILES), os.path.relpath(dst, FILES)))
            try: os.rmdir(p)
            except OSError: L(f"NONEMPTY dir left: {os.path.relpath(p, FILES)}")
            L(f"MERGED dir {os.path.relpath(p, FILES)} into {os.path.relpath(t, FILES)}")
        else:
            L(f"CONFLICT dir-vs-file, left: {os.path.relpath(p, FILES)}")
        continue
    os.makedirs(os.path.dirname(t), exist_ok=True)
    os.rename(p, t)
    renames.append((os.path.relpath(p, FILES), os.path.relpath(t, FILES)))

L(f"TOTAL renames: {len(renames)}")

# --- 3. DB updates ---
con = sqlite3.connect(DB)
cur = con.cursor()
cur.execute("DELETE FROM file_metadata WHERE path LIKE '/users/%/Books%' ESCAPE '\\' OR path LIKE '/users/%/Books' ESCAPE '\\'")
L(f"deleted Books metadata rows: {cur.rowcount}")
file_moves = [(o, n) for o, n in renames]
# apply deepest-old-first so prefix updates don't cascade wrongly; use exact match only
moved = skipped = 0
for old_rel, new_rel in sorted(file_moves, key=lambda x: len(x[0]), reverse=True):
    old_p, new_p = "/" + old_rel, "/" + new_rel
    cur.execute("SELECT is_collection FROM file_metadata WHERE path=?", (new_p,))
    tgt = cur.fetchone()
    cur.execute("SELECT is_collection FROM file_metadata WHERE path=?", (old_p,))
    src = cur.fetchone()
    if tgt is not None and src is None:
        continue  # nothing to move
    if tgt is not None:
        # collision: keep file rows over collection rows
        if src and not src[0] and tgt[0]:
            cur.execute("DELETE FROM file_metadata WHERE path=?", (new_p,))
        else:
            L(f"DB CONFLICT kept both rows: {old_p}")
            skipped += 1
            continue
    cur.execute("UPDATE file_metadata SET path=? WHERE path=?", (new_p, old_p))
    moved += cur.rowcount
con.commit()
L(f"metadata rows moved: {moved}, conflicts-skipped: {skipped}")
# prune collection rows pointing at gone dirs
pruned = 0
for (p_,), in cur.execute("SELECT path FROM file_metadata WHERE is_collection=1"):
    if not os.path.isdir(os.path.join(FILES, p_.lstrip("/"))):
        cur.execute("DELETE FROM file_metadata WHERE path=?", (p_,))
        pruned += cur.rowcount
con.commit()
L(f"pruned orphan collection rows: {pruned}")
con.close()
L("MIGRATION DONE")
