# static-portal-deployment Specification

## Purpose
TBD - created by archiving change hosted-webui-landing-demo. Update Purpose after archive.
## Requirements
### Requirement: Firmware Flash Isolation
All marketing assets, simulation data, and hosted scripts SHALL reside in the `web/` directory and MUST NOT be referenced by `CMakeLists.txt` or compiled into `pico_fsdata.inc`.

#### Scenario: Compiling RP2350 firmware binary
- **WHEN** building the embedded firmware with `make AgentLA`
- **THEN** `pico_set_lwip_httpd_content` bundles only `firmware/html/` files, producing an unbloated `pico_fsdata.inc`

### Requirement: Automated Static Site Deployment
The repository SHALL include a GitHub Actions workflow to publish the `web/` directory to GitHub Pages upon commits to the `main` branch.

#### Scenario: Triggering CI deployment
- **WHEN** changes to `web/` are pushed to the repository default branch
- **THEN** GitHub Pages action deploys the static files directly to the public web hosting URL with zero manual steps

