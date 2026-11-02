## Context

AgentLA currently treats flexible hardware instructions as compile-time constants. `board_settings.h` fixes Adapter pins at GPIO 10–19, Capture at GPIO 20–29, and shares several Programmer pins with Adapter, resulting in brittle assumptions and mode conflicts. Programmers (`GPIO0–9`) are hardware-bound to the target and remain fixed, but Adapter I2C/UART/SPI/CAN, Logic Analyzer, Scope, and ADC need to be claimable on flexible GPIOs at runtime.

The design also needs to preserve single-core runtime discipline: AgentLA owns the flexible pins and cannot safely release them while active capture, SPI flash, I2C, or UART operations are in progress.

## Goals / Non-Goals

**Goals:**
- Make all `GPIO10–29` programmatically claimable by Adapter, Capture, Scope, and ADC roles.
- Keep `GPIO0–9` programmer resources permanently reserved in all firmware builds and command modes.
- Use RP2350 function support plus package-specific analog input constraints to validate claims before any pin function is changed.
- Require explicit claim handles for all flexible role use, with no unclaimed default behavior.
- Model a capture bank as a span of pins rather than a fixed ten-channel bank.
- Support tools and tests that can retarget their interface pins without editing the board.
- Provide deterministic error codes for capacity, conflict, capacity, or ADC-invalid claims.

**Non-Goals:**
- No automatic discovery of physical wiring or board topology in v1.
- No software model for changing `GPIO0–9`.
- No full concurrent multi-bank capture; v1 allows exactly one bank claim at a time.
- No backward compatibility with commands that implicitly use historical adapter/capture pin macros.
- No reinterpretation of physical pin differences across QFN package variants other than analog availability.

## Decisions

### 1. Fixed pool and flexible pool are physically different

**Decision:** `GPIO0–9` are always reserved as Programmer-only resources, and flexible roles may claim only `GPIO10–29`.

**Rationale:** The Programmer interface has dedicated electrical connections and SWD/nRST/BUS constraints. Retargeting it would make programming unreproducible and risk target damage.

**Alternatives considered:** Allow Programmer I2C to flex onto larger banks. Rejected because it would blur the fixed hardware boundary and complicate safety guarantees.

### 2. Strict blank-state claim model

**Decision:** No flexible role is usable before an explicit claim. The registry starts empty, and each claim returns a handle.

**Rationale:** This forces correctness in tests and avoids hidden assumptions. It also improves automation and error detection.

**Alternatives considered:** Warm handles that lazily assume default pins. Rejected because defaults would silently hide board-profile mismatches and weaken requirement discipline.

### 3. Full per-pin role claims

**Decision:** Claim requests specify role type, requested pins, and the requested RP2350 function. Claims must be validated as a unit.

**Rationale:** Maximum flexibility requires explicit resource selection. Bundled defaults would reintroduce hidden wiring assumptions.

**Alternatives considered:** High-level profile-only claim with fixed pin bundle macros. Rejected because it limits retargeting.

### 4. Span-based capture ownership

**Decision:** A caller may choose an arbitrary subset of capture pins. The claim reserves the full inclusive physical span `[min,max]` as exclusive capture ownership, and the host receives both the selected-channel mask and span width.

**Rationale:** RP2350 capture hardware samples contiguous banks efficiently. Claiming only selected endpoints would leave other pins in the span unconstrained and create contention. The span requirement is a deliberate safety and simplicity boundary.

**Alternatives considered:** Arbitrary discontinuous pin masking without exclusivity. Rejected because it risks contention and makes PIO mapping ambiguous.

### 5. One active capture bank at a time

**Decision:** Allow multiple capture claims to be defined, but allow only one to be active.

**Rationale:** A single state machine/DMA path keeps timing, memory, and host parsing manageable while still unlocking flexible placement.

### 6. ADC as dependent capability

**Decision:** ADC claims are allowed only on GPIO pins whose analog function physically exists in the selected package. On the current pin-allocation baseline, only `GPIO26–29` are analog-capable.

**Rationale:** ADC capability is a hardware fact, not a role preference. Reporting invalid ADC pin claims early prevents invalid test initialization.

### 7. Explicit lifecycle and auto-stop/release

**Decision:** Ownership handles must be explicitly released. If releasing an active role, the system shall automatically quiesce it, then release the handle. Conflicts are returned, not overwritten.

**Rationale:** This supports user-driven control and protects active operations.

**Alternatives considered:** Wait-and-fail timeouts on active roles. Rejected because it makes test execution brittle and can mask ordering errors.

### 11. Shared input observation
- **Decision:** Multiple read-only observers may sample a flexible pin while a single active driver retains it, provided the underlying function remains an input and no output driver or electrical side effect is created.
- **Rationale:** This gives diagnostic tests useful visibility without promoting overlapping control authority.
- **Alternatives:** Fully exclusive ownership for all uses. Rejected because harmless passive observation improves flexible troubleshooting.

### 12. Runtime pin inventory
- **Decision:** Firmware shall report the package and analog-capable pins it can safely offer rather than only deriving them from compile-time constants.
- **Rationale:** This keeps flexible assignment compatible with package variants and API clients.
- **Alternatives:** Static documentation only. Rejected because exports and tests benefit from runtime truth.

### 13. Safe read-only discovery
- **Decision:** Automatic interface discovery shall be an explicit, input-only mode that cannot mutate bus pins, append transactions, or assume contention.
- **Rationale:** It preserves device safety while improving automation.
- **Alternatives:** Full active probing by default. Rejected because it may disturb targets.

### 14. Transparent bundles
- **Decision:** The SDK may provide bundles that expand to explicit handle-level claims, but ownership records shall remain per-role and per-resource.
- **Rationale:** Convenience should not collapse into hidden assumptions.
- **Alternatives:** Explicit claims only. Rejected as excessively verbose.

### 15. Bundles are all-or-nothing
- **Decision:** Transparent SDK bundles expand to explicit claims but must be entirely successful or entirely failed; no partial claims are retained.
- **Rationale:** Partial claims leave ambiguous state and make failure handling complicated.
- **Alternatives:** Best-effort subsets with diagnostics. Rejected for v1 safety and predictability.

### 16. Initial capture width limit
- **Decision:** v1 shall target an initial maximum capture span width of 8 bits, with documented stretch targets to 16 and 20 once DMA and timing are measured. Registry rejection at hard limits remains the fallback.
- **Rationale:** This balances flexibility with safe resource planning and test reliability.
- **Alternatives:** Full flexible-width claims from the outset. Rejected because DMA, FIFO, and buffer trade-offs remain unmeasured.

### 17. Input-only shared observers
- **Decision:** Shared observers are permitted only on already-input pins and are rejected for pins whose active driver could be output or analog-triggering.
- **Rationale:** This prevents passive observation from becoming a hidden drive conflict.

### 18. Idempotent release
- **Decision:** Repeated release requests shall be idempotent once quiesce is in progress or complete.
- **Rationale:** Automation and host SDKs benefit from deterministic retry behavior without duplicate side effects.

### 19. Report-only discovery
- **Decision:** Automatic discovery shall report eligible and conflicting resources but shall not mutate pins, initiate I/O, or acquire claims beyond ephemeral safe observation.
- **Rationale:** It enables useful test retargeting without surprising safety behavior.

### 20. Interrupt ownership resource
- **Decision:** Interrupts and edge callbacks shall be external lifecycle resources tied to an otherwise valid capture or pin claim rather than implicit firmware behavior.
- **Rationale:** This preserves explicit resource tracking and avoids dangling callbacks after release.

## Risks / Trade-offs
- [Risk] Large refactor of adapter and capture initialization paths. → Mitigation: use separate ownership state structures and gates.
- [Risk] Capture bank sizing and PIO memory limits may constrain high-speed sampling. → Mitigation: report available maximum width per claim.
- [Risk] Adapter role claims may expose wiring errors. → Mitigation: return peer conflict diagnostics before physical pin changes.
- [Risk] Automatic quiesce can briefly disturb shared signals. → Mitigation: only use it at explicit release time and require stable states.
- [Risk] Tests and SDK carry all claim complexity. → Mitigation: provide a clear handle lifecycle.
- [Risk] The one-bank-at-a-time constraint may limit parallel diagnosis. → Mitigation: document the limitation and consider multi-bank later.

## Migration Plan

1. Add a pin-capability table and ownership model behind a build guard.
2. Refactor Adapter roles to use claim handles for current commands.
3. Refactor Capture into a handle-based bank model.
4. Update host SDK/test tools to create and destroy claims explicitly.
5. Remove fixed flexible-pin assumptions and update docs.
6. Run adapter, capture, and SDK tests under the new explicit model.

Rollback strategy: retain the prior macros at compile time only until ownership implementation is verified; actual behavior should use new claims once complete.

## Open Questions

- Should claim handles support one-to-many pin use when multiple logical channels share the same physical wire?
- What should the maximum safe capture span be under default and stretch mode?
- How should analog-only roles participate in a mixed digital capture bank?
