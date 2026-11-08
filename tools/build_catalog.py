#!/usr/bin/env python3
"""
AgentLA App Catalog Generator
Scans apps/*/app.json manifests, calculates package footprints,
and automatically generates apps/catalog.json.
"""

import json
import os
import sys
from pathlib import Path

DEFAULT_GITHUB_REPO_URL = "https://github.com/rambros3d/AgentLA/tree/embassy/apps"


def calculate_dir_size(app_dir: Path) -> int:
    """Calculate the total size of files inside an app directory."""
    total = 0
    for root, _, files in os.walk(app_dir):
        for f in files:
            p = Path(root) / f
            if not f.startswith(".") and not f.endswith(".tmp"):
                total += p.stat().st_size
    return total


def generate_catalog(apps_dir: Path = None, output_file: Path = None) -> dict:
    if apps_dir is None:
        apps_dir = (Path(__file__).parent.parent / "apps").resolve()
    if output_file is None:
        output_file = apps_dir / "catalog.json"

    packages_dir = apps_dir / "packages"
    catalog_entries = []

    # Enumerate subdirectories with app.json
    for item in sorted(apps_dir.iterdir()):
        if not item.is_dir() or item.name == "packages" or item.name.startswith("."):
            continue

        manifest_path = item / "app.json"
        if not manifest_path.is_file():
            continue

        try:
            with open(manifest_path, "r", encoding="utf-8") as f:
                data = json.load(f)
        except Exception as e:
            print(f"[build_catalog] WARNING: Failed to parse {manifest_path}: {e}", file=sys.stderr)
            continue

        app_id = data.get("id") or data.get("name") or item.name
        title = data.get("title") or data.get("name") or app_id.replace("-", " ").title()
        version = data.get("version", "1.0.0")
        author = data.get("author", "AgentLA Team")
        description = data.get("description", "")
        category = data.get("category", "General")
        tags = data.get("tags") or [category]
        requires = data.get("requires", [])

        # Icon URL: relative path "app-id/icon.svg" or absolute URL/data URI
        raw_icon = data.get("icon", "icon.svg")
        if raw_icon.startswith("http://") or raw_icon.startswith("https://") or raw_icon.startswith("data:"):
            icon_url = raw_icon
        else:
            icon_url = f"{item.name}/{raw_icon.lstrip('./')}"

        # App source URL on GitHub
        app_url = data.get("homepage") or f"{DEFAULT_GITHUB_REPO_URL}/{item.name}"

        # Package URL & Size
        tar_path = packages_dir / f"{item.name}.tar"
        if tar_path.is_file():
            size_bytes = tar_path.stat().st_size
            package_url = f"/packages/{item.name}.tar"
        else:
            size_bytes = calculate_dir_size(item)
            package_url = None

        entry = {
            "id": app_id,
            "name": title,
            "version": version,
            "author": author,
            "description": description,
            "category": category,
            "tags": tags,
            "requires": requires,
            "size_bytes": size_bytes,
            "icon_url": icon_url,
            "app_url": app_url,
        }
        if package_url:
            entry["package_url"] = package_url

        catalog_entries.append(entry)

    catalog = {
        "version": "1.0.0",
        "name": "AgentLA Official Catalog",
        "apps": catalog_entries,
    }

    with open(output_file, "w", encoding="utf-8") as f:
        json.dump(catalog, f, indent=2)
        f.write("\n")

    print(f"[build_catalog] Successfully generated {output_file} with {len(catalog_entries)} apps:")
    for a in catalog_entries:
        print(f"  • {a['id']:<16} v{a['version']:<6} ({a['size_bytes']} bytes)")

    return catalog


if __name__ == "__main__":
    generate_catalog()
