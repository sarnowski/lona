// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Function and closure allocation methods for Process.

use crate::Vaddr;
use crate::platform::MemorySpace;
use crate::value::{HeapClosure, HeapCompiledFn, Value};

use super::Process;

impl Process {
    /// Allocate a compiled function on the young heap.
    ///
    /// A compiled function is a pure function (no captures) with bytecode and constants.
    ///
    /// Returns a `Value::CompiledFn` pointing to the allocated function, or `None` if OOM.
    pub fn alloc_compiled_fn<M: MemorySpace>(
        &mut self,
        mem: &mut M,
        arity: u8,
        variadic: bool,
        num_locals: u8,
        code: &[u32],
        constants: &[Value],
    ) -> Option<Value> {
        let code_len = code.len();
        let constants_len = constants.len();
        let total_size = HeapCompiledFn::alloc_size(code_len, constants_len);

        // Allocate space (align to 8 bytes for Values in constant pool)
        let addr = self.alloc(total_size, 8)?;

        // Write header
        let header = HeapCompiledFn {
            arity,
            variadic,
            num_locals,
            padding: 0,
            code_len: code_len as u32,
            constants_len: constants_len as u32,
            source_line: 0, // Source tracking: Phase 5, task 5.14
            padding2: 0,
            source_file: Vaddr::null(), // Source tracking: Phase 5, task 5.14
        };
        mem.write(addr, header);

        // Write bytecode
        let code_addr = addr.add(HeapCompiledFn::bytecode_offset() as u64);
        for (i, &instr) in code.iter().enumerate() {
            let instr_addr = code_addr.add((i * core::mem::size_of::<u32>()) as u64);
            mem.write(instr_addr, instr);
        }

        // Write constants
        let constants_addr = addr.add(HeapCompiledFn::constants_offset(code_len) as u64);
        for (i, &constant) in constants.iter().enumerate() {
            let const_addr = constants_addr.add((i * core::mem::size_of::<Value>()) as u64);
            mem.write(const_addr, constant);
        }

        Some(Value::compiled_fn(addr))
    }

    /// Read a compiled function's header from the heap.
    ///
    /// Returns `None` if the value is not a compiled function.
    #[must_use]
    pub fn read_compiled_fn<M: MemorySpace>(
        &self,
        mem: &M,
        value: Value,
    ) -> Option<HeapCompiledFn> {
        let Value::CompiledFn(addr) = value else {
            return None;
        };
        Some(mem.read(addr))
    }

    /// Read a compiled function's bytecode instruction at the given index.
    ///
    /// Returns `None` if the value is not a compiled function or index is out of bounds.
    #[must_use]
    pub fn read_compiled_fn_code<M: MemorySpace>(
        &self,
        mem: &M,
        value: Value,
        index: usize,
    ) -> Option<u32> {
        let Value::CompiledFn(addr) = value else {
            return None;
        };

        let header: HeapCompiledFn = mem.read(addr);
        if index >= header.code_len as usize {
            return None;
        }

        let code_addr = addr.add(HeapCompiledFn::bytecode_offset() as u64);
        let instr_addr = code_addr.add((index * core::mem::size_of::<u32>()) as u64);
        Some(mem.read(instr_addr))
    }

    /// Read a compiled function's constant at the given index.
    ///
    /// Returns `None` if the value is not a compiled function or index is out of bounds.
    #[must_use]
    pub fn read_compiled_fn_constant<M: MemorySpace>(
        &self,
        mem: &M,
        value: Value,
        index: usize,
    ) -> Option<Value> {
        let Value::CompiledFn(addr) = value else {
            return None;
        };

        let header: HeapCompiledFn = mem.read(addr);
        if index >= header.constants_len as usize {
            return None;
        }

        let constants_addr =
            addr.add(HeapCompiledFn::constants_offset(header.code_len as usize) as u64);
        let const_addr = constants_addr.add((index * core::mem::size_of::<Value>()) as u64);
        Some(mem.read(const_addr))
    }

    /// Allocate a closure on the young heap.
    ///
    /// A closure is a function paired with captured values from its environment.
    ///
    /// Returns a `Value::Closure` pointing to the allocated closure, or `None` if OOM.
    pub fn alloc_closure<M: MemorySpace>(
        &mut self,
        mem: &mut M,
        function: Vaddr,
        captures: &[Value],
    ) -> Option<Value> {
        let captures_len = captures.len();
        let total_size = HeapClosure::alloc_size(captures_len);

        // Allocate space (align to 8 bytes for Values)
        let addr = self.alloc(total_size, 8)?;

        // Write header
        let header = HeapClosure {
            function,
            captures_len: captures_len as u32,
            padding: 0,
        };
        mem.write(addr, header);

        // Write captures
        let captures_addr = addr.add(HeapClosure::captures_offset() as u64);
        for (i, &capture) in captures.iter().enumerate() {
            let cap_addr = captures_addr.add((i * core::mem::size_of::<Value>()) as u64);
            mem.write(cap_addr, capture);
        }

        Some(Value::closure(addr))
    }

    /// Read a closure's header from the heap.
    ///
    /// Returns `None` if the value is not a closure.
    #[must_use]
    pub fn read_closure<M: MemorySpace>(&self, mem: &M, value: Value) -> Option<HeapClosure> {
        let Value::Closure(addr) = value else {
            return None;
        };
        Some(mem.read(addr))
    }

    /// Read a closure's captured value at the given index.
    ///
    /// Returns `None` if the value is not a closure or index is out of bounds.
    #[must_use]
    pub fn read_closure_capture<M: MemorySpace>(
        &self,
        mem: &M,
        value: Value,
        index: usize,
    ) -> Option<Value> {
        let Value::Closure(addr) = value else {
            return None;
        };

        let header: HeapClosure = mem.read(addr);
        if index >= header.captures_len as usize {
            return None;
        }

        let captures_addr = addr.add(HeapClosure::captures_offset() as u64);
        let cap_addr = captures_addr.add((index * core::mem::size_of::<Value>()) as u64);
        Some(mem.read(cap_addr))
    }

    /// Get the underlying function address from a closure.
    ///
    /// Returns `None` if the value is not a closure.
    #[must_use]
    pub fn read_closure_function<M: MemorySpace>(&self, mem: &M, value: Value) -> Option<Vaddr> {
        let Value::Closure(addr) = value else {
            return None;
        };

        let header: HeapClosure = mem.read(addr);
        Some(header.function)
    }

    // --- Deep copy methods (Tasks 3.29, 3.30) ---

    /// Copy a compiled function to a new location on the heap.
    ///
    /// Creates a complete copy of the function including its bytecode and constants.
    /// The copy is independent of the original - modifying one does not affect the other.
    ///
    /// This is used for:
    /// - Copying functions to realm code region (Phase 5)
    /// - Process spawning (copying functions to child process)
    ///
    /// Returns a new `Value::CompiledFn` pointing to the copy, or `None` if OOM.
    pub fn copy_compiled_fn<M: MemorySpace>(&mut self, mem: &mut M, src: Value) -> Option<Value> {
        let Value::CompiledFn(src_addr) = src else {
            return None;
        };

        // Read source header
        let header: HeapCompiledFn = mem.read(src_addr);

        // Calculate sizes
        let code_len = header.code_len as usize;
        let constants_len = header.constants_len as usize;
        let total_size = HeapCompiledFn::alloc_size(code_len, constants_len);

        // Allocate destination
        let dst_addr = self.alloc(total_size, 8)?;

        // Copy header
        mem.write(dst_addr, header);

        // Copy bytecode
        let src_code_addr = src_addr.add(HeapCompiledFn::bytecode_offset() as u64);
        let dst_code_addr = dst_addr.add(HeapCompiledFn::bytecode_offset() as u64);
        for i in 0..code_len {
            let offset = (i * core::mem::size_of::<u32>()) as u64;
            let instr: u32 = mem.read(src_code_addr.add(offset));
            mem.write(dst_code_addr.add(offset), instr);
        }

        // Copy constants
        let src_constants_addr = src_addr.add(HeapCompiledFn::constants_offset(code_len) as u64);
        let dst_constants_addr = dst_addr.add(HeapCompiledFn::constants_offset(code_len) as u64);
        for i in 0..constants_len {
            let offset = (i * core::mem::size_of::<Value>()) as u64;
            let constant: Value = mem.read(src_constants_addr.add(offset));
            mem.write(dst_constants_addr.add(offset), constant);
        }

        Some(Value::compiled_fn(dst_addr))
    }

    /// Copy a closure to a new location on the heap.
    ///
    /// Creates a complete copy of the closure including:
    /// - A deep copy of the underlying compiled function
    /// - A copy of all captured values
    ///
    /// This is used for:
    /// - Copying closures to realm code region (Phase 5)
    /// - Process spawning (copying closures to child process)
    ///
    /// Returns a new `Value::Closure` pointing to the copy, or `None` if OOM.
    pub fn copy_closure<M: MemorySpace>(&mut self, mem: &mut M, src: Value) -> Option<Value> {
        let Value::Closure(src_addr) = src else {
            return None;
        };

        // Read source header
        let header: HeapClosure = mem.read(src_addr);
        let captures_len = header.captures_len as usize;

        // Deep copy the underlying function
        let src_func = Value::compiled_fn(header.function);
        let dst_func = self.copy_compiled_fn(mem, src_func)?;
        let Value::CompiledFn(dst_func_addr) = dst_func else {
            return None;
        };

        // Allocate destination closure
        let total_size = HeapClosure::alloc_size(captures_len);
        let dst_addr = self.alloc(total_size, 8)?;

        // Write header with new function pointer
        let dst_header = HeapClosure {
            function: dst_func_addr,
            captures_len: header.captures_len,
            padding: 0,
        };
        mem.write(dst_addr, dst_header);

        // Copy captures
        let src_captures_addr = src_addr.add(HeapClosure::captures_offset() as u64);
        let dst_captures_addr = dst_addr.add(HeapClosure::captures_offset() as u64);
        for i in 0..captures_len {
            let offset = (i * core::mem::size_of::<Value>()) as u64;
            let capture: Value = mem.read(src_captures_addr.add(offset));
            mem.write(dst_captures_addr.add(offset), capture);
        }

        Some(Value::closure(dst_addr))
    }
}
