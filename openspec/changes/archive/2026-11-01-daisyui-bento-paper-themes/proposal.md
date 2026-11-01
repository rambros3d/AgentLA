## Why

The current WebUI dark and light themes feel generic. Replacing them with DaisyUI v5's `black` (true OLED pitch-black) for dark mode and `emerald` (clean Swiss technical drafting paper) for light mode, combined with Linear Bento grid cards and subtle engineering millimeter grid lines, provides a distinctive, highly legible, professional bench instrument appearance.

## What Changes

- Implement DaisyUI v5 `black` and `emerald` design tokens natively using modern `oklch()` CSS properties without adding npm or Tailwind dependencies.
- Re-skin `firmware/html/style.css` with Linear Bento cards, micro-borders (`1px solid var(--border-color)`), and subtle top specular rim highlights.
- Add an engineering millimeter drafting grid overlay to the background canvas.
- Update telemetry pills, segmented controls, and pin chips to match the new color palettes.
- Synchronize styling to the public web portal in `web/landing.css` and `web/demo/style.css`.
- Ensure zero firmware bloat in `pico_fsdata.inc`.

## Capabilities

### New Capabilities
- `daisyui-bento-theme-system`: Native oklch theme engine implementing DaisyUI `black` and `emerald` tokens with Linear Bento card styling and technical paper grid details.

### Modified Capabilities
<!-- None: Hardware, protocols, and RPC APIs remain unchanged -->

## Impact

- **CSS Files**: `firmware/html/style.css`, `web/landing.css`, `web/demo/style.css`.
- **Firmware Binary**: Zero flash bloat.
