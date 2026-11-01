## 1. Embedded WebUI Stylesheet Overhaul

- [x] 1.1 Update `firmware/html/style.css` with DaisyUI v5 `black` and `emerald` `oklch()` design tokens
- [x] 1.2 Implement Linear Bento card styling with 1px micro-borders and top specular rim highlights
- [x] 1.3 Add technical millimeter drafting grid background pattern
- [x] 1.4 Update telemetry pills, segmented controls, and 30-pin dynamic matrix chips to match the new palettes

## 2. Public Web Portal & Demo Synchronization

- [x] 2.1 Update `web/landing.css` with matching DaisyUI `black` and `emerald` tokens and Bento styling
- [x] 2.2 Synchronize `web/demo/style.css` with the updated embedded stylesheet

## 3. Verification & Deployment

- [x] 3.1 Rebuild firmware (`make -C build-work AgentLA`) and verify 0 errors and zero binary flash inflation
- [x] 3.2 Commit, push to `dev`, and deploy to GitHub Pages
