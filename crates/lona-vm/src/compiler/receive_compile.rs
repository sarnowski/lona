// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Receive compilation for the `receive` special form.
//!
//! Compiles selective receive expressions to bytecode. Reuses pattern matching
//! infrastructure from `match_compile` for clause compilation.
//!
//! ```text
//! (receive
//!   pattern1 body1
//!   pattern2 when guard body2
//!   :after timeout-ms timeout-body)
//! ```
//!
//! Compiles to:
//!
//! ```text
//! [RECV_TIMEOUT_INIT]  (if :after present)
//! RECV_PEEK → wait_label
//! [pattern tests + body + RECV_ACCEPT + JUMP end] per clause
//! RECV_NEXT → recv_peek
//! wait_label: RECV_WAIT → recv_peek
//! [timeout body]  (if :after present)
//! end_label:
//! ```

#[cfg(any(test, feature = "std"))]
use std::vec::Vec;

#[cfg(not(any(test, feature = "std")))]
use alloc::vec::Vec;

use crate::bytecode::{encode_abx, op};
use crate::platform::MemorySpace;
use crate::term::Term;

use super::match_compile::{
    LabelManager, MAX_CLAUSES, MatchClause, PatternBinding, PatternContext,
};
use super::{CompileError, Compiler};

impl<M: MemorySpace> Compiler<'_, M> {
    /// Compile the `receive` special form.
    ///
    /// Syntax: `(receive pattern1 body1 pattern2 when guard body2 ... :after timeout body)`
    ///
    /// The `:after` clause is optional. Without it, `receive` blocks indefinitely.
    ///
    /// # Errors
    ///
    /// Returns `CompileError` if the receive syntax is malformed or too complex.
    pub fn compile_receive(
        &mut self,
        args: Term,
        target: u8,
        temp_base: u8,
    ) -> Result<u8, CompileError> {
        // Parse clauses and optional :after
        let (clauses, timeout) = self.parse_receive_args(args)?;

        if clauses.is_empty() && timeout.is_none() {
            return Err(CompileError::InvalidSyntax);
        }

        // Timeout-only receive (no clauses): skip the scan loop entirely.
        // Just emit RECV_TIMEOUT_INIT + RECV_WAIT + timeout body.
        if clauses.is_empty() {
            if let Some([timeout_expr, timeout_body]) = &timeout {
                let timeout_reg = temp_base;
                let next = temp_base
                    .checked_add(1)
                    .ok_or(CompileError::ExpressionTooComplex)?;
                self.compile_expr(*timeout_expr, timeout_reg, next)?;
                self.chunk
                    .emit(encode_abx(op::RECV_TIMEOUT_INIT, timeout_reg, 0));
                // RECV_WAIT jumps back to itself: re-checks for messages, then
                // re-checks timeout. For timeout-only receives this is immediate.
                let wait_pos = self.chunk.code_len();
                self.chunk
                    .emit(encode_abx(op::RECV_WAIT, 0, wait_pos as u32));
                // Timeout body (falls through when timeout expires)
                self.compile_expr(*timeout_body, target, next)?;
                return Ok(next);
            }
        }

        let mut labels = LabelManager::new();

        // Reserve labels
        let end_label = labels.reserve().ok_or(CompileError::ExpressionTooComplex)?;
        let recv_peek_label = labels.reserve().ok_or(CompileError::ExpressionTooComplex)?;
        let wait_label = labels.reserve().ok_or(CompileError::ExpressionTooComplex)?;

        // Reserve per-clause fail labels
        let mut clause_fail_labels = [0usize; MAX_CLAUSES];
        for label in clause_fail_labels.iter_mut().take(clauses.len()) {
            *label = labels.reserve().ok_or(CompileError::ExpressionTooComplex)?;
        }

        // Message register: where RECV_PEEK puts the current message
        let msg_reg = temp_base;
        let clause_temp = temp_base
            .checked_add(1)
            .ok_or(CompileError::ExpressionTooComplex)?;

        // --- Emit timeout initialization (if :after present) ---
        // Note: RECV_TIMEOUT_INIT also resets the mailbox save position.
        // For no-timeout receives, no explicit reset is needed because:
        // - If a previous receive matched, RECV_ACCEPT reset save to 0
        // - A no-timeout receive blocks forever until a match, so save
        //   is always 0 when the next receive expression starts.
        if let Some([timeout_expr, _]) = &timeout {
            let timeout_reg = clause_temp;
            let next = clause_temp
                .checked_add(1)
                .ok_or(CompileError::ExpressionTooComplex)?;
            self.compile_expr(*timeout_expr, timeout_reg, next)?;
            self.chunk
                .emit(encode_abx(op::RECV_TIMEOUT_INIT, timeout_reg, 0));
        }

        // --- recv_peek label ---
        labels.resolve(recv_peek_label, self.chunk.code_len());

        // RECV_PEEK msg_reg, wait_label
        let site = self.chunk.code_len();
        self.chunk.emit(encode_abx(op::RECV_PEEK, msg_reg, 0));
        labels.add_patch_site_abx(wait_label, site);

        // --- Compile each clause ---
        for (i, clause) in clauses.iter().enumerate() {
            let fail_label = clause_fail_labels[i];

            // Emit pattern tests
            let mut bindings: Vec<PatternBinding> = Vec::new();
            let pat_ctx = PatternContext {
                fail_label,
                temp_base: clause_temp,
            };
            let next_temp = self.emit_pattern_tests(
                &clause.pattern,
                msg_reg,
                &pat_ctx,
                &mut bindings,
                &mut labels,
            )?;

            // Emit guard (if present)
            if let Some(guard) = clause.guard {
                let saved_bindings_len = self.bindings_len;
                self.add_pattern_bindings(&bindings)?;

                let guard_temp = next_temp;
                let guard_next = guard_temp
                    .checked_add(1)
                    .ok_or(CompileError::ExpressionTooComplex)?;
                self.compile_expr(guard, guard_temp, guard_next)?;

                let site = self.chunk.code_len();
                self.chunk
                    .emit(encode_abx(op::JUMP_IF_FALSE, guard_temp, 0));
                labels.add_patch_site_abx(fail_label, site);

                self.bindings_len = saved_bindings_len;
            }

            // RECV_ACCEPT — remove matched message BEFORE body (BEAM semantics:
            // if body crashes, message is already consumed)
            self.chunk.emit(encode_abx(op::RECV_ACCEPT, 0, 0));

            // Compile body with bindings in scope
            let saved_bindings_len = self.bindings_len;
            self.add_pattern_bindings(&bindings)?;
            self.compile_expr(clause.body, target, next_temp)?;
            self.bindings_len = saved_bindings_len;

            // JUMP end_label
            let site = self.chunk.code_len();
            self.chunk.emit(encode_abx(op::JUMP, 0, 0));
            labels.add_patch_site_abx(end_label, site);

            // Resolve fail label for this clause
            labels.resolve(fail_label, self.chunk.code_len());
        }

        // --- No clause matched: advance and loop ---
        if !clauses.is_empty() {
            let site = self.chunk.code_len();
            self.chunk.emit(encode_abx(op::RECV_NEXT, 0, 0));
            labels.add_patch_site_abx(recv_peek_label, site);
        }

        // --- wait_label: all messages scanned, no match ---
        labels.resolve(wait_label, self.chunk.code_len());

        let site = self.chunk.code_len();
        self.chunk.emit(encode_abx(op::RECV_WAIT, 0, 0));
        labels.add_patch_site_abx(recv_peek_label, site);

        // --- Timeout body (if :after present, falls through from RECV_WAIT) ---
        if let Some([_, timeout_body]) = &timeout {
            self.compile_expr(*timeout_body, target, clause_temp)?;
        }

        // --- End label ---
        labels.resolve(end_label, self.chunk.code_len());

        // Patch all labels
        for i in 0..labels.count {
            labels.patch_all(i, self.chunk.code_mut());
        }

        Ok(clause_temp)
    }

    /// Parse receive arguments into clauses and optional timeout.
    ///
    /// Returns `(clauses, timeout)` where timeout is `Some((timeout_expr, timeout_body))`.
    fn parse_receive_args(
        &self,
        args: Term,
    ) -> Result<(Vec<MatchClause>, Option<[Term; 2]>), CompileError> {
        let mut clauses = Vec::new();
        let mut timeout = None;
        let mut current = args;

        while !current.is_nil() {
            if clauses.len() >= MAX_CLAUSES {
                return Err(CompileError::ExpressionTooComplex);
            }

            // Peek at the next element to check for :after
            let (first, _) = self
                .proc
                .read_term_pair(self.mem, current)
                .ok_or(CompileError::InvalidSyntax)?;

            if first.is_keyword() && self.is_after_keyword(first) {
                // Parse :after timeout-ms timeout-body
                timeout = Some(self.parse_after_clause(current)?);
                break;
            }

            // Not :after — parse as a regular clause using shared infrastructure
            let clause = self.parse_single_clause(&mut current)?;
            clauses.push(clause);
        }

        Ok((clauses, timeout))
    }

    /// Parse the `:after timeout-ms timeout-body` clause.
    fn parse_after_clause(&self, current: Term) -> Result<[Term; 2], CompileError> {
        // Skip :after keyword
        let (_, rest1) = self
            .proc
            .read_term_pair(self.mem, current)
            .ok_or(CompileError::InvalidSyntax)?;

        // Read timeout expression
        let (timeout_expr, rest2) = self
            .proc
            .read_term_pair(self.mem, rest1)
            .ok_or(CompileError::InvalidSyntax)?;

        // Read timeout body
        let (timeout_body, rest3) = self
            .proc
            .read_term_pair(self.mem, rest2)
            .ok_or(CompileError::InvalidSyntax)?;

        if !rest3.is_nil() {
            return Err(CompileError::InvalidSyntax);
        }

        Ok([timeout_expr, timeout_body])
    }

    /// Check if a keyword term is `:after`.
    fn is_after_keyword(&self, term: Term) -> bool {
        let Some(index) = term.as_keyword_index() else {
            return false;
        };
        self.realm
            .keyword_name(self.mem, index)
            .is_some_and(|name| name == "after")
    }
}
