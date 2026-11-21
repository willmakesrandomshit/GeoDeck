"""Geometry Dash save file codec and crash-safe snapshot creation."""

import base64
import os
import re
import shutil
import tempfile
import zlib
from datetime import datetime
from typing import Tuple

SAVE_FILES = (
    "CCGameManager.dat",
    "CCGameManager2.dat",
    "CCLocalLevels.dat",
    "CCLocalLevels2.dat",
)


def decode_gd_save(raw: bytes) -> bytes:
    """Decode Geometry Dash XOR 0x0B, base64 URL-safe, zlib compressed save data."""
    xored = bytes(byte ^ 11 for byte in raw)
    b64_str = xored.decode("ascii", errors="ignore").replace("-", "+").replace("_", "/")
    padded = b64_str + "=" * ((4 - len(b64_str) % 4) % 4)
    compressed = base64.b64decode(padded)
    return zlib.decompress(compressed, 15 | 32)


def encode_gd_save(data: bytes) -> bytes:
    """Encode raw XML save into Geometry Dash's zlib + base64 + XOR 0x0B format."""
    compressed = zlib.compress(data)
    b64 = base64.b64encode(compressed).decode("ascii").replace("+", "-").replace("/", "_")
    return bytes(byte ^ 11 for byte in b64.encode("ascii"))


class SaveService:
    """Inspect and snapshot local Windows Geometry Dash save data."""

    @staticmethod
    def get_save_dir() -> str:
        """Return the Windows save path, or an empty path if it cannot be resolved."""
        local_app = os.environ.get("LOCALAPPDATA")
        if not local_app:
            return ""
        return os.path.join(local_app, "GeometryDash")

    @classmethod
    def create_snapshot(cls, backup_dir: str, tag: str = "manual") -> Tuple[bool, str]:
        """Create a complete timestamped snapshot without exposing partial backups."""
        save_dir = cls.get_save_dir()
        if not save_dir or not os.path.isdir(save_dir):
            return False, "Geometry Dash save directory not found."

        sources = [
            (name, os.path.join(save_dir, name))
            for name in SAVE_FILES
            if os.path.isfile(os.path.join(save_dir, name))
        ]
        if not sources:
            return False, "No save files found to snapshot."

        safe_tag = re.sub(r"[^A-Za-z0-9_-]+", "-", str(tag)).strip("-_")[:40] or "manual"
        timestamp = datetime.now().strftime("%Y%m%d_%H%M%S_%f")
        target_dir = os.path.join(backup_dir, f"snapshot_{timestamp}_{safe_tag}")
        os.makedirs(backup_dir, exist_ok=True)
        staging_dir = tempfile.mkdtemp(prefix=".geodesync-snapshot-", dir=backup_dir)

        try:
            for name, source in sources:
                shutil.copy2(source, os.path.join(staging_dir, name))
            os.rename(staging_dir, target_dir)
        except OSError as error:
            shutil.rmtree(staging_dir, ignore_errors=True)
            return False, f"Could not create save snapshot: {error}"

        return True, target_dir
