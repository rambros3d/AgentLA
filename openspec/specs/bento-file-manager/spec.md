# bento-file-manager Specification

## Purpose
Provides a dedicated standalone Bento Storage Explorer WebUI (`files.html`) styled using DaisyUI 5 and Tailwind CSS, featuring partition storage metrics, directory traversal, breadcrumbs navigation, inline modal file viewer/editor, and file uploader/manager for `user_fs` and read-only `system_fs`.

## Requirements

### Requirement: Standalone Bento File Manager Page
The system SHALL provide a dedicated standalone WebUI page at `files.html` (accessible via `/files` and `/files.html`) styled using DaisyUI 5 and Tailwind CSS that renders a Bento Storage Explorer for both `system_fs` and `user_fs`.

#### Scenario: Navigating to files.html
- **WHEN** a user navigates to `/files` or `/files.html`
- **THEN** the browser loads `files.html` displaying the Bento Storage Explorer with top navigation bar, partition metrics, and directory canvas.

#### Scenario: Theme synchronization
- **WHEN** `files.html` loads
- **THEN** it reads `localStorage` for `agentla-theme` and `agentla-theme-mode` and applies the theme attributes before DOM render without visual flicker.

### Requirement: Partition Metrics Display
The File Manager SHALL display real-time storage metrics for both `system_fs` and `user_fs` using DaisyUI `.stats` and `.progress` components, visibly distinguishing the 512 KB read-only system partition from the dynamic user storage.

#### Scenario: Loading storage metrics
- **WHEN** the File Manager initializes
- **THEN** it queries `/api/v1/fs/stat` and renders total bytes, used bytes, and percent utilization for both partitions with distinct visual badges (`badge-neutral` for System, `badge-primary` for User).

### Requirement: Breadcrumb and Partition Navigation
The File Manager SHALL provide interactive breadcrumb navigation (`.breadcrumbs`) and a segmented partition switcher (`.join`) allowing users to switch between `sys:/` and `user:/` roots and jump directly to any parent directory.

#### Scenario: Switching partitions
- **WHEN** the user clicks the "System" or "User" segment in the partition switcher
- **THEN** the File Manager updates the active root and re-fetches the directory listing for the selected partition's root.

#### Scenario: Clicking breadcrumb segment
- **WHEN** the user clicks any parent directory segment in the breadcrumbs bar
- **THEN** the File Manager navigates directly to that folder and updates the URL hash/state.

### Requirement: Strict Read-Only UI for System Partition
When browsing `system_fs`, the File Manager SHALL strictly prohibit mutating actions by disabling or hiding upload buttons, new folder triggers, delete actions, and renaming controls.

#### Scenario: Viewing system_fs
- **WHEN** the active partition is `system_fs`
- **THEN** the UI displays a ` Read-Only` status badge, disables file upload dropzones, hides the "New Folder" button, and omits destructive options from item context menus.

### Requirement: Full Read-Write UI for User Partition
When browsing `user_fs`, the File Manager SHALL provide controls to upload files, create new folders, rename items, delete files or directories, and download files.

#### Scenario: Creating a folder in user_fs
- **WHEN** the user clicks "New Folder" and confirms a valid folder name
- **THEN** the system sends a `POST /api/v1/fs/mkdir` request and refreshes the directory listing upon success.

#### Scenario: Uploading a file to user_fs
- **WHEN** the user drags a file into the upload dropzone or selects a file via file input
- **THEN** the system streams the file to `POST /api/v1/fs/write?root=user&path=...` and displays upload progress.

#### Scenario: Deleting an entry in user_fs
- **WHEN** the user selects "Delete" on an item and confirms in the confirmation dialog
- **THEN** the system sends a `DELETE /api/v1/fs/entry?root=user&path=...` request and removes the item from view.

### Requirement: In-Place Text and Code Editor Modal
The File Manager SHALL include an embedded DaisyUI `.modal` dialog capable of previewing and editing text files (`.json`, `.toml`, `.js`, `.html`, `.css`, `.txt`, `.csv`) up to 256 KB.

#### Scenario: Opening text file in user_fs
- **WHEN** the user clicks an editable text file in `user_fs`
- **THEN** the editor modal opens with the file content loaded in a monospace textarea, displaying a "Save Changes" button.

#### Scenario: Opening text file in system_fs
- **WHEN** the user clicks a text file in `system_fs`
- **THEN** the editor modal opens in read-only mode with the "Save Changes" button disabled and an informational banner indicating that system ROM assets cannot be modified.
