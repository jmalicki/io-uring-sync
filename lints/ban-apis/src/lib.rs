//! # Banned APIs Lint for io-uring-sync
//!
//! This Dylint crate provides a custom Rust linter that enforces the `io_uring`-first policy
//! for the `io-uring-sync` project. It flags direct usage of `libc::`, `nix::`, and `std::fs::`
//! APIs to encourage the use of `io_uring` operations via `compio` and `compio-fs-extended`.
//!
//! ## Purpose
//!
//! The `io-uring-sync` project aims to be a high-performance file synchronization tool that
//! leverages Linux's `io_uring` interface for maximum efficiency. Direct usage of traditional
//! POSIX APIs (`libc`, `nix`, `std::fs`) bypasses the performance benefits of `io_uring` and
//! should be avoided unless there's a strong technical justification.
//!
//! ## What This Lint Catches
//!
//! This linter detects and flags:
//!
//! 1. **Direct imports**: `use libc::*`, `use nix::*`, `use std::fs::*`
//! 2. **Symbol usage**: Any reference to symbols from these crates, even when imported
//! 3. **Method calls**: Calls to methods from these crates
//! 4. **Function calls**: Direct function calls to these APIs
//!
//! ## How It Works
//!
//! The linter uses Rust's type system and def-path resolution to track the origin of symbols,
//! ensuring that even aliased or re-exported APIs are caught. It operates at the AST level
//! to provide precise error reporting with helpful suggestions.
//!
//! ## Banned APIs
//!
//! - **`libc::`**: Raw FFI bindings to C standard library functions
//! - **`nix::`**: Safe Rust bindings to Unix system calls
//! - **`std::fs::`**: Standard library filesystem operations
//!
//! ## Approved Alternatives
//!
//! Instead of banned APIs, use:
//! - **`compio::fs`**: Async filesystem operations via `io_uring`
//! - **`compio-fs-extended`**: Extended `io_uring` operations (fadvise, fallocate, etc.)
//! - **`io_uring::opcode::*`**: Direct `io_uring` opcodes when available
//!
//! ## Exception Handling
//!
//! When a banned API must be used (e.g., for operations not yet available in `io_uring`),
//! add an explicit `#[allow(BAN_LIBC_NIX_STDFS)]` attribute with a comment explaining
//! the technical justification.
//!
//! ## Example
//!
//! ```rust,ignore
//! // ❌ BAD - Will be flagged
//! use libc::statx;
//! use std::fs::read_to_string;
//!
//! // ✅ GOOD - Use io_uring alternatives
//! use compio::fs::File;
//! use compio_fs_extended::metadata::statx;
//! ```

#![feature(rustc_private)]

extern crate rustc_hir;
extern crate rustc_lint;
extern crate rustc_middle;
extern crate rustc_session;
extern crate rustc_span;

use rustc_hir as hir;
use rustc_hir::{Expr, ExprKind, Item, ItemKind};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_session::{declare_lint, impl_lint_pass};
use rustc_span::Span;

/// Lint that flags usage of banned APIs (`libc::`, `nix::`, `std::fs::`)
///
/// This lint enforces the `io_uring`-first policy by detecting and flagging direct usage
/// of traditional POSIX APIs that bypass the performance benefits of `io_uring`.
///
/// The lint operates by:
/// 1. Resolving symbol definitions to their original crate paths
/// 2. Checking import statements for banned crate roots
/// 3. Analyzing expression usage to catch method calls and function calls
///
/// This ensures that even aliased or re-exported APIs are caught, providing
/// comprehensive coverage of banned API usage.
declare_lint! {
    pub BAN_LIBC_NIX_STDFS,
    Warn,
    "ban usages of libc::, nix::, or std::fs:: APIs (resolves imports)"
}

/// The main lint pass implementation
pub struct BanApis;
impl_lint_pass!(BanApis => [BAN_LIBC_NIX_STDFS]);

/// Checks if a def-path string represents a banned API
///
/// This function determines whether a resolved symbol path belongs to one of the
/// banned API crates. It checks for:
/// - `libc::` - Raw FFI bindings to C standard library
/// - `nix::` - Safe Rust bindings to Unix system calls  
/// - `std::fs::` - Standard library filesystem operations
///
/// # Arguments
///
/// * `path` - The resolved def-path string to check
///
/// # Returns
///
/// `true` if the path represents a banned API, `false` otherwise
fn is_banned_path_str(path: &str) -> bool {
    path.starts_with("libc::") || path.starts_with("nix::") || path.starts_with("std::fs::")
}

/// Resolves a def-id to its full def-path string
///
/// This function uses the type context to resolve a definition ID to its
/// complete path string, which includes the crate name and module hierarchy.
/// This is essential for detecting banned APIs even when they're imported
/// or re-exported through other modules.
///
/// # Arguments
///
/// * `cx` - The late context containing type information
/// * `def_id` - The definition ID to resolve
///
/// # Returns
///
/// The full def-path string for the given definition ID
fn def_path_str<'tcx>(cx: &LateContext<'tcx>, def_id: rustc_hir::def_id::DefId) -> String {
    cx.tcx.def_path_str(def_id)
}

/// Emits a lint warning for banned API usage
///
/// This function creates and emits a structured lint warning that includes:
/// - The specific banned API that was detected
/// - A helpful suggestion to use `io_uring` alternatives
/// - Proper span information for IDE integration
///
/// # Arguments
///
/// * `cx` - The late context for lint emission
/// * `span` - The source span where the violation occurred
/// * `what` - The specific banned API that was detected
fn lint_banned<'tcx>(cx: &LateContext<'tcx>, span: Span, what: &str) {
    cx.span_lint(BAN_LIBC_NIX_STDFS, span, |diag| {
        diag.build(&format!("banned API usage: {}", what))
            .help("use io_uring via compio/compio-fs-extended or approved abstraction")
            .emit();
    });
}

impl<'tcx> LateLintPass<'tcx> for BanApis {
    /// Checks import statements for banned crate usage
    ///
    /// This method analyzes `use` statements to detect direct imports from
    /// banned crates. It checks the first segment of the import path to identify
    /// imports from `libc`, `nix`, or `std::fs`.
    ///
    /// # Arguments
    ///
    /// * `cx` - The late context for lint operations
    /// * `item` - The item being checked (should be a `use` statement)
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        if let ItemKind::Use(path, _) = &item.kind {
            let segs: Vec<_> = path.segments.iter().map(|s| s.ident.to_string()).collect();
            if !segs.is_empty() {
                let head = segs.get(0).map(String::as_str).unwrap_or("");
                if head == "libc"
                    || head == "nix"
                    || (head == "std" && segs.get(1).map(String::as_str) == Some("fs"))
                {
                    lint_banned(cx, item.span, &segs.join("::"));
                }
            }
        }
    }

    /// Checks expressions for banned API usage
    ///
    /// This method analyzes expressions to detect usage of banned APIs through:
    /// - Path expressions (direct symbol references)
    /// - Method calls (calls to methods from banned crates)
    /// - Function calls (calls to functions from banned crates)
    ///
    /// It uses def-path resolution to track the origin of symbols, ensuring
    /// that even aliased or re-exported APIs are caught.
    ///
    /// # Arguments
    ///
    /// * `cx` - The late context for lint operations
    /// * `expr` - The expression being checked
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        match &expr.kind {
            ExprKind::Path(qpath) => {
                // Check direct path references (e.g., `libc::statx`)
                let res = cx.qpath_res(qpath, expr.hir_id);
                if let rustc_hir::def::Res::Def(_, def_id) = res {
                    let p = def_path_str(cx, def_id);
                    if is_banned_path_str(&p) {
                        lint_banned(cx, expr.span, &p);
                    }
                }
            }
            ExprKind::MethodCall(_, _, _, _) => {
                // Check method calls (e.g., `file.read_to_string()`)
                if let Some(def_id) = cx.typeck_results().type_dependent_def_id(expr.hir_id) {
                    let p = def_path_str(cx, def_id);
                    if is_banned_path_str(&p) {
                        lint_banned(cx, expr.span, &p);
                    }
                }
            }
            ExprKind::Call(callee, _) => {
                // Check function calls (e.g., `std::fs::read_to_string()`)
                if let ExprKind::Path(qp) = &callee.kind {
                    let res = cx.qpath_res(qp, callee.hir_id);
                    if let rustc_hir::def::Res::Def(_, def_id) = res {
                        let p = def_path_str(cx, def_id);
                        if is_banned_path_str(&p) {
                            lint_banned(cx, callee.span, &p);
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

/// Registers the custom lints with the Rust compiler
///
/// This function is called by the Rust compiler to register our custom lints.
/// It registers the `BAN_LIBC_NIX_STDFS` lint and the `BanApis` lint pass.
///
/// # Arguments
///
/// * `_sess` - The compiler session (unused)
/// * `lint_store` - The lint store to register lints with
#[no_mangle]
pub fn register_lints(_sess: &rustc_session::Session, lint_store: &mut rustc_lint::LintStore) {
    lint_store.register_lints(&[&BAN_LIBC_NIX_STDFS]);
    lint_store.register_late_pass(|_| Box::new(BanApis));
}
