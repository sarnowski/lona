# Appendix A: Grammar
This appendix provides a formal grammar for Lonala in EBNF notation.

```ebnf
(* Top-level *)
program     = form* ;
form        = literal | symbol | list | vector | map | reader-macro ;

(* Literals *)
literal     = nil | boolean | number | string | keyword ;
nil         = "nil" ;
boolean     = "true" | "false" ;
number      = integer | float | ratio ;
integer     = decimal-int | hex-int | binary-int | octal-int ;
decimal-int = ["-"] digit+ ;
hex-int     = "0" ("x" | "X") hex-digit+ ;
binary-int  = "0" ("b" | "B") ("0" | "1")+ ;
octal-int   = "0" ("o" | "O") octal-digit+ ;
float       = ["-"] digit+ "." digit+ [exponent]
            | ["-"] digit+ exponent
            | "##Inf" | "##-Inf" | "##NaN" ;
exponent    = ("e" | "E") ["+" | "-"] digit+ ;
ratio       = ["-"] digit+ "/" digit+ ;
string      = '"' string-char* '"' ;
string-char = escape-seq | (any char except '"' and '\') ;
escape-seq  = "\\" | '\"' | "\n" | "\r" | "\t" ;
keyword     = ":" symbol-name ;

(* Symbols *)
symbol      = symbol-name | qualified-symbol ;
symbol-name = symbol-start symbol-char* ;
qualified-symbol = symbol-name "/" symbol-name ;
symbol-start = letter | special-char ;
symbol-char = letter | digit | special-char ;
special-char = "*" | "+" | "!" | "-" | "_" | "'" | "?" | "<" | ">" | "=" ;
letter      = "a".."z" | "A".."Z" ;
digit       = "0".."9" ;
hex-digit   = digit | "a".."f" | "A".."F" ;
octal-digit = "0".."7" ;

(* Collections *)
list        = "(" form* ")" ;
vector      = "[" form* "]" ;
map         = "{" (form form)* "}" ;

(* Reader macros *)
reader-macro = quote | syntax-quote | unquote | unquote-splice ;
quote         = "'" form ;
syntax-quote  = "`" form ;
unquote       = "~" form ;
unquote-splice = "~@" form ;

(* Whitespace and comments *)
whitespace  = " " | "\t" | "\n" | "\r" | "," ;
comment     = ";" (any char except newline)* ;
```

---

