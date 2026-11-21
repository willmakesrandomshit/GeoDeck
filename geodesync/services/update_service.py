"""Centralized update management with resilient multi-tier fallbacks."""

import json
import urllib.request
import xml.etree.ElementTree as ET
from typing import Optional, Dict, Any, List

from geodesync.core.config import GITHUB_REPO, APP_VERSION
from geodesync.core.versioning import parse_version


def parse_version_tuple(ver_str: str) -> tuple:
    """Parse a strictly numeric release version; malformed versions sort below valid ones."""
    return parse_version(ver_str) or ()


def is_newer_version(candidate: str, current: str = APP_VERSION) -> bool:
    """Return True only when the candidate is a valid, strictly newer version."""
    candidate_version = parse_version(candidate)
    current_version = parse_version(current)
    return candidate_version is not None and current_version is not None and candidate_version > current_version


class UpdateService:
    """Check public releases without sending ambient credentials."""

    def __init__(self, repo: str = GITHUB_REPO):
        self.repo = repo
        self.headers = {"User-Agent": "GeodeSync-Client"}
        self.releases_api = f"https://api.github.com/repos/{repo}/releases?per_page=100"
        self.config_url = f"https://raw.githubusercontent.com/{repo}/main/version.json"

    def check_for_update(self, timeout: float = 8.0) -> Optional[Dict[str, Any]]:
        """Fetch the newest stable release using API, Atom, then manifest fallbacks."""
        releases: List[Dict[str, Any]] = []

        try:
            req = urllib.request.Request(self.releases_api, headers=self.headers)
            with urllib.request.urlopen(req, timeout=timeout) as resp:
                data = json.loads(resp.read().decode("utf-8"))
                if isinstance(data, list):
                    releases = [item for item in data if isinstance(item, dict) and item.get("tag_name")]
        except (OSError, ValueError):
            pass

        if not releases:
            try:
                latest_api = f"https://api.github.com/repos/{self.repo}/releases/latest"
                req = urllib.request.Request(latest_api, headers=self.headers)
                with urllib.request.urlopen(req, timeout=timeout) as resp:
                    data = json.loads(resp.read().decode("utf-8"))
                    if isinstance(data, dict) and data.get("tag_name"):
                        releases = [data]
            except (OSError, ValueError):
                pass

        if not releases:
            try:
                atom_url = f"https://github.com/{self.repo}/releases.atom"
                req = urllib.request.Request(atom_url, headers=self.headers)
                with urllib.request.urlopen(req, timeout=timeout) as resp:
                    root = ET.fromstring(resp.read().decode("utf-8", errors="replace"))
                namespace = "{http://www.w3.org/2005/Atom}"
                for entry in root.findall(f"{namespace}entry"):
                    link = entry.find(f"{namespace}link")
                    updated = entry.find(f"{namespace}updated")
                    if link is None:
                        continue
                    href = link.get("href", "")
                    tag = href.rstrip("/").rsplit("/", 1)[-1]
                    if tag and parse_version(tag) is not None:
                        release = {"tag_name": tag, "html_url": href}
                        if updated is not None and updated.text:
                            release["published_at"] = updated.text
                        releases.append(release)
            except (OSError, ValueError, ET.ParseError):
                pass

        if not releases:
            try:
                req = urllib.request.Request(self.config_url, headers=self.headers)
                with urllib.request.urlopen(req, timeout=timeout) as resp:
                    cfg = json.loads(resp.read().decode("utf-8"))
                app_version = str(cfg.get("app_version", "")).strip() if isinstance(cfg, dict) else ""
                if parse_version(app_version) is not None:
                    tag = f"v{app_version.lstrip('v')}"
                    releases.append({
                        "tag_name": tag,
                        "html_url": f"https://github.com/{self.repo}/releases/tag/{tag}",
                    })
            except (OSError, ValueError):
                pass

        valid_releases = [
            release for release in releases
            if parse_version(release.get("tag_name")) is not None
            and not release.get("draft")
            and not release.get("prerelease")
        ]
        if not valid_releases:
            return None

        latest_release = max(
            valid_releases,
            key=lambda release: parse_version(release.get("tag_name")),
        )
        tag = str(latest_release["tag_name"]).lstrip("v")
        return latest_release if is_newer_version(tag) else None
