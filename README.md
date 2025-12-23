# Lona

**LISP Machines Never Died. They Evolved.**

Lona is an operating system for developers who want full transparency and control over their computing stack. It unifies the strong security of the **seL4 microkernel** with the introspective power of a **LISP machine** and the fault-tolerant concurrency of **Erlang/OTP**. Programmed entirely in **Lonala**, a Clojure-inspired language, Lona lets you inspect, debug, and live-patch every layer of the system—from drivers to applications—without reboots, without opaque binaries, and without sacrificing security.

**[Documentation](https://lona.systems/)** · **[Goals](https://lona.systems/goals/)** · **[Installation](https://lona.systems/installation/)**

---

## Quickstart

> [!WARNING]
> This project is in early development — interfaces may change. Read the [Roadmap](https://lona.systems/roadmap/) for current status.

Requires Docker and GNU Make 4.0+ (on macOS: `brew install make`, use `gmake`).

```bash
# Build development environment (first time only)
make docker

# Run full test suite (format, lint, unit tests, integration tests)
make test

# Boot Lona in QEMU
make run-aarch64    # ARM64
make run-x86_64     # x86_64

# Serve documentation locally
make docs-local
```

---

## License

Lona is free software under the [GNU General Public License v3](docs/license.md).

Copyright © 2025 Tobias Sarnowski.
