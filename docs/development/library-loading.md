# Library Loading

Lonala namespaces are loaded from an embedded tar archive bundled into the root task ELF.

---

## Archive Format

The standard library is packaged as a USTAR tar archive (`lonalib.tar`) with no compression. Files are organized by namespace path:

```
lonalib.tar
├── lona/
│   ├── core.lona      # Bootstrap (loaded first)
│   ├── init.lona      # Init process
│   └── ...
└── ...
```

Namespace resolution follows Clojure conventions:

| Namespace | File Path |
|-----------|-----------|
| `lona.core` | `lona/core.lona` |
| `lona.process` | `lona/process.lona` |
| `my.app.server` | `my/app/server.lona` |

## Embedding

The archive is embedded at compile time using Rust's `include_bytes!` macro:

```rust
static LONALIB_TAR: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/lonalib.tar"));
```

This places the archive in the ELF's `.rodata` section, which seL4 maps read-only when loading the root task. Benefits:

- **Single artifact** — one ELF file to deploy
- **Cross-platform** — works identically on x86_64 and aarch64
- **Zero-copy** — archive bytes accessed directly from mapped memory

## Namespace Resolution

The `NamespaceSource` trait abstracts namespace lookup, enabling multiple backend implementations:

```rust
/// Abstraction for namespace file lookup.
pub trait NamespaceSource {
    /// Resolve a namespace to its source bytes.
    /// e.g., "lona.core" → contents of lona/core.lona
    fn resolve(&self, namespace: &str) -> Option<&[u8]>;
}
```

## TarSource

The `TarSource` struct wraps the embedded archive and implements `NamespaceSource`:

```rust
pub struct TarSource<'a> {
    archive: TarArchiveRef<'a>,
}

impl TarSource<'static> {
    /// Create from embedded archive
    pub fn embedded() -> Result<Self, TarSourceError>;
}

impl<'a> TarSource<'a> {
    /// Iterate over all entries
    pub fn entries(&self) -> impl Iterator<Item = ArchiveEntry<'a>>;
}

impl NamespaceSource for TarSource<'_> {
    fn resolve(&self, namespace: &str) -> Option<&[u8]>;
}
```

Each `ArchiveEntry` provides:
- `filename()` — returns the file path
- `data()` — returns the file contents as `&[u8]`

## ChainedSource

The `ChainedSource` struct combines multiple sources with priority ordering (first match wins):

```rust
pub struct ChainedSource<'a> {
    sources: &'a [&'a dyn NamespaceSource],
}

impl<'a> ChainedSource<'a> {
    /// Create from a slice of sources (first match wins)
    pub const fn new(sources: &'a [&'a dyn NamespaceSource]) -> Self;
}

impl NamespaceSource for ChainedSource<'_> {
    fn resolve(&self, namespace: &str) -> Option<&[u8]>;
}
```

This enables patterns like local filesystem overriding embedded sources during development.

## Build Process

1. `build.rs` creates `lonalib.tar` from `lib/` directory using USTAR format
2. Cargo embeds the tar via `include_bytes!`
3. Root task initializes `TarSource` at boot and lists embedded files
