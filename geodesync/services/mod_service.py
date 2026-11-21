"""Geode mod discovery, inspection, and safe toggle operations."""

import json
import os
import zipfile
from typing import List

from geodesync.core.models import ModInfo

MAX_MOD_METADATA_BYTES = 1024 * 1024


class ModService:
    """Manages .geode mods within a game directory."""

    @staticmethod
    def get_mods_dir(game_path: str) -> str:
        return os.path.join(game_path, "geode", "mods")

    @classmethod
    def list_mods(cls, game_path: str) -> List[ModInfo]:
        """Scan geode/mods for installed mods, skipping invalid or oversized metadata."""
        mods_dir = cls.get_mods_dir(game_path)
        if not os.path.isdir(mods_dir):
            return []

        results: List[ModInfo] = []
        for filename in os.listdir(mods_dir):
            full_path = os.path.join(mods_dir, filename)
            if not os.path.isfile(full_path):
                continue

            enabled = filename.endswith(".geode")
            disabled = filename.endswith(".geode.disabled")
            if not (enabled or disabled):
                continue

            suffix = ".geode.disabled" if disabled else ".geode"
            mod_id = filename[:-len(suffix)]
            name = mod_id
            version = "unknown"
            developer = "unknown"

            try:
                with zipfile.ZipFile(full_path, "r") as archive:
                    metadata = archive.getinfo("mod.json")
                    if metadata.file_size <= MAX_MOD_METADATA_BYTES:
                        with archive.open(metadata) as stream:
                            raw = stream.read(MAX_MOD_METADATA_BYTES + 1)
                        if len(raw) <= MAX_MOD_METADATA_BYTES:
                            data = json.loads(raw.decode("utf-8"))
                            if isinstance(data, dict):
                                mod_id = data.get("id") if isinstance(data.get("id"), str) else mod_id
                                name = data.get("name") if isinstance(data.get("name"), str) else name
                                version = data.get("version") if isinstance(data.get("version"), str) else version
                                developer = data.get("developer") if isinstance(data.get("developer"), str) else developer
            except (OSError, EOFError, ValueError, RuntimeError, KeyError, zipfile.BadZipFile, zipfile.LargeZipFile):
                # A corrupt or nonstandard mod archive should not hide other installed mods.
                pass

            results.append(ModInfo(
                id=mod_id,
                name=name,
                version=version,
                developer=developer,
                enabled=enabled,
                path=full_path,
            ))
        return results

    @classmethod
    def toggle_mod(cls, mod: ModInfo) -> bool:
        """Enable or disable a mod without replacing an existing destination."""
        source = mod.path
        if not os.path.isfile(source):
            return False

        if mod.enabled:
            if not source.endswith(".geode"):
                return False
            destination = source + ".disabled"
        else:
            if not source.endswith(".geode.disabled"):
                return False
            destination = source[:-len(".disabled")]

        if os.path.lexists(destination):
            return False
        try:
            os.rename(source, destination)
        except OSError:
            return False

        mod.path = destination
        mod.enabled = not mod.enabled
        return True
