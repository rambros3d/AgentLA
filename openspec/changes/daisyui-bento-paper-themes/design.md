## Context

The AgentLA WebUI requires a visual style that matches its identity as a high-speed bench multi-tool. We fuse DaisyUI v5 `black` and `emerald` color palettes with Linear Bento card elevation and Swiss technical drafting paper geometry.

## Goals / Non-Goals

**Goals:**
- Implement DaisyUI v5 `black` as default dark theme and `emerald` as default light theme in pure CSS `oklch()`.
- Add Linear Bento cards (micro-borders, subtle inset highlight `box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.06)`).
- Add subtle engineering millimeter grid canvas texture.
- Maintain zero npm/build dependencies and zero firmware flash bloat.

**Non-Goals:**
- Introducing external CSS frameworks or Node.js compilers.

## Decisions

### 1. Pure CSS `oklch()` Custom Properties
- Modern browser support for `oklch()` allows using DaisyUI v5 exact color formulas directly without any translation step or framework build.

### 2. Linear Bento Cards with Paper Grid
- Surfaces are elevated via `--color-base-200` with razor 1px `--border-color`.
- A 24px grid pattern is applied to `#main-workspace` and `body`.
