## RENAMED Requirements
- FROM: `Standalone Bento File Manager Page`
- TO: `Standalone Industrial Storage Console Page`

## MODIFIED Requirements

### Requirement: In-Place Text and Code Editor Modal
The File Manager SHALL include an embedded DaisyUI `.modal` dialog capable of previewing and editing text files (`.json`, `.toml`, `.js`, `.html`, `.css`, `.txt`, `.csv`) up to 256 KB. The extension allow-list and the 256 KB cap SHALL be enforced before the modal is populated and again before a save is sent. Files outside the allow-list or above the cap SHALL NOT be loaded into the textarea.

#### Scenario: Opening text file in user_fs
- **WHEN** the user clicks an editable text file in `user_fs`
- **THEN** the editor modal opens with the file content loaded in a monospace textarea, displaying a "Save Changes" button.

#### Scenario: Opening text file in system_fs
- **WHEN** the user clicks a text file in `system_fs`
- **THEN** the editor modal opens in read-only mode with the "Save Changes" button disabled and an informational banner indicating that system ROM assets cannot be modified.

#### Scenario: Opening a non-text asset
- **WHEN** the user clicks a file whose extension is not in the editable set, including `.gz`, `.woff2`, `.flm`, and `.bin`
- **THEN** the editor modal SHALL NOT open and the File Manager SHALL display a message stating that only text files can be previewed or edited.

#### Scenario: Opening an oversized text file
- **WHEN** the user clicks a text file whose reported size exceeds 256 KB
- **THEN** the editor modal SHALL NOT load its content and SHALL display a message stating the 256 KB edit limit.

## ADDED Requirements

### Requirement: Complete Directory Listing Feedback
The File Manager SHALL render only directory listing responses that parsed successfully as JSON. If a listing response is malformed, unavailable, or not an array, the File Manager SHALL surface an explicit error state and SHALL NOT substitute demo or mock entries, and SHALL NOT present the directory as empty on device firmware.

#### Scenario: Malformed listing payload
- **WHEN** `GET /api/v1/fs/list` returns HTTP 200 with a truncated or non-array body
- **THEN** the File Manager displays an error state identifying the listing as unavailable, rather than an empty-directory message.

#### Scenario: Listing request failure
- **WHEN** `GET /api/v1/fs/list` returns a non-OK status or the request throws
- **THEN** the File Manager displays an error state and retains the previously rendered entries until the user retries.

### Requirement: Single Theme Authority
The Files page SHALL delegate theme toggling and persistence to `agentla-bridge.js`. It SHALL NOT define a second local theme toggle handler, `storage` listener, or `BroadcastChannel` listener that mutates theme attributes on the document root.

#### Scenario: Theme toggle on the Files page
- **WHEN** the user clicks `#btn-theme-toggle` on `files.html`
- **THEN** the theme mode cycles exactly once through the bridge handler and the resulting theme is applied without duplicate listeners.
