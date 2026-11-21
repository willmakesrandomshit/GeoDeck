"""Domain data models for GeodeSync."""

from dataclasses import dataclass
from typing import Optional


@dataclass
class GameInstallation:
    path: str
    is_steam: bool = False
    version: Optional[str] = None
    manifest_locked: bool = False
    geode_installed: bool = False
    geode_version: Optional[str] = None
    exe_path: Optional[str] = None
    geode_dir: Optional[str] = None


@dataclass
class ModInfo:
    id: str
    name: str
    version: str
    developer: str = "Unknown"
    enabled: bool = True
    path: str = ""
    is_incompatible: bool = False
    incompatibility_reason: str = ""
    description: str = ""

    @property
    def file_path(self) -> str:
        """Compatibility alias for services that use the explicit file-path name."""
        return self.path

    @file_path.setter
    def file_path(self, value: str) -> None:
        self.path = value


@dataclass
class DiagnosticResult:
    title: str
    status: str  # PASS, WARN, FAIL, INFO
    details: str
    color: str
    action_label: Optional[str] = None


@dataclass
class SaveSnapshot:
    name: str
    folder_path: str
    timestamp: float
    size_bytes: int
    is_healthy: bool = True
    summary: str = ""
