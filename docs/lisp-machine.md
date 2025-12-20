# The LISP Machine Heritage

This document describes Lona's vision for interactive system development, drawing from the LISP machine tradition and adapting it for a modern capability-based microkernel environment.

## Historical Context

### What Were LISP Machines?

LISP machines were specialized computers developed at MIT's AI Lab starting in 1973 and commercialized by Symbolics, Lisp Machines Inc., and Texas Instruments throughout the 1980s. Unlike conventional computers that ran operating systems written in low-level languages, LISP machines ran systems written entirely in Lisp—from device drivers to the garbage collector to the window manager.

The defining characteristic was not merely that they ran Lisp, but that **everything was a live object**. The boundary between "the program" and "the system" dissolved. Users didn't operate a computer; they inhabited a world of interconnected objects that could be inspected, modified, and extended at any moment.

### Core Principles

**1. No Separation Between Development and Runtime**

Traditional systems have distinct phases: write code, compile it, run it, observe failure, stop, fix, repeat. LISP machines eliminated these boundaries. Code was compiled incrementally, one function at a time. The running system immediately incorporated changes. Debugging happened without stopping the world.

**2. Semantic Output**

Every piece of text or graphics displayed on screen maintained its connection to the underlying Lisp object. When the system showed a filename, it remembered that string represented a file. Later commands seeking a file could use any previously displayed filename directly. The screen was not a passive display but an active interface to live data.

**3. The Debugger as Primary Interface**

Errors were not catastrophic events requiring restart but opportunities for exploration. When something went wrong, the debugger presented the full program state—call stack, local variables, available recovery options. Users could evaluate code in the context of any stack frame, fix the problem, and continue execution from where it paused.

**4. Restarts Instead of Exceptions**

Rather than exceptions that unwind the stack and destroy context, LISP machines used a condition/restart system. When an error occurred, the system offered multiple recovery paths: retry with different arguments, return a substitute value, skip the operation, or escalate to a supervisor. The user—or automated code—chose how to proceed.

### The Symbolics Experience

Symbolics Genera, the most sophisticated LISP machine operating system, provided:

- **The Lisp Listener**: A REPL that was simultaneously a shell, debugger, and object browser. Output was clickable; previously displayed objects could be used as input to new commands.
- **The Inspector**: A tool for navigating object graphs interactively, examining slots, modifying values, and understanding data structures.
- **Dynamic Windows**: A presentation-based UI where all output was typed and context-sensitive.
- **Document Examiner**: Hypertext documentation integrated with the development environment.
- **Source for Everything**: The entire operating system source was available, inspectable, and modifiable.

Systems ran for months without rebooting because any problem could be diagnosed and fixed in place.

## Lona's Interpretation

### Philosophy

Lona combines three traditions:

1. **seL4 Microkernel**: Formally verified, capability-based security
2. **LISP Machine**: Runtime introspection, hot-patching, live debugging
3. **Erlang/OTP**: Lightweight processes, supervision trees, "let it crash"

The synthesis: a system where **processes are isolated by hardware-enforced capabilities** but **individually transparent and debuggable**. Crashes are contained by the supervisor hierarchy, yet when desired, the user can pause a failing process, inspect its state, fix the code, and resume—all without affecting other processes.

### What Lona Preserves

- **Everything inspectable**: Any value, function, process, or capability can be examined
- **Non-destructive errors**: Errors pause execution; they don't destroy state
- **In-context evaluation**: Execute arbitrary code in the scope of any stack frame
- **Hot patching**: Redefine functions without restarting processes
- **Source availability**: System code is accessible and modifiable

### What Lona Adds

- **Capability-based security**: Inspection and modification require appropriate capabilities
- **Process isolation**: One process debugging doesn't affect others
- **Supervision integration**: The debugger is one option; supervisor restart is another
- **Domain boundaries**: Processes exist within security domains with distinct privilege levels

### What Lona Omits

- **Mouse-based UI**: Lona's core is keyboard-only; graphical interfaces can be built on top
- **Presentation types**: The semantic output concept may come later; the foundation comes first
- **Persistent images**: Lona emphasizes source files and reproducible builds over saved images

## The Lona REPL

The REPL is Lona's primary user interface. It is not merely a read-eval-print loop but the control surface for the entire system.

### Basic Interaction

```
╭─ Lona REPL ─────────────────────────────────────────────────────────────────╮
│ Lona 0.1.0 on seL4/aarch64                                                  │
│ Type (help) for commands, Ctrl-D to exit                                    │
╰─────────────────────────────────────────────────────────────────────────────╯

user> (+ 1 2 3)
6

user> (def x 42)
#'user/x

user> (defn greet [name]
        (str "Hello, " name "!"))
#'user/greet

user> (greet "World")
"Hello, World!"
```

The REPL understands s-expression structure and handles multi-line input naturally:

```
user> (defn factorial [n]
│       (if (<= n 1)
│         1
│         (* n (factorial (- n 1)))))
#'user/factorial
```

The `│` indicates continuation; the REPL waits for balanced parentheses.

### Keyboard Bindings

```
[Tab]    Complete symbol or show candidates
[↑/↓]    Navigate command history
[C-c]    Interrupt current evaluation
[C-d]    Exit REPL (with confirmation if processes running)
[C-l]    Clear screen
[?]      Toggle help bar
```

### Documentation and Introspection

```
user> (doc map)

╭─ map ───────────────────────────────────────────────────────────────────────╮
│ (map f coll)                                                                │
│ (map f coll & colls)                                                        │
│                                                                             │
│ Returns a lazy sequence of applying f to each element of coll.              │
│ If multiple collections provided, f is applied to corresponding elements.   │
╰─────────────────────────────────────────────────────────────────────────────╯

user> (source factorial)

(defn factorial [n]
  (if (<= n 1)
    1
    (* n (factorial (- n 1)))))

user> (apropos "fact")
user/factorial
math/factorize
math/factor-of?

user> (meta #'factorial)
{:name factorial
 :ns user
 :arglists ([n])
 :doc nil
 :file "<repl>"
 :line 12}
```

### Bytecode Inspection

```
user> (disassemble factorial)

╭─ Bytecode: factorial ───────────────────────────────────────────────────────╮
│ 0000: LOAD_LOCAL    0        ; n                                            │
│ 0002: PUSH_CONST    0        ; 1                                            │
│ 0004: CALL          <=       ; (<= n 1)                                     │
│ 0006: JUMP_IF_FALSE 000c                                                    │
│ 0008: PUSH_CONST    0        ; 1                                            │
│ 000a: RETURN                                                                │
│ 000c: LOAD_LOCAL    0        ; n                                            │
│ 000e: LOAD_LOCAL    0        ; n                                            │
│ 0010: PUSH_CONST    0        ; 1                                            │
│ 0012: CALL          -        ; (- n 1)                                      │
│ 0014: CALL          factorial                                               │
│ 0016: CALL          *        ; (* n ...)                                    │
│ 0018: RETURN                                                                │
╰─────────────────────────────────────────────────────────────────────────────╯
```

## The Debugger

When errors occur, Lona does not crash. It pauses and presents the debugger.

### Error Presentation

```
user> (factorial -3)

╭─ ERROR ─────────────────────────────────────────────────────────────────────╮
│ Stack overflow                                                              │
│ Infinite recursion detected in: factorial                                   │
╰─────────────────────────────────────────────────────────────────────────────╯

Restarts:
  [1] :abort      - Abort and return to REPL
  [2] :retry      - Retry with different arguments
  [3] :return     - Return a value from this frame
  [4] :supervisor - Let supervisor handle this process

Backtrace (most recent first):
  0: (factorial -2043)                           <-- you are here
  1: (factorial -2042)
  2: (factorial -2041)
  ...
  2047: (factorial -3)                           <-- original call
  2048: <repl>

debug[0]> _
```

### Debugger Commands

```
[1-9]    Select a restart
[u]      Move up the stack (toward caller)
[d]      Move down the stack (toward callee)
[NUM]    Jump to frame number
[l]      Show local variables in current frame
[e]      Enter eval mode in current frame
[s]      Show source code for current frame
[c]      Continue execution (if paused, not errored)
[q]      Quit debugger, return to REPL
```

### Stack Navigation

```
debug[0]> d                                      ; move down one frame

Backtrace (most recent first):
  0: (factorial -2043)
→ 1: (factorial -2042)                           <-- you are here
  2: (factorial -2041)
  ...

debug[1]> l                                      ; show locals

╭─ Locals in frame 1 ─────────────────────────────────────────────────────────╮
│ n = -2042                                                                   │
╰─────────────────────────────────────────────────────────────────────────────╯

debug[1]> 2047                                   ; jump to original call

→ 2047: (factorial -3)                           <-- you are here
  2048: <repl>

debug[2047]> l

╭─ Locals in frame 2047 ────────────────────────────────────────────────────────╮
│ n = -3                                                                        │
╰───────────────────────────────────────────────────────────────────────────────╯
```

### In-Frame Evaluation

The most powerful debugger feature: evaluating arbitrary code in the context of a specific stack frame.

```
debug[2047]> e                                   ; enter eval mode

eval[2047]> n                                    ; evaluate 'n'
-3

eval[2047]> (<= n 1)                             ; test the condition
true

eval[2047]> ; The bug: (<= -3 1) is true, so we recurse instead of stopping

eval[2047]> (exit)                               ; return to debugger
debug[2047]> _
```

### Hot Patching

Fix bugs without leaving the debugger:

```
debug[2047]> s                                   ; show source

╭─ Source: factorial ─────────────────────────────────────────────────────────╮
│ (defn factorial [n]                                                         │
│   (if (<= n 1)                        ; <- BUG: should handle n < 0         │
│     1                                                                       │
│     (* n (factorial (- n 1)))))                                             │
╰─────────────────────────────────────────────────────────────────────────────╯

debug[2047]> e

eval[2047]> (defn factorial [n]
│             (cond
│               (< n 0) (error "factorial: negative argument" {:n n})
│               (<= n 1) 1
│               :else (* n (factorial (- n 1)))))
#'user/factorial                                 ; function redefined!

eval[2047]> (exit)

debug[2047]> 2                                   ; select restart [2] :retry

╭─ Restart: retry ────────────────────────────────────────────────────────────╮
│ Enter new arguments for (factorial n):                                      │
╰─────────────────────────────────────────────────────────────────────────────╯

retry> 5
120                                              ; success with fixed code!

user> _
```

### Custom Restarts

Programs can define their own recovery options:

```
user> (defn safe-divide [a b]
        (restart-case (/ a b)
          (:use-value [v]
            :report "Supply a value to use instead"
            :interactive (fn [] [(prompt "Value: ")])
            v)
          (:return-nil []
            :report "Return nil"
            nil)
          (:retry-with [new-b]
            :report "Retry with different divisor"
            :interactive (fn [] [(prompt "New divisor: ")])
            (safe-divide a new-b))))
#'user/safe-divide

user> (safe-divide 10 0)

╭─ ERROR ─────────────────────────────────────────────────────────────────────╮
│ Division by zero                                                            │
╰─────────────────────────────────────────────────────────────────────────────╯

Restarts:
  [1] :use-value  - Supply a value to use instead
  [2] :return-nil - Return nil
  [3] :retry-with - Retry with different divisor
  [4] :abort      - Abort and return to REPL

debug[0]> 3

retry-with> 2
5                                                ; 10 / 2 = 5

user> _
```

## The Inspector

The inspector provides interactive navigation of data structures.

### Basic Usage

```
user> (def state {:users [{:name "Alice" :id 1} {:name "Bob" :id 2}]
                  :config {:debug true :max-conn 100}})
#'user/state

user> (inspect state)

╭─ Inspector ─────────────────────────────────────────────────────────────────╮
│ HashMap (2 entries)                                                         │
├─────────────────────────────────────────────────────────────────────────────┤
│ [0] :users ──→ Vector (2 items)                                             │
│ [1] :config ─→ HashMap (2 entries)                                          │
╰─────────────────────────────────────────────────────────────────────────────╯

inspect> _
```

### Navigation

```
inspect> 0                                       ; drill into :users

╭─ Inspector: state → :users ─────────────────────────────────────────────────╮
│ Vector (2 items)                                                            │
├─────────────────────────────────────────────────────────────────────────────┤
│ [0] {:name "Alice", :id 1}                                                  │
│ [1] {:name "Bob", :id 2}                                                    │
╰─────────────────────────────────────────────────────────────────────────────╯

inspect> 0                                       ; drill into first user

╭─ Inspector: state → :users → [0] ───────────────────────────────────────────╮
│ HashMap (2 entries)                                                         │
├─────────────────────────────────────────────────────────────────────────────┤
│ [0] :name ─→ "Alice"                                                        │
│ [1] :id ───→ 1                                                              │
├─────────────────────────────────────────────────────────────────────────────┤
│ Type: HashMap                                                               │
│ Path: state → :users → [0]                                                  │
╰─────────────────────────────────────────────────────────────────────────────╯

inspect> ^                                       ; go back up
inspect> q                                       ; quit inspector
```

### Inspector Commands

```
[0-9]    Select entry to drill into
[^]      Go up one level
[e]      Eval expression ($ refers to current value)
[m]      Modify a slot
[y]      Yank current value to REPL as $1, $2, etc.
[t]      Show type information
[q]      Quit inspector
```

### Modification

```
inspect> m 1                                     ; modify slot [1]

modify> 200                                      ; new value
Modified: :max-conn = 200

inspect> e                                       ; evaluate with current value

eval-inspect> (assoc $ :timeout 30)              ; $ is current object
{:debug true, :max-conn 200, :timeout 30}

eval-inspect> (set! $ (assoc $ :timeout 30))     ; actually modify
Modified.
```

### Function Inspection

```
user> (inspect factorial)

╭─ Inspector: Function ───────────────────────────────────────────────────────╮
│ #'user/factorial                                                            │
├─────────────────────────────────────────────────────────────────────────────┤
│ [0] :name ─────→ factorial                                                  │
│ [1] :arglists ─→ ([n])                                                      │
│ [2] :ns ───────→ user                                                       │
│ [3] :source ───→ (defn factorial [n] ...)                                   │
│ [4] :bytecode ─→ <Chunk 18 bytes>                                           │
╰─────────────────────────────────────────────────────────────────────────────╯
```

## Processes

Lona processes are lightweight, isolated units of execution inspired by Erlang.

### Listing Processes

```
user> (processes)

╭─ Processes ─────────────────────────────────────────────────────────────────╮
│ PID    NAME           STATUS      MESSAGES   MEMORY    DOMAIN              │
├─────────────────────────────────────────────────────────────────────────────┤
│ 0      <init>         running     0          4 KB      kernel              │
│ 1      <repl>         running     0          128 KB    user                │
│ 1042   :worker-1      waiting     3          16 KB     user                │
│ 1043   :worker-2      running     0          12 KB     user                │
│ 1044   :logger        waiting     47         8 KB      system              │
╰─────────────────────────────────────────────────────────────────────────────╯

user> (process-info 1042)

╭─ Process :worker-1 (pid=1042) ──────────────────────────────────────────────╮
│ Status:      waiting (in receive)                                           │
│ Domain:      user                                                           │
│ Supervisor:  :main-sup (pid=1040)                                           │
│ Mailbox:     3 messages                                                     │
│ Memory:      16 KB                                                          │
│ Reductions:  142,847                                                        │
│ Links:       [1043, 1044]                                                   │
│ Monitors:    [:db-connection]                                               │
╰─────────────────────────────────────────────────────────────────────────────╯
```

### Spawning Processes

```
user> (spawn (fn []
        (loop []
          (let [msg (receive)]
            (println "Got:" msg)
            (recur)))))
#<Process pid=1050>

user> (spawn :name :my-worker
             :supervisor :main-sup
             (fn [] (do-work)))
#<Process :my-worker pid=1051>
```

### Sending Messages

```
user> (send 1050 {:type :hello :data "world"})
:ok

; Process 1050 prints:
Got: {:type :hello :data "world"}

user> (send :my-worker :ping)
:ok
```

### Process Supervision

```
user> (supervisors)

╭─ Supervision Tree ──────────────────────────────────────────────────────────╮
│ :root-sup (pid=100)                                                         │
│ ├── :main-sup (pid=1040)                                                    │
│ │   ├── :worker-1 (pid=1042) [running]                                      │
│ │   ├── :worker-2 (pid=1043) [running]                                      │
│ │   └── :my-worker (pid=1051) [running]                                     │
│ └── :system-sup (pid=200)                                                   │
│     └── :logger (pid=1044) [running]                                        │
╰─────────────────────────────────────────────────────────────────────────────╯

user> (supervisor-info :main-sup)

╭─ Supervisor :main-sup ──────────────────────────────────────────────────────╮
│ Strategy:    :one-for-one                                                   │
│ Intensity:   3 restarts per 5 seconds                                       │
│ Children:    3 active                                                       │
│ Restarts:    7 total (2 in last hour)                                       │
╰─────────────────────────────────────────────────────────────────────────────╯
```

## Process Debugging

### Debugging a Running Process

Attach a debugger to a process to catch future errors:

```
user> (debug-attach 1042)
Debugger attached to :worker-1 (pid=1042)
Process will pause on errors instead of crashing.

user> ; Send a message that will cause an error
user> (send 1042 {:type :divide :a 10 :b 0})

╭─ PROCESS BREAK: :worker-1 (pid=1042) ───────────────────────────────────────╮
│ Division by zero                                                            │
│ Process paused. Other processes continue running.                           │
╰─────────────────────────────────────────────────────────────────────────────╯

Backtrace:
  0: (/ a b)
  1: (handle-message msg)
  2: (loop [] ...)
  3: <process-entry>

proc-debug[0]> _
```

### Process Debugger Commands

```
[u/d]    Move up/down stack
[l]      Show locals
[e]      Eval in frame
[s]      Show source
[c]      Continue execution
[n]      Step to next expression
[k]      Kill process
[r]      Release (detach debugger, let supervisor handle)
[q]      Detach and return to REPL (process remains paused)
```

### Examining Process State

```
proc-debug[0]> l

╭─ Locals in frame 0 ─────────────────────────────────────────────────────────╮
│ a = 10                                                                      │
│ b = 0                                                                       │
╰─────────────────────────────────────────────────────────────────────────────╯

proc-debug[0]> e

eval[0]> msg
{:type :divide :a 10 :b 0}

eval[0]> (exit)

proc-debug[0]> m                                 ; show mailbox

╭─ Mailbox: :worker-1 ────────────────────────────────────────────────────────╮
│ [0] {:type :ping}                                                           │
│ [1] {:type :status-request :from 1}                                         │
╰─────────────────────────────────────────────────────────────────────────────╯
```

### Fix and Continue

```
proc-debug[0]> e

eval[0]> (defn handle-divide [{:keys [a b]}]
│          (if (zero? b)
│            {:error :division-by-zero}
│            {:result (/ a b)}))
#'worker/handle-divide

eval[0]> (exit)

proc-debug[0]> c                                 ; continue
{:error :division-by-zero}                       ; process continues with fix

user> (debug-detach 1042)
Debugger detached from :worker-1
```

### Debugging Crashed Processes

When a process crashes without a debugger attached:

```
╭─ PROCESS CRASH ─────────────────────────────────────────────────────────────╮
│ Process :worker-2 (pid=1043) terminated                                     │
│ Reason: Division by zero                                                    │
│ Supervisor :main-sup restarting (attempt 1/3)                               │
╰─────────────────────────────────────────────────────────────────────────────╯

user> ; We can examine the crash post-mortem:
user> (debug-crash 1043)

╭─ Crash Report: :worker-2 (pid=1043) ────────────────────────────────────────╮
│ Exit reason: Division by zero                                               │
│ Crashed at: 2024-01-15 14:32:17                                             │
│ Uptime: 3h 42m                                                              │
│ Last message: {:type :calculate :expr "(/ 1 0)"}                            │
╰─────────────────────────────────────────────────────────────────────────────╯

Backtrace at crash:
  0: (/ 1 0)
  1: (eval-expr expr)
  2: (handle-message msg)
  3: <process-entry>

crash-debug[0]> l

╭─ Locals at crash ───────────────────────────────────────────────────────────╮
│ expr = "(/ 1 0)"                                                            │
│ msg = {:type :calculate :expr "(/ 1 0)"}                                    │
╰─────────────────────────────────────────────────────────────────────────────╯
```

### Spawning with Debug Mode

Start a process with the debugger pre-attached:

```
user> (spawn :name :test-worker
             :debug true                         ; debugger attached from start
             (fn []
               (let [x (compute-value)]
                 (/ 100 x))))
#<Process :test-worker pid=1060 DEBUG>

; When error occurs:
╭─ PROCESS BREAK: :test-worker (pid=1060) ────────────────────────────────────╮
│ Division by zero                                                            │
│ Process paused. Debug mode active.                                          │
╰─────────────────────────────────────────────────────────────────────────────╯

proc-debug[0]> l

╭─ Locals in frame 0 ─────────────────────────────────────────────────────────╮
│ x = 0                                                                       │
╰─────────────────────────────────────────────────────────────────────────────╯

proc-debug[0]> e

eval[0]> (set! x 1)                              ; modify the local
1

eval[0]> (exit)

proc-debug[0]> c                                 ; continue with fixed value
100                                              ; process completes successfully
```

## Domains and Capabilities

Lona runs on seL4, a capability-based microkernel. Capabilities control what actions processes can perform.

### Viewing Domain Information

```
user> (domains)

╭─ Domains ───────────────────────────────────────────────────────────────────╮
│ NAME        PROCESSES   CAPABILITIES                                        │
├─────────────────────────────────────────────────────────────────────────────┤
│ kernel      1           [memory, irq, device, domain-create]                │
│ system      2           [memory, ipc, spawn, file-read]                     │
│ user        5           [ipc, spawn]                                        │
│ untrusted   0           [ipc-limited]                                       │
╰─────────────────────────────────────────────────────────────────────────────╯

user> (domain-info :user)

╭─ Domain: user ──────────────────────────────────────────────────────────────╮
│ Capabilities:                                                               │
│   - ipc: Can send/receive messages to processes in same domain              │
│   - spawn: Can create new processes within this domain                      │
│                                                                             │
│ Processes: 5                                                                │
│   <repl> (pid=1), :worker-1 (pid=1042), :worker-2 (pid=1043),              │
│   :my-worker (pid=1051), :test-worker (pid=1060)                            │
│                                                                             │
│ Memory quota: 16 MB (4.2 MB used)                                           │
│ Process quota: 100 (5 used)                                                 │
╰─────────────────────────────────────────────────────────────────────────────╯
```

### Capability Inspection

```
user> (capabilities)

╭─ Capabilities for <repl> (pid=1) ───────────────────────────────────────────╮
│ [0] ipc:user           - IPC within user domain                             │
│ [1] spawn:user         - Spawn processes in user domain                     │
│ [2] debug:user         - Debug processes in user domain                     │
│ [3] inspect:user       - Inspect values in user domain                      │
│ [4] memory:16MB        - Allocate up to 16MB                                │
╰─────────────────────────────────────────────────────────────────────────────╯

user> (inspect-capability 2)

╭─ Capability: debug:user ────────────────────────────────────────────────────╮
│ Type:        debug                                                          │
│ Scope:       user domain                                                    │
│ Permissions:                                                                │
│   - Attach debugger to processes in user domain                             │
│   - Inspect stack frames and locals                                         │
│   - Evaluate code in process context                                        │
│   - Pause/resume process execution                                          │
│ Restrictions:                                                               │
│   - Cannot debug processes in other domains                                 │
│   - Cannot modify kernel or system processes                                │
╰─────────────────────────────────────────────────────────────────────────────╯
```

### Cross-Domain Operations

```
user> (debug-attach 200)                         ; system domain process

╭─ ERROR ─────────────────────────────────────────────────────────────────────╮
│ Permission denied                                                           │
│ Cannot debug process in domain 'system' from domain 'user'                  │
│ Required capability: debug:system                                           │
╰─────────────────────────────────────────────────────────────────────────────╯

user> (with-capability (request-capability :debug :system)
        (debug-attach 200))
; This would prompt for elevated privileges or fail based on policy
```

### Capability-Aware Spawning

```
user> (spawn :domain :untrusted
             :capabilities [:ipc-limited]
             (fn []
               (handle-untrusted-code)))
#<Process pid=2000 domain=untrusted>

user> ; Process 2000 cannot spawn children, access files, or debug
user> ; It can only send limited IPC messages
```

## Error Philosophy

### The Debugger is Not Failure

Traditional systems treat errors as exceptional, catastrophic events. The program crashes, state is lost, users restart from scratch. Lona treats errors as **pause points for exploration**.

When an error occurs:
1. Execution pauses (not terminates)
2. Full state is preserved
3. User chooses how to proceed
4. Work can continue from where it stopped

### Restarts vs Exceptions

Exceptions unwind the stack:
```
; Traditional exception handling
try {
    riskyOperation();
} catch (Error e) {
    // Stack is unwound. Context is lost.
    // Can only abort or retry from here.
}
```

Restarts preserve the stack:
```
; Lona's condition/restart system
(restart-case (risky-operation)
  (:retry []
    :report "Try again"
    (risky-operation))
  (:use-default []
    :report "Use default value"
    :default-value)
  (:skip []
    :report "Skip this operation"
    nil))

; When error occurs, user sees all options
; and can evaluate code before choosing
```

### Supervisor Integration

The debugger is one tool; supervision is another. They complement each other:

```
; Debugger: Interactive investigation
; - Attach to running process
; - Pause on errors
; - Inspect and fix in place
; - Continue execution

; Supervisor: Automated recovery
; - Process crashes are contained
; - Supervisor restarts according to strategy
; - Restart limits prevent infinite loops
; - Other processes unaffected
```

Users can choose per-situation:

```
debug[0]> 4                                      ; select :supervisor restart

╭─ Supervisor Handoff ────────────────────────────────────────────────────────╮
│ Releasing process :worker-1 to supervisor :main-sup                         │
│ Supervisor will apply restart strategy: :one-for-one                        │
╰─────────────────────────────────────────────────────────────────────────────╯

user> ; Process restarts automatically, fresh state
```

## Complete Help Reference

```
user> (help)

╭─ Lona REPL Help ────────────────────────────────────────────────────────────╮
│                                                                             │
│ REPL Commands:                                                              │
│   (help)              Show this help                                        │
│   (help <topic>)      Help on specific topic                                │
│   (doc <fn>)          Show function documentation                           │
│   (source <fn>)       Show function source code                             │
│   (apropos "str")     Search for symbols containing "str"                   │
│   (inspect <val>)     Interactive object inspector                          │
│   (disassemble <fn>)  Show bytecode for function                            │
│                                                                             │
│ Process Commands:                                                           │
│   (processes)         List all processes                                    │
│   (process-info pid)  Detailed info about a process                         │
│   (spawn ...)         Create new process                                    │
│   (send pid msg)      Send message to process                               │
│   (kill pid)          Terminate a process                                   │
│                                                                             │
│ Debugging Commands:                                                         │
│   (debug-attach pid)  Attach debugger to running process                    │
│   (debug-detach pid)  Detach debugger from process                          │
│   (debug-crash pid)   Examine crashed process post-mortem                   │
│                                                                             │
│ Supervision Commands:                                                       │
│   (supervisors)       Show supervision tree                                 │
│   (supervisor-info n) Info about a supervisor                               │
│                                                                             │
│ Domain/Capability Commands:                                                 │
│   (domains)           List security domains                                 │
│   (domain-info name)  Info about a domain                                   │
│   (capabilities)      Show current process capabilities                     │
│                                                                             │
│ Keyboard Shortcuts:                                                         │
│   Tab     Complete    C-c    Interrupt    C-d    Exit                       │
│   ↑/↓     History     C-l    Clear        ?      Toggle help                │
│                                                                             │
│ Topics: (help 'debugger) (help 'inspector) (help 'processes)                │
│         (help 'restarts) (help 'capabilities) (help 'supervisors)           │
╰─────────────────────────────────────────────────────────────────────────────╯
```

## Vision Summary

Lona's REPL embodies a fundamental belief: **the user should always be in control**. Errors don't crash programs; they invite investigation. Processes don't run as black boxes; they expose their internal state. The system doesn't hide its implementation; it makes everything inspectable and modifiable.

This is not about providing developer tools on top of a conventional system. It's about building a system where introspection and modification are foundational capabilities—where the question is never "can I see what's happening?" but "what do I want to do about it?"

The combination of capability-based security with live debugging creates something new: a system that is both maximally transparent to authorized users and maximally protected against unauthorized access. You can inspect any process in your domain, but you cannot touch processes outside your capabilities. Security and transparency coexist.

This is the LISP machine vision, adapted for a world of networked services, untrusted code, and formal verification. Not nostalgia for the past, but lessons from the past applied to the challenges of today.
