## MODIFIED Requirements

### Requirement: Circular Dot-Matrix Display Font
The WebUI SHALL bundle a subsetted circular dot-matrix font (`ndot57.woff2`) supporting full ASCII uppercase and lowercase characters (`A-Z`, `a-z`, `0-9`, symbols) under `webui/fonts/` and serve it through `picoserve` with `font/woff2` MIME headers.

#### Scenario: Loading NDot57 font
- **WHEN** the browser requests `/fonts/ndot57.woff2`
- **THEN** the server returns HTTP 200 with `Content-Type: font/woff2` and long-term cache headers.

#### Scenario: Display font binding in industrial mode
- **WHEN** `data-style="industrial"` is active
- **THEN** headings (`h1`, `h2`, `h3`), hero metric numbers, system clock, and launcher tile labels apply `font-family: 'NDot57', ui-monospace, monospace` with wide letter-spacing, supporting mixed-case titles without falling back to system fonts.

## ADDED Requirements

### Requirement: Hybrid Monospace Hierarchy
The WebUI SHALL enforce a 3-tier hybrid typography system: circular dot-matrix (`NDot57`) for headings and display tiles, technical monospace with expanded tracking (`ui-monospace, monospace`) for all labels, badges, parameters, buttons, and tables, and system sans-serif for long descriptive paragraphs and documentation blocks.

#### Scenario: Telemetry and button typography
- **WHEN** buttons, badges, table cells, or telemetry values render
- **THEN** they display in technical monospace with `letter-spacing: 0.08em` and uppercase styling.

#### Scenario: Paragraph readability
- **WHEN** multi-sentence instructional text or help popups render
- **THEN** they display in clean system sans-serif for optimal sustained reading comprehension.
