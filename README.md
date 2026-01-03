# Lona

A capability-secure operating system built on seL4, combining BEAM-style lightweight processes with a Clojure-inspired LISP dialect.

## Quick Start

### Prerequisites

- Docker (for cross-compilation and OS image builds)
- Make
- Python 3.11+ (for MCP development tools)

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
| `run-x86_64` | Run x86_64 in QEMU |
| `run-aarch64` | Run aarch64 in QEMU |
| `x86_64-test` | Build and run E2E tests for x86_64 |
| `aarch64-test` | Build and run E2E tests for aarch64 |
| `integration-test` | Run E2E tests for all architectures |
| **`verify`** | Run all checks including integration tests |
| `venv` | Create Python virtual environment for MCP tools |
| `mcp` | Run MCP development REPL server |
| **`clean`** | Remove Rust build cache |

## Documentation

See `docs/` for detailed specifications:
- `concept.md` - System architecture
- `lonala.md` - Language specification
- `lonala-process.md` - Process primitives
- `lonala-kernel.md` - seL4 operations
- `lonala-io.md` - Device driver primitives

## License

Copyright 2026 Tobias Sarnowski

This project is licensed under the GNU General Public License v3.0 or later — see [docs/license.md](docs/license.md) for details.
