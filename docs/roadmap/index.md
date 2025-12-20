# Lona Operating System - Comprehensive Implementation Roadmap

This document provides a detailed, task-level breakdown of the Lona operating system implementation. Each task is scoped to fit within a single agent context window and has clear dependencies.

---

## Table of Contents

1. [Overview](#overview)
2. [Milestones](#milestones)
3. [Task Status](#task-status)

---

## Overview

### Design Principles

1. **Lonala-First**: All functionality achievable in Lonala MUST be implemented in Lonala
2. **Correct Solutions Only**: No shortcuts, workarounds, or deferred solutions
3. **Test-First Development**: All bug fixes require failing tests first
4. **BEAM-Style Concurrency**: Lightweight processes, message passing, supervision trees
5. **seL4 Security**: Capability-based isolation at domain boundaries

### Architecture Layers

```
┌─────────────────────────────────────────────────────────────────┐
│                    Applications (Lonala)                        │
│         HTTP Server, Telnet, User Applications                  │
├─────────────────────────────────────────────────────────────────┤
│                    Network Stack (Lonala)                       │
│              TCP, UDP, ICMP, IPv4/IPv6, ARP                     │
├─────────────────────────────────────────────────────────────────┤
│                    Filesystem (Lonala)                          │
│                    VFS, FAT Driver                              │
├─────────────────────────────────────────────────────────────────┤
│                    Device Drivers (Lonala)                      │
│            UART, VirtIO Block, VirtIO Net                       │
├─────────────────────────────────────────────────────────────────┤
│                    Standard Library (Lonala)                    │
│    Collections, Strings, I/O, Process Patterns, Tests           │
├─────────────────────────────────────────────────────────────────┤
│                    Lonala Runtime (Rust)                        │
│  VM, Compiler, GC, Scheduler, Domains, Native Primitives        │
├─────────────────────────────────────────────────────────────────┤
│                    seL4 Microkernel                             │
│         Capabilities, IPC, Scheduling, Memory                   │
└─────────────────────────────────────────────────────────────────┘
```

---

## Milestones

| # | Milestone | Summary |
|---|-----------|---------|
| 1 | [Rust Foundation](milestone-01-rust-foundation.md) | Complete all Rust code for the runtime: VM with processes, garbage collection, domain isolation, condition/restart system, debug infrastructure (Two-Mode Architecture), and all native primitives. After this milestone, no new Rust code should be needed for the runtime. |
| 2 | [Lonala Standard Library](milestone-02-lonala-standard-library.md) | Implement complete standard library in Lonala: test framework, core functions (map/filter/reduce), control flow macros, collections, protocols, strings, process patterns (GenServer, Supervisor), and lazy sequences. Enables self-hosting via `eval` and `load`. |
| 3 | [UART Driver](milestone-03-uart-driver.md) | Implement abstract UART driver in Lonala with platform implementations for ARM64 (PL011) and x86_64 (16550). First driver running in isolated domain with capability-controlled MMIO access. |
| 4 | [Init System](milestone-04-init-system.md) | Implement Lonala init system that bootstraps the OS: platform detection, driver supervision tree, and Rust-to-Lonala handoff. Establishes the root supervision hierarchy for all system services. |
| 5 | [Lonala REPL](milestone-05-lonala-repl.md) | Replace interim Rust REPL with pure Lonala implementation: line editing, multi-line input, history, and error handling. Removes Rust REPL code, completing the transition to Lonala userspace. |
| 6 | [Block Storage](milestone-06-block-storage.md) | Implement VirtIO infrastructure (VirtQueue, common layer, device discovery) and VirtIO block driver with async I/O. Provides BlockDevice protocol for filesystem layer. |
| 7 | [Filesystem](milestone-07-filesystem.md) | Implement VFS abstraction and FAT filesystem (FAT12/16/32) with file handles, path resolution, long filenames, and mount system. Enables reading files and navigating directories. |
| 8 | [Persistent Storage](milestone-08-persistent-storage.md) | Complete file write support with cluster allocation, directory modification (create/delete/rename), and durability guarantees via fsync/sync. Applications can now persist data to disk. |
| 9 | [Network Driver](milestone-09-network-driver.md) | Implement VirtIO network driver with frame buffer management, RX/TX virtqueues, and IRQ handling. Provides NetDevice protocol as foundation for network stack. Can run parallel with M6-M8. |
| 10 | [ARP](milestone-10-arp.md) | Implement Address Resolution Protocol with cache, request/reply handling, timeouts, and GenServer interface. Enables IPv4 address to MAC address resolution for local network communication. |
| 11 | [IP Stack](milestone-11-ip-stack.md) | Implement IPv4 and IPv6 packet handling, routing tables with longest prefix match, and interface address management. Unified IP server handles protocol demultiplexing and integrates with ARP/NDP. |
| 12 | [Transport Protocols](milestone-12-transport-protocols.md) | Implement ICMP (v4/v6 echo), UDP with sockets, and complete TCP (state machine, 3-way handshake, flow control, congestion control). Provides socket API for application networking. |
| 13 | [Telnet Server](milestone-13-telnet-server.md) | Implement telnet daemon connecting remote clients to REPL sessions with per-user domain isolation. Enables remote administration and development over the network. |
| 14 | [HTTP/1 Server](milestone-14-http1-server.md) | Implement HTTP/1.1 server with request parsing, response generation, static file serving with Content-Type detection, range requests, and keep-alive. Serves web content from filesystem. |
| 15 | [TLS](milestone-15-tls.md) | Implement cryptographic primitives (SHA-256/384, AES-GCM, ChaCha20-Poly1305, RSA, ECDSA, X25519) and TLS 1.2/1.3 with X.509 certificate handling. Enables encrypted network connections. |
| 16 | [HTTP/2 with ACME](milestone-16-http2-with-acme.md) | Implement HTTP/2 with binary framing, stream multiplexing, HPACK header compression, and ALPN upgrade from HTTP/1. Includes ACME client for automatic Let's Encrypt certificate provisioning and renewal. |

---

## Task Status

This section provides a complete index of all tasks across all milestones. Use this to track progress during development.

**Legend**: `open` = not started, `done` = completed

---

### Milestone 1: Rust Foundation

#### Phase 1.0: Arithmetic Primitives

| Task | Name | Status |
|------|------|--------|
| 1.0.1 | Native Addition and Subtraction | done |
| 1.0.2 | Native Multiplication and Division | done |
| 1.0.3 | Modulo | done |
| 1.0.4 | Comparison - Equality | done |
| 1.0.5 | Comparison - Ordering | done |

#### Phase 1.1: Core Value Type Extensions

| Task | Name | Status |
|------|------|--------|
| 1.1.1 | Keyword Value Type | done |
| 1.1.2 | Set Value Type | done |
| 1.1.3 | Collection Literal Syntax | done |
| 1.1.4 | Binary Value Type | open |
| 1.1.5 | Metadata System - Value Storage | open |
| 1.1.6 | Metadata System - Reader Syntax | open |
| 1.1.7 | Metadata System - Compiler Integration | open |

#### Phase 1.2: Language Feature Completion

| Task | Name | Status |
|------|------|--------|
| 1.2.1 | Multi-Arity Function Support | open |
| 1.2.2 | Closure Implementation | open |
| 1.2.3 | Sequential Destructuring | open |
| 1.2.4 | Associative Destructuring | open |
| 1.2.5 | Nested Destructuring | open |
| 1.2.6 | [Proper Tail Calls - Compiler](../development/tco.md) | open |
| 1.2.7 | [Proper Tail Calls - VM Trampoline](../development/tco.md) | open |
| 1.2.8 | [Proper Tail Calls - Integration Tests](../development/tco.md) | open |
| 1.2.9 | Pattern Matching - Core Infrastructure | open |
| 1.2.10 | Case Special Form | open |
| 1.2.11 | Gensym Implementation | open |

#### Phase 1.3: Namespace System

| Task | Name | Status |
|------|------|--------|
| 1.3.1 | Namespace Data Structure | open |
| 1.3.2 | Var System | open |
| 1.3.3 | Namespace Declaration (`ns`) | open |
| 1.3.4 | Require/Use/Refer Implementation | open |
| 1.3.5 | Qualified Symbol Resolution | open |
| 1.3.6 | Private Vars | open |
| 1.3.7 | Dynamic Var Declaration | open |
| 1.3.8 | Per-Process Binding Stack | open |
| 1.3.9 | `binding` Special Form | open |
| 1.3.10 | `defnative` Special Form | open |

#### Phase 1.4: Process Model

| Task | Name | Status |
|------|------|--------|
| 1.4.1 | Process Data Structure | open |
| 1.4.2 | Per-Process Heap | open |
| 1.4.3 | Process Registry | open |
| 1.4.4 | Mailbox Implementation | open |
| 1.4.5 | Scheduler - Run Queue | open |
| 1.4.6 | Scheduler - Context Switching | open |
| 1.4.7 | Scheduler - Cooperative Yielding | open |
| 1.4.8 | Scheduler - Preemptive Scheduling | open |
| 1.4.9 | Spawn Primitive | open |
| 1.4.10 | Self and Exit Primitives | open |
| 1.4.11 | Send Primitive - Intra-Domain | open |
| 1.4.12 | Receive Special Form - Basic | open |
| 1.4.13 | Receive with Timeout | open |
| 1.4.14 | Selective Receive | open |

#### Phase 1.5: Garbage Collection

| Task | Name | Status |
|------|------|--------|
| 1.5.1 | Root Discovery | open |
| 1.5.2 | Tri-Color Marking | open |
| 1.5.3 | Write Barrier | open |
| 1.5.4 | Sweep Phase | open |
| 1.5.5 | Generational Optimization | open |
| 1.5.6 | GC Scheduling | open |

#### Phase 1.6: Domain Isolation & IPC

| Task | Name | Status |
|------|------|--------|
| 1.6.1 | VSpace Manager | open |
| 1.6.2 | CSpace Manager | open |
| 1.6.3 | Domain Data Structure | open |
| 1.6.4 | Domain Creation Primitive | open |
| 1.6.5 | Shared Memory Regions | open |
| 1.6.6 | Inter-Domain IPC - Notification | open |
| 1.6.7 | Inter-Domain IPC - Message Passing | open |
| 1.6.8 | Capability Transfer | open |
| 1.6.9 | Code Sharing Between Domains | open |

#### Phase 1.7: Fault Tolerance

| Task | Name | Status |
|------|------|--------|
| 1.7.1 | Process Linking | open |
| 1.7.2 | Process Monitoring | open |
| 1.7.3 | Exit Signals | open |
| 1.7.4 | Panic Implementation | open |
| 1.7.5 | Cross-Domain Fault Tolerance | open |

#### Phase 1.8: Native Primitives

| Task | Name | Status |
|------|------|--------|
| 1.8.1 | Type Predicates - Complete Set | open |
| 1.8.2 | Bitwise Operations | open |
| 1.8.3 | Collection Primitives - nth, count, conj | open |
| 1.8.4 | Map Operations - get, assoc, dissoc, keys, vals | open |
| 1.8.5 | Set Operations - disj, contains? | open |
| 1.8.6 | Binary Operations | open |
| 1.8.7 | Symbol Operations | open |
| 1.8.8 | Metadata Operations | open |
| 1.8.9 | MMIO Primitives | open |
| 1.8.10 | DMA Primitives | open |
| 1.8.11 | IRQ Primitives | open |
| 1.8.12 | Time Primitives | open |
| 1.8.13 | Atom Primitives | open |
| 1.8.14 | Sorted Collections - Basic | open |
| 1.8.15 | Sorted Collections - Custom Comparators | open |
| 1.8.16 | Regular Expressions - Compilation | open |
| 1.8.17 | Regular Expressions - Matching | open |
| 1.8.18 | String Primitive Operations | open |
| 1.8.19 | `apply` Native Primitive (CRITICAL) | open |
| 1.8.20 | `type-of` Native Primitive (CRITICAL) | open |
| 1.8.21 | `identical?` Native Primitive | open |
| 1.8.22 | `native-print` Bootstrap Primitive (CRITICAL) | open |
| 1.8.23 | `string-concat` Native Primitive | open |
| 1.8.24 | `read-string` Native Primitive | open |
| 1.8.25 | `seq` Native Primitive | open |
| 1.8.26 | x86 Port I/O Primitives | open |

#### Phase 1.9: Integration & Spec Tests

| Task | Name | Status |
|------|------|--------|
| 1.9.1 | Spec Test Framework Enhancement | open |
| 1.9.2 | Process Integration Tests | open |
| 1.9.3 | Domain Integration Tests | open |
| 1.9.4 | GC Integration Tests | open |
| 1.9.5 | Full System Integration Test | open |
| 1.9.6 | Hot Code Loading Tests | open |
| 1.9.7 | Cross-Domain Code Isolation Tests | open |
| 1.9.8 | Dynamic Binding Tests | open |

#### Phase 1.10: Condition/Restart System

| Task | Name | Status |
|------|------|--------|
| 1.10.1 | Condition Type and Signal | open |
| 1.10.2 | Handler Binding Infrastructure | open |
| 1.10.3 | `handler-bind` Macro | open |
| 1.10.4 | Restart Registry | open |
| 1.10.5 | `restart-case` Macro | open |
| 1.10.6 | `invoke-restart` Function | open |
| 1.10.7 | Basic Condition REPL Integration | open |

#### Phase 1.11: Introspection System

| Task | Name | Status |
|------|------|--------|
| 1.11.1 | Source Storage and Retrieval | open |
| 1.11.2 | `source` and `disassemble` Functions | open |
| 1.11.3 | Namespace Introspection | open |
| 1.11.4 | Process Introspection | open |
| 1.11.5 | Domain Introspection | open |
| 1.11.6 | Tracing Infrastructure | open |
| 1.11.7 | Hot Code Propagation | open |

#### Phase 1.12: Debug Infrastructure

| Task | Name | Status |
|------|------|--------|
| 1.12.1 | Process Debug State | open |
| 1.12.2 | Debug Attach/Detach | open |
| 1.12.3 | Panic Behavior in Debug Mode | open |
| 1.12.4 | Stack Frame Reification | open |
| 1.12.5 | In-Frame Evaluation | open |
| 1.12.6 | Debug Control Operations | open |
| 1.12.7 | Breakpoint Infrastructure | open |
| 1.12.8 | Breakpoint via Dispatch Table | open |
| 1.12.9 | Trace-to-Break Upgrade | open |
| 1.12.10 | Debugger REPL Integration | open |
| 1.12.11 | Supervisor Debug Awareness | open |

---

### Milestone 2: Lonala Standard Library

#### Phase 2.1: Test Framework

| Task | Name | Status |
|------|------|--------|
| 2.1.1 | Test Namespace Foundation | open |
| 2.1.2 | Test Runner | open |
| 2.1.3 | Fixtures and Setup/Teardown | open |
| 2.1.4 | Test Integration with Build | open |

#### Phase 2.2: Core Functions

| Task | Name | Status |
|------|------|--------|
| 2.2.1 | Sequence Functions - Basic | open |
| 2.2.2 | Sequence Functions - Transformation | open |
| 2.2.3 | Sequence Functions - Construction | open |
| 2.2.4 | Sequence Functions - Combination | open |
| 2.2.5 | Sequence Functions - Partitioning | open |
| 2.2.6 | Higher-Order Functions | open |
| 2.2.7 | apply Wrapper | open |

#### Phase 2.3: Control Flow Macros

| Task | Name | Status |
|------|------|--------|
| 2.3.1 | Conditional Macros | open |
| 2.3.2 | Let Variants | open |
| 2.3.3 | Boolean Macros | open |
| 2.3.4 | Threading Macros | open |
| 2.3.5 | Iteration Macros | open |

#### Phase 2.4: Collection Functions

| Task | Name | Status |
|------|------|--------|
| 2.4.0 | Collection Constructors (`list`, `vector`, `hash-map`, `hash-set`) | open |
| 2.4.1 | Collection Predicates | open |
| 2.4.2 | Collection Transformation | open |
| 2.4.3 | Collection Analysis | open |
| 2.4.4 | Map Functions | open |
| 2.4.5 | Set Functions | open |

#### Phase 2.5: Protocol System

| Task | Name | Status |
|------|------|--------|
| 2.5.1 | Protocol Definition (`defprotocol`) | open |
| 2.5.2 | Type-Based Extension (`extend-type`) | open |
| 2.5.3 | Map-Based Extension (`extend`) | open |
| 2.5.4 | Protocol Predicates and Introspection | open |

#### Phase 2.6: String Functions

| Task | Name | Status |
|------|------|--------|
| 2.6.1 | String Basics | open |
| 2.6.2 | String Transformation | open |
| 2.6.3 | String Analysis | open |

#### Phase 2.7: Numeric Functions

| Task | Name | Status |
|------|------|--------|
| 2.7.1 | Numeric Operations | open |
| 2.7.2 | Math Functions | open |

#### Phase 2.8: I/O Functions

| Task | Name | Status |
|------|------|--------|
| 2.8.1 | Print Functions | open |
| 2.8.2 | Read Functions | open |

#### Phase 2.9: Process Functions

| Task | Name | Status |
|------|------|--------|
| 2.9.1 | Process Utilities | open |
| 2.9.2 | GenServer Pattern | open |
| 2.9.3 | Named Process Registry | open |
| 2.9.4 | Supervisor Behaviors | open |
| 2.9.5 | Atom Watches and Validators | open |

#### Phase 2.10: Error Handling

| Task | Name | Status |
|------|------|--------|
| 2.10.1 | Result Functions | open |
| 2.10.2 | Error Handling Macros | open |

#### Phase 2.11: Lazy Sequences

| Task | Name | Status |
|------|------|--------|
| 2.11.1 | LazySeq Type | open |
| 2.11.2 | Lazy Versions of Seq Functions | open |

#### Phase 2.12: Standard Library Tests

| Task | Name | Status |
|------|------|--------|
| 2.12.1 | Core Function Tests | open |
| 2.12.2 | String and Numeric Tests | open |
| 2.12.3 | Process and Result Tests | open |

#### Phase 2.13: Self-Hosting

| Task | Name | Status |
|------|------|--------|
| 2.13.1 | eval Function | open |
| 2.13.2 | load Function | open |
| 2.13.3 | Sorted Collection Subsequences | open |

---

### Milestone 3: UART Driver

#### Phase 3.1: UART Abstraction

| Task | Name | Status |
|------|------|--------|
| 3.1.1 | UART Protocol Definition | open |
| 3.1.2 | UART GenServer | open |

#### Phase 3.2: Platform Implementations

| Task | Name | Status |
|------|------|--------|
| 3.2.1 | ARM64 UART (PL011) | open |
| 3.2.2 | x86_64 UART (16550) | open |

#### Phase 3.3: Integration

| Task | Name | Status |
|------|------|--------|
| 3.3.1 | UART Driver Domain | open |
| 3.3.2 | UART Driver Tests | open |

---

### Milestone 4: Init System

#### Phase 4.1: Init Process

| Task | Name | Status |
|------|------|--------|
| 4.1.1 | Init Main Function | open |
| 4.1.2 | Platform Detection | open |

#### Phase 4.2: Driver Supervision

| Task | Name | Status |
|------|------|--------|
| 4.2.1 | Driver Supervisor | open |
| 4.2.2 | UART Initialization | open |

#### Phase 4.3: Rust Handoff

| Task | Name | Status |
|------|------|--------|
| 4.3.1 | Boot Handoff | open |

---

### Milestone 5: Lonala REPL

#### Phase 5.1: REPL Core

| Task | Name | Status |
|------|------|--------|
| 5.1.1 | REPL Main Loop | open |
| 5.1.2 | Line Editor | open |

#### Phase 5.2: REPL Features

| Task | Name | Status |
|------|------|--------|
| 5.2.1 | Error Handling | open |
| 5.2.2 | Multi-line Input | open |
| 5.2.3 | History | open |

#### Phase 5.3: Integration

| Task | Name | Status |
|------|------|--------|
| 5.3.1 | REPL Domain | open |
| 5.3.2 | Remove Rust REPL | open |

---

### Milestone 6: Block Storage

#### Phase 6.1: VirtIO Infrastructure

| Task | Name | Status |
|------|------|--------|
| 6.1.1 | VirtQueue Abstraction | open |
| 6.1.2 | VirtIO Common Layer | open |
| 6.1.3 | VirtIO Device Discovery | open |

#### Phase 6.2: Block Driver

| Task | Name | Status |
|------|------|--------|
| 6.2.1 | Block Device Protocol | open |
| 6.2.2 | VirtIO Block Implementation | open |

#### Phase 6.3: Integration

| Task | Name | Status |
|------|------|--------|
| 6.3.1 | Block Driver Domain | open |
| 6.3.2 | Block Driver Tests | open |

---

### Milestone 7: Filesystem

#### Phase 7.1: VFS Layer

| Task | Name | Status |
|------|------|--------|
| 7.1.1 | VFS Abstraction | open |
| 7.1.2 | File Handles | open |
| 7.1.3 | Path Resolution | open |

#### Phase 7.2: FAT Implementation

| Task | Name | Status |
|------|------|--------|
| 7.2.1 | FAT Structures | open |
| 7.2.2 | FAT File Operations | open |
| 7.2.3 | FAT Directory Operations | open |

#### Phase 7.3: Integration

| Task | Name | Status |
|------|------|--------|
| 7.3.1 | Filesystem Server | open |
| 7.3.2 | Mount System | open |
| 7.3.3 | Init Integration | open |
| 7.3.4 | Filesystem Tests | open |

---

### Milestone 8: Persistent Storage

#### Phase 8.1: Write Operations

| Task | Name | Status |
|------|------|--------|
| 8.1.1 | File Write Support | open |
| 8.1.2 | Directory Modification | open |

#### Phase 8.2: Durability

| Task | Name | Status |
|------|------|--------|
| 8.2.1 | Sync Operations | open |
| 8.2.2 | Persistence Tests | open |

---

### Milestone 9: Network Driver

#### Phase 9.1: Network Abstraction

| Task | Name | Status |
|------|------|--------|
| 9.1.1 | Network Device Protocol | open |
| 9.1.2 | Frame Buffer Management | open |

#### Phase 9.2: VirtIO Net

| Task | Name | Status |
|------|------|--------|
| 9.2.1 | VirtIO Net Implementation | open |
| 9.2.2 | Network Driver Domain | open |

#### Phase 9.3: Integration

| Task | Name | Status |
|------|------|--------|
| 9.3.1 | Network Driver Tests | open |

---

### Milestone 10: ARP

#### Phase 10.1: ARP Implementation

| Task | Name | Status |
|------|------|--------|
| 10.1.1 | ARP Table | open |
| 10.1.2 | ARP Protocol | open |
| 10.1.3 | ARP Server | open |
| 10.1.4 | ARP Tests | open |

---

### Milestone 11: IP Stack

#### Phase 11.1: IPv4

| Task | Name | Status |
|------|------|--------|
| 11.1.1 | IPv4 Packet Handling | open |
| 11.1.2 | IPv4 Routing | open |
| 11.1.3 | IPv4 Interface | open |

#### Phase 11.2: IPv6

| Task | Name | Status |
|------|------|--------|
| 11.2.1 | IPv6 Packet Handling | open |
| 11.2.2 | IPv6 Routing | open |

#### Phase 11.3: Integration

| Task | Name | Status |
|------|------|--------|
| 11.3.1 | IP Server | open |
| 11.3.2 | IP Tests | open |

---

### Milestone 12: Transport Protocols

#### Phase 12.1: ICMP

| Task | Name | Status |
|------|------|--------|
| 12.1.1 | ICMPv4 Implementation | open |
| 12.1.2 | ICMPv6 Implementation | open |

#### Phase 12.2: UDP

| Task | Name | Status |
|------|------|--------|
| 12.2.1 | UDP Protocol | open |
| 12.2.2 | UDP Sockets | open |

#### Phase 12.3: TCP

| Task | Name | Status |
|------|------|--------|
| 12.3.1 | TCP State Machine | open |
| 12.3.2 | TCP Connection Setup | open |
| 12.3.3 | TCP Data Transfer | open |
| 12.3.4 | TCP Flow Control | open |
| 12.3.5 | TCP Congestion Control | open |
| 12.3.6 | TCP Connection Teardown | open |

#### Phase 12.4: Socket API

| Task | Name | Status |
|------|------|--------|
| 12.4.1 | TCP Sockets | open |

#### Phase 12.5: Tests

| Task | Name | Status |
|------|------|--------|
| 12.5.1 | Transport Tests | open |

---

### Milestone 13: Telnet Server

#### Phase 13.1: Telnet Protocol

| Task | Name | Status |
|------|------|--------|
| 13.1.1 | Telnet Basics | open |

#### Phase 13.2: Telnet Server

| Task | Name | Status |
|------|------|--------|
| 13.2.1 | Connection Handler | open |
| 13.2.2 | REPL Integration | open |

#### Phase 13.3: Configuration

| Task | Name | Status |
|------|------|--------|
| 13.3.1 | Boot Configuration | open |
| 13.3.2 | Telnet Tests | open |

---

### Milestone 14: HTTP/1 Server

#### Phase 14.1: HTTP Protocol

| Task | Name | Status |
|------|------|--------|
| 14.1.1 | HTTP Request Parsing | open |
| 14.1.2 | HTTP Response Generation | open |

#### Phase 14.2: HTTP Server

| Task | Name | Status |
|------|------|--------|
| 14.2.1 | HTTP Server Core | open |
| 14.2.2 | Static File Serving | open |
| 14.2.3 | Error Handling | open |

#### Phase 14.3: Integration

| Task | Name | Status |
|------|------|--------|
| 14.3.1 | HTTP Configuration | open |
| 14.3.2 | HTTP Tests | open |

---

### Milestone 15: TLS

#### Phase 15.1: Cryptographic Primitives

| Task | Name | Status |
|------|------|--------|
| 15.1.1 | Hash Functions | open |
| 15.1.2 | Symmetric Encryption | open |
| 15.1.3 | Asymmetric Cryptography | open |

#### Phase 15.2: TLS Protocol

| Task | Name | Status |
|------|------|--------|
| 15.2.1 | TLS Record Layer | open |
| 15.2.2 | TLS Handshake - Client | open |
| 15.2.3 | TLS Handshake - Server | open |
| 15.2.4 | TLS 1.3 Support | open |

#### Phase 15.3: Certificate Management

| Task | Name | Status |
|------|------|--------|
| 15.3.1 | X.509 Parsing | open |
| 15.3.2 | Certificate Storage | open |

#### Phase 15.4: Tests

| Task | Name | Status |
|------|------|--------|
| 15.4.1 | Crypto Tests | open |
| 15.4.2 | TLS Tests | open |

---

### Milestone 16: HTTP/2 with ACME

#### Phase 16.1: HTTP/2 Protocol

| Task | Name | Status |
|------|------|--------|
| 16.1.1 | HTTP/2 Framing | open |
| 16.1.2 | HTTP/2 Streams | open |
| 16.1.3 | HPACK | open |

#### Phase 16.2: HTTP/2 Server

| Task | Name | Status |
|------|------|--------|
| 16.2.1 | HTTP/2 Server Core | open |
| 16.2.2 | HTTP/1-HTTP/2 Upgrade | open |

#### Phase 16.3: ACME

| Task | Name | Status |
|------|------|--------|
| 16.3.1 | ACME Client | open |
| 16.3.2 | Certificate Renewal | open |

#### Phase 16.4: Tests

| Task | Name | Status |
|------|------|--------|
| 16.4.1 | HTTP/2 Tests | open |
| 16.4.2 | ACME Tests | open |
