// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Virtual machine interpreter for Lonala bytecode.

use alloc::vec;
use alloc::vec::Vec;

use lona_core::chunk::{Chunk, Constant};
use lona_core::error_context::TypeExpectation;
use lona_core::integer::Integer;
use lona_core::opcode::{
    Opcode, decode_a, decode_b, decode_op, decode_opcode_byte, rk_index, rk_is_constant,
};
use lona_core::source;
use lona_core::symbol::{self, Interner};
use lona_core::value::{self, Function, Value};
use lonala_compiler::MacroRegistry;

use super::error::{Error, Kind as ErrorKind};
use super::frame::Frame;
use super::globals::Globals;
use super::natives::{NativeFn, Registry as NativeRegistry};

mod ops_arithmetic;
mod ops_control;
mod ops_data;

/// Maximum call stack depth to prevent stack overflow.
pub(super) const MAX_CALL_DEPTH: usize = 256;

/// Default register file size.
const DEFAULT_REGISTER_COUNT: usize = 256;

/// Result of dispatching a single opcode.
///
/// Used internally by the interpreter to control execution flow.
pub(super) enum DispatchResult {
    /// Continue to the next instruction.
    Continue,
    /// Return from the current function with a value.
    Return(Value),
    /// Perform a tail call (replace current frame instead of pushing new one).
    ///
    /// When a function call is in tail position, returning this variant
    /// signals to the trampoline to replace the current frame and continue
    /// execution without growing the Rust stack.
    TailCall(TailCallData),
}

/// Data needed to perform a tail call without growing the stack.
///
/// When a function call is in tail position, instead of recursively calling
/// `run()`, we return this data to the trampoline loop which replaces the
/// current frame and continues execution.
pub(super) struct TailCallData {
    /// The function to call.
    function: Function,
    /// Arguments to pass to the function.
    arguments: Vec<Value>,
    /// Source ID for error reporting.
    source: source::Id,
}

impl TailCallData {
    /// Creates new tail call data.
    #[inline]
    pub(super) const fn new(function: Function, arguments: Vec<Value>, source: source::Id) -> Self {
        Self {
            function,
            arguments,
            source,
        }
    }

    /// Returns the source ID.
    #[inline]
    pub(super) const fn source(&self) -> source::Id {
        self.source
    }
}

/// Result of setting up a tail call frame.
///
/// Contains all data needed to create a new frame for a tail call.
pub(super) struct TailCallSetup {
    /// The chunk to execute.
    chunk: alloc::sync::Arc<Chunk>,
    /// Source ID for error reporting.
    source: source::Id,
    /// Captured upvalues for the function.
    upvalues: alloc::sync::Arc<[Value]>,
}

impl TailCallSetup {
    /// Returns the chunk Arc, consuming self.
    #[inline]
    pub(super) fn into_parts(
        self,
    ) -> (
        alloc::sync::Arc<Chunk>,
        source::Id,
        alloc::sync::Arc<[Value]>,
    ) {
        (self.chunk, self.source, self.upvalues)
    }
}

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

    /// Sets up registers and returns frame parameters for a tail call.
    ///
    /// This is the core of tail call optimization. Instead of creating a
    /// new frame on the stack, we:
    /// 1. Find the matching arity body for the function being called
    /// 2. Handle rest parameters (collect extra args into a list)
    /// 3. Set up argument registers at the given base
    /// 4. Return the chunk and upvalues for the new frame
    ///
    /// The caller (trampoline) uses these to create a replacement frame.
    ///
    /// # Parameters
    /// - `data`: The tail call data containing function and arguments
    /// - `base`: The base register index for the new frame's arguments
    /// - `location`: Source location for error reporting
    ///
    /// # Returns
    /// A `TailCallSetup` containing the chunk, source ID, and upvalues for the new frame.
    pub(super) fn setup_tail_call(
        &mut self,
        data: TailCallData,
        base: usize,
        location: source::Location,
    ) -> Result<TailCallSetup, Error> {
        use lona_core::error_context::ArityExpectation;
        use lona_core::list::List;

        let argc = data.arguments.len();

        // Find matching arity body
        let body = data.function.find_body(argc).ok_or_else(|| {
            // Build arity expectation for error message
            let bodies = data.function.bodies();
            let expectation = if let &[ref first] = bodies {
                if first.has_rest() {
                    ArityExpectation::AtLeast(first.arity())
                } else {
                    ArityExpectation::Exact(first.arity())
                }
            } else {
                bodies
                    .first()
                    .map_or(ArityExpectation::Exact(0), |first_body| {
                        ArityExpectation::Exact(first_body.arity())
                    })
            };
            Error::new(
                ErrorKind::ArityMismatch {
                    callable: None,
                    expected: expectation,
                    got: u8::try_from(argc).unwrap_or(u8::MAX),
                },
                location,
            )
        })?;

        let fixed_arity = body.arity();
        let has_rest = body.has_rest();

        // Build the effective arguments list
        // Fixed args: arguments[0..fixed_arity]
        // Rest arg (if has_rest): arguments[fixed_arity..] collected into a list
        let effective_args: alloc::vec::Vec<Value> = if has_rest {
            let fixed_arity_usize = usize::from(fixed_arity);
            let mut effective = alloc::vec::Vec::with_capacity(fixed_arity_usize.saturating_add(1));
            // Add fixed arguments
            for arg in data.arguments.iter().take(fixed_arity_usize) {
                effective.push(arg.clone());
            }
            // Collect rest arguments into a list
            let rest_elements: alloc::vec::Vec<Value> = data
                .arguments
                .iter()
                .skip(fixed_arity_usize)
                .cloned()
                .collect();
            effective.push(Value::List(List::from_vec(rest_elements)));
            effective
        } else {
            data.arguments
        };

        // Ensure we have enough registers
        let needed_registers = base.saturating_add(usize::from(body.chunk().max_registers()));
        if needed_registers > self.registers.len() {
            self.registers.resize(needed_registers, Value::Nil);
        }

        // Set up argument registers (R[0], R[1], ... relative to base)
        for (idx, arg) in effective_args.into_iter().enumerate() {
            let reg_idx = base.saturating_add(idx);
            if let Some(reg) = self.registers.get_mut(reg_idx) {
                *reg = arg;
            }
        }

        Ok(TailCallSetup {
            chunk: alloc::sync::Arc::clone(body.chunk_arc()),
            source: data.source,
            upvalues: alloc::sync::Arc::clone(data.function.upvalues_arc()),
        })
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

    // =========================================================================
    // Helper Methods
    // =========================================================================

    /// Gets a value from a register.
    pub(super) fn get_register(&self, index: u8, frame: &Frame<'_>) -> Result<Value, Error> {
        let absolute_index = frame.base().saturating_add(usize::from(index));
        self.registers.get(absolute_index).cloned().ok_or_else(|| {
            Error::new(
                ErrorKind::InvalidRegister { index },
                frame.current_location(),
            )
        })
    }

    /// Sets a value in a register.
    pub(super) fn set_register(
        &mut self,
        index: u8,
        value: Value,
        frame: &Frame<'_>,
    ) -> Result<(), Error> {
        let absolute_index = frame.base().saturating_add(usize::from(index));
        let reg = self.registers.get_mut(absolute_index).ok_or_else(|| {
            Error::new(
                ErrorKind::InvalidRegister { index },
                frame.current_location(),
            )
        })?;
        *reg = value;
        Ok(())
    }

    /// Gets a value from an RK field (register or constant).
    ///
    /// Note: Function constants are not expected in RK operands, so this
    /// method uses the static conversion that doesn't handle them.
    pub(super) fn get_rk(&self, rk: u8, frame: &Frame<'_>) -> Result<Value, Error> {
        if rk_is_constant(rk) {
            let const_index = u16::from(rk_index(rk));
            Self::rk_constant_to_value(frame.chunk(), const_index, frame)
        } else {
            self.get_register(rk, frame)
        }
    }

    /// Loads a constant and converts it to a value.
    ///
    /// Handles function constants by creating a `Value::Function` with an
    /// `Arc<Chunk>` for the function's bytecode.
    pub(super) fn load_constant(
        chunk: &Chunk,
        index: u16,
        frame: &Frame<'_>,
    ) -> Result<Value, Error> {
        let constant = chunk.get_constant(index).ok_or_else(|| {
            Error::new(
                ErrorKind::InvalidConstant { index },
                frame.current_location(),
            )
        })?;

        Self::convert_constant(constant)
    }

    /// Converts a constant to a value, handling function constants.
    pub(super) fn convert_constant(constant: &Constant) -> Result<Value, Error> {
        Ok(match *constant {
            Constant::Bool(val) => Value::Bool(val),
            Constant::Integer(num) => Value::Integer(Integer::from_i64(num)),
            Constant::Float(num) => Value::Float(num),
            Constant::Symbol(id) => Value::from(id),
            Constant::Keyword(id) => Value::Keyword(id),
            Constant::String(ref text) => {
                Value::String(lona_core::string::HeapStr::from(text.as_str()))
            }
            Constant::List(ref elements) => {
                let values: Result<alloc::vec::Vec<Value>, Error> =
                    elements.iter().map(Self::convert_constant).collect();
                Value::List(lona_core::list::List::from_vec(values?))
            }
            Constant::Vector(ref elements) => {
                let values: Result<alloc::vec::Vec<Value>, Error> =
                    elements.iter().map(Self::convert_constant).collect();
                Value::Vector(lona_core::vector::Vector::from_vec(values?))
            }
            Constant::Map(ref pairs) => {
                let converted_pairs: Result<alloc::vec::Vec<(Value, Value)>, Error> = pairs
                    .iter()
                    .map(|&(ref key, ref val)| {
                        let key_val = Self::convert_constant(key)?;
                        let val_val = Self::convert_constant(val)?;
                        Ok((key_val, val_val))
                    })
                    .collect();
                Value::Map(lona_core::map::Map::from_pairs(converted_pairs?))
            }
            Constant::Function {
                ref bodies,
                ref name,
            } => {
                // Convert each FunctionBodyData to FunctionBody
                use lona_core::value::FunctionBody;
                let fn_bodies: alloc::vec::Vec<FunctionBody> = bodies
                    .iter()
                    .map(|body| {
                        let chunk_arc = alloc::sync::Arc::new((*body.chunk).clone());
                        FunctionBody::new(chunk_arc, body.arity, body.has_rest)
                    })
                    .collect();
                Value::Function(Function::new_simple(fn_bodies, name.clone()))
            }
            // Handle Nil and future Constant variants (Constant is #[non_exhaustive])
            Constant::Nil | _ => Value::Nil,
        })
    }

    /// Converts a constant pool entry to a value (static version for RK operands).
    ///
    /// Does not handle function constants since they are not expected in RK positions.
    pub(super) fn rk_constant_to_value(
        chunk: &Chunk,
        index: u16,
        frame: &Frame<'_>,
    ) -> Result<Value, Error> {
        let constant = chunk.get_constant(index).ok_or_else(|| {
            Error::new(
                ErrorKind::InvalidConstant { index },
                frame.current_location(),
            )
        })?;

        Self::convert_simple_constant(constant)
    }

    /// Converts a simple (non-function) constant to a value.
    pub(super) fn convert_simple_constant(constant: &Constant) -> Result<Value, Error> {
        Ok(match *constant {
            Constant::Bool(val) => Value::Bool(val),
            Constant::Integer(num) => Value::Integer(Integer::from_i64(num)),
            Constant::Float(num) => Value::Float(num),
            Constant::Symbol(id) => Value::from(id),
            Constant::Keyword(id) => Value::Keyword(id),
            Constant::String(ref text) => {
                Value::String(lona_core::string::HeapStr::from(text.as_str()))
            }
            Constant::List(ref elements) => {
                let values: Result<alloc::vec::Vec<Value>, Error> =
                    elements.iter().map(Self::convert_simple_constant).collect();
                Value::List(lona_core::list::List::from_vec(values?))
            }
            Constant::Vector(ref elements) => {
                let values: Result<alloc::vec::Vec<Value>, Error> =
                    elements.iter().map(Self::convert_simple_constant).collect();
                Value::Vector(lona_core::vector::Vector::from_vec(values?))
            }
            Constant::Map(ref pairs) => {
                let converted_pairs: Result<alloc::vec::Vec<(Value, Value)>, Error> = pairs
                    .iter()
                    .map(|&(ref key, ref val)| {
                        let key_val = Self::convert_simple_constant(key)?;
                        let val_val = Self::convert_simple_constant(val)?;
                        Ok((key_val, val_val))
                    })
                    .collect();
                Value::Map(lona_core::map::Map::from_pairs(converted_pairs?))
            }
            // Handle Nil, Function, and future Constant variants
            Constant::Nil | Constant::Function { .. } | _ => Value::Nil,
        })
    }

    /// Gets a symbol ID from a constant pool entry.
    pub(super) fn get_symbol_from_constant(
        chunk: &Chunk,
        index: u16,
        frame: &Frame<'_>,
    ) -> Result<symbol::Id, Error> {
        let constant = chunk.get_constant(index).ok_or_else(|| {
            Error::new(
                ErrorKind::InvalidConstant { index },
                frame.current_location(),
            )
        })?;

        match *constant {
            Constant::Symbol(id) => Ok(id),
            Constant::Nil
            | Constant::Bool(_)
            | Constant::Integer(_)
            | Constant::Float(_)
            | Constant::String(_)
            | Constant::Keyword(_)
            | Constant::List(_)
            | Constant::Vector(_)
            | _ => Err(Error::new(
                ErrorKind::TypeError {
                    operation: "symbol lookup",
                    expected: TypeExpectation::Symbol,
                    got: constant_type_name_to_kind(constant),
                    operand: None,
                },
                frame.current_location(),
            )),
        }
    }
}

/// Converts a constant type name to a `value::Kind` for error reporting.
const fn constant_type_name_to_kind(constant: &Constant) -> value::Kind {
    match *constant {
        Constant::Nil => value::Kind::Nil,
        Constant::Bool(_) => value::Kind::Bool,
        Constant::Integer(_) => value::Kind::Integer,
        Constant::Float(_) => value::Kind::Float,
        Constant::String(_) => value::Kind::String,
        Constant::Symbol(_) => value::Kind::Symbol,
        Constant::Keyword(_) => value::Kind::Keyword,
        Constant::List(_) => value::Kind::List,
        Constant::Vector(_) => value::Kind::Vector,
        Constant::Map(_) => value::Kind::Map,
        Constant::Function { .. } | _ => value::Kind::Function,
    }
}
