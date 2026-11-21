"""Game detection, state checks, and process inspection."""

import os
import subprocess
import sys
from typing import Optional

from geodesync.core.models import GameInstallation
from geodesync.platform.process_manager import safe_run_command


class GameService:
    """Detects and monitors Geometry Dash installations."""

    COMMON_STEAM_PATHS = [
        r"C:\Program Files (x86)\Steam\steamapps\common\Geometry Dash",
        r"C:\Program Files\Steam\steamapps\common\Geometry Dash",
        r"D:\Steam\steamapps\common\Geometry Dash",
        r"D:\SteamLibrary\steamapps\common\Geometry Dash",
        r"E:\SteamLibrary\steamapps\common\Geometry Dash",
    ]

    @classmethod
    def is_game_running(cls) -> bool:
        """Check whether Geometry Dash is running without waiting indefinitely."""
        if sys.platform == "win32":
            try:
                result = safe_run_command(
                    ["tasklist", "/fi", "imagename eq GeometryDash.exe", "/nh"],
                    timeout=5,
                )
                return "GeometryDash.exe" in (result.stdout or "")
            except (OSError, subprocess.SubprocessError):
                return False

        process_pattern = "Geometry Dash" if sys.platform == "darwin" else "GeometryDash"
        try:
            result = safe_run_command(
                ["pgrep", "-f", process_pattern],
                timeout=5,
            )
            return result.returncode == 0 and bool((result.stdout or "").strip())
        except (OSError, subprocess.SubprocessError):
            return False

    @classmethod
    def auto_detect(cls) -> Optional[GameInstallation]:
        """Find Geometry Dash in common Windows Steam library locations."""
        for path in cls.COMMON_STEAM_PATHS:
            if os.path.isdir(path):
                exe = os.path.join(path, "GeometryDash.exe")
                if os.path.isfile(exe):
                    geode_dir = os.path.join(path, "geode")
                    return GameInstallation(
                        path=path,
                        is_steam=True,
                        exe_path=exe,
                        geode_installed=os.path.isdir(geode_dir),
                        geode_dir=geode_dir if os.path.isdir(geode_dir) else None,
                    )
        return None
