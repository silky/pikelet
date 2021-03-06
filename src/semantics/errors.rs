//! Errors that might be produced during semantic analysis

use codespan::ByteSpan;
use codespan_reporting::Diagnostic;
use std::fmt;

use syntax::core::{Name, RcType};
use syntax::var::Debruijn;

/// An internal error. These are bugs!
#[derive(Debug, Fail, Clone, PartialEq)]
pub enum InternalError {
    #[fail(display = "Unsubstituted debruijn index: `{}{}`.", name, index)]
    UnsubstitutedDebruijnIndex {
        span: ByteSpan,
        name: Name,
        index: Debruijn,
    },
    #[fail(display = "Undefined name `{}`.", name)]
    UndefinedName { var_span: ByteSpan, name: Name },
}

impl InternalError {
    pub fn span(&self) -> ByteSpan {
        match *self {
            InternalError::UnsubstitutedDebruijnIndex { span, .. } => span,
            InternalError::UndefinedName { var_span, .. } => var_span,
        }
    }

    pub fn to_diagnostic(&self) -> Diagnostic {
        match *self {
            InternalError::UnsubstitutedDebruijnIndex {
                span,
                ref name,
                index,
            } => Diagnostic::new_bug(format!("unsubstituted debruijn index: `{}{}`", name, index,))
                .with_primary_label(span, "index found here"),
            InternalError::UndefinedName { ref name, var_span } => {
                Diagnostic::new_bug(format!("cannot find `{}` in scope", name))
                    .with_primary_label(var_span, "not found in this scope")
            },
        }
    }
}

/// An error produced during typechecking
#[derive(Debug, Clone, PartialEq)] // FIXME: Derive `Fail`
pub enum TypeError {
    NotAFunctionType {
        fn_span: ByteSpan,
        arg_span: ByteSpan,
        found: RcType,
    },
    FunctionParamNeedsAnnotation {
        param_span: ByteSpan,
        var_span: Option<ByteSpan>,
        name: Name,
    },
    Mismatch {
        span: ByteSpan,
        found: RcType,
        expected: RcType,
    },
    UnexpectedFunction {
        span: ByteSpan,
        expected: RcType,
    },
    ExpectedUniverse {
        span: ByteSpan,
        found: RcType,
    },
    UndefinedName {
        var_span: ByteSpan,
        name: Name,
    },
    Internal(InternalError),
}

impl TypeError {
    /// Convert the error into a diagnostic message
    pub fn to_diagnostic(&self) -> Diagnostic {
        match *self {
            TypeError::Internal(ref err) => err.to_diagnostic(),
            TypeError::NotAFunctionType {
                fn_span,
                arg_span,
                ref found,
            } => Diagnostic::new_error(format!(
                "applied an argument to a term that was not a function - found type `{}`",
                found,
            )).with_primary_label(fn_span, "the term")
                .with_secondary_label(arg_span, "the applied argument"),
            TypeError::FunctionParamNeedsAnnotation {
                param_span,
                var_span: _, // TODO
                ref name,
            } => Diagnostic::new_error(format!(
                "type annotation needed for the function parameter `{}`",
                name
            )).with_primary_label(param_span, "the parameter that requires an annotation"),
            TypeError::UnexpectedFunction {
                span, ref expected, ..
            } => Diagnostic::new_error(format!(
                "found a function but expected a term of type `{}`",
                expected,
            )).with_primary_label(span, "the function"),
            TypeError::Mismatch {
                span,
                ref found,
                ref expected,
            } => Diagnostic::new_error(format!(
                "found a term of type `{}`, but expected a term of type `{}`",
                found, expected,
            )).with_primary_label(span, "the term"),
            TypeError::ExpectedUniverse { ref found, span } => {
                Diagnostic::new_error(format!("expected type, found value `{}`", found))
                    .with_primary_label(span, "the value")
            },
            TypeError::UndefinedName { ref name, var_span } => {
                Diagnostic::new_error(format!("cannot find `{}` in scope", name))
                    .with_primary_label(var_span, "not found in this scope")
            },
        }
    }
}

impl From<InternalError> for TypeError {
    fn from(src: InternalError) -> TypeError {
        TypeError::Internal(src)
    }
}

impl fmt::Display for TypeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            TypeError::NotAFunctionType { ref found, .. } => {
                write!(f, "Applied an argument to a non-function type `{}`", found,)
            },
            TypeError::FunctionParamNeedsAnnotation { ref name, .. } => write!(
                f,
                "Type annotation needed for the function parameter `{}`",
                name,
            ),
            TypeError::Mismatch {
                ref found,
                ref expected,
                ..
            } => write!(
                f,
                "Type mismatch: found `{}` but `{}` was expected",
                found, expected,
            ),
            TypeError::UnexpectedFunction { ref expected, .. } => {
                write!(f, "Found a function but expected `{}`", expected,)
            },
            TypeError::ExpectedUniverse { ref found, .. } => {
                write!(f, "Found `{}` but a universe was expected", found,)
            },
            TypeError::UndefinedName { ref name, .. } => write!(f, "Undefined name `{}`", name),
            TypeError::Internal(ref err) => write!(f, "Internal error - this is a bug! {}", err),
        }
    }
}
