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

use super::error::{Error, Kind as ErrorKind};
use super::frame::Frame;
use super::globals::Globals;
use super::natives::{NativeFn, Registry as NativeRegistry};

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
    /// Optional macro registry for introspection functions.
    /// Passed to native functions via `NativeContext`.
    macro_registry: Option<&'interner MacroRegistry>,
    /// Current call depth for stack overflow protection.
    call_depth: usize,
    /// Current source ID for error reporting.
    current_source: source::Id,
}

impl<'interner> Vm<'interner> {
    /// Creates a new virtual machine.
    #[inline]
    #[must_use]
    pub fn new(interner: &'interner Interner) -> Self {
        Self {
            registers: vec![Value::Nil; DEFAULT_REGISTER_COUNT],
            globals: Globals::new(),
            interner,
            natives: NativeRegistry::new(),
            macro_registry: None,
            call_depth: 0,
            current_source: source::Id::new(0_u32),
        }
    }

    /// Registers a native function for a symbol.
    ///
    /// The symbol must already be interned in the interner passed to [`Vm::new`].
    #[inline]
    pub fn register_native(&mut self, symbol: symbol::Id, func: NativeFn) {
        self.natives.register(symbol, func);
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
