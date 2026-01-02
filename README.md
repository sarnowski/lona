# Lona

A capability-secure operating system built on seL4, combining BEAM-style lightweight processes with a Clojure-inspired LISP dialect.

## Quick Start

### Prerequisites

- Rust nightly toolchain
- Docker (for cross-compilation and OS image builds)
- cargo-llvm-cov (`cargo install cargo-llvm-cov`)

### Build OS Images

```bash
make x86_64         # Build x86_64 release image
make aarch64        # Build aarch64 release image
```

Images are output to `dist/<arch>/` ready for deployment.

### Run in QEMU

```bash
make run-x86_64     # Run x86_64 image in QEMU
make run-aarch64    # Run aarch64 image in QEMU
```

### Verify

```bash
make verify         # Run all checks (format, clippy, test, integration-test)
```

### Clean

```bash
make clean          # Remove all build artifacts
```

## All Make Targets

| Target | Description |
|--------|-------------|
| `format` | Check code formatting |
| `clippy` | Run lints |
| `test` | Run tests with 60% coverage requirement |
| `env` | Build Docker build environment |
| **`x86_64`** | Build x86_64 release image |
| **`aarch64`** | Build aarch64 release image |
| `x86_64-debug` | Build x86_64 debug image |
| `aarch64-debug` | Build aarch64 debug image |
| `run-x86_64` | Run x86_64 in QEMU |
| `run-aarch64` | Run aarch64 in QEMU |
| `x86_64-test` | Build and run E2E tests for x86_64 |
| `aarch64-test` | Build and run E2E tests for aarch64 |
| `integration-test` | Run E2E tests for all architectures |
| **`verify`** | Run all checks including integration tests |
| **`clean`** | Remove all build artifacts |

## Documentation

See `docs/` for detailed specifications:
- `concept.md` - System architecture
- `lonala.md` - Language specification
- `lonala-process.md` - Process primitives
- `lonala-kernel.md` - seL4 operations
- `lonala-io.md` - Device driver primitives
