# Tree-sitter Grammar for Lonala

A [Tree-sitter](https://tree-sitter.github.io/) grammar for **Lonala**, the programming language for the [Lona operating system](https://codeberg.org/sarnowski/lona).

## Features

- Syntax highlighting for Lonala code
- Support for literal types (numbers, strings, keywords, symbols)
- Collection literals (lists, vectors, maps, sets)
- Reader macros (quote, syntax-quote, unquote, metadata, etc.)
- Special form and macro recognition for enhanced highlighting
- Anonymous function parameters (`%`, `%1`, `%2`, `%&`)

## Building

From the repository root:

```bash
make tree-sitter
```

Or manually:

```bash
cd tools/tree-sitter-lonala
npm install
npm run build
npm run test
```

## Editor Integration

### Zed

Zed has built-in Tree-sitter support. To use this grammar:

1. Build the grammar:
   ```bash
   make tree-sitter
   ```

2. Add to your Zed configuration (`~/.config/zed/settings.json`):
   ```json
   {
     "languages": {
       "Lonala": {
         "tab_size": 2
       }
     }
   }
   ```

3. Install the grammar by copying to Zed's grammars directory:
   ```bash
   mkdir -p ~/.config/zed/grammars
   cp -r tools/tree-sitter-lonala ~/.config/zed/grammars/tree-sitter-lonala
   ```

4. Create a language configuration at `~/.config/zed/languages/lonala/config.toml`:
   ```toml
   name = "Lonala"
   grammar = "lonala"
   path_suffixes = ["lona"]
   line_comments = ["; "]
   ```

*Note: These instructions are for manual setup. Zed extension support is planned for a future milestone, which will simplify installation.*

### Neovim

Neovim uses [nvim-treesitter](https://github.com/nvim-treesitter/nvim-treesitter) for Tree-sitter integration.

1. Build the grammar:
   ```bash
   make tree-sitter
   ```

2. Add the parser to nvim-treesitter. In your Neovim configuration:
   ```lua
   local parser_config = require("nvim-treesitter.parsers").get_parser_configs()

   parser_config.lonala = {
     install_info = {
       url = "~/Projects/lona/tools/tree-sitter-lonala",  -- adjust path
       files = {"src/parser.c"},
       branch = "main",
     },
     filetype = "lona",
   }

   vim.filetype.add({
     extension = {
       lona = "lona",
     },
   })
   ```

3. Install the parser:
   ```vim
   :TSInstall lonala
   ```

4. Copy highlight queries to Neovim's runtime:
   ```bash
   mkdir -p ~/.config/nvim/queries/lonala
   cp tools/tree-sitter-lonala/queries/highlights.scm ~/.config/nvim/queries/lonala/
   ```

### Other Editors

For editors with Tree-sitter support, point them to the generated parser in:
- Parser: `tools/tree-sitter-lonala/src/parser.c`
- Highlights: `tools/tree-sitter-lonala/queries/highlights.scm`

## Usage

### Parsing a file

```bash
npx tree-sitter parse path/to/file.lona
```

### Highlighting a file

```bash
npx tree-sitter highlight path/to/file.lona
```

### Running tests

```bash
npm run test
```

Or from repository root:

```bash
make tree-sitter-test
```

## Grammar Structure

The grammar recognizes:

| Node Type | Description | Example |
|-----------|-------------|---------|
| `number` | All numeric literals | `42`, `-3.14`, `0xFF`, `22/7` |
| `string` | String literals | `"hello"`, `"line\nbreak"` |
| `symbol` | Identifiers | `foo`, `my-fn`, `ns/name` |
| `keyword` | Keywords | `:foo`, `::bar`, `:ns/key` |
| `boolean` | Boolean values | `true`, `false` |
| `nil` | Nil value | `nil` |
| `list` | Parenthesized lists | `(+ 1 2)` |
| `vector` | Bracketed vectors | `[1 2 3]` |
| `map` | Braced maps | `{:a 1 :b 2}` |
| `set` | Hash-braced sets | `#{1 2 3}` |
| `quote` | Quote reader macro | `'x` |
| `syntax_quote` | Syntax-quote | `` `x `` |
| `unquote` | Unquote | `~x` |
| `unquote_splice` | Unquote-splice | `~@xs` |
| `metadata` | Metadata annotation | `^:private foo` |
| `var_quote` | Var quote | `#'my-var` |
| `discard` | Discard reader macro | `#_ignored` |
| `anon_fn` | Anonymous function | `#(+ % 1)` |
| `comment` | Line comments | `; comment` |

## Contributing

1. Make changes to `grammar.js`
2. Regenerate the parser: `npm run build`
3. Run tests: `npm run test`
4. Add test cases to `test/corpus/` for new features

### Test Format

Tests use Tree-sitter's test format:

```
================================================================================
Test Name
================================================================================

source code here

--------------------------------------------------------------------------------

(expected_tree_structure)
```

## License

GPL-3.0-or-later
