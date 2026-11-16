# network-gdb-server Specification

## Purpose
Provides an on-probe TCP GDB Remote Serial Protocol (RSP) server on port 3333, enabling host debuggers (`arm-none-eabi-gdb`) to attach, inspect registers/memory, single-step, and set hardware breakpoints over the network without host-side debug daemons.
## Requirements
### Requirement: GDB Remote Serial Protocol TCP server
The system SHALL run a TCP server listening on port 3333 implementing the GDB Remote Serial Protocol (RSP) over the network, routing core debug controls and system memory accesses to the corresponding CoreSight Access Ports.

#### Scenario: GDB client connection
- **WHEN** an external GDB client connects to `tcp://10.10.10.10:3333`
- **THEN** the system accepts the socket connection, negotiates RSP packets, and halts target on initial connection if requested.

#### Scenario: Register and memory inspection
- **WHEN** GDB sends register read (`$g#`) or memory read (`$m<addr>,<len>#`) commands
- **THEN** the system executes SWD CoreSight access and returns formatted hex response packets according to GDB RSP specification.

#### Scenario: Dual-AP memory access on ARMv8-M (STM32H5) targets
- **WHEN** GDB issues a memory read (`$m08000000,10#`) or write command for a target detected with ARMv8-M architecture (`DPIDR == 0x6BA02477` or `0x0BE12477`)
- **THEN** the server routes memory transfers through AP 1 (`AHB-AP`) using CSW `0x23000012` while maintaining core debug registers on the appropriate port, returning valid hex payload data instead of error `E01`.

#### Scenario: Execution control and single stepping
- **WHEN** GDB issues continue (`$c#`) or single-step (`$s#`) commands
- **THEN** the system updates DHCSR registers to resume or step the target core and returns halt status on breakpoint or watchpoint hit.

### Requirement: Breakpoint management
The system SHALL support inserting and removing hardware breakpoints via GDB `$Z0` and `$z0` packets using Cortex-M FPB (Flash Patch and Breakpoint unit).

#### Scenario: Hardware breakpoint toggle
- **WHEN** GDB requests inserting a hardware breakpoint at address $A$
- **THEN** the system configures an available FPB comparator register with address $A$ and returns `OK`

