# Lona

**See Everything. Change Anything.**

Lona is a general-purpose operating system that brings together three powerful paradigms: the **seL4 microkernel** for formally verified, capability-based security; the **LISP machine philosophy** for complete runtime introspection and hot-patching; and the **Erlang/OTP concurrency model** for massive concurrency, fault tolerance, and the "let it crash" philosophy. The result is an operating system where users have complete visibility and control over every aspect of the running system, where failures are contained and automatically recoverable, and where the full power of modern concurrent programming is available at every level of the stack.

## Prerequisites

- **Docker** - Required for the build environment
- **GNU Make 4.0+** - On macOS, install with `brew install make` and use `gmake` instead of `make`

## Quickstart

```bash
# Build the Docker development environment (first time only)
make docker-build

# Build and verify Rust code quality (runs fmt + clippy)
make build

# Build the complete bootable image
make image

# Run in QEMU
make run

# Clean build artifacts (for a fresh build)
make clean
```

## Documentation

See the [docs/](docs/) directory for detailed documentation, including:

- [Goals and Design](docs/goals.md) - Detailed technical vision and architecture
- [License](docs/license.md) - Full license text

## License

Copyright (C) 2025 Tobias Sarnowski

This program is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.

This program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details.

You should have received a copy of the GNU General Public License along with this program. If not, see <https://www.gnu.org/licenses/>.
