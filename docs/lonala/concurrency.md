# Concurrency
> **Status**: *Planned for Phases 9-12*

Lonala provides Erlang-style lightweight processes and message passing.

## 13.1 Processes (Planned)

```clojure
; Spawn a new process
(spawn (fn [] (print "Hello from process!")))

; Get current process ID
(self)

; Exit current process
(exit :normal)
```

## 13.2 Message Passing (Planned)

```clojure
; Send message to process
(send pid {:type :greeting :text "Hello"})

; Receive messages with pattern matching
(receive
  {:type :greeting :text text}
    (print "Got greeting:" text)
  {:type :shutdown}
    (exit :normal)
  (after 5000
    (print "Timeout!")))
```

## 13.3 Supervision (Planned)

```clojure
(def-supervisor my-supervisor
  :strategy :one-for-one
  :children
  [{:id :worker-1 :start #(spawn worker-fn [])}
   {:id :worker-2 :start #(spawn worker-fn [])}])
```

### Supervision Strategies

| Strategy | Behavior |
|----------|----------|
| `:one-for-one` | Only restart the failed child |
| `:one-for-all` | Restart all children if one fails |
| `:rest-for-one` | Restart failed child and all started after it |

## 13.4 Linking and Monitoring (Planned)

```clojure
(link pid)           ; bidirectional link
(unlink pid)
(spawn-link fn args) ; spawn and link atomically

(monitor pid)        ; unidirectional monitor
(demonitor ref)
```

### Links vs Monitors

| Feature | Link | Monitor |
|---------|------|---------|
| Direction | Bidirectional | Unidirectional |
| On exit | Both processes notified | Only monitor notified |
| Exit propagation | Yes (by default) | No |
| Use case | Tightly coupled processes | Observing processes |

## 13.5 Process Recovery

See [Special Forms: Process Termination](special-forms.md#68-process-termination) for how `panic!` interacts with supervision.

---

