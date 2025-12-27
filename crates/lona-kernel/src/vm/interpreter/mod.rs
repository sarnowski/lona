// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Virtual machine interpreter for Lonala bytecode.

use alloc::vec;
use alloc::vec::Vec;

use lona_core::chunk::Chunk;
use lona_core::opcode::{Opcode, decode_a, decode_b, decode_op, decode_opcode_byte};
use lona_core::source;
use lona_core::symbol::{self, Interner};
use lona_core::value::Value;
use lonala_compiler::MacroRegistry;

use crate::namespace::{Registry as NamespaceRegistry, SourceLoader};

use super::error::{Error, Kind as ErrorKind};
use super::frame::Frame;
use super::globals::Globals;
use super::natives::{NativeFn, Registry as NativeRegistry, VmNativeFn, VmNativeRegistry};

mod helpers;
mod ops_arithmetic;
mod ops_control;
mod ops_data;
pub(super) mod types;

pub(super) use types::{DispatchResult, TailCallData, TailCallSetup};

/// Maximum call stack depth to prevent stack overflow.
pub(super) const MAX_CALL_DEPTH: usize = 256;

/// Default register file size.
const DEFAULT_REGISTER_COUNT: usize = 256;

/// The Lonala virtual machine.
///
/// Executes compiled bytecode from `Chunk` objects. Register-based design
/// similar to Lua, with up to 256 registers per frame.
pub struct Vm<'interner> {
    /// Register file for storing values during execution.
    registers: Vec<Value>,
    /// Global variable storage.
    globals: Globals,
    /// Symbol interner for resolving symbol names.
    interner: &'interner Interner,
    /// Registry of native functions.
    natives: NativeRegistry,
    /// Registry of mutable-access native functions (those needing `&mut Vm`).
    mut_natives: VmNativeRegistry,
    /// Optional macro registry for introspection functions.
    /// Passed to native functions via `NativeContext`.
    macro_registry: Option<&'interner MacroRegistry>,
    /// Current call depth for stack overflow protection.
    call_depth: usize,
    /// Current source ID for error reporting.
    current_source: source::Id,
    /// Current namespace for symbol resolution in REPL sessions.
    /// Defaults to "user" namespace.
    current_namespace: symbol::Id,
    /// Optional source loader for namespace loading.
    /// When set, enables `require` to load namespace source code.
    loader: Option<&'interner dyn SourceLoader>,
    /// Stack of namespaces currently being loaded, for cycle detection.
    /// Used by `require_namespace` to detect circular dependencies.
    loading_stack: Vec<symbol::Id>,
    /// Namespace registry for tracking loaded namespaces.
    /// Used by `require_namespace` to avoid reloading and for symbol resolution.
    namespace_registry: NamespaceRegistry,
}

impl<'interner> Vm<'interner> {
    /// Creates a new virtual machine.
    #[inline]
    #[must_use]
    pub fn new(interner: &'interner Interner) -> Self {
        let default_ns = interner.intern("user");
        Self {
            registers: vec![Value::Nil; DEFAULT_REGISTER_COUNT],
            globals: Globals::new(),
            interner,
            natives: NativeRegistry::new(),
            mut_natives: VmNativeRegistry::new(),
            macro_registry: None,
            call_depth: 0,
            current_source: source::Id::new(0_u32),
            current_namespace: default_ns,
            loader: None,
            loading_stack: vec![],
            namespace_registry: NamespaceRegistry::new(interner),
        }
    }

    /// Sets the source loader for namespace loading.
    ///
    /// When set, the VM can load namespaces via `require`. Without a loader,
    /// `require` will fail with a `NoSourceLoader` error.
    #[inline]
    pub const fn set_loader(&mut self, loader: &'interner dyn SourceLoader) {
        self.loader = Some(loader);
    }

    /// Returns a reference to the source loader, if set.
    #[inline]
    #[must_use]
    pub const fn loader(&self) -> Option<&'interner dyn SourceLoader> {
        self.loader
    }

    /// Checks if a namespace is currently being loaded (cycle detection).
    ///
    /// Returns `true` if the namespace is in the loading stack, indicating
    /// a circular dependency would occur if we tried to load it.
    #[inline]
    #[must_use]
    pub fn is_loading(&self, ns: symbol::Id) -> bool {
        self.loading_stack.contains(&ns)
    }

    /// Pushes a namespace onto the loading stack.
    ///
    /// Call this before loading a namespace to enable cycle detection.
    #[inline]
    pub fn push_loading(&mut self, ns: symbol::Id) {
        self.loading_stack.push(ns);
    }

    /// Pops a namespace from the loading stack.
    ///
    /// Call this after a namespace has been loaded (successfully or not).
    #[inline]
    pub fn pop_loading(&mut self) {
        let _popped = self.loading_stack.pop();
    }

    /// Returns a slice of the current loading stack.
    ///
    /// Used for error reporting when a circular dependency is detected.
    #[inline]
    #[must_use]
    pub fn loading_stack(&self) -> &[symbol::Id] {
        &self.loading_stack
    }

    /// Registers a native function for a symbol.
    ///
    /// The symbol must already be interned in the interner passed to [`Vm::new`].
    #[inline]
    pub fn register_native(&mut self, symbol: symbol::Id, func: NativeFn) {
        self.natives.register(symbol, func);
    }

    /// Registers a VM-native function for a symbol.
    ///
    /// VM-native functions have access to mutable VM state, unlike regular
    /// native functions. Used for primitives like `require`, `namespace-add-alias`, etc.
    #[inline]
    pub fn register_vm_native(&mut self, symbol: symbol::Id, func: VmNativeFn) {
        self.mut_natives.register(symbol, func);
    }

    /// Looks up a VM-native function by symbol ID.
    ///
    /// Returns `Some(func)` if a VM-native function is registered for the symbol,
    /// `None` otherwise.
    #[inline]
    #[must_use]
    pub fn get_vm_native(&self, symbol: symbol::Id) -> Option<VmNativeFn> {
        self.mut_natives.get(symbol)
    }

    /// Sets the macro registry for introspection functions.
    ///
    /// When set, the macro registry is passed to native functions via
    /// `NativeContext`, allowing `macro?`, `macroexpand-1`, and `macroexpand`
    /// to access macro definitions.
    #[inline]
    pub const fn set_macro_registry(&mut self, registry: &'interner MacroRegistry) {
        self.macro_registry = Some(registry);
    }

    /// Sets a global variable.
    ///
    /// Used to register built-in functions as globals.
    #[inline]
    pub fn set_global(&mut self, symbol: symbol::Id, value: Value) {
        self.globals.set(symbol, value);
    }

    /// Registers a primitive in the `lona.core` namespace.
    ///
    /// This creates a Var for the primitive in `lona.core`, making it available
    /// for auto-refer into other namespaces. Also sets the value in globals
    /// for backward compatibility with the native function dispatch.
    ///
    /// Call [`namespace_registry_mut().refer_core_to_all()`](crate::namespace::Registry::refer_core_to_all)
    /// after registering all primitives to propagate them to other namespaces.
    #[inline]
    pub fn register_core_primitive(&mut self, symbol: symbol::Id, value: Value) {
        // Set in globals for native function dispatch
        self.globals.set(symbol, value.clone());

        // Intern as a Var in lona.core namespace
        let core_name = self.namespace_registry.core_name();
        if let Some(core_ns) = self.namespace_registry.get_mut(core_name) {
            let _var = core_ns.intern(symbol, value);
        }
    }

    /// Returns a reference to the symbol interner.
    #[inline]
    #[must_use]
    pub const fn interner(&self) -> &'interner Interner {
        self.interner
    }

    /// Returns a mutable reference to the global variables.
    ///
    /// Use this to register native functions or set initial global values.
    #[inline]
    #[must_use]
    pub const fn globals_mut(&mut self) -> &mut Globals {
        &mut self.globals
    }

    /// Returns a reference to the global variables.
    #[inline]
    #[must_use]
    pub const fn globals(&self) -> &Globals {
        &self.globals
    }

    /// Returns the current source ID for error reporting.
    #[inline]
    #[must_use]
    pub const fn current_source(&self) -> source::Id {
        self.current_source
    }

    /// Sets the current source ID for error reporting.
    ///
    /// Call this before executing code from a specific source.
    #[inline]
    pub const fn set_source(&mut self, source: source::Id) {
        self.current_source = source;
    }

    /// Returns the current namespace symbol.
    ///
    /// This reflects the namespace set by the most recent `(ns ...)` form
    /// executed. Defaults to "user" for new VMs.
    #[inline]
    #[must_use]
    pub const fn current_namespace(&self) -> symbol::Id {
        self.current_namespace
    }

    /// Sets the current namespace for symbol resolution.
    ///
    /// This is used by the REPL to restore the namespace context between
    /// evaluations. Also updates the namespace registry's current namespace.
    #[inline]
    pub fn set_current_namespace(&mut self, ns: symbol::Id) {
        self.current_namespace = ns;
        self.namespace_registry.switch_to(ns);
    }

    /// Returns a reference to the namespace registry.
    #[inline]
    #[must_use]
    pub const fn namespace_registry(&self) -> &NamespaceRegistry {
        &self.namespace_registry
    }

    /// Returns a mutable reference to the namespace registry.
    #[inline]
    #[must_use]
    pub const fn namespace_registry_mut(&mut self) -> &mut NamespaceRegistry {
        &mut self.namespace_registry
    }

    /// Prepares to load a namespace, performing all pre-load checks.
    ///
    /// Returns `Ok(None)` if already loaded, `Ok(Some(source))` if loading needed.
    /// After calling, caller should: `push_loading` → compile/execute → `pop_loading`.
    ///
    /// # Errors
    ///
    /// Returns error for: circular dependency, no loader configured, namespace not found.
    #[inline]
    pub fn prepare_require<'source>(
        &self,
        ns_name: symbol::Id,
    ) -> Result<Option<&'source str>, Error>
    where
        'interner: 'source,
    {
        // 1. Check if namespace is already loaded
        if self.namespace_registry.contains(ns_name) {
            return Ok(None);
        }

        // 2. Check for circular dependency
        if self.loading_stack.contains(&ns_name) {
            return Err(Error::new(
                ErrorKind::CircularDependency {
                    namespace: ns_name,
                    stack: self.loading_stack.clone(),
                },
                source::Location::new(self.current_source, lona_core::span::Span::default()),
            ));
        }

        // 3. Check if source loader is configured
        let Some(loader) = self.loader else {
            return Err(Error::new(
                ErrorKind::NoSourceLoader,
                source::Location::new(self.current_source, lona_core::span::Span::default()),
            ));
        };

        // 4. Load the source code
        let ns_str = self.interner.resolve(ns_name);
        let Some(source_code) = loader.load_source(&ns_str) else {
            return Err(Error::new(
                ErrorKind::NamespaceNotFound { namespace: ns_name },
                source::Location::new(self.current_source, lona_core::span::Span::default()),
            ));
        };

        Ok(Some(source_code))
    }

    /// Executes a chunk of bytecode and returns the result.
    ///
    /// Uses a trampoline loop to handle tail calls without growing the Rust
    /// stack. When a function returns `TailCall`, the trampoline replaces the
    /// current frame and continues execution instead of making a recursive call.
    ///
    /// # Errors
    ///
    /// Returns an error if execution fails due to:
    /// - Invalid opcodes
    /// - Type errors in operations
    /// - Undefined global variables
    /// - Division by zero
    /// - Stack overflow
    #[inline]
    pub fn execute(&mut self, chunk: &Chunk) -> Result<Value, Error> {
        use alloc::sync::Arc;

        // Reset registers to nil
        for reg in &mut self.registers {
            *reg = Value::Nil;
        }

        // Reset call depth
        self.call_depth = 0;

        // Store chunk in Arc for uniform handling with tail calls.
        // This allows the trampoline to replace the chunk when a tail call occurs.
        let mut current_chunk: Arc<Chunk> = Arc::new(chunk.clone());
        let mut current_source = self.current_source;
        let mut current_upvalues: Arc<[Value]> = Arc::from([]);

        // Trampoline loop - handles tail calls without growing Rust stack
        loop {
            // Create frame in inner scope so it's dropped before we update current_chunk
            let result = {
                let mut frame = Frame::with_upvalues(
                    &current_chunk,
                    0,
                    current_source,
                    Arc::clone(&current_upvalues),
                );
                self.run(&mut frame)?
            };

            match result {
                DispatchResult::Continue => {
                    // run() should never return Continue - it's only used internally.
                    // If this happens, it's an internal VM bug.
                    return Err(Error::new(
                        ErrorKind::NotImplemented {
                            feature: "internal error: run() returned Continue",
                        },
                        source::Location::new(current_source, lona_core::span::Span::default()),
                    ));
                }
                DispatchResult::Return(value) => return Ok(value),
                DispatchResult::TailCall(data) => {
                    // Get location for error reporting before consuming data
                    let location =
                        source::Location::new(data.source(), lona_core::span::Span::default());

                    // Set up registers and get new frame parameters
                    // Base is 0 for top-level execution
                    let setup = self.setup_tail_call(data, 0, location)?;

                    // Update frame parameters for next iteration
                    // Note: call_depth is NOT incremented - that's the key to TCO
                    (current_chunk, current_source, current_upvalues) = setup.into_parts();
                }
            }
        }
    }

    /// Executes a chunk of bytecode with a specific source ID.
    ///
    /// This is a convenience method that sets the source ID before execution.
    ///
    /// # Errors
    ///
    /// Returns an error if execution fails.
    #[inline]
    pub fn execute_with_source(
        &mut self,
        chunk: &Chunk,
        source: source::Id,
    ) -> Result<Value, Error> {
        self.set_source(source);
        self.execute(chunk)
    }

    /// Executes a chunk of bytecode with initial argument values.
    ///
    /// The arguments are placed in registers R\[0\], R\[1\], ..., R\[n-1\] before
    /// execution begins. This is used for macro expansion where the macro
    /// transformer receives its arguments as register values.
    ///
    /// Uses the same trampoline loop as `execute()` for tail call handling.
    ///
    /// # Errors
    ///
    /// Returns an error if execution fails due to:
    /// - Invalid opcodes
    /// - Type errors in operations
    /// - Undefined global variables
    /// - Division by zero
    /// - Stack overflow
    #[inline]
    pub fn execute_with_args(&mut self, chunk: &Chunk, args: &[Value]) -> Result<Value, Error> {
        use alloc::sync::Arc;

        // Reset registers to nil
        for reg in &mut self.registers {
            *reg = Value::Nil;
        }

        // Set up argument registers
        for (idx, arg) in args.iter().enumerate() {
            if let Some(reg) = self.registers.get_mut(idx) {
                *reg = arg.clone();
            }
        }

        // Reset call depth
        self.call_depth = 0;

        // Store chunk in Arc for uniform handling with tail calls
        let mut current_chunk: Arc<Chunk> = Arc::new(chunk.clone());
        let mut current_source = self.current_source;
        let mut current_upvalues: Arc<[Value]> = Arc::from([]);

        // Trampoline loop - handles tail calls without growing Rust stack
        loop {
            // Create frame in inner scope so it's dropped before we update current_chunk
            let result = {
                let mut frame = Frame::with_upvalues(
                    &current_chunk,
                    0,
                    current_source,
                    Arc::clone(&current_upvalues),
                );
                self.run(&mut frame)?
            };

            match result {
                DispatchResult::Continue => {
                    // run() should never return Continue - it's only used internally.
                    // If this happens, it's an internal VM bug.
                    return Err(Error::new(
                        ErrorKind::NotImplemented {
                            feature: "internal error: run() returned Continue",
                        },
                        source::Location::new(current_source, lona_core::span::Span::default()),
                    ));
                }
                DispatchResult::Return(value) => return Ok(value),
                DispatchResult::TailCall(data) => {
                    // Get location for error reporting before consuming data
                    let location =
                        source::Location::new(data.source(), lona_core::span::Span::default());

                    // Set up registers and get new frame parameters
                    let setup = self.setup_tail_call(data, 0, location)?;

                    // Update frame parameters for next iteration
                    (current_chunk, current_source, current_upvalues) = setup.into_parts();
                }
            }
        }
    }

    /// Main execution loop.
    ///
    /// Returns a `DispatchResult` indicating how the function terminated:
    /// - `Return(value)`: Function returned normally with a value
    /// - `TailCall(data)`: Function wants to tail-call another function
    ///
    /// The `Continue` variant is never returned from `run()` - it's only used
    /// internally to continue the dispatch loop.
    pub(super) fn run(&mut self, frame: &mut Frame<'_>) -> Result<DispatchResult, Error> {
        loop {
            let Some(instruction) = frame.fetch() else {
                // End of bytecode - return nil by default
                return Ok(DispatchResult::Return(Value::Nil));
            };

            let Some(opcode) = decode_op(instruction) else {
                return Err(Error::new(
                    ErrorKind::InvalidOpcode {
                        byte: decode_opcode_byte(instruction),
                        pc: frame.pc().saturating_sub(1),
                    },
                    frame.current_location(),
                ));
            };

            match self.dispatch(opcode, instruction, frame)? {
                DispatchResult::Continue => {}
                result @ (DispatchResult::Return(_) | DispatchResult::TailCall(_)) => {
                    return Ok(result);
                }
            }
        }
    }

    /// Dispatches a single opcode and returns the result.
    #[expect(
        clippy::cognitive_complexity,
        reason = "[approved] Large switch over all opcodes is inherent to the dispatch design"
    )]
    fn dispatch(
        &mut self,
        opcode: Opcode,
        instruction: u32,
        frame: &mut Frame<'_>,
    ) -> Result<DispatchResult, Error> {
        match opcode {
            // Data Movement
            Opcode::Move => self.op_move(instruction, frame)?,
            Opcode::LoadK => self.op_load_k(instruction, frame)?,
            Opcode::LoadNil => self.op_load_nil(instruction, frame),
            Opcode::LoadTrue => self.op_load_true(instruction, frame)?,
            Opcode::LoadFalse => self.op_load_false(instruction, frame)?,

            // Globals
            Opcode::GetGlobal => self.op_get_global(instruction, frame)?,
            Opcode::SetGlobal => self.op_set_global(instruction, frame)?,
            Opcode::SetGlobalMeta => self.op_set_global_meta(instruction, frame)?,
            Opcode::GetGlobalVar => self.op_get_global_var(instruction, frame)?,

            // Arithmetic
            Opcode::Add => self.op_add(instruction, frame)?,
            Opcode::Sub => self.op_sub(instruction, frame)?,
            Opcode::Mul => self.op_mul(instruction, frame)?,
            Opcode::Div => self.op_div(instruction, frame)?,
            Opcode::Mod => self.op_mod(instruction, frame)?,
            Opcode::Neg => self.op_neg(instruction, frame)?,

            // Comparison
            Opcode::Eq => self.op_eq(instruction, frame)?,
            Opcode::Lt => self.op_lt(instruction, frame)?,
            Opcode::Le => self.op_le(instruction, frame)?,
            Opcode::Gt => self.op_gt(instruction, frame)?,
            Opcode::Ge => self.op_ge(instruction, frame)?,
            Opcode::Not => self.op_not(instruction, frame)?,

            // Control Flow
            Opcode::Jump => Self::op_jump(instruction, frame),
            Opcode::JumpIf => self.op_jump_if(instruction, frame)?,
            Opcode::JumpIfNot => self.op_jump_if_not(instruction, frame)?,

            // Function Calls
            Opcode::Call => {
                self.op_call(instruction, frame)?;
            }
            Opcode::TailCall => {
                return self.op_tail_call(instruction, frame);
            }
            Opcode::Return => {
                let dest = decode_a(instruction);
                let count = decode_b(instruction);
                return Ok(DispatchResult::Return(self.op_return(dest, count, frame)?));
            }

            // Closure Operations
            Opcode::GetUpvalue => self.op_get_upvalue(instruction, frame)?,
            Opcode::Closure => self.op_closure(instruction, frame)?,

            // Pattern Matching
            Opcode::CaseFail => {
                return self.op_case_fail(instruction, frame);
            }

            // Namespace Operations
            Opcode::SetNamespace => self.op_set_namespace(instruction, frame)?,

            // Handle future Opcode variants (Opcode is #[non_exhaustive])
            _ => {
                return Err(Error::new(
                    ErrorKind::InvalidOpcode {
                        byte: decode_opcode_byte(instruction),
                        pc: frame.pc().saturating_sub(1),
                    },
                    frame.current_location(),
                ));
            }
        }

        Ok(DispatchResult::Continue)
    }
}
