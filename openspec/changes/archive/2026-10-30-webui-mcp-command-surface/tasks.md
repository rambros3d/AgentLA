## 1. Discovery and command contract

- [x] 1.1 Audit every firmware command for WebUI safety classes.
- [x] 1.2 complete command class metadata exposed through `/mcp`.
- [x] 1.3 Define JSON-RPC request and response schema for command calls.
- [x] 1.4 Update network response templates and error codes.

## 2. Command bridge implementation

- [x] 2.1 Add `cmd` request parser and command-string normalizer.
- [x] 2.2 Add safety class filtering.
- [x] 2.3 Handle query requests and JSON body requests consistently.
- [x] 2.4 Enforce request payload limits.
- [x] 2.5 Add operation handles for long commands.

## 3. WebUI field integration

- [x] 3.1 Add Adapter/I2C and SPI form fields and command mapping.
- [x] 3.2 Implement Flasher chunking and progress response.
- [x] 3.3 Implement capture form and span metadata UI.
- [x] 3.4 Implement debugger field mapping and address-aware reads.
- [x] 3.5 Add destructive-operation confirmation workflow.

## 4. Verification and status hygiene

- [x] 4.1 Verify command bridge with a synthetic client.
- [x] 4.2 Verify one complete WebUI workflow.
- [x] 4.3 Mark archived WebUI milestones as scaffold-only until evidence exists.
- [x] 4.4 Validate progress events and boundary checks.