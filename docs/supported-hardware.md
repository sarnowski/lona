# Supported Hardware

This document lists the hardware platforms and configurations supported by Lona.

---

## Supported Architectures

| Architecture | Status | Notes |
|--------------|--------|-------|
| **x86_64** | Supported | Primary development target |
| **aarch64** | Supported | ARM 64-bit |

---

## Hardware Requirements

### Mandatory Requirements

These features are required for Lona to run:

| Feature | x86_64 | aarch64 | Purpose |
|---------|--------|---------|---------|
| **64-bit CPU** | Required | Required | Address space layout |
| **MMU** | Required | Required | VSpace isolation (realm security) |
| **Timer** | APIC | GIC | MCS scheduling, preemption |

### Security-Critical Features

These features are required for full security isolation:

| Feature | x86_64 | aarch64 | Purpose |
|---------|--------|---------|---------|
| **IOMMU** | Intel VT-d | ARM SMMU | DMA isolation for drivers |

See [Hardware Requirements in Architecture Overview](architecture/index.md#hardware-requirements) for details on IOMMU requirements.

---

## Platform Support Matrix

### Development Platforms

| Platform | Architecture | IOMMU | Driver Isolation | Status |
|----------|--------------|-------|------------------|--------|
| **QEMU virt (aarch64)** | aarch64 | virtio-iommu | Full | Primary dev target |
| **QEMU q35 (x86_64)** | x86_64 | Intel VT-d emulation | Full | Primary dev target |

QEMU configuration for full IOMMU support:

```bash
# x86_64 with IOMMU
qemu-system-x86_64 -machine q35 -device intel-iommu,intremap=on ...

# aarch64 with IOMMU
qemu-system-aarch64 -machine virt -device virtio-iommu-pci ...
```

### Server Platforms

| Platform | Architecture | IOMMU | Driver Isolation | Status |
|----------|--------------|-------|------------------|--------|
| **Servers with VT-d** | x86_64 | Intel VT-d | Full | Supported |
| **Servers with SMMU** | aarch64 | ARM SMMU | Full | Supported |
| **Cloud VMs** | Varies | Usually not exposed | Trusted drivers only | Limited |

### Embedded Platforms

| Platform | Architecture | IOMMU | Driver Isolation | Status |
|----------|--------------|-------|------------------|--------|
| **Raspberry Pi 4** | aarch64 | None | Trusted drivers only | Limited |
| **Raspberry Pi 5** | aarch64 | Non-standard* | Trusted drivers only | Limited |

*Raspberry Pi 5 has custom Broadcom IOMMUs that are not ARM SMMU-compatible.

---

## IOMMU and Security

### With IOMMU (Full Security)

When IOMMU is present and enabled:

- Driver realms are **fully isolated**
- DMA is restricted to allocated regions only
- A compromised driver **cannot** access other realms' memory
- Drivers can be treated as **untrusted code**

### Without IOMMU (Reduced Security)

When IOMMU is unavailable:

- Driver realms are **trusted** (part of TCB)
- DMA can access any physical memory
- A compromised driver **can** access any memory in the system
- Only run **audited, trusted driver code**

At boot, Lona detects IOMMU availability and logs the security status:

- With IOMMU: `IOMMU enabled, DMA isolation active`
- Without IOMMU: `WARNING: No IOMMU detected. Driver realms are TRUSTED. DMA isolation disabled.`

---

## seL4 Platform Requirements

Lona inherits seL4's platform requirements. Key considerations:

- **MCS scheduling**: Lona uses seL4's MCS (Mixed Criticality Scheduling) configuration
- **Hypervisor mode**: Not required for Lona
- **Formal verification**: seL4's formal verification applies only to specific single-core configurations; Lona's multi-core MCS configuration is not formally verified

See the [seL4 Supported Platforms](https://docs.sel4.systems/Hardware/) for detailed seL4 hardware support.

---

## Future Platforms

The following platforms are under consideration for future support:

| Platform | Architecture | Notes |
|----------|--------------|-------|
| RISC-V | riscv64 | Pending seL4 RISC-V maturity |
