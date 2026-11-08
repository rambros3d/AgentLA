## ADDED Requirements

### Requirement: Circular Dot-Matrix Display Font
The WebUI SHALL bundle a subsetted circular dot-matrix font (`ndot57.woff2`) under `webui/fonts/` and serve it through `picoserve` with `font/woff2` MIME headers.

#### Scenario: Loading NDot57 font
- **WHEN** the browser requests `/fonts/ndot57.woff2`
- **THEN** the server returns HTTP 200 with `Content-Type: font/woff2` and long-term cache headers.

#### Scenario: Display font binding in industrial mode
- **WHEN** `data-style="industrial"` is active
- **THEN** headings (`h1`, `h2`, `h3`), hero metric numbers, system clock, and launcher tile labels apply `font-family: 'NDot57', ui-monospace, monospace` with wide letter-spacing.

### Requirement: Technical Monospace Metadata Hierarchy
Under the industrial persona, hardware telemetry, pin numbers, memory addresses, baud rates, and status tags SHALL render in uppercase monospaced typography with expanded letter-spacing.

#### Scenario: Telemetry badge rendering
- **WHEN** a status badge or hardware parameter renders in industrial mode
- **THEN** it applies `font-family: ui-monospace, monospace`, `letter-spacing: 0.12em`, and `text-transform: uppercase`.

### Requirement: Body Text Legibility Preservation
The WebUI SHALL NOT apply the dot-matrix font to small body paragraphs, code editors, file names, or terminal log output, keeping them in clean, high-legibility sans-serif or standard monospace.

#### Scenario: Viewing file list or code editor
- **WHEN** user views file names or edits text in a modal dialog
- **THEN** the content renders in clean system sans-serif or standard monospace rather than dot-matrix.
