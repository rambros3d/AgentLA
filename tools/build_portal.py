#!/usr/bin/env python3
"""
AgentLA Static Portal Staging Pipeline
Assembles the public marketing landing page (website/) and interactive WebUI demo (webui/)
into a unified static distribution directory (_site/) suitable for GitHub Pages or local preview.
"""

import argparse
import http.server
import os
import shutil
import socketserver
import sys
from pathlib import Path


def stage_portal(
    repo_root: Path,
    output_dir: Path,
    clean: bool = True,
    board: str = "demo",
) -> dict:
    website_dir = (repo_root / "website").resolve()
    webui_dir = (repo_root / "webui").resolve()
    boards_dir = (repo_root / "boards").resolve()
    apps_dir = (repo_root / "apps").resolve()

    if not website_dir.is_dir():
        raise FileNotFoundError(f"website/ directory not found: {website_dir}")
    if not webui_dir.is_dir():
        raise FileNotFoundError(f"webui/ directory not found: {webui_dir}")

    output_dir = output_dir.resolve()
    if clean and output_dir.exists():
        shutil.rmtree(output_dir)

    output_dir.mkdir(parents=True, exist_ok=True)
    demo_dir = output_dir / "demo"
    demo_dir.mkdir(parents=True, exist_ok=True)

    copied_marketing = 0
    copied_webui = 0

    # 1. Stage website/ (marketing landing page) at output root
    for item in sorted(website_dir.iterdir()):
        if item.name == "demo" or item.name.startswith("."):
            continue
        # Skip symlinks that point to webui or apps
        if item.is_symlink():
            continue
        dest = output_dir / item.name
        if item.is_dir():
            shutil.copytree(item, dest, dirs_exist_ok=True)
        else:
            shutil.copy2(item, dest)
        copied_marketing += 1

    # 2. Stage webui/ (full instrument workbench & launcher) into demo/
    for root, dirs, files in os.walk(webui_dir, followlinks=True):
        rel_root = Path(root).relative_to(webui_dir)
        dest_root = demo_dir / rel_root
        dest_root.mkdir(parents=True, exist_ok=True)

        for f in files:
            if f.startswith(".") or f.endswith(".tmp") or f.endswith(".bak"):
                continue
            src_file = Path(root) / f
            dest_file = dest_root / f
            shutil.copy2(src_file, dest_file)
            copied_webui += 1

    # 3. Stage board profile assets (boards/demo/) into demo/
    board_dir = boards_dir / board
    if board_dir.is_dir():
        for f in ["board.json", "favicon.svg", "logo.svg", "pinout.svg"]:
            src = board_dir / f
            if src.is_file():
                shutil.copy2(src, demo_dir / f)

    # 4. Stage apps catalog & official apps if available
    apps_catalog = apps_dir / "catalog.json"
    if apps_catalog.is_file():
        demo_apps_dir = demo_dir / "apps"
        demo_apps_dir.mkdir(parents=True, exist_ok=True)
        shutil.copy2(apps_catalog, demo_apps_dir / "catalog.json")

        packages_dir = demo_dir / "packages"
        packages_dir.mkdir(parents=True, exist_ok=True)
        shutil.copy2(apps_catalog, packages_dir / "catalog.json")

    # Also copy apps directory structure if present
    if apps_dir.is_dir():
        dest_apps = demo_dir / "apps"
        dest_apps.mkdir(parents=True, exist_ok=True)
        for app_folder in apps_dir.iterdir():
            if app_folder.is_dir() and (app_folder / "app.json").is_file():
                target_app_dir = dest_apps / app_folder.name
                shutil.copytree(app_folder, target_app_dir, dirs_exist_ok=True)

        # Copy prebuilt package tarballs
        pkg_source = apps_dir / "packages"
        if pkg_source.is_dir():
            dest_pkgs = demo_dir / "packages"
            dest_pkgs.mkdir(parents=True, exist_ok=True)
            for pkg in pkg_source.glob("*.tar"):
                shutil.copy2(pkg, dest_pkgs / pkg.name)

    # 5. Create .nojekyll for GitHub Pages
    (output_dir / ".nojekyll").touch()

    print(f"[build_portal] Staged marketing portal ({copied_marketing} items) -> {output_dir}")
    print(f"[build_portal] Staged demo WebUI ({copied_webui} items) -> {demo_dir}")
    print(f"[build_portal] Configured board: '{board}'")
    print(f"[build_portal] Ready for deployment or preview.")

    return {
        "output_dir": output_dir,
        "demo_dir": demo_dir,
        "copied_marketing": copied_marketing,
        "copied_webui": copied_webui,
    }


class PortalHTTPRequestHandler(http.server.SimpleHTTPRequestHandler):
    extensions_map = {
        **http.server.SimpleHTTPRequestHandler.extensions_map,
        ".wasm": "application/wasm",
        ".svg": "image/svg+xml",
        ".json": "application/json",
        ".js": "application/javascript",
        ".css": "text/css",
    }

    def end_headers(self):
        self.send_header("Access-Control-Allow-Origin", "*")
        self.send_header("Cache-Control", "no-cache, no-store, must-revalidate")
        super().end_headers()


def serve_portal(directory: Path, port: int = 8080, bind: str = "127.0.0.1"):
    os.chdir(directory)
    handler = PortalHTTPRequestHandler
    with socketserver.TCPServer((bind, port), handler) as httpd:
        print("=" * 60)
        print(f"AgentLA Portal Dev Server running at http://{bind}:{port}/")
        print(f"  • Landing Page:    http://{bind}:{port}/")
        print(f"  • Fullscreen Demo: http://{bind}:{port}/demo/")
        print(f"  • Embedded Demo:   http://{bind}:{port}/demo/?embed=1")
        print("Press Ctrl+C to stop.")
        print("=" * 60)
        try:
            httpd.serve_forever()
        except KeyboardInterrupt:
            print("\nShutting down server.")


def main():
    parser = argparse.ArgumentParser(description="Assemble AgentLA Static Landing Portal & WebUI Demo")
    parser.add_argument("--repo-root", default=".", help="Root of AgentLA repository")
    parser.add_argument("--output", default="_site", help="Destination output directory (default: _site)")
    parser.add_argument("--board", default="demo", help="Board profile to use for demo (default: demo)")
    parser.add_argument("--no-clean", action="store_true", help="Do not wipe destination directory before copying")
    parser.add_argument("--serve", nargs="?", const=8080, type=int, help="Serve staged portal locally on port (default: 8080)")
    parser.add_argument("--bind", default="127.0.0.1", help="Network interface to bind dev server to (default: 127.0.0.1)")

    args = parser.parse_args()
    repo_root = Path(args.repo_root).resolve()
    output_dir = Path(args.output).resolve()

    res = stage_portal(repo_root, output_dir, clean=not args.no_clean, board=args.board)

    if args.serve:
        serve_portal(output_dir, port=args.serve, bind=args.bind)

    return 0


if __name__ == "__main__":
    sys.exit(main())
