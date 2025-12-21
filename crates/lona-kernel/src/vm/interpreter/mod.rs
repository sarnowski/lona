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
        // Reset registers to nil
        for reg in &mut self.registers {
            *reg = Value::Nil;
        }

        // Reset call depth
        self.call_depth = 0;

        let mut frame = Frame::new(chunk, 0, self.current_source);
        self.run(&mut frame)
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
    /// The arguments are placed in registers R[0], R[1], ..., R[n-1] before
    /// execution begins. This is used for macro expansion where the macro
    /// transformer receives its arguments as register values.
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

        let mut frame = Frame::new(chunk, 0, self.current_source);
        self.run(&mut frame)
    }

    /// Main execution loop.
    pub(super) fn run(&mut self, frame: &mut Frame<'_>) -> Result<Value, Error> {
        loop {
            let Some(instruction) = frame.fetch() else {
                // End of bytecode - return nil by default
                return Ok(Value::Nil);
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
                    // TailCall will be fully implemented in Phase 4
                    // For now, treat as regular call
                    self.op_call(instruction, frame)?;
                }
                Opcode::Return => {
                    let dest = decode_a(instruction);
                    let count = decode_b(instruction);
                    return self.op_return(dest, count, frame);
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
        }
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
                Value::Function(Function::new(fn_bodies, name.clone()))
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
        Constant::Function { .. } | _ => value::Kind::Function,
    }
}
