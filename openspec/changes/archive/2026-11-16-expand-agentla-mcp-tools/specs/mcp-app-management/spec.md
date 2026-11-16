## ADDED Requirements

### Requirement: Application Listing Tool
The MCP server SHALL provide an `app_list` tool that returns a JSON list of all installed LittleFS applications and filesystem storage capacity.

#### Scenario: Listing installed applications
- **WHEN** client invokes MCP tool `app_list` with empty arguments
- **THEN** server returns a JSON response listing installed application identifiers, titles, sizes, and available LittleFS bytes

### Requirement: Application Installation Tool
The MCP server SHALL provide an `app_install` tool that installs or updates an application from the local catalog or package bundle into LittleFS.

#### Scenario: Installing catalog application by ID
- **WHEN** client invokes MCP tool `app_install` with `name` parameter matching a catalog ID
- **THEN** server unpacks the package tarball into `/apps/<name>` and returns success status

### Requirement: Application Uninstallation Omission
The MCP server SHALL NOT expose an application uninstallation or deletion tool, restricting removal exclusively to manual user actions.

#### Scenario: Attempting uninstallation over MCP
- **WHEN** client invokes tool `app_uninstall` or `app_delete`
- **THEN** server returns JSON-RPC method or tool not found error code
