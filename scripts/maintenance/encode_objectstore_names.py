"""Run ON TrueNAS with ferro-server STOPPED.

Aligns on-disk filenames with object_store's local-backend path encoding.

object_store percent-encodes these characters when resolving a virtual
path to a file path:  %  #  [  ]  ^  {  }  ~  `  "
The OCIS->Ferro migration wrote DECODED names instead, so GETs of any
file whose name contains one of those characters resolved to a
non-existent encoded path (404) while listings (which decode) kept
showing them.

This script walks the files tree deepest-first and renames every
file/dir whose name contains a mismatch character to its encoded form
(single sweep per character, conflicts suffixed .enc-conflict-N), then
rewrites file_metadata rows whose path changes accordingly.

Empirical probe (CharsetProbe/): space ! $ & ' ( ) + , ; = @ round-trip
literally; the set above does not.
"""
import os, sys, shutil, sqlite3, urllib.parse

DATA = "/mnt/pool_HDD_x2/tank/datasources/sis/appdata/ferro/data"
FILES = os.path.join(DATA, "files")
DB = os.path.join(DATA, "ferro.db")

MISMATCH = {"%": "%25", "#": "%23", "[": "%5B", "]": "%5D",
            "^": "%5E", "{": "%7B", "}": "%7D", "~": "%7E",
            "`": "%60", '"': "%22"}

def encode_name(name: str) -> str:
    out = []
    for ch in name:
        out.append(MISMATCH.get(ch, ch))
    return "".join(out)

def needs_encode(name: str) -> bool:
    return any(ch in MISMATCH for ch in name)

log = open("/tmp/encode_names.log", "w")
def L(m):
    print(m, flush=True); log.write(m + "\n")

renames = []  # (old_abs, new_abs)

# 1. Rename deepest-first so child paths stay valid while parents rename.
for root, dirs, files in os.walk(FILES, topdown=False):
    for name in dirs + files:
        old = os.path.join(root, name)
        if not needs_encode(name):
            continue
        new_name = encode_name(name)
        new = os.path.join(root, new_name)
        if os.path.exists(new):
            if os.path.isdir(old) and os.path.isdir(new):
                # merge: move children of old into new, then rmdir old
                for child in os.listdir(old):
                    csrc = os.path.join(old, child)
                    cdst = os.path.join(new, child)
                    if os.path.exists(cdst):
                        L(f"CONFLICT child kept: {csrc}")
                        continue
                    os.rename(csrc, cdst)
                    renames.append((csrc, cdst))
                try:
                    os.rmdir(old)
                    L(f"MERGED dir {old} -> {new}")
                except OSError:
                    L(f"NONEMPTY dir left: {old}")
                continue
            base, ext = os.path.splitext(new)
            n = 1
            while os.path.exists(f"{base}.enc-conflict-{n}{ext}"):
                n += 1
            new = f"{base}.enc-conflict-{n}{ext}"
            L(f"CONFLICT suffixed: {new}")
        os.renames(old, new)  # creates intermediate dirs if needed
        renames.append((old, new))

L(f"total renames: {len(renames)}")

# 2. Metadata: rewrite rows whose path changes under the same encoding.
con = sqlite3.connect(DB)
cur = con.cursor()
rows = cur.execute("SELECT path FROM file_metadata").fetchall()
moved = skipped_missing = 0
for (p,) in rows:
    segs = p.split("/")
    new_segs = [encode_name(s) for s in segs]
    new = "/".join(new_segs)
    if new == p:
        continue
    disk_exists = os.path.exists(os.path.join(FILES, new.lstrip("/")))
    cur.execute("UPDATE file_metadata SET path=? WHERE path=?", (new, p))
    if cur.rowcount:
        moved += cur.rowcount
    if not disk_exists:
        skipped_missing += 1
con.commit()
L(f"metadata rows rewritten: {moved} (targets missing on disk: {skipped_missing})")
con.close()
L("ENCODE PASS DONE")
