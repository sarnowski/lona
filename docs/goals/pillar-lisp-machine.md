# Pillar III: LISP Machine — The Living System

> *"The Inspectable Machine"*

## Why LISP Machine Philosophy?

Modern operating systems are opaque. Source code is separate from the running system. Debugging requires special tools, core dumps, and restarts. Modifying behavior requires recompilation, redeployment, downtime.

The LISP machines of the 1980s (Symbolics, MIT CADR, TI Explorer) took a radically different approach. The running system was a living, malleable environment:

- Every function could be inspected and disassembled
- Every value could be examined at runtime
- Code could be modified while the system ran
- The debugger was integrated into the development experience

These machines were discontinued for economic reasons, not technical ones. Their philosophy remains sound.

Lona revives this philosophy for modern systems programming:

1. **Source is canonical**: No binaries, no bytecode-only distribution
2. **Everything is introspectable**: Every value, function, and process
3. **Live modification**: Change running code without restarts
4. **Interactive debugging**: Pause, inspect, modify, continue

---

## Philosophy: The Inspectable Machine

Traditional systems draw a hard line between "development" and "production":

```
Development          Production
┌──────────┐        ┌──────────┐
│ Source   │ ──────→│ Binary   │
│ Debugger │        │ Logs     │
│ REPL     │        │ Metrics  │
└──────────┘        └──────────┘
```

When production breaks, you can't directly inspect the running system. You collect logs, try to reproduce locally, guess at causes.

LISP machine philosophy erases this distinction:

```
The Living System
┌─────────────────────────────────┐
│ Source = Running Code           │
│ Debugger Always Available       │
│ REPL Is The Shell               │
│ Modify Without Restart          │
└─────────────────────────────────┘
```

The system you develop on is the same as the system in production. The tools you use for development work identically on a running server.

---

## What LISP Machine Philosophy Forces in Lona

### 1. Source-Only Distribution

There is no ahead-of-time compilation. There are no pre-compiled binaries. The only way to get code into Lona is to load source files.

```
Traditional:                      Lona:
┌─────────────┐                  ┌─────────────┐
│ Source      │                  │ Source      │
└──────┬──────┘                  └──────┬──────┘
       │ compile                        │
       ▼                                │ load
┌─────────────┐                         │
│ Binary      │                         │
└──────┬──────┘                         │
       │ deploy                         ▼
       ▼                         ┌─────────────┐
┌─────────────┐                  │ Lona        │
│ Target      │                  │ compiles    │
│ Machine     │                  │ and caches  │
└─────────────┘                  └─────────────┘
```

The compiler is part of Lona, not a separate tool. Loading source:
1. Parses the source
2. Compiles to bytecode
3. Caches bytecode for fast subsequent loads
4. Source is always retained for introspection

### Why Source-Only?

| Benefit | Explanation |
|---------|-------------|
| **Total transparency** | You can always see exactly what code is running |
| **Full debugging** | Source is always available—no "missing symbols" |
| **Platform independence** | Same source runs on ARM, x86, any architecture |
| **Security auditing** | Inspect any code before loading |
| **Hot patching** | Modify source, reload—natural workflow |
| **Reproducibility** | Same source produces same behavior |

### 2. Per-Definition Storage

Source code is stored **per-definition**, not just as files. Each function, macro, or value has:

| Component | Description |
|-----------|-------------|
| **Source text** | The raw source code (preserves formatting, comments) |
| **Provenance** | Where this definition came from (file, REPL, network) |
| **Compiled form** | Bytecode compiled from the source |

```clojure
;; Query provenance of any definition
(provenance tcp/handle-packet)
;; => {:origin :file
;;     :file "tcp.lona"
;;     :line 142
;;     :timestamp #inst "2025-12-15T10:00:00"}

;; After REPL modification
(provenance tcp/handle-packet)
;; => {:origin :repl
;;     :session "session-42"
;;     :timestamp #inst "2025-12-15T14:30:00"
;;     :previous {:origin :file :file "tcp.lona" :line 142}}
```

### 3. Hot-Patching

Lonala uses **late binding**: function calls are resolved through a dispatch table at runtime, not compiled to direct jumps.

```
Function call: (process-packet pkt)
       │
       ▼
Dispatch table: process-packet → <bytecode-ptr>
       │
       ▼
Execute bytecode
```

When you redefine a function:
1. New bytecode is compiled
2. Dispatch table is updated to point to new bytecode
3. All future calls use the new implementation
4. No recompilation of callers needed

```clojure
;; Production system has a bug
(source net/checksum)
;; => (defn checksum [data]
;;      (reduce + data))  ; Bug: doesn't handle overflow

;; Fix it live
(defn net/checksum [data]
  (reduce #(bit-and (+ %1 %2) 0xFFFF) 0 data))

;; All future packets use the fixed checksum
;; No restart, no downtime
```

### 4. The REPL as Primary Interface

The Read-Eval-Print Loop is not a development convenience—it's the primary system interface:

```clojure
lona> (list-processes)
#{{:pid 1 :name init :domain "root" :status :waiting}
  {:pid 2 :name uart-main :domain "driver:uart" :status :waiting}
  {:pid 3 :name repl :domain "user:admin" :status :running}
  ...}

lona> (domain-info "driver:net")
{:name "driver:net"
 :parent "drivers"
 :capabilities #{:nic-device :nic-irq :packet-buffer}
 :memory-used 2458624
 :processes [{:pid 4 :name rx-handler} {:pid 5 :name tx-handler}]}

lona> (source net/handle-rx)
(defn handle-rx [packet]
  (let [header (parse-header packet)]
    (route-packet header packet)))
```

System administration, debugging, and development all happen through the same interface.

---

## Two-Mode Architecture

LISP machines were interactive—when errors occurred, you could inspect and fix them. But production servers must self-heal without human intervention.

Lona reconciles these requirements with a **Two-Mode Architecture**:

| Mode | Trigger | Error Behavior | Use Case |
|------|---------|----------------|----------|
| **Production** | Default | Crash, supervisor restarts | Servers |
| **Debug** | Debugger attached | Pause, user inspects | Troubleshooting |

### Production Mode (Default)

Errors follow BEAM/OTP "let it crash" philosophy:
- Process crashes immediately
- Supervisor detects and restarts
- System self-heals

### Debug Mode (Debugger Attached)

Errors pause execution instead of crashing:
- Full stack preserved
- Locals accessible
- Multiple restart options available
- User chooses how to proceed

```clojure
;; Attach debugger to running process
(debug-attach pid)

;; Now errors pause instead of crashing
;; When error occurs:

╭─ PROCESS BREAK: :worker-1 (pid=1042) ─────────────────────────╮
│ Division by zero                                               │
│ Process paused. Other processes continue running.              │
╰────────────────────────────────────────────────────────────────╯

Restarts:
  [1] :abort      - Crash, trigger supervisor restart
  [2] :use-value  - Return a substitute value
  [3] :retry      - Retry with different arguments

proc-debug[0]> l                    ; show locals
╭─ Locals in frame 0 ───────────────────────────────────────────╮
│ a = 10                                                         │
│ b = 0                                                          │
╰────────────────────────────────────────────────────────────────╯

proc-debug[0]> (set! b 2)           ; fix the bug
2

proc-debug[0]> 3                    ; retry
5                                   ; 10 / 2 = 5, process continues
```

This gives you:
- **Production resilience**: Unattended systems self-heal
- **Development power**: Full debugger when you need it
- **Per-process granularity**: Debug one process while others run

---

## Condition/Restart System

Inspired by Common Lisp, Lonala provides conditions and restarts that separate error detection from error handling:

### Traditional Exceptions

```
Error occurs → Stack unwinds → Context lost → Handler runs
```

### Conditions

```
Error occurs → Condition signaled → Handler inspects → Restart chosen → Execution continues
```

The stack is **not unwound** until a recovery strategy is chosen. The full context remains available.

```clojure
;; Low-level code signals conditions and provides restarts
(defn read-config [path]
  (restart-case
    (if (file-exists? path)
      (parse-config (slurp path))
      (signal :file-not-found {:path path}))

    (:retry []
      "Try reading the file again"
      (read-config path))

    (:use-default []
      "Use default configuration"
      default-config)

    (:use-value [config]
      "Provide a configuration value"
      config)))

;; High-level code decides how to handle
(handler-bind
  [:file-not-found
   (fn [c]
     (if (= (:path c) "/etc/critical.conf")
       (invoke-restart :abort)
       (invoke-restart :use-default)))]

  (start-application))
```

---

## Runtime Introspection

Everything in the running system is inspectable:

### Process Inspection

```clojure
(process-info pid)
;; => {:pid 42
;;     :name worker-3
;;     :domain "services"
;;     :function worker/main-loop
;;     :status :waiting
;;     :heap-size 8192
;;     :message-queue-len 2}

(process-messages pid)
;; => [{:from pid-23 :msg {:request :status}}
;;     {:from pid-47 :msg {:data [1 2 3]}}]

(process-backtrace pid)
;; => [{:function worker/main-loop :line 42}
;;     {:function worker/handle-request :line 15}]
```

### Function Inspection

```clojure
(source some-function)      ; View source code
(disassemble some-function) ; View bytecode
(provenance some-function)  ; Where did this definition come from?
```

### Tracing

```clojure
;; Trace function calls
(trace-calls 'tcp/handle-packet {:args true :return true})
;; >> [12:34:56.789] tcp/handle-packet called with: [{:src ...}]
;; << [12:34:56.791] tcp/handle-packet returned: :ok

;; Trace messages to a process
(trace-messages pid {:send true :receive true})
```

---

## Implications for Lona Design

LISP machine philosophy shapes these Lona design decisions:

| Decision | Driven By |
|----------|-----------|
| Source-only distribution | Complete transparency |
| Per-definition storage | Hot-patching, provenance |
| Late binding | Live modification without restart |
| REPL as shell | Interactive system administration |
| Two-Mode debugging | Production resilience + development power |
| Condition system | Non-destructive error handling |

---

## Summary

LISP Machine philosophy provides Lona with:

| Guarantee | Mechanism |
|-----------|-----------|
| **Total transparency** | Source-only distribution |
| **Live modification** | Late binding, hot-patching |
| **Interactive debugging** | Two-Mode architecture |
| **Error recovery** | Condition/restart system |
| **System administration** | REPL as primary interface |

**The Bottom Line**: In Lona, the running system is not a black box. Every piece of code can be inspected, every value examined, every function modified. Development and operations use the same tools on the same system.

---

## Further Reading

- [Core Concepts: Dispatch Table](core-concepts.md#dispatch-table)
- [Core Concepts: Condition/Restart](core-concepts.md#conditionrestart)
- [System Design: Hot-Patching Mechanics](system-design.md#hot-patching-mechanics)
- [System Design: Two-Mode Architecture](system-design.md#two-mode-architecture)
- [Symbolics LISP Machine](https://en.wikipedia.org/wiki/Symbolics)
