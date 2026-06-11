import subprocess

from gi.repository import Nautilus, GObject


class FerroSyncExtension(GObject.GObject, Nautilus.ColumnProvider, Nautilus.InfoProvider):
    SYNC_COLUMN = "FerroSync::sync_status"
    EMBLEM_SYNCED = "emblem-default"
    EMBLEM_SYNCING = "emblem-synchronizing"
    EMBLEM_ERROR = "emblem-important"

    def __init__(self):
        pass

    def get_columns(self):
        return [
            Nautilus.Column(
                name=self.SYNC_COLUMN,
                attribute="sync_status",
                label="Ferro Sync",
                description="Ferro sync status",
            )
        ]

    def update_file_info(self, file):
        path = file.get_location().get_path()
        if path is None:
            file.add_string_attribute("sync_status", "")
            return
        try:
            result = subprocess.run(
                ["gio", "info", "-a", "metadata::emblems", path],
                capture_output=True,
                text=True,
                timeout=2,
            )
            if result.returncode != 0:
                file.add_string_attribute("sync_status", "")
                return
            output = result.stdout
            if self.EMBLEM_SYNCED in output:
                label = "\u2713 Synced"
            elif self.EMBLEM_SYNCING in output:
                label = "\u21bb Syncing"
            elif self.EMBLEM_ERROR in output:
                label = "\u26a0 Error"
            else:
                label = ""
            file.add_string_attribute("sync_status", label)
        except Exception:
            file.add_string_attribute("sync_status", "")
