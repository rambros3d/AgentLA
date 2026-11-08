## MODIFIED Requirements

### Requirement: Visual Flash Partition Map & Dedicated Partition Cards
The diagnostics page SHALL display a proportional segmented linear memory bar and geometry table representing the Flash layout (`BOOT_TABLE`, `FW_SLOT_A`, `FW_SLOT_B`, `SYSTEM_FS`, `USER_FS`, `TRAIL`), with dedicated cards for `WebUI & Storage (SYSTEM_FS)` and `User Storage (USER_FS)` containing in-place formatting controls.

#### Scenario: Viewing memory geometry and partition state
- **WHEN** the diagnostics page loads
- **THEN** it highlights the currently active firmware execution slot in the segmented linear bar and displays offset boundaries and LittleFS headroom for both `SYSTEM_FS` (512 KB at `0x00260000`) and `USER_FS` (1136 KB at `0x002E0000`).
