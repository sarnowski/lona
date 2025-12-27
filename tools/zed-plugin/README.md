# Lonala Zed Extension

Provides Lonala language support for Zed editor.

## Features

- Syntax highlighting via Tree-sitter
- Bracket matching and auto-closing
- Lisp-style auto-indentation
- Code outline (def, defn, defmacro forms)
- LSP integration for semantic tokens

## Installation

### Prerequisites

1. Install the LSP server:
   ```bash
   cargo install --path crates/lonala-lsp
   ```

2. Ensure `lonala-lsp` is in your PATH.

### Install Extension

#### As Dev Extension (for development)
1. Build the extension: `make zed-plugin`
2. In Zed: Extensions > Install Dev Extension
3. Select the `tools/zed-plugin` directory

#### From Extension Store (future)
Coming soon - extension will be published to Zed's extension store.

## Troubleshooting

**"lonala-lsp not found in PATH"**

Ensure the LSP binary is installed and accessible:
```bash
which lonala-lsp
```

If not found, install it:
```bash
cargo install --path crates/lonala-lsp
```
