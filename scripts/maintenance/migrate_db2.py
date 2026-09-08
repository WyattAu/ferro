"""Run ON TrueNAS with server STOPPED. Pass 2: rewrite file_metadata paths to decoded form."""
import sqlite3, urllib.parse, os

DB = "/mnt/pool_HDD_x2/tank/datasources/sis/appdata/ferro/data/ferro.db"
FILES = "/mnt/pool_HDD_x2/tank/datasources/sis/appdata/ferro/data/files"

def dec(s):
    prev = None
    while prev != s:
        prev, s = s, urllib.parse.unquote(prev if prev is not None else s)
    return s

con = sqlite3.connect(DB)
cur = con.cursor()
rows = cur.execute("SELECT path, is_collection FROM file_metadata WHERE path LIKE '%\\%%' ESCAPE '\\'").fetchall()
print(f"pct-rows: {len(rows)}", flush=True)
moved = del_empty_coll = conflicts = 0
for old, is_coll in rows:
    new = dec(old)
    if new == old:
        continue
    tgt = cur.execute("SELECT is_collection FROM file_metadata WHERE path=?", (new,)).fetchone()
    if tgt is None:
        cur.execute("UPDATE file_metadata SET path=? WHERE path=?", (new, old))
        moved += cur.rowcount
    else:
        # collision: file rows win over collection rows
        if not is_coll and tgt[0]:
            cur.execute("DELETE FROM file_metadata WHERE path=?", (new,))
            cur.execute("UPDATE file_metadata SET path=? WHERE path=?", (new, old))
            moved += 1
        else:
            conflicts += 1
            print(f"DB CONFLICT kept: {old}", flush=True)
con.commit()
print(f"moved={moved} conflicts={conflicts}", flush=True)
# prune collection rows with no disk dir and no children rows
pruned = 0
for (p,), in cur.execute("SELECT path FROM file_metadata WHERE is_collection=1").fetchall():
    disk = os.path.join(FILES, p.lstrip("/"))
    if os.path.isdir(disk):
        continue
    kids = cur.execute("SELECT 1 FROM file_metadata WHERE path LIKE ? ESCAPE '\\' LIMIT 1", (p.replace("%", "\\%").replace("_", "\\_") + "/%",)).fetchone()
    if kids is None:
        cur.execute("DELETE FROM file_metadata WHERE path=?", (p,))
        pruned += cur.rowcount
con.commit()
print(f"pruned={pruned}", flush=True)
left = cur.execute("SELECT count(*) FROM file_metadata WHERE path LIKE '%\\%%' ESCAPE '\\'").fetchone()[0]
print(f"pct-rows remaining: {left}")
con.close()
print("PASS2 DONE")
