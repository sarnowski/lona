# Lona

## See Everything. Change Anything.

Lona is a next-generation operating system that brings together the best ideas from three revolutionary computing paradigms: the **seL4 microkernel** for verified security, the **LISP machine philosophy** for complete runtime transparency, and the **Erlang/OTP concurrency model** for fault-tolerant distributed computing.

The result is an operating system where you have complete visibility and control over every aspect of the running system, where failures are contained and automatically recovered, and where the full power of modern concurrent programming is available at every level of the stack.

---

## Key Features

### Complete Runtime Transparency

Unlike traditional operating systems that hide their internals behind opaque binaries and debugging requires special tools, Lona treats the running system as a **living, inspectable environment**:

- **Every function** can be inspected, disassembled, and understood
- **Every value** can be examined at runtime
- **Every process** can be queried for its state
- **Every piece of code** can be modified without stopping the system

Found a bug in a network driver? Connect via UART, find the problematic function, fix it, and continue -- no reboot required.

### Erlang-Style Fault Tolerance

Lona embraces the "let it crash" philosophy from Erlang/OTP:

- **Lightweight processes** -- run millions of concurrent processes with minimal overhead
- **Process isolation** -- a crash in one process doesn't corrupt others
- **Supervision trees** -- automatically restart failed processes
- **Self-healing systems** -- build services that recover from failures without human intervention

### Verified Security Foundation

Built on the formally verified seL4 microkernel, Lona provides hardware-enforced security:

- **Capability-based access control** -- every resource access requires an unforgeable capability token
- **Strong isolation** -- domains cannot access resources they weren't explicitly granted
- **Principle of least privilege** -- components receive only the capabilities they need
- **No bypass possible** -- security is enforced at the kernel level, not by convention

### Single Language, Full Stack

**Lonala**, Lona's system programming language, combines:

- **Clojure's elegance** -- S-expression syntax, immutable data structures, powerful metaprogramming
- **Erlang's concurrency** -- processes, message passing, pattern matching, hot code loading
- **Systems programming power** -- direct hardware access, inline assembly, capability manipulation

Write everything from device drivers to high-level applications in one unified, expressive language.

### Source-Only Distribution

Lona takes a radical approach: **source code is the only distributable format**.

- **Total transparency** -- you can always see exactly what code is running
- **Full debugging everywhere** -- source is always available, no "missing symbols"
- **Platform independence** -- same package works on ARM64, x86_64, any supported platform
- **Security auditing** -- inspect any package before loading

---

## Architecture

### Processes and Domains

Lona introduces two primary abstractions:

**Process** -- The fundamental unit of execution, inspired by Erlang/BEAM:

- Extremely lightweight (hundreds of bytes overhead)
- Millions can run concurrently
- Communicates exclusively via message passing
- Independently garbage collected

**Domain** -- A security and memory isolation boundary:

- Hardware-enforced memory isolation via seL4
- Contains one or more processes
- Holds capabilities determining resource access
- Forms a hierarchy where privilege can never be escalated

### Zero-Copy High Performance

Capability-controlled shared memory enables zero-copy data transfer between domains:

```
net-driver → tcp-stack → application
          shared      shared
          memory      memory
```

High-throughput scenarios like networking achieve kernel-bypass-level performance while maintaining strong security isolation.

### Hot Code Loading

Modify the system without stopping it:

```clojure
;; Fix a bug in production
lona> (defn net/checksum [data]
        (reduce #(bit-and (+ %1 %2) 0xFFFF) 0 data))

;; All future calls use the new implementation immediately
```

---

## Target Platforms

| Platform | Architecture | Use Case |
|----------|--------------|----------|
| **QEMU** | ARM64, x86_64 | Development and testing |
| **Raspberry Pi 4** | ARM64 | Embedded systems, education |
| **AWS Graviton** | ARM64 | Cloud server deployment |
| **x86_64 servers** | x86_64 | Traditional server infrastructure |

---

## Use Cases

- **Network appliances** -- routers, firewalls, load balancers with live debugging
- **Embedded systems** -- IoT devices, industrial controllers with fault tolerance
- **Cloud infrastructure** -- specialized server workloads with strong isolation
- **Education** -- learn operating system concepts in a live, inspectable environment

---

## For Developers Who Refuse Opacity

Lona is for those who believe "you can't see that" and "you can't change that" are unacceptable answers.

Whether you're building fault-tolerant distributed systems, security-critical embedded devices, or simply want to understand every layer of your computing stack, Lona provides the transparency and power you need.

**The operating system of the inspectable machine.**

---

## Getting Started

Check out the [Goals Document](goals.md) for detailed technical information about Lona's design and architecture.

## License

Copyright (C) 2025 Tobias Sarnowski

Lona is free software, released under the [GNU General Public License v3](license.md).
