# AgentLA Virtual Lab Harness

This folder is the virtual test area for AgentLA while hardware is unavailable.

## What runs locally

- `test_pin_owner.c` + `firmware/src/pin_owner.c`
  - Native C validation of ownership invariants, fixed-flexible boundary, claims, observers, conflicts, release, and idempotent completion.
- `test_sdk_claims.py`
  - SDK-level virtual validation using a mocked connection, verifying handle creation and command formatting.

## Build/run

```bash
make 2>/dev/null || true
cd /run/media/sun/halo/AgentLA
gcc -Wall -Wextra -Iagentla-labwired/stub -I firmware/src \
    agentla-labwired/test_pin_owner.c firmware/src/pin_owner.c \
    -o agentla-labwired/test_pin_owner
agentla-labwired/test_pin_owner

PYTHONPATH=/run/media/sun/halo/AgentLA/sdk python3 agentla-labwired/test_sdk_claims.py
```

## LabWired applicability

LabWired is a deterministic simulator service and RP2350 models exist, but AgentLA relies on TinyUSB CDC and network-stack interactions. Therefore register/scratch assertions are a weak fit for the ownership command protocol. The virtual checks here focus on the codec-level invariants and SDK handle contracts rather than emulating physical USB traffic.

LabWired CI may still be used later for firmware startup or isolated peripheral smoke checks, with a system manifest targeted to the genuine RP2350 board model. The agentla-labwired folder keeps those future manifests separate from the native tests.