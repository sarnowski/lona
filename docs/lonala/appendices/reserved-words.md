# Appendix D: Reserved Words and Symbols
The following symbols have special meaning in Lonala:

## Special Forms

- `def`
- `defmacro`
- `let`
- `if`
- `do`
- `fn`
- `quote`
- `syntax-quote`
- `unquote`
- `unquote-splicing`

## Reserved for Future Use

- `loop`
- `recur`
- `ns`
- `require`
- `use`
- `import`
- `binding`
- `spawn`
- `send`
- `receive`

## Error Handling

- `panic!`
- `assert!`
- `with`
- `ok->`
- `ok->>`

## Boolean and Nil

- `true`
- `false`
- `nil`

## Reader Macro Characters

| Character | Purpose |
|-----------|---------|
| `'` | Quote |
| `` ` `` | Syntax-quote |
| `~` | Unquote |
| `~@` | Unquote-splicing |
| `^` | Metadata |
| `#` | Dispatch (various reader macros) |
| `@` | Deref |

## Naming Conventions

| Pattern | Meaning | Example |
|---------|---------|---------|
| `foo?` | Predicate (returns boolean) | `nil?`, `empty?` |
| `foo!` | Side-effecting or may panic | `swap!`, `reset!`, `panic!` |
| `*foo*` | Dynamic variable | `*ns*`, `*out*` |
| `foo->bar` | Conversion function | `str->int` |
| `-foo` | Private/internal | `-parse-impl` |

---

