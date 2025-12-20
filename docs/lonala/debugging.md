# Debugging

This document specifies Lonala's debugging capabilities, including the Two-Mode Architecture that reconciles BEAM-style resilience with LISP-machine debuggability.

## Two-Mode Architecture

Lona processes operate in one of two modes:

| Mode | Default | Behavior on Error | Use Case |
|------|---------|-------------------|----------|
| **Production** | Yes | Process crashes, supervisor restarts | Server workloads |
| **Debug** | No | Process pauses, user inspects | Live troubleshooting |

### Production Mode

In production mode (the default), errors follow the BEAM/OTP "let it crash" philosophy:

1. `panic!` or unhandled conditions terminate the process
2. The supervisor detects the exit and applies its restart strategy
3. The system self-heals without human intervention

```clojure
;; In production mode, this crashes the process
(defn process-request [req]
  (let [user (get-user (:user-id req))]
    (when (nil? user)
      (panic! "User not found" {:user-id (:user-id req)}))
    (handle-request user req)))

;; Supervisor restarts the process automatically
```

### Debug Mode

When a debugger is attached to a process, it switches to debug mode:

1. `panic!` and unhandled conditions **pause** execution instead of crashing
2. The debugger presents the error, available restarts, and stack
3. The developer can inspect, modify, and choose how to proceed

```clojure
;; Attach debugger to a running process
(debug-attach pid)

;; The process now pauses on errors instead of crashing
```

### Mode Transitions

```
                    ┌─────────────────┐
                    │  Process Start  │
                    └────────┬────────┘
                             │
                             ▼
                    ┌─────────────────┐
         ┌──────────│ Production Mode │◄──────────┐
         │          └────────┬────────┘           │
         │                   │                    │
         │          debug-attach                  │
         │                   │                    │
         │                   ▼                    │
         │          ┌─────────────────┐           │
         │          │   Debug Mode    │───────────┘
         │          └────────┬────────┘  debug-detach
         │                   │
    on error            on error
         │                   │
         ▼                   ▼
┌─────────────────┐  ┌─────────────────┐
│  Crash & Exit   │  │  Pause & Wait   │
│  (supervisor    │  │  (debugger UI)  │
│   restarts)     │  │                 │
└─────────────────┘  └─────────────────┘
```

## Debug Primitives

### Attaching and Detaching

```clojure
;; Attach debugger to a process
(debug-attach pid)
;; => :ok

;; Detach debugger, return to production mode
(debug-detach pid)
;; => :ok

;; Check if debugger is attached
(debug-attached? pid)
;; => true | false

;; Attach with options
(debug-attach pid {:break-on [:panic :condition]
                   :trace-calls true})
```

### Pausing and Resuming

```clojure
;; Externally pause a running process
(debug-pause pid)
;; => :ok (process enters :debugging state)

;; Resume a paused process
(debug-continue pid)
;; => :ok

;; Single-step execution
(debug-step pid)
;; => :ok (executes one expression, then pauses)

;; Step over function calls
(debug-step-over pid)
;; => :ok

;; Step out of current function
(debug-step-out pid)
;; => :ok
```

### Stack Frame Inspection

```clojure
;; Get all stack frames for a paused process
(debug-frames pid)
;; => [{:index 0 :function 'user/divide :line 42 :file "user.lona"}
;;     {:index 1 :function 'user/calculate :line 15 :file "user.lona"}
;;     ...]

;; Get local variables in a frame
(debug-locals pid frame-index)
;; => {:a 10 :b 0 :result nil}

;; Get source code for a frame
(debug-source pid frame-index)
;; => "(defn divide [a b]\n  (/ a b))"

;; Evaluate expression in frame context
(debug-eval pid frame-index '(+ a b))
;; => 10

;; Modify a local variable
(debug-set-local! pid frame-index 'b 2)
;; => :ok
```

### Process State

Processes have the following states:

| State | Description |
|-------|-------------|
| `:running` | Actively executing code |
| `:waiting` | Blocked in `receive`, waiting for message |
| `:suspended` | Paused by scheduler (normal preemption) |
| `:debugging` | Paused by debugger (special state) |

The `:debugging` state is special:
- Process does not process messages from its mailbox
- Process accepts debug commands via a separate channel
- Supervisor recognizes this state and does not restart the process
- Timeouts from linked processes may need special handling

```clojure
;; Check process state
(process-status pid)
;; => :running | :waiting | :suspended | :debugging

;; Supervisors should handle :debugging specially
(defn supervisor-check [child-pid]
  (case (process-status child-pid)
    :debugging :ignore    ; Don't restart, being debugged
    :dead      :restart
    _          :ok))
```

## Breakpoints

Lona supports three types of breakpoints, all using pattern matching for conditions.

### Entry Breakpoints

Break when a function is called with matching arguments:

```clojure
;; Break when process-request receives admin user
(set-breakpoint :call 'api/process-request
                :pattern [{:user "admin" & _}])

;; Break on any call to the function
(set-breakpoint :call 'db/execute-query)

;; Break with guard condition
(set-breakpoint :call 'math/divide
                :pattern [_ b]
                :when (zero? b))
```

### Return Breakpoints

Break when a function returns a matching value:

```clojure
;; Break when function returns an error
(set-breakpoint :return 'api/fetch-user
                :pattern {:error _})

;; Break on specific error type
(set-breakpoint :return 'db/query
                :pattern {:error {:type :connection-failed & _}})
```

### Receive Breakpoints

Break when a process receives a matching message:

```clojure
;; Break on shutdown message
(set-breakpoint :receive pid
                :pattern {:type :shutdown})

;; Break on abnormal shutdown only
(set-breakpoint :receive pid
                :pattern {:type :shutdown :reason reason}
                :when (not= reason :normal))
```

### Breakpoint Management

```clojure
;; List all breakpoints
(list-breakpoints)
;; => [{:id 1 :type :call :target 'api/process-request ...}
;;     {:id 2 :type :return :target 'api/fetch-user ...}]

;; Remove a breakpoint
(clear-breakpoint breakpoint-id)

;; Disable without removing
(disable-breakpoint breakpoint-id)

;; Re-enable
(enable-breakpoint breakpoint-id)

;; Clear all breakpoints
(clear-all-breakpoints)
```

### Breakpoint Actions

Breakpoints can trigger different actions:

```clojure
;; Pause execution (default)
(set-breakpoint :return 'foo :pattern {:error _}
                :action :pause)

;; Log without pausing (tracing)
(set-breakpoint :return 'foo :pattern {:error _}
                :action :log)

;; Execute custom function
(set-breakpoint :return 'foo :pattern {:error _}
                :action (fn [ctx] (log/warn "Error in foo" ctx)))
```

## Tracing

Non-blocking observation of system behavior, inspired by Erlang's `:dbg` module.

### Function Tracing

```clojure
;; Trace all calls to a function
(trace-calls 'tcp/handle-packet)
;; >> [12:34:56.789] tcp/handle-packet called with: [{:src ...}]
;; << [12:34:56.791] tcp/handle-packet returned: :ok

;; Trace with options
(trace-calls 'tcp/handle-packet
             {:args true        ; Show arguments
              :return true      ; Show return value
              :timestamps true  ; Include timestamps
              :limit 100})      ; Stop after 100 traces

;; Stop tracing
(untrace-calls 'tcp/handle-packet)
```

### Message Tracing

```clojure
;; Trace messages to/from a process
(trace-messages pid {:send true :receive true})
;; >> [12:34:57.001] pid-42 <- {:request :data} from pid-10
;; << [12:34:57.003] pid-42 -> {:response :ok} to pid-10

;; Trace with pattern filter
(trace-messages pid {:receive true
                     :pattern {:type :error & _}})
```

### Cross-Domain Tracing

```clojure
;; Trace IPC between domains (requires capability)
(trace-domain-ipc "driver:net" "tcp-stack")
```

### Trace-to-Break

Convert a non-blocking trace to a blocking breakpoint:

```clojure
;; Start with tracing
(def trace-id (trace-calls 'api/process-request
                           {:return true
                            :pattern {:error _}}))

;; When you see something interesting, upgrade to breakpoint
(trace-to-break trace-id)
;; Now the trace becomes a breakpoint and will pause on match
```

## The Debugger UI

When a process pauses (due to error or breakpoint), the debugger presents an interactive interface.

### Error Presentation

```
╭─ PROCESS BREAK: :worker-1 (pid=1042) ───────────────────────────╮
│ Division by zero                                                 │
│ Process paused. Other processes continue running.                │
╰──────────────────────────────────────────────────────────────────╯

Restarts:
  [1] :abort      - Abort and trigger supervisor restart
  [2] :use-value  - Return a substitute value
  [3] :retry      - Retry with different arguments

Backtrace (most recent first):
  0: (/ a b)                                    <-- you are here
  1: (calculate request)
  2: (handle-message msg)
  3: <process-entry>

proc-debug[0]> _
```

### Debugger Commands

| Command | Description |
|---------|-------------|
| `u` | Move up the stack (toward caller) |
| `d` | Move down the stack (toward callee) |
| `NUM` | Jump to frame number |
| `l` | Show local variables in current frame |
| `e` | Enter eval mode in current frame |
| `s` | Show source code for current frame |
| `c` | Continue execution |
| `n` | Step to next expression |
| `o` | Step out of current function |
| `1-9` | Select a restart |
| `q` | Detach debugger, return to REPL |

### In-Frame Evaluation

```
proc-debug[0]> e

eval[0]> a                          ; evaluate local variable
10

eval[0]> b
0

eval[0]> (zero? b)                  ; evaluate expression
true

eval[0]> (set! b 2)                 ; modify local
2

eval[0]> (exit)                     ; return to debugger

proc-debug[0]> c                    ; continue with modified value
5                                   ; 10 / 2 = 5
```

## Cross-Domain Debugging

Debugging across domain boundaries requires capabilities.

### Debug Capabilities

```clojure
;; Check if you can debug a domain
(has-capability? :debug "driver:net")
;; => true | false

;; Request debug capability (may prompt or fail)
(request-capability :debug "driver:net")
```

### Spawning with Debug Access

```clojure
;; Grant debug access when spawning
(spawn worker-fn []
       {:domain "worker:1"
        :grant-debug-to ["admin" "monitoring"]})

;; Now "admin" domain can attach debugger to "worker:1"
```

### Remote Debugging

```clojure
;; Connect to a remote Lona system
(def remote (connect-remote "192.168.1.10:4242" credentials))

;; List processes on remote system
(remote-eval remote '(list-processes))

;; Attach debugger to remote process
(debug-attach-remote remote pid)
```

## Supervisor Integration

Supervisors must be aware of the debug state.

### Supervisor Behavior

When a child process is in `:debugging` state:
- Supervisor does **not** consider it crashed
- Supervisor does **not** attempt restart
- Restart intensity counters are **not** affected
- Supervisor waits indefinitely (or until timeout configured)

```clojure
;; Configure supervisor patience for debugged children
(def-supervisor my-sup
  :strategy :one-for-one
  :debug-timeout :infinity    ; Wait forever for debugged children
  :children [...])

;; Or with a timeout
(def-supervisor my-sup
  :strategy :one-for-one
  :debug-timeout 300000       ; Wait 5 minutes, then force-crash
  :children [...])
```

### The "Crash Now" Option

In debug mode, the user can choose to let the supervisor handle the error:

```
Restarts:
  [1] :abort      - Abort and trigger supervisor restart  <-- this one
  [2] :use-value  - Return a substitute value
  [3] :retry      - Retry with different arguments

proc-debug[0]> 1

╭─ Supervisor Handoff ────────────────────────────────────────────╮
│ Releasing process :worker-1 to supervisor :main-sup             │
│ Supervisor will apply restart strategy: :one-for-one            │
╰──────────────────────────────────────────────────────────────────╯

;; Process crashes, supervisor restarts it
```

## Implementation Notes

### Process Debug Flag

Each process has a debug flag in its control structure:

```
Process {
  pid: ProcessId,
  state: ProcessState,  // :running, :waiting, :suspended, :debugging
  debug_mode: bool,     // true when debugger attached
  debug_channel: Option<Channel>,  // for debug commands
  breakpoints: Vec<Breakpoint>,
  ...
}
```

### Breakpoint Trampolines

Breakpoints are implemented via dispatch table modification:

1. Original: `foo → bytecode-A`
2. With breakpoint: `foo → breakpoint-trampoline → bytecode-A`

The trampoline:
1. Checks if pattern matches arguments (for entry breakpoints)
2. If match and action is `:pause`, suspends process
3. Otherwise, jumps to original bytecode
4. For return breakpoints, wraps the return path similarly

### Performance Considerations

- Debug mode has minimal overhead when no breakpoints are set
- Breakpoints add pattern-matching cost on each call/return
- Tracing is asynchronous and uses ring buffers to minimize impact
- Production mode (no debugger attached) has zero debug overhead
