# Services

This document covers inter-realm communication through the service model: how realms expose services, establish connections, and communicate directly.

---

## Overview

Realms communicate through **services**. A service is a named endpoint that other realms can connect to. The Lona Memory Manager maintains a service registry and controls access through capabilities.

Key properties:

- **Capability-based access**: A realm can only connect to services it has been granted access to
- **Direct IPC after setup**: Once connected, realms communicate directly via seL4 IPC (Memory Manager not in the path)
- **Policy at connection time**: Access control is enforced when establishing connections, not on every message

```
SERVICE MODEL OVERVIEW
════════════════════════════════════════════════════════════════════════

┌─────────────────────────────────────────────────────────────────────┐
│                        MEMORY MANAGER                               │
│                                                                     │
│  Service Registry:                                                  │
│  ┌───────────────────────────────────────────────────────────────┐  │
│  │  "database"  → Realm B endpoint  (access: [Realm A, Realm C]) │  │
│  │  "logger"    → Realm C endpoint  (access: [all children])     │  │
│  │  "network"   → Realm D endpoint  (access: [Realm A])          │  │
│  └───────────────────────────────────────────────────────────────┘  │
│                                                                     │
│  Handles: registration, connection requests, access control        │
└─────────────────────────────────────────────────────────────────────┘
                              │
            ┌─────────────────┼─────────────────┐
            │                 │                 │
            ▼                 ▼                 ▼
     ┌───────────┐     ┌───────────┐     ┌───────────┐
     │  Realm A  │     │  Realm B  │     │  Realm C  │
     │           │     │ (database)│     │ (logger)  │
     │  Client   │════▶│  Service  │     │  Service  │
     └───────────┘     └───────────┘     └───────────┘
            direct IPC
         (after setup)
```

---

## Service Lifecycle

### 1. Registration

A realm registers a service with the Memory Manager:

```
Realm B                              Memory Manager
   │                                       │
   │  Register "database"                  │
   ├──────────────────────────────────────▶│
   │                                       │  Records:
   │                                       │    name: "database"
   │                                       │    endpoint: Realm B
   │                                       │    access policy: (from config)
   │  OK                                   │
   │◀──────────────────────────────────────┤
```

### 2. Connection

A client realm requests a connection:

```
Realm A                              Memory Manager
   │                                       │
   │  Connect to "database"                │
   ├──────────────────────────────────────▶│
   │                                       │
   │                                       │  Policy check:
   │                                       │  - Is Realm A allowed?
   │                                       │  - Check grants, hierarchy
   │                                       │
   │                                       │  If allowed:
   │                                       │  - Mint Send cap to B's endpoint
   │                                       │
   │  Send capability to "database"        │
   │◀──────────────────────────────────────┤
```

### 3. Direct Communication

Once connected, realms communicate directly:

```
Realm A                                                         Realm B
   │                                                               │
   │              seL4 IPC (using granted Send cap)                │
   ├──────────────────────────────────────────────────────────────▶│
   │                                                               │
   │                          Reply                                │
   │◀──────────────────────────────────────────────────────────────┤
   │                                                               │

Memory Manager is NOT in the communication path.
Full seL4 IPC performance.
```

---

## Access Control

Access to services is controlled through capabilities. A realm can only connect to a service if granted permission.

### Grant Mechanisms

| Mechanism | Description |
|-----------|-------------|
| **Parent grants** | Parent explicitly grants child access to specific services |
| **Init configuration** | System startup config defines initial service access |
| **Hierarchy rules** | Children can communicate with parent; siblings need explicit grant |
| **Service policy** | Service registration can specify allowed clients |

### Revocation

Access can be revoked:

- Parent revokes child's capability
- Memory Manager invalidates connection (e.g., service realm terminated)
- seL4 capability revocation propagates to all derived caps

---

## Process-to-Process Addressing

Within a realm, processes are addressed by PID. Across realms, addressing uses the service connection:

```
CROSS-REALM PROCESS ADDRESSING
════════════════════════════════════════════════════════════════════════

Realm A                              Realm B ("database" service)
┌─────────────────────┐              ┌─────────────────────┐
│                     │              │                     │
│  Process 1          │              │  Process 10 (main)  │
│      │              │              │      │              │
│      │ send to      │   service    │      │              │
│      │ "database"   │─────────────▶│      ▼              │
│      │              │   connection │  Router dispatches  │
│      ▼              │              │  to handler process │
│                     │              │                     │
└─────────────────────┘              └─────────────────────┘

The service realm decides how to route incoming messages to its processes.
```

---

## Comparison with Intra-Realm Messaging

| Aspect | Intra-Realm | Inter-Realm (Services) |
|--------|-------------|------------------------|
| **Addressing** | PID | Service name + connection |
| **Message format** | Deep copy | Serialized (IPC buffer) |
| **Latency** | ~100-500 ns | ~1-10 µs |
| **Access control** | Same realm = same trust | Capability required |
| **Setup** | None (same realm) | Registration + connection |

---

## Summary

| Aspect | Description |
|--------|-------------|
| **Registration** | Realms register named services with Memory Manager |
| **Connection** | Clients request connections; MM enforces access policy |
| **Communication** | Direct seL4 IPC after setup (no broker) |
| **Access control** | Capability-based; grants from parent, config, or service policy |
| **Revocation** | Capabilities can be revoked to cut off access |
