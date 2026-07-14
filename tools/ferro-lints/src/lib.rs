#![feature(rustc_private)]

extern crate rustc_ast;
extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_lint;
extern crate rustc_session;
extern crate rustc_span;

use rustc_lint::{EarlyContext, EarlyLintPass, LintContext};
use rustc_session::{declare_lint, declare_lint_pass};

declare_lint! {
    pub FERRO_SAFETY_COMMENT,
    Warn,
    "unsafe blocks must have a preceding SAFETY doc comment"
}

declare_lint! {
    pub FERRO_NO_UNWRAP_CRITICAL,
    Warn,
    "unwrap() and expect() should not be used in critical path functions"
}

declare_lint! {
    pub FERRO_SECRET_NO_DEBUG,
    Warn,
    "types with secret fields should not derive Debug"
}

declare_lint_pass!(FerroLints => [FERRO_SAFETY_COMMENT, FERRO_NO_UNWRAP_CRITICAL, FERRO_SECRET_NO_DEBUG]);

struct SafetyCommentDiagnostic;

impl<'a> rustc_errors::Diagnostic<'a, ()> for SafetyCommentDiagnostic {
    fn into_diag(self, dcx: rustc_errors::DiagCtxtHandle<'a>, _level: rustc_errors::Level) -> rustc_errors::Diag<'a, ()> {
        let mut diag = dcx.struct_warn("unsafe block without SAFETY comment");
        diag.help("add a `// SAFETY:` comment explaining why the unsafe operation is safe");
        diag
    }
}

struct NoUnwrapDiagnostic;

impl<'a> rustc_errors::Diagnostic<'a, ()> for NoUnwrapDiagnostic {
    fn into_diag(self, dcx: rustc_errors::DiagCtxtHandle<'a>, _level: rustc_errors::Level) -> rustc_errors::Diag<'a, ()> {
        let mut diag = dcx.struct_warn("unwrap() or expect() used in critical path function");
        diag.help("consider using proper error handling with `?` or `match`");
        diag
    }
}

struct SecretDebugDiagnostic;

impl<'a> rustc_errors::Diagnostic<'a, ()> for SecretDebugDiagnostic {
    fn into_diag(self, dcx: rustc_errors::DiagCtxtHandle<'a>, _level: rustc_errors::Level) -> rustc_errors::Diag<'a, ()> {
        let mut diag = dcx.struct_warn("type with secret field derives Debug");
        diag.help("implement Debug manually with redacted field values");
        diag
    }
}

fn is_critical_crate(cx: &EarlyContext<'_>, span: rustc_span::Span) -> bool {
    let sm = cx.sess().source_map();
    let src_path = sm.span_to_filename(span);
    match &src_path {
        rustc_span::FileName::Real(path) => {
            if let Some(local_path) = path.local_path() {
                let path_str = local_path.to_string_lossy();
                return path_str.contains("ferro-server")
                    || path_str.contains("ferro-auth")
                    || path_str.contains("ferro-crypto");
            }
            false
        }
        _ => false,
    }
}

fn has_secret_field_name(name: &str) -> bool {
    let lower = name.to_lowercase();
    lower.contains("password")
        || lower.contains("secret")
        || lower.contains("token")
        || lower.contains("key")
}

impl EarlyLintPass for FerroLints {
    fn check_expr(&mut self, cx: &EarlyContext<'_>, expr: &rustc_ast::Expr) {
        if let rustc_ast::ExprKind::MethodCall(method, ..) = &expr.kind {
            let name = method.seg.ident.as_str();
            if name == "unwrap" || name == "expect" {
                if is_critical_crate(cx, expr.span) {
                    cx.emit_span_lint(FERRO_NO_UNWRAP_CRITICAL, expr.span, NoUnwrapDiagnostic);
                }
            }
        }
    }

    fn check_item(&mut self, cx: &EarlyContext<'_>, item: &rustc_ast::Item) {
        if let rustc_ast::ItemKind::Struct(_ident, _generics, variant_data) = &item.kind {
            let has_debug_derive = item.attrs.iter().any(|attr| {
                if let rustc_ast::AttrKind::Normal(normal) = &attr.kind {
                    if let Some(ident) = normal.item.path.segments.last() {
                        return ident.ident.as_str() == "derive";
                    }
                }
                false
            });

            if has_debug_derive {
                for field in variant_data.fields() {
                    if has_secret_field_name(field.ident.as_ref().map(|i| i.as_str()).unwrap_or("")) {
                        cx.emit_span_lint(FERRO_SECRET_NO_DEBUG, item.span, SecretDebugDiagnostic);
                        break;
                    }
                }
            }
        }
    }

    fn check_block(&mut self, cx: &EarlyContext<'_>, block: &rustc_ast::Block) {
        if matches!(block.rules, rustc_ast::BlockCheckMode::Unsafe(rustc_ast::UnsafeSource::UserProvided)) {
            let span = block.span;
            let sm = cx.sess().source_map();
            if let Ok(snippet) = sm.span_to_snippet(span) {
                if !snippet.contains("SAFETY") {
                    cx.emit_span_lint(FERRO_SAFETY_COMMENT, span, SafetyCommentDiagnostic);
                }
            }
        }
    }
}

#[no_mangle]
pub fn register_lints(store: &mut rustc_lint::LintStore) {
    store.register_early_pass(|| Box::new(FerroLints));
}

pub fn lint_group() -> &'static str {
    "ferro-lints"
}
