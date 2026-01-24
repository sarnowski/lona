// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Match compilation for the `match` special form.
//!
//! Compiles pattern matching expressions to bytecode. Handles:
//! - Literal patterns (integers, keywords, booleans, nil, strings)
//! - Binding patterns (variable capture)
//! - Wildcard patterns (`_`)
//! - Tuple and vector destructuring
//! - Map destructuring
//! - Guard clauses (`when` expressions)

#[cfg(any(test, feature = "std"))]
use std::vec::Vec;

#[cfg(not(any(test, feature = "std")))]
use alloc::vec::Vec;

use crate::bytecode::{BX_MASK, encode_abc, encode_abx, op};
use crate::intrinsics::id as intrinsic_id;
use crate::platform::MemorySpace;
use crate::value::Value;

use super::pattern::Pattern;
use super::{Binding, CompileError, Compiler, MAX_PARAMS, MAX_SYMBOL_NAME_LEN};

/// Maximum number of match clauses.
const MAX_CLAUSES: usize = 32;

/// Maximum number of labels in a match expression.
const MAX_LABELS: usize = 64;

/// Maximum patch sites per label.
const MAX_PATCH_SITES: usize = 16;

/// Instruction format for patching.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum PatchFormat {
    /// `ABx` format: fail label in bits 0-17 (18 bits).
    #[default]
    Abx,
    /// ABC format: fail label in C field (bits 0-8, 9 bits).
    Abc,
}

/// A patch site with its format.
#[derive(Clone, Copy, Default)]
struct PatchSite {
    /// Instruction index to patch.
    index: usize,
    /// Format of the instruction.
    format: PatchFormat,
}

/// A pending label for forward references.
#[derive(Clone, Copy)]
struct PendingLabel {
    /// Instruction indices where this label is referenced (for patching).
    patch_sites: [PatchSite; MAX_PATCH_SITES],
    /// Number of patch sites.
    patch_count: usize,
    /// Resolved position (instruction index), or 0 if unresolved.
    resolved: usize,
    /// Whether this label has been resolved.
    is_resolved: bool,
}

impl Default for PendingLabel {
    fn default() -> Self {
        Self {
            patch_sites: [PatchSite::default(); MAX_PATCH_SITES],
            patch_count: 0,
            resolved: 0,
            is_resolved: false,
        }
    }
}

/// Label manager for forward jumps in match compilation.
struct LabelManager {
    /// Pending labels.
    labels: [PendingLabel; MAX_LABELS],
    /// Number of labels allocated.
    count: usize,
}

impl LabelManager {
    /// Create a new label manager.
    fn new() -> Self {
        Self {
            labels: core::array::from_fn(|_| PendingLabel::default()),
            count: 0,
        }
    }

    /// Reserve a new label for forward reference.
    fn reserve(&mut self) -> Option<usize> {
        if self.count >= MAX_LABELS {
            return None;
        }
        let id = self.count;
        self.labels[id] = PendingLabel::default();
        self.count += 1;
        Some(id)
    }

    /// Add a patch site for a label using `ABx` format (18-bit Bx field).
    const fn add_patch_site_abx(&mut self, label_id: usize, instr_idx: usize) -> bool {
        self.add_patch_site_impl(label_id, instr_idx, PatchFormat::Abx)
    }

    /// Add a patch site for a label using ABC format (9-bit C field).
    const fn add_patch_site_abc(&mut self, label_id: usize, instr_idx: usize) -> bool {
        self.add_patch_site_impl(label_id, instr_idx, PatchFormat::Abc)
    }

    /// Add a patch site with the specified format.
    const fn add_patch_site_impl(
        &mut self,
        label_id: usize,
        instr_idx: usize,
        format: PatchFormat,
    ) -> bool {
        if label_id >= self.count {
            return false;
        }
        let label = &mut self.labels[label_id];
        if label.patch_count >= MAX_PATCH_SITES {
            return false;
        }
        label.patch_sites[label.patch_count] = PatchSite {
            index: instr_idx,
            format,
        };
        label.patch_count += 1;
        true
    }

    /// Resolve a label to the current position.
    const fn resolve(&mut self, label_id: usize, target: usize) {
        if label_id < self.count {
            self.labels[label_id].resolved = target;
            self.labels[label_id].is_resolved = true;
        }
    }

    /// Patch all instructions referencing a label.
    fn patch_all(&self, label_id: usize, code: &mut [u32]) {
        if label_id >= self.count {
            return;
        }
        let label = &self.labels[label_id];
        if !label.is_resolved {
            return;
        }
        let target = label.resolved;
        for i in 0..label.patch_count {
            let patch = &label.patch_sites[i];
            if patch.index < code.len() {
                let old = code[patch.index];
                code[patch.index] = match patch.format {
                    PatchFormat::Abx => {
                        // ABx format: preserve opcode and A field, replace Bx (bits 0-17)
                        let opcode = (old >> 26) & 0x3F;
                        let a = (old >> 18) & 0xFF;
                        (opcode << 26) | (a << 18) | (target as u32 & BX_MASK)
                    }
                    PatchFormat::Abc => {
                        // ABC format: preserve opcode, A, and B fields, replace C (bits 0-8)
                        let upper = old & 0xFFFF_FE00; // Keep bits 9-31
                        upper | (target as u32 & 0x1FF) // Replace C field (bits 0-8)
                    }
                };
            }
        }
    }
}

/// A binding created by pattern matching.
#[derive(Clone, Copy)]
struct PatternBinding {
    /// Symbol name.
    name: [u8; MAX_SYMBOL_NAME_LEN],
    /// Length of the name.
    name_len: u8,
    /// X register where the value is currently stored.
    source_reg: u8,
}

/// A parsed match clause (pattern, optional guard, body).
struct MatchClause {
    /// The pattern for this clause.
    pattern: Pattern,
    /// Optional guard expression (`when` clause).
    guard: Option<Value>,
    /// Body expression.
    body: Value,
}

/// Context for compiling a single match clause.
struct ClauseContext {
    /// Register holding the value being matched.
    match_val_reg: u8,
    /// Label to jump to if this clause fails.
    fail_label: usize,
    /// Label to jump to after successful match.
    end_label: usize,
    /// Target register for the result.
    target: u8,
    /// First available temp register.
    temp_base: u8,
}

/// Context for emitting pattern tests.
struct PatternContext {
    /// Label to jump to if pattern doesn't match.
    fail_label: usize,
    /// First available temp register.
    temp_base: u8,
}

impl<M: MemorySpace> Compiler<'_, M> {
    /// Compile the `match` special form.
    ///
    /// Syntax: `(match expr pattern1 body1 pattern2 when guard2 body2 ...)`
    ///
    /// Each clause is either:
    /// - `pattern body` - pattern without guard
    /// - `pattern when guard body` - pattern with guard
    ///
    /// # Errors
    ///
    /// Returns `CompileError::InvalidSyntax` if the match syntax is malformed.
    /// Returns `CompileError::ExpressionTooComplex` if there are too many clauses or labels.
    pub fn compile_match(
        &mut self,
        args: Value,
        target: u8,
        temp_base: u8,
    ) -> Result<u8, CompileError> {
        // Parse match arguments
        let (expr, clauses) = self.parse_match_args(args)?;

        // Compile the expression to match, result in temp_base
        let match_val_reg = temp_base;
        let next_temp = temp_base
            .checked_add(1)
            .ok_or(CompileError::ExpressionTooComplex)?;
        self.compile_expr(expr, match_val_reg, next_temp)?;

        // Initialize label manager
        let mut labels = LabelManager::new();

        // Reserve end label (where all successful matches jump to)
        let end_label = labels.reserve().ok_or(CompileError::ExpressionTooComplex)?;

        // Reserve clause labels (one per clause for fallthrough)
        let mut clause_labels = [0usize; MAX_CLAUSES];
        for (i, clause_label) in clause_labels.iter_mut().enumerate().take(clauses.len()) {
            if i + 1 < clauses.len() {
                // Next clause label
                *clause_label = labels.reserve().ok_or(CompileError::ExpressionTooComplex)?;
            }
        }

        // Reserve badmatch label (for when no clause matches)
        let badmatch_label = labels.reserve().ok_or(CompileError::ExpressionTooComplex)?;

        // Compile each clause
        for (i, clause) in clauses.iter().enumerate() {
            // Determine fail label for this clause
            let fail_label = if i + 1 < clauses.len() {
                clause_labels[i]
            } else {
                badmatch_label
            };

            let ctx = ClauseContext {
                match_val_reg,
                fail_label,
                end_label,
                target,
                temp_base: next_temp,
            };

            // Compile this clause
            self.compile_match_clause(clause, &ctx, &mut labels)?;

            // Resolve the next clause label (if any)
            if i + 1 < clauses.len() {
                labels.resolve(clause_labels[i], self.chunk.code_len());
            }
        }

        // Badmatch: raise error when no clause matches
        labels.resolve(badmatch_label, self.chunk.code_len());
        self.emit_badmatch_error(match_val_reg);
        // BADMATCH terminates the process, so no jump needed

        // End label: successful match ends here
        labels.resolve(end_label, self.chunk.code_len());

        // Patch all labels
        for i in 0..labels.count {
            labels.patch_all(i, &mut self.chunk.code);
        }

        Ok(next_temp)
    }

    /// Parse match arguments into expression and clauses.
    fn parse_match_args(&self, args: Value) -> Result<(Value, Vec<MatchClause>), CompileError> {
        // First element is the expression to match
        let first_pair = self
            .proc
            .read_pair(self.mem, args)
            .ok_or(CompileError::InvalidSyntax)?;
        let expr = first_pair.first;

        // Rest are pattern-body pairs (possibly with guards)
        let mut clauses = Vec::new();
        let mut current = first_pair.rest;

        while !current.is_nil() {
            if clauses.len() >= MAX_CLAUSES {
                return Err(CompileError::ExpressionTooComplex);
            }

            let clause = self.parse_single_clause(&mut current)?;
            clauses.push(clause);
        }

        if clauses.is_empty() {
            return Err(CompileError::InvalidSyntax);
        }

        Ok((expr, clauses))
    }

    /// Parse a single match clause from the argument list.
    fn parse_single_clause(&self, current: &mut Value) -> Result<MatchClause, CompileError> {
        // Get pattern
        let pair = self
            .proc
            .read_pair(self.mem, *current)
            .ok_or(CompileError::InvalidSyntax)?;
        let pattern_val = pair.first;
        let pattern = self.parse_pattern(pattern_val)?;

        // Check for `when` keyword (guard)
        let pair2 = self
            .proc
            .read_pair(self.mem, pair.rest)
            .ok_or(CompileError::InvalidSyntax)?;

        let (guard, body, rest) = if pair2.first.is_symbol() {
            // Check if it's the `when` keyword
            let name = self
                .proc
                .read_string(self.mem, pair2.first)
                .ok_or(CompileError::InvalidSyntax)?;

            if name == "when" {
                // Guard: `pattern when guard body`
                let guard_pair = self
                    .proc
                    .read_pair(self.mem, pair2.rest)
                    .ok_or(CompileError::InvalidSyntax)?;
                let guard = guard_pair.first;

                let body_pair = self
                    .proc
                    .read_pair(self.mem, guard_pair.rest)
                    .ok_or(CompileError::InvalidSyntax)?;
                let body = body_pair.first;

                (Some(guard), body, body_pair.rest)
            } else {
                // No guard, the symbol is part of the body
                (None, pair2.first, pair2.rest)
            }
        } else {
            // No guard
            (None, pair2.first, pair2.rest)
        };

        *current = rest;
        Ok(MatchClause {
            pattern,
            guard,
            body,
        })
    }

    /// Compile a single match clause.
    fn compile_match_clause(
        &mut self,
        clause: &MatchClause,
        ctx: &ClauseContext,
        labels: &mut LabelManager,
    ) -> Result<(), CompileError> {
        // Emit pattern tests
        let mut bindings: Vec<PatternBinding> = Vec::new();
        let pat_ctx = PatternContext {
            fail_label: ctx.fail_label,
            temp_base: ctx.temp_base,
        };
        let next_temp = self.emit_pattern_tests(
            &clause.pattern,
            ctx.match_val_reg,
            &pat_ctx,
            &mut bindings,
            labels,
        )?;

        // If there's a guard, evaluate it and jump to fail_label if false
        if let Some(guard) = clause.guard {
            self.compile_guard(guard, &bindings, next_temp, ctx, labels)?;
        }

        // Add bindings to scope for body evaluation
        let saved_bindings_len = self.bindings_len;
        self.add_pattern_bindings(&bindings)?;

        // Compile body - use next_temp from pattern tests to avoid overwriting
        // extracted elements (including wildcards which use temp registers but
        // don't create bindings)
        self.compile_expr(clause.body, ctx.target, next_temp)?;

        // Restore bindings
        self.bindings_len = saved_bindings_len;

        // Jump to end
        let site = self.chunk.code_len();
        self.chunk.emit(encode_abx(op::JUMP, 0, 0));
        labels.add_patch_site_abx(ctx.end_label, site);

        Ok(())
    }

    /// Compile guard expression and emit conditional jump.
    fn compile_guard(
        &mut self,
        guard: Value,
        bindings: &[PatternBinding],
        pattern_next_temp: u8,
        ctx: &ClauseContext,
        labels: &mut LabelManager,
    ) -> Result<(), CompileError> {
        // Add bindings to scope for guard evaluation
        let saved_bindings_len = self.bindings_len;
        self.add_pattern_bindings(bindings)?;

        // Compile guard to temp register - use pattern_next_temp to avoid
        // overwriting extracted elements (including wildcards which use temp
        // registers but don't create bindings)
        let guard_temp = pattern_next_temp;
        let next_temp = guard_temp
            .checked_add(1)
            .ok_or(CompileError::ExpressionTooComplex)?;
        self.compile_expr(guard, guard_temp, next_temp)?;

        // JUMP_IF_FALSE guard_temp, fail_label
        let site = self.chunk.code_len();
        self.chunk
            .emit(encode_abx(op::JUMP_IF_FALSE, guard_temp, 0));
        labels.add_patch_site_abx(ctx.fail_label, site);

        // Restore bindings
        self.bindings_len = saved_bindings_len;
        Ok(())
    }

    /// Emit pattern tests, branching to `fail_label` on mismatch.
    fn emit_pattern_tests(
        &mut self,
        pattern: &Pattern,
        value_reg: u8,
        ctx: &PatternContext,
        bindings: &mut Vec<PatternBinding>,
        labels: &mut LabelManager,
    ) -> Result<u8, CompileError> {
        match pattern {
            Pattern::Wildcard => Ok(ctx.temp_base),

            Pattern::Binding { name, name_len } => {
                bindings.push(PatternBinding {
                    name: *name,
                    name_len: *name_len,
                    source_reg: value_reg,
                });
                Ok(ctx.temp_base)
            }

            Pattern::Literal(lit) => self.emit_literal_pattern(value_reg, *lit, ctx, labels),

            Pattern::Tuple(patterns) => {
                self.emit_tuple_pattern(patterns, value_reg, ctx, bindings, labels)
            }

            Pattern::TupleRest { head, rest } => {
                self.emit_tuple_rest_pattern(head, rest, value_reg, ctx, bindings, labels)
            }

            Pattern::Vector(patterns) => {
                self.emit_vector_pattern(patterns, value_reg, ctx, bindings, labels)
            }

            Pattern::Map(pairs) => self.emit_map_pattern(pairs, value_reg, ctx, bindings, labels),
        }
    }

    /// Emit tests for a literal pattern.
    fn emit_literal_pattern(
        &mut self,
        value_reg: u8,
        lit: Value,
        ctx: &PatternContext,
        labels: &mut LabelManager,
    ) -> Result<u8, CompileError> {
        let lit_temp = ctx.temp_base;
        self.compile_constant(lit, lit_temp)?;

        let site = self.chunk.code_len();
        self.chunk
            .emit(encode_abc(op::IS_EQ, value_reg, u16::from(lit_temp), 0));
        labels.add_patch_site_abc(ctx.fail_label, site);

        ctx.temp_base
            .checked_add(1)
            .ok_or(CompileError::ExpressionTooComplex)
    }

    /// Emit tests for a tuple pattern.
    fn emit_tuple_pattern(
        &mut self,
        patterns: &[Pattern],
        value_reg: u8,
        ctx: &PatternContext,
        bindings: &mut Vec<PatternBinding>,
        labels: &mut LabelManager,
    ) -> Result<u8, CompileError> {
        // Test: is tuple?
        let site = self.chunk.code_len();
        self.chunk.emit(encode_abx(op::IS_TUPLE, value_reg, 0));
        labels.add_patch_site_abx(ctx.fail_label, site);

        // Test: correct arity?
        let arity = patterns.len();
        let site = self.chunk.code_len();
        self.chunk
            .emit(encode_abc(op::TEST_ARITY, value_reg, arity as u16, 0));
        labels.add_patch_site_abc(ctx.fail_label, site);

        // Extract and match each element
        self.emit_collection_elements(
            patterns,
            value_reg,
            ctx,
            bindings,
            labels,
            op::GET_TUPLE_ELEM,
        )
    }

    /// Emit tests for a tuple rest pattern `[h & t]`.
    fn emit_tuple_rest_pattern(
        &mut self,
        head: &[Pattern],
        rest: &Pattern,
        value_reg: u8,
        ctx: &PatternContext,
        bindings: &mut Vec<PatternBinding>,
        labels: &mut LabelManager,
    ) -> Result<u8, CompileError> {
        // Test: is tuple?
        let site = self.chunk.code_len();
        self.chunk.emit(encode_abx(op::IS_TUPLE, value_reg, 0));
        labels.add_patch_site_abx(ctx.fail_label, site);

        // Test: has at least head.len() elements?
        // This prevents crashes when extracting head elements from a short tuple.
        if !head.is_empty() {
            let site = self.chunk.code_len();
            self.chunk.emit(encode_abc(
                op::TEST_ARITY_GE,
                value_reg,
                head.len() as u16,
                0,
            ));
            labels.add_patch_site_abc(ctx.fail_label, site);
        }

        // Extract and match head elements
        let mut next_temp = self.emit_collection_elements(
            head,
            value_reg,
            ctx,
            bindings,
            labels,
            op::GET_TUPLE_ELEM,
        )?;

        // Extract rest as a new tuple using TUPLE_SLICE
        // TUPLE_SLICE dest, src, start_index
        let rest_reg = next_temp;
        next_temp = next_temp
            .checked_add(1)
            .ok_or(CompileError::ExpressionTooComplex)?;

        self.chunk.emit(encode_abc(
            op::TUPLE_SLICE,
            rest_reg,
            u16::from(value_reg),
            head.len() as u16,
        ));

        // Now match the rest pattern against the sliced tuple
        match rest {
            Pattern::Wildcard => {
                // Just discard the rest
            }
            Pattern::Binding { name, name_len } => {
                bindings.push(PatternBinding {
                    name: *name,
                    name_len: *name_len,
                    source_reg: rest_reg,
                });
            }
            _ => {
                // Nested pattern on rest - recursively match
                let sub_ctx = PatternContext {
                    fail_label: ctx.fail_label,
                    temp_base: next_temp,
                };
                next_temp = self.emit_pattern_tests(rest, rest_reg, &sub_ctx, bindings, labels)?;
            }
        }

        Ok(next_temp)
    }

    /// Emit tests for a vector pattern.
    fn emit_vector_pattern(
        &mut self,
        patterns: &[Pattern],
        value_reg: u8,
        ctx: &PatternContext,
        bindings: &mut Vec<PatternBinding>,
        labels: &mut LabelManager,
    ) -> Result<u8, CompileError> {
        // Test: is vector?
        let site = self.chunk.code_len();
        self.chunk.emit(encode_abx(op::IS_VECTOR, value_reg, 0));
        labels.add_patch_site_abx(ctx.fail_label, site);

        // Test: correct length?
        let len = patterns.len();
        let site = self.chunk.code_len();
        self.chunk
            .emit(encode_abc(op::TEST_VEC_LEN, value_reg, len as u16, 0));
        labels.add_patch_site_abc(ctx.fail_label, site);

        // Extract and match each element
        self.emit_collection_elements(patterns, value_reg, ctx, bindings, labels, op::GET_VEC_ELEM)
    }

    /// Emit element extraction and pattern tests for a collection (tuple or vector).
    fn emit_collection_elements(
        &mut self,
        patterns: &[Pattern],
        value_reg: u8,
        ctx: &PatternContext,
        bindings: &mut Vec<PatternBinding>,
        labels: &mut LabelManager,
        get_elem_op: u8,
    ) -> Result<u8, CompileError> {
        let mut next_temp = ctx.temp_base;
        for (i, sub_pattern) in patterns.iter().enumerate() {
            let elem_reg = next_temp;
            next_temp = next_temp
                .checked_add(1)
                .ok_or(CompileError::ExpressionTooComplex)?;

            self.chunk.emit(encode_abc(
                get_elem_op,
                elem_reg,
                u16::from(value_reg),
                i as u16,
            ));

            let sub_ctx = PatternContext {
                fail_label: ctx.fail_label,
                temp_base: next_temp,
            };
            next_temp =
                self.emit_pattern_tests(sub_pattern, elem_reg, &sub_ctx, bindings, labels)?;
        }
        Ok(next_temp)
    }

    /// Emit tests for a map pattern.
    fn emit_map_pattern(
        &mut self,
        pairs: &[(Value, Pattern)],
        value_reg: u8,
        ctx: &PatternContext,
        bindings: &mut Vec<PatternBinding>,
        labels: &mut LabelManager,
    ) -> Result<u8, CompileError> {
        // Test: is map?
        let site = self.chunk.code_len();
        self.chunk.emit(encode_abx(op::IS_MAP, value_reg, 0));
        labels.add_patch_site_abx(ctx.fail_label, site);

        // For each key-value pair, check key exists then get the value and match the pattern
        let mut next_temp = ctx.temp_base;
        for (key, value_pattern) in pairs {
            // Load key to temp
            let key_temp = next_temp;
            next_temp = next_temp
                .checked_add(1)
                .ok_or(CompileError::ExpressionTooComplex)?;
            self.compile_constant(*key, key_temp)?;

            // First check if key exists using CONTAINS intrinsic
            // Move map to X1, key to X2
            self.chunk
                .emit(encode_abc(op::MOVE, 1, u16::from(value_reg), 0));
            self.chunk
                .emit(encode_abc(op::MOVE, 2, u16::from(key_temp), 0));

            // INTRINSIC CONTAINS, 2 - result in X0 (true/false)
            self.chunk
                .emit(encode_abc(op::INTRINSIC, intrinsic_id::CONTAINS, 2, 0));

            // JUMP_IF_FALSE X0, fail_label - if key doesn't exist, pattern fails
            let site = self.chunk.code_len();
            self.chunk.emit(encode_abx(op::JUMP_IF_FALSE, 0, 0));
            labels.add_patch_site_abx(ctx.fail_label, site);

            // Key exists, now get the value
            // Move map to X1, key to X2 (may have been clobbered by intrinsic)
            self.chunk
                .emit(encode_abc(op::MOVE, 1, u16::from(value_reg), 0));
            self.chunk
                .emit(encode_abc(op::MOVE, 2, u16::from(key_temp), 0));

            // INTRINSIC GET, 2
            self.chunk
                .emit(encode_abc(op::INTRINSIC, intrinsic_id::GET, 2, 0));

            // Result is in X0, move to elem_reg
            let elem_reg = next_temp;
            next_temp = next_temp
                .checked_add(1)
                .ok_or(CompileError::ExpressionTooComplex)?;
            self.chunk.emit(encode_abc(op::MOVE, elem_reg, 0, 0));

            let sub_ctx = PatternContext {
                fail_label: ctx.fail_label,
                temp_base: next_temp,
            };
            next_temp =
                self.emit_pattern_tests(value_pattern, elem_reg, &sub_ctx, bindings, labels)?;
        }

        Ok(next_temp)
    }

    /// Emit bytecode for a badmatch error.
    ///
    /// Emits a BADMATCH instruction that terminates the process with
    /// `RuntimeError::Badmatch { value }`.
    fn emit_badmatch_error(&mut self, value_reg: u8) {
        self.chunk.emit(encode_abx(op::BADMATCH, value_reg, 0));
    }

    /// Add pattern bindings to the compiler's scope.
    fn add_pattern_bindings(&mut self, bindings: &[PatternBinding]) -> Result<(), CompileError> {
        for binding in bindings {
            if self.bindings_len >= MAX_PARAMS {
                return Err(CompileError::ExpressionTooComplex);
            }
            self.bindings[self.bindings_len] = Binding {
                name: binding.name,
                name_len: binding.name_len,
                register: binding.source_reg,
            };
            self.bindings_len += 1;
        }
        Ok(())
    }
}
