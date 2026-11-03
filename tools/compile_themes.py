#!/usr/bin/env python3
"""
AgentLA Theme Compiler
Parses DaisyUI @plugin "daisyui/theme" definitions in themes/*.css,
generates firmware/html/themes.css with [data-theme="<name>"] attribute rules,
and outputs firmware/html/themes.json with theme metadata.
"""

import argparse
import json
import os
import re
import sys
from pathlib import Path


def parse_theme_css(content: str) -> tuple[dict, dict]:
    """
    Parses a single theme CSS file containing @plugin "daisyui/theme" { ... }.
    Returns (metadata_dict, css_properties_dict).
    """
    plugin_match = re.search(r'@plugin\s+["\']daisyui/theme["\']\s*\{([^}]+)\}', content, re.DOTALL)
    if not plugin_match:
        raise ValueError("No valid @plugin \"daisyui/theme\" block found")

    block = plugin_match.group(1)
    meta = {
        "name": "",
        "default": False,
        "isDefault": False,
        "prefersdark": False,
        "prefersDark": False,
        "color-scheme": "light",
        "colorScheme": "light",
    }
    props = {}

    for line in block.splitlines():
        line = line.strip()
        if not line or line.startswith("/*") or line.startswith("//"):
            continue

        if ":" not in line:
            continue

        key, val = line.split(":", 1)
        key = key.strip()
        val = val.rstrip(";").strip()

        # Strip surrounding quotes if string
        val_clean = val
        if (val_clean.startswith('"') and val_clean.endswith('"')) or (val_clean.startswith("'") and val_clean.endswith("'")):
            val_clean = val_clean[1:-1]

        if key == "name":
            meta["name"] = val_clean
        elif key == "default":
            is_def = val_clean.lower() == "true"
            meta["default"] = is_def
            meta["isDefault"] = is_def
        elif key in ("prefersdark", "prefersDark"):
            is_dark = val_clean.lower() == "true"
            meta["prefersdark"] = is_dark
            meta["prefersDark"] = is_dark
        elif key in ("color-scheme", "colorScheme"):
            meta["color-scheme"] = val_clean
            meta["colorScheme"] = val_clean
            props["color-scheme"] = val_clean
        else:
            props[key] = val

    if not meta["name"]:
        raise ValueError("Theme missing required 'name' property")

    return meta, props


def compile_themes(themes_dir: Path, output_css: Path, output_json: Path) -> list[dict]:
    themes_dir = themes_dir.resolve()
    if not themes_dir.is_dir():
        raise FileNotFoundError(f"Themes directory does not exist: {themes_dir}")

    theme_files = sorted(themes_dir.glob("*.css"))
    if not theme_files:
        raise FileNotFoundError(f"No .css files found in {themes_dir}")

    themes_meta = []
    css_blocks = [
        "/* ==========================================================================",
        "   AgentLA Compiled DaisyUI Themes",
        "   Generated automatically by tools/compile_themes.py - DO NOT EDIT DIRECTLY",
        "   ========================================================================== */",
        ""
    ]

    parsed_themes = []
    default_theme_entry = None

    for tf in theme_files:
        content = tf.read_text(encoding="utf-8")
        meta, props = parse_theme_css(content)
        parsed_themes.append((meta, props))
        themes_meta.append(meta)
        if meta.get("default") and meta.get("prefersdark"):
            default_theme_entry = (meta, props)

    if not default_theme_entry and parsed_themes:
        default_theme_entry = parsed_themes[0]

    # Emit standalone :root fallback at the very top of themes.css
    if default_theme_entry:
        d_meta, d_props = default_theme_entry
        d_name = d_meta["name"]
        d_scheme = d_meta["color-scheme"]
        root_lines = [
            f"/* Fallback Root Variables (Default Dark Theme: {d_name}) */",
            ":root {"
        ]
        root_lines.append(f"    color-scheme: {d_scheme};")
        for k, v in d_props.items():
            if k == "color-scheme":
                continue
            root_lines.append(f"    {k}: {v};")
        root_lines.append("}\n")
        css_blocks.append("\n".join(root_lines))

    for meta, props in parsed_themes:
        name = meta["name"]
        color_scheme = meta["color-scheme"]
        is_default = meta["default"]
        prefers_dark = meta["prefersdark"]

        # Build selector list
        selectors = [f'[data-theme="{name}"]']
        if is_default:
            if prefers_dark:
                selectors = ['[data-theme="dark"]', f'[data-theme="{name}"]']
            else:
                selectors = ['[data-theme="light"]', f'[data-theme="{name}"]']

        selector_str = ", ".join(selectors)

        css_lines = [f"/* Theme: {name} (scheme: {color_scheme}, default: {is_default}) */", f"{selector_str} {{"]
        css_lines.append(f"    color-scheme: {color_scheme};")
        for k, v in props.items():
            if k == "color-scheme":
                continue
            css_lines.append(f"    {k}: {v};")
        css_lines.append("}\n")

        css_blocks.append("\n".join(css_lines))

    output_css.parent.mkdir(parents=True, exist_ok=True)
    output_css.write_text("\n".join(css_blocks), encoding="utf-8")
    print(f"[compile_themes] Wrote CSS: {output_css}")

    output_json.parent.mkdir(parents=True, exist_ok=True)
    output_json.write_text(json.dumps(themes_meta, indent=2), encoding="utf-8")
    print(f"[compile_themes] Wrote JSON: {output_json}")

    return themes_meta


def main() -> int:
    parser = argparse.ArgumentParser(description="Compile DaisyUI theme CSS files into unified themes.css and themes.json")
    parser.add_argument("--themes-dir", default="themes", help="Directory containing theme CSS files")
    parser.add_argument("--output-css", default="firmware/html/themes.css", help="Output path for compiled themes.css")
    parser.add_argument("--output-json", default="firmware/html/themes.json", help="Output path for themes.json metadata")
    args = parser.parse_args()

    try:
        themes = compile_themes(Path(args.themes_dir), Path(args.output_css), Path(args.output_json))
        print(f"[compile_themes] Successfully compiled {len(themes)} themes: {[t['name'] for t in themes]}")
        return 0
    except Exception as e:
        print(f"[compile_themes] ERROR: {e}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
