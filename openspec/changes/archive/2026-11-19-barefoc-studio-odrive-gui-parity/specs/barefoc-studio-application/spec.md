## REMOVED Requirements

### Requirement: User-Installable Application Packaging
**Reason**: BareFOC Studio was promoted from an optional USER_FS application to a built-in SYSTEM_FS application by the `native-barefoc-system-app` change. USER_FS packaging (`apps/barefoc/`, `apps/packages/barefoc.tar`, catalog entry) no longer exists in the repository, and this spec has been stale since that promotion.

**Migration**: Use the canonical system asset and routes defined by `native-barefoc-system-app`: `webui/barefoc.html` packaged by `tools/pack_webui.py`, served at `/motor` (aliases `/foc`, `/barefoc`). All current and future behavior requirements for the studio are tracked by `native-barefoc-system-app` and the new parity capabilities (`motor-target-discovery`, `motor-parameter-explorer`, `motor-dashboard-builder`, `motor-setup-wizard`, `motor-arbitrary-waveform-plotting`).
