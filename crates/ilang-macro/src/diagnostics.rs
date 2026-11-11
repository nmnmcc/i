//! Turn expansion failures into ordinary, source-located compiler diagnostics.
use crate::{body_engine::Error, session::Failure};
use ilang_host::syntax::DefinitionAbort;
use ilang_package::{Diagnostic, SourceId};

pub fn body(source: SourceId, error: Error) -> Vec<Diagnostic> {
    let code = ilang_check::code::MACRO_RESERVED;
    match error {
        Error::Compile(diagnostics) | Error::Preparation(diagnostics) => diagnostics,
        Error::SameModule(span) => vec![Diagnostic::error(
            code,
            Some(source),
            span,
            "a macro cannot be used in the module that defines it",
        )],
        Error::Depth(span) => vec![Diagnostic::error(
            code,
            Some(source),
            span,
            "macro expansion exceeds depth 256",
        )],
        Error::Conversion(error) => vec![Diagnostic::error(
            error.code.as_str(),
            Some(SourceId(error.origin.syntax.scope().module)),
            error.origin.syntax.span(),
            error.message,
        )],
        Error::Resource {
            error,
            macro_name,
            invocation_span,
            ..
        } => vec![Diagnostic::error(
            code,
            Some(source),
            invocation_span,
            format!("macro `{macro_name}`: {error}"),
        )],
        Error::Run {
            failure,
            macro_name,
            invocation_span,
            ..
        } => {
            let (location, span, message) = match failure {
                Failure::Syntax(error) => (source, error.span, error.message),
                Failure::Aborted(DefinitionAbort::Deferred { at }) => (
                    SourceId(at.scope().module),
                    at.span(),
                    "type definition query remained unresolved".into(),
                ),
                Failure::Aborted(
                    DefinitionAbort::CompileError { message, at }
                    | DefinitionAbort::MacroError { message, at },
                ) => (SourceId(at.scope().module), at.span(), message),
                Failure::Stopped(stop) => {
                    let message = if stop.message.is_none()
                        && (stop.trap.contains("stack") || stop.trap.contains("memory"))
                    {
                        format!("compiler resource failure: {stop}")
                    } else {
                        format!("macro stopped: {stop}")
                    };
                    (source, invocation_span, message)
                }
                Failure::Host(error) => (source, invocation_span, format!("{error:#}")),
            };
            vec![Diagnostic::error(
                code,
                Some(location),
                span,
                format!("macro `{macro_name}`: {message}"),
            )]
        }
    }
}
