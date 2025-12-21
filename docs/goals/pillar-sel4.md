# Pillar I: seL4 — The Fortress

> *"Capabilities, Not Permissions"*

## Why seL4?

Traditional operating systems enforce security through permission bits, access control lists, and trust hierarchies. These mechanisms are implemented in millions of lines of code, riddled with bugs, and regularly bypassed by attackers.

seL4 takes a fundamentally different approach:

1. **Formal Verification**: The kernel's correctness is mathematically proven. The C implementation is proven to match the specification. The specification is proven to enforce security properties.

2. **Minimal TCB**: The Trusted Computing Base is approximately 10,000 lines of verified C—small enough to audit, proven correct.

3. **Capability-Based Security**: Every resource access requires an unforgeable token (capability). There are no ambient permissions, no privilege escalation paths.

Lona builds on this foundation. seL4 doesn't just run Lona—it enforces Lona's security model at the hardware level.

---

## Philosophy: Capabilities, Not Permissions

In traditional systems, security is about *who you are*. Processes run as users, users belong to groups, files have permission bits. The kernel decides access based on identity.

In seL4 (and Lona), security is about *what you hold*. Processes hold capabilities—unforgeable tokens that grant specific access to specific resources. No capability, no access. Period.

### Properties of Capabilities

| Property | Meaning |
|----------|---------|
| **Unforgeable** | Capabilities can only be created by the kernel |
| **Delegable** | A capability holder can grant it to others |
| **Attenuable** | Capabilities can be weakened when delegated (read-write → read-only) |
| **Revocable** | The grantor can invalidate delegated capabilities |

### Example: Driver Capabilities

```clojure
;; A network driver receives exactly what it needs
(defn start-net-driver [nic-cap irq-cap buffer-cap]
  ;; nic-cap: access to network hardware registers
  ;; irq-cap: ability to handle network interrupts
  ;; buffer-cap: shared memory for packet buffers
  ;;
  ;; This domain CANNOT access: disk, UART, other memory, other IRQs
  ...)
```

The driver cannot escalate privileges. It cannot access resources beyond its capabilities. If compromised, the damage is contained to what it was explicitly granted.

---

## What seL4 Forces in Lona

seL4 imposes hard constraints that shape Lona's entire architecture:

### 1. Explicit Authority

Nothing happens implicitly. Every resource access—memory, hardware, IPC—requires a capability. This means:

- Processes cannot "reach out" to resources they shouldn't access
- All authority is traceable (who granted what to whom)
- Sandboxing is the default, not a special configuration

### 2. The Domain as Trust Boundary

seL4 provides two primitives that Lona combines into the **Domain** abstraction:

| seL4 Primitive | Purpose | Lona Mapping |
|----------------|---------|--------------|
| **VSpace** | Virtual address space | Memory isolation |
| **CSpace** | Capability space | Authority scope |

A Domain in Lona is a VSpace + CSpace pair. It represents:
- A memory isolation boundary (processes in different Domains cannot share memory unless explicitly granted)
- An authority scope (the set of capabilities this Domain holds)

### 3. Hierarchical Delegation

Capabilities flow downward in a tree:

```
Root Domain (holds all capabilities at boot)
├── grants network caps → Driver Domain
│                         └── cannot grant disk caps (doesn't have them)
├── grants disk caps → Storage Domain
└── grants user caps → User Domain
    └── grants subset → Sandboxed App Domain
```

A Domain can only delegate capabilities it possesses. Privilege never escalates.

### 4. No Ambient Authority

There is no "root user" that bypasses security. There are no magic UIDs. Even the init process only has the capabilities explicitly provided at boot. Every capability grant is auditable.

---

## The Domain Abstraction

Lona presents seL4's primitives through a single unified concept: the **Domain**.

### Domain Properties

| Property | Description |
|----------|-------------|
| **Memory Isolation** | Processes in different Domains cannot access each other's memory |
| **Capability Scope** | A Domain holds a set of capabilities determining what it can access |
| **Hierarchy** | Domains form a tree; children can only receive capabilities from parents |
| **Contains Processes** | A Domain is not an execution unit—it contains one or more Processes |

### Domain is the ONLY Security Boundary

Within a Domain, there is no security boundary. Processes in the same Domain:
- Share the same memory space
- Share the same capabilities
- Can inspect and modify each other

This is intentional. A Domain is a single trust zone. If you don't trust code, put it in a separate Domain.

Between Domains, security is absolute. seL4 enforces:
- Memory isolation (hardware MMU)
- Capability separation (kernel CSpace)
- IPC mediation (kernel message passing)

### Mapping to seL4

| Lona Concept | seL4 Primitive | Notes |
|--------------|----------------|-------|
| Domain | VSpace + CSpace | Unified abstraction |
| Process | Multiplexed on TCBs | Lona runtime manages scheduling |
| Capability | seL4 Capability | First-class in Lonala |
| Message | seL4 IPC | Cross-domain communication |

seL4's Thread Control Blocks (TCBs) are an implementation detail. Lona creates TCBs as needed (typically one per CPU core per Domain) and multiplexes lightweight Processes onto them.

---

## Security Model

### The Single Rule

**A Domain can only access resources for which it holds a capability.**

Everything else follows from this:

| Question | Answer |
|----------|--------|
| Can process A read process B's memory? | Only if in the same Domain, or A holds a memory capability for B's region |
| Can a driver access arbitrary hardware? | Only if it holds the relevant device capabilities |
| Can a user process become root? | There is no root. It can only use capabilities it was granted |
| Can malware spread across domains? | Only by exploiting seL4 (proven correct) or the granting process |

### Principle of Least Privilege

Lona enforces minimal capability grants:

```clojure
;; Running untrusted downloaded code
(spawn untrusted-code/main [downloaded-data]
       {:domain "sandbox:untrusted"
        :capabilities []              ; NONE - pure computation only
        :memory-limit (megabytes 32)
        :can-spawn-domains false})

;; This code:
;; - Cannot access any hardware
;; - Cannot communicate except via reply to spawner
;; - Cannot create child domains
;; - Cannot escape its memory limit
;; - Can be killed at any time
```

### Revocation

When a capability is revoked, the revocation cascades:

```
Parent grants cap-A to Child
Child grants cap-A to Grandchild

Parent revokes cap-A from Child
→ Child loses cap-A
→ Grandchild loses cap-A (cascade)
```

This ensures that authority can always be withdrawn by whoever granted it.

---

## Implications for Lona Design

seL4's constraints shape these Lona design decisions:

| Decision | Driven By |
|----------|-----------|
| Domains as containers for Processes | VSpace/CSpace model |
| Message passing for cross-domain IPC | seL4 IPC enforcement |
| Capability-controlled shared memory | Can't share without capability |
| No global namespace | Would bypass capability model |
| Supervisor-granted capabilities | Hierarchical delegation |

---

## Summary

seL4 provides Lona with:

| Guarantee | Mechanism |
|-----------|-----------|
| **Memory isolation** | Hardware MMU + VSpace |
| **Authority control** | Capability tokens |
| **No privilege escalation** | Hierarchical delegation |
| **Minimal attack surface** | 10K lines verified kernel |
| **Provable security** | Formal verification |

**The Bottom Line**: In Lona, security isn't enforced by careful programming or defensive coding. It's enforced by the kernel, which is mathematically proven correct. Bugs in userspace can cause crashes, but they cannot violate security boundaries.

---

## Further Reading

- [Core Concepts: Domain](core-concepts.md#domain)
- [System Design: Security Mechanics](system-design.md#security-mechanics)
- [seL4 Whitepaper](https://sel4.systems/About/seL4-whitepaper.pdf)
- [seL4 Reference Manual](https://sel4.systems/Info/Docs/seL4-manual-latest.pdf)
