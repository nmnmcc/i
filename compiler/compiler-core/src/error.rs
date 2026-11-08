// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2019 The Gleam contributors

#![allow(clippy::unwrap_used, clippy::expect_used)]
use crate::ast::{self};
use crate::build::{Origin, Outcome, Target};
use crate::diagnostic::{Diagnostic, ExtraLabel, Label, Location};

use crate::parse::error::ParseErrorDetails;
use crate::strings::{to_snake_case, to_upper_camel_case};
use crate::type_::collapse_links;
use crate::type_::error::{
    IncorrectArityContext, InvalidImportKind, MissingAnnotation, ModuleValueUsageContext, Named,
    RecordField, UnexpectedLabelledArgKind, UnknownField, UnknownTypeHint,
    UnsafeRecordUpdateReason,
};
use crate::type_::printer::{Names, Printer};
use crate::type_::{FieldAccessUsage, error::PatternMatchKind};
use crate::{ast::BinOp, parse::error::ParseErrorType, type_::Type};
use crate::{bit_array, diagnostic::Level, type_::UnifyErrorSituation};
use ecow::EcoString;
use itertools::Itertools;
use src_span::SrcSpan;
use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt::Debug;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use termcolor::Buffer;
use thiserror::Error;
use vec1::Vec1;

use camino::Utf8PathBuf;

pub type Name = EcoString;

pub type Result<Ok, Err = Error> = std::result::Result<Ok, Err>;

#[cfg(test)]
pub mod tests;

macro_rules! wrap_format {
    ($($tts:tt)*) => {
        crate::error::wrap(&format!($($tts)*))
    }
}
pub(crate) use wrap_format;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct UnknownImportDetails {
    pub module: Name,
    pub location: SrcSpan,
    pub path: Utf8PathBuf,
    pub src: EcoString,
    pub modules: Vec<EcoString>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ImportCycleLocationDetails {
    pub location: SrcSpan,
    pub path: Utf8PathBuf,
    pub src: EcoString,
}

/// Describes where a defined module comes from.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct DefinedModuleOrigin {
    /// The package defining the module
    pub package_name: EcoString,
    /// The path to the module
    pub path: Utf8PathBuf,
}

/// A module for which type checking failed.
#[derive(Debug, Eq, PartialEq, Error, Clone)]
#[error("type checking failed")]
pub struct FailedModule {
    pub path: Utf8PathBuf,
    pub src: EcoString,
    pub errors: Vec1<crate::type_::Error>,
    pub names: Box<Names>,
}

#[derive(Debug, Eq, PartialEq, Error, Clone)]
#[error("module couldn't be analysed")]
pub struct SkippedModule {
    pub path: Utf8PathBuf,
    pub code: EcoString,
    pub name: EcoString,
    /// The location where the module is importing a dependency that (directly
    /// or indirectly) was skipped.
    pub location: SrcSpan,
    pub reason: SkipReason,
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum SkipReason {
    /// The imported module has an error.
    DependencyHasError { name: EcoString },
    /// The imported module was skipped too, because it ultimately depends on
    /// some module with an error.
    DependencyWasSkipped {
        name: EcoString,
        /// The name of the module that does have an error, making it so this
        /// one cannot be compiled.
        erroring_module: EcoString,
    },
}

impl SkipReason {
    /// Returns the name of the module whose errors made it so the skipped
    /// module couldn't be compiled.
    pub fn erroring_module(&self) -> EcoString {
        match self {
            SkipReason::DependencyHasError { name } => name.clone(),
            SkipReason::DependencyWasSkipped {
                erroring_module, ..
            } => erroring_module.clone(),
        }
    }
}

#[derive(Debug, Eq, PartialEq, Error, Clone)]
pub enum Error {
    #[error("failed to parse i source code")]
    Parse {
        path: Utf8PathBuf,
        src: EcoString,
        error: Box<crate::parse::error::ParseError>,
    },

    #[error("type checking failed")]
    Type {
        /// All the modules that were skipped because they depend (directly or
        /// indirectly) on some other module that has an error.
        skipped_modules: Vec<SkippedModule>,
        /// All the modules that were analysed and had an error.
        /// This maps from module name to information about the module.
        failed_modules: HashMap<EcoString, FailedModule>,
    },

    #[error("unknown import {import}")]
    UnknownImport {
        import: EcoString,
        // Boxed to prevent this variant from being overly large
        details: Box<UnknownImportDetails>,
    },

    #[error("duplicate module {module}")]
    DuplicateModule {
        module: Name,
        first: DefinedModuleOrigin,
        second: DefinedModuleOrigin,
    },

    #[error("cyclical module imports")]
    ImportCycle {
        modules: Vec1<(EcoString, ImportCycleLocationDetails)>,
    },

    #[error("cyclical package dependencies")]
    PackageCycle { packages: Vec<EcoString> },

    #[error("{action:?} {path:?} failed: {cause:?}")]
    FileIo {
        kind: FileKind,
        action: FileIoAction,
        path: Utf8PathBuf,
        cause: FileIoCause,
    },

    #[error("Non Utf-8 Path: {path}")]
    NonUtf8Path { path: PathBuf },

    #[error("Failed to send LSP message: {error}")]
    LspMessageSendFailed { error: String },

    #[error("Failed to receive LSP message: {error}")]
    LspMessageReceiveFailed { error: String },

    #[error("io operation failed")]
    StandardIo {
        action: StandardIoAction,
        err: Option<std::io::ErrorKind>,
    },

    #[error("source code incorrectly formatted")]
    Format { problem_files: Vec<Unformatted> },

    #[error("{module} is not a valid module name")]
    InvalidModuleName { module: String },

    #[error("{module} is not module")]
    ModuleDoesNotExist {
        module: EcoString,
        suggestion: Option<EcoString>,
    },

    #[error("{module} does not have a main function")]
    ModuleDoesNotHaveMainFunction { module: EcoString, origin: Origin },

    #[error("{module} does not have a public main function")]
    MainFunctionIsPrivate { module: EcoString },

    #[error("{module}'s main function has the wrong arity so it can not be run")]
    MainFunctionHasWrongArity { module: EcoString, arity: usize },

    #[error("{module}'s main function does not support the current target")]
    MainFunctionDoesNotSupportTarget { module: EcoString, target: Target },

    #[error("{input} is not a valid version. {error}")]
    InvalidVersionFormat { input: String, error: String },

    #[error("unable to find project root")]
    UnableToFindProjectRoot { path: String },

    #[error("warnings are not permitted")]
    ForbiddenWarnings { count: usize },

    #[error("The package {0} is listed in dependencies and dev_dependencies")]
    DuplicateDependency(EcoString),

    #[error("The package {package} uses unsupported build tools {build_tools:?}")]
    UnsupportedBuildTool {
        package: String,
        build_tools: Vec<EcoString>,
    },

    #[error(
        "The package {package} requires an i version satisfying \
{required_version} and you are using v{ilang_version}"
    )]
    IncompatibleCompilerVersion {
        package: String,
        required_version: String,
        ilang_version: String,
    },
}

impl<T> From<Error> for Outcome<T, Error> {
    fn from(error: Error) -> Self {
        Outcome::TotalFailure(error)
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum StandardIoAction {
    Read,
    Write,
}

impl StandardIoAction {
    fn text(&self) -> &'static str {
        match self {
            StandardIoAction::Read => "read from",
            StandardIoAction::Write => "write to",
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum FileIoAction {
    Link(Utf8PathBuf),
    Open,
    // Path is `Option` here since there are some cases where we do not have
    // destination, like when writing ZIP archives
    Copy(Option<Utf8PathBuf>),
    Read,
    Parse,
    Delete,
    // Rename,
    Create,
    WriteTo,
    Canonicalise,
    UpdatePermissions,
    FindParent,
    ReadMetadata,
    Lock,
}

impl FileIoAction {
    fn text(&self) -> &'static str {
        match self {
            FileIoAction::Link(..) => "link",
            FileIoAction::Open => "open",
            FileIoAction::Lock => "lock",
            FileIoAction::Copy(..) => "copy",
            FileIoAction::Read => "read",
            FileIoAction::Parse => "parse",
            FileIoAction::Delete => "delete",
            // FileIoAction::Rename => "rename",
            FileIoAction::Create => "create",
            FileIoAction::WriteTo => "write to",
            FileIoAction::FindParent => "find the parent of",
            FileIoAction::Canonicalise => "canonicalise",
            FileIoAction::UpdatePermissions => "update permissions of",
            FileIoAction::ReadMetadata => "read metadata of",
        }
    }

    fn is_link(&self) -> bool {
        match self {
            FileIoAction::Link(..) => true,

            FileIoAction::Open
            | FileIoAction::Copy(..)
            | FileIoAction::Read
            | FileIoAction::Lock
            | FileIoAction::Parse
            | FileIoAction::Delete
            | FileIoAction::Create
            | FileIoAction::WriteTo
            | FileIoAction::Canonicalise
            | FileIoAction::UpdatePermissions
            | FileIoAction::FindParent
            | FileIoAction::ReadMetadata => false,
        }
    }

    /// Returns a destination path of action, if any
    fn destination(&self) -> Option<&Utf8PathBuf> {
        match self {
            FileIoAction::Link(destination) | FileIoAction::Copy(Some(destination)) => {
                Some(destination)
            }

            FileIoAction::Open
            | FileIoAction::Read
            | FileIoAction::Lock
            | FileIoAction::Parse
            | FileIoAction::Delete
            | FileIoAction::Create
            | FileIoAction::WriteTo
            | FileIoAction::Canonicalise
            | FileIoAction::UpdatePermissions
            | FileIoAction::FindParent
            | FileIoAction::ReadMetadata
            | FileIoAction::Copy(None) => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileIoCause {
    Other(String),
    CacheMetadataFormatIncorrect,
}

impl FileIoCause {
    pub fn std_io(error: std::io::Error) -> Self {
        Self::Other(error.to_string())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileKind {
    File,
    Directory,
}

impl FileKind {
    fn text(&self) -> &'static str {
        match self {
            FileKind::File => "file",
            FileKind::Directory => "directory",
        }
    }
}

// https://github.com/rust-lang/rust/blob/03994e498df79aa1f97f7bbcfd52d57c8e865049/compiler/rustc_span/src/edit_distance.rs
pub fn edit_distance(a: &str, b: &str, limit: usize) -> Option<usize> {
    let mut a = &a.chars().collect::<Vec<_>>()[..];
    let mut b = &b.chars().collect::<Vec<_>>()[..];

    if a.len() < b.len() {
        std::mem::swap(&mut a, &mut b);
    }

    let min_dist = a.len() - b.len();
    // If we know the limit will be exceeded, we can return early.
    if min_dist > limit {
        return None;
    }

    // Strip common prefix.
    while !b.is_empty() && !a.is_empty() {
        let (b_first, b_rest) = b.split_last().expect("Failed to split 'b' slice");
        let (a_first, a_rest) = a.split_last().expect("Failed to split 'a' slice");

        if b_first == a_first {
            a = a_rest;
            b = b_rest;
        } else {
            break;
        }
    }

    // If either string is empty, the distance is the length of the other.
    // We know that `b` is the shorter string, so we don't need to check `a`.
    if b.is_empty() {
        return Some(min_dist);
    }

    let mut prev_prev = vec![usize::MAX; b.len() + 1];
    let mut prev = (0..=b.len()).collect::<Vec<_>>();
    let mut current = vec![0; b.len() + 1];

    // row by row
    for i in 1..=a.len() {
        if let Some(element) = current.get_mut(0) {
            *element = i;
        }
        let a_idx = i - 1;

        // column by column
        for j in 1..=b.len() {
            let b_idx = j - 1;

            // There is no cost to substitute a character with itself.
            let substitution_cost = match (a.get(a_idx), b.get(b_idx)) {
                (Some(&a_char), Some(&b_char)) => {
                    if a_char == b_char {
                        0
                    } else {
                        1
                    }
                }
                _ => panic!("Index out of bounds"),
            };

            let insertion = current.get(j - 1).map_or(usize::MAX, |&x| x + 1);

            if let Some(value) = current.get_mut(j) {
                *value = std::cmp::min(
                    // deletion
                    prev.get(j).map_or(usize::MAX, |&x| x + 1),
                    std::cmp::min(
                        // insertion
                        insertion,
                        // substitution
                        prev.get(j - 1)
                            .map_or(usize::MAX, |&x| x + substitution_cost),
                    ),
                );
            }

            if (i > 1)
                && (j > 1)
                && let (Some(&a_val), Some(&b_val_prev), Some(&a_val_prev), Some(&b_val)) = (
                    a.get(a_idx),
                    b.get(b_idx - 1),
                    a.get(a_idx - 1),
                    b.get(b_idx),
                )
                && (a_val == b_val_prev)
                && (a_val_prev == b_val)
            {
                // transposition
                if let Some(curr) = current.get_mut(j)
                    && let Some(&prev_prev_val) = prev_prev.get(j - 2)
                {
                    *curr = std::cmp::min(*curr, prev_prev_val + 1);
                }
            }
        }

        // Rotate the buffers, reusing the memory.
        [prev_prev, prev, current] = [prev, current, prev_prev];
    }

    // `prev` because we already rotated the buffers.
    let distance = match prev.get(b.len()) {
        Some(&d) => d,
        None => usize::MAX,
    };
    (distance <= limit).then_some(distance)
}

fn edit_distance_with_substrings(a: &str, b: &str, limit: usize) -> Option<usize> {
    let n = a.chars().count();
    let m = b.chars().count();

    // Check one isn't less than half the length of the other. If this is true then there is a
    // big difference in length.
    let big_len_diff = (n * 2) < m || (m * 2) < n;
    let len_diff = m.abs_diff(n);
    let distance = edit_distance(a, b, limit + len_diff)?;

    // This is the crux, subtracting length difference means exact substring matches will now be 0
    let score = distance - len_diff;

    // If the score is 0 but the words have different lengths then it's a substring match not a full
    // word match
    let score = if score == 0 && len_diff > 0 && !big_len_diff {
        1 // Exact substring match, but not a total word match so return non-zero
    } else if !big_len_diff {
        // Not a big difference in length, discount cost of length difference
        score + len_diff.div_ceil(2)
    } else {
        // A big difference in length, add back the difference in length to the score
        score + len_diff
    };

    (score <= limit).then_some(score)
}

/// Returns a "Did you mean ...?" suggestion using the string from `options` that's most similar to
/// `name`.
pub fn did_you_mean(name: &str, options: &[EcoString]) -> Option<String> {
    // If only one option is given, return that option.
    // This seems to solve the `unknown_variable_3` test.
    if options.len() == 1 {
        return options
            .first()
            .map(|option| format!("Did you mean `{option}`?"));
    }

    // Check for case-insensitive matches.
    // This solves the comparison to small and single character terms,
    // such as the test on `type_vars_must_be_declared`.
    if let Some(exact_match) = options
        .iter()
        .find(|&option| option.eq_ignore_ascii_case(name))
    {
        return Some(format!("Did you mean `{exact_match}`?"));
    }

    // Calculate the threshold as one third of the name's length, with a minimum of 1.
    let threshold = std::cmp::max(name.chars().count() / 3, 1);

    // Filter and sort options based on edit distance.
    options
        .iter()
        .filter(|&option| option != ast::CAPTURE_VARIABLE)
        .sorted()
        .filter_map(|option| {
            edit_distance_with_substrings(option, name, threshold)
                .map(|distance| (option, distance))
        })
        .min_by_key(|&(_, distance)| distance)
        .map(|(option, _)| format!("Did you mean `{option}`?"))
}

fn to_ordinal(value: u32) -> String {
    match value % 10 {
        // All numbers starting with 1 end in `th` (11th, 12th, 13th, etc.)
        _ if value / 10 == 1 => format!("{value}th"),
        1 => format!("{value}st"),
        2 => format!("{value}nd"),
        3 => format!("{value}rd"),
        _ => format!("{value}th"),
    }
}

impl Error {
    pub fn pretty_string(&self) -> String {
        let mut nocolor = Buffer::no_color();
        self.pretty(&mut nocolor);
        String::from_utf8(nocolor.into_inner()).expect("Error printing produced invalid utf8")
    }

    pub fn pretty(&self, buffer: &mut Buffer) {
        for diagnostic in self.to_diagnostics() {
            diagnostic.write(buffer);
            writeln!(buffer).expect("write new line after diagnostic");
        }
    }

    pub fn skipped_files_diagnostics(&self) -> HashMap<Utf8PathBuf, Diagnostic> {
        let Error::Type {
            skipped_modules, ..
        } = self
        else {
            return HashMap::new();
        };

        skipped_modules
            .iter()
            .map(|skipped_module| {
                let text = match &skipped_module.reason {
                    SkipReason::DependencyHasError { .. } => {
                        "The current module cannot be analysed because this \
imported module has errors."
                            .to_string()
                    }
                    SkipReason::DependencyWasSkipped {
                        erroring_module, ..
                    } => {
                        format!(
                            "The current module cannot be analysed because \
this imported module depends on the `{erroring_module}` module, which has \
errors."
                        )
                    }
                };

                let diagnostic = Diagnostic {
                    title: "Imported module with errors".into(),
                    text: wrap(&text),
                    level: Level::Error,
                    location: Some(Location {
                        src: skipped_module.code.clone(),
                        path: skipped_module.path.clone(),
                        label: Label {
                            text: None,
                            span: skipped_module.location,
                        },
                        extra_labels: vec![],
                    }),
                    hint: Some(format!(
                        "fix the errors in the `{}` module first.",
                        skipped_module.reason.erroring_module()
                    )),
                };

                (skipped_module.path.clone(), diagnostic)
            })
            .collect()
    }

    pub fn to_diagnostics(&self) -> Vec<Diagnostic> {
        match self {
            Error::InvalidModuleName { module } => vec![Diagnostic {
                title: "Invalid module name".into(),
                text: format!(
                    "`{module}` is not a valid module name.
Module names can only contain lowercase letters, underscore, and
forward slash and must not end with a slash."
                ),
                level: Level::Error,
                location: None,
                hint: None,
            }],

            Error::ModuleDoesNotExist { module, suggestion } => {
                let hint = match suggestion {
                    Some(suggestion) => format!("Did you mean `{suggestion}`?"),
                    None => {
                        // If the module ends with "_dev", it should be created in the "dev"
                        // directory, if it ends with "_test", it should be created in the "test"
                        // directory, and otherwise it should be created in "src" directory
                        let directory = if module.ends_with("_dev") {
                            "dev"
                        } else if module.ends_with("_test") {
                            "test"
                        } else {
                            "src"
                        };

                        format!("Try creating the file `{directory}/{module}.i`.")
                    }
                };
                vec![Diagnostic {
                    title: "Module does not exist".into(),
                    text: format!("Module `{module}` was not found."),
                    level: Level::Error,
                    location: None,
                    hint: Some(hint),
                }]
            }

            Error::ModuleDoesNotHaveMainFunction { module, origin } => vec![Diagnostic {
                title: "Module does not have a main function".into(),
                text: wrap_format!(
                    "`{module}` does not have a main function so the module can not be run."
                ),
                level: Level::Error,
                location: None,
                hint: Some(format!(
                    "Add a public `main` function to `{}/{module}.i`.",
                    origin.folder_name()
                )),
            }],

            Error::MainFunctionIsPrivate { module } => vec![Diagnostic {
                title: "Module does not have a public main function".into(),
                text: wrap_format!(
                    "`{module}` has a main function, but it is private, so it cannot be run."
                ),
                level: Level::Error,
                location: None,
                hint: Some(wrap_format!(
                    "Make the `main` function in the `{module}` module public."
                )),
            }],

            Error::MainFunctionDoesNotSupportTarget { module, target } => vec![Diagnostic {
                title: "Target not supported".into(),
                text: wrap_format!(
                    "`{module}` has a main function, but it does not support the {target} \
target, so it cannot be run.",
                    target = target.as_presentable_str(),
                ),
                level: Level::Error,
                location: None,
                hint: None,
            }],

            Error::MainFunctionHasWrongArity { module, arity } => vec![Diagnostic {
                title: "Main function has wrong arity".into(),
                text: wrap_format!(
                    "`{module}.main` should take no arguments, but it takes {arity}."
                ),
                level: Level::Error,
                location: None,
                hint: Some(
                    "Change the function signature of main to `pub fn main() -> Nil`.".into(),
                ),
            }],

            Error::UnableToFindProjectRoot { path } => {
                let text = wrap_format!(
                    "We were unable to find i.toml.

We searched in {path} and all parent directories."
                );
                vec![Diagnostic {
                    title: "Project not found".into(),
                    text,
                    hint: None,
                    level: Level::Error,
                    location: None,
                }]
            }

            Error::DuplicateModule {
                module,
                first,
                second,
            } => {
                // If the conflicting modules come from the same package just
                // showing the same package name twice is not all that useful.
                // So we will show the path to each one!
                let should_show_path = first.package_name == second.package_name;
                let format_origin = |origin: &DefinedModuleOrigin| {
                    if should_show_path {
                        format!("at {}", origin.path)
                    } else {
                        format!("by the package {}", origin.package_name)
                    }
                };

                let text = format!(
                    "The module `{module}` is defined multiple times.

It is first defined {}
It is defined a second time {}",
                    format_origin(first),
                    format_origin(second)
                );

                vec![Diagnostic {
                    title: "Duplicate module".into(),
                    text,
                    hint: None,
                    level: Level::Error,
                    location: None,
                }]
            }

            Error::FileIo {
                kind,
                action,
                path,
                cause,
            } => {
                let mut text = format!(
                    "An error occurred while trying to {} this {}:

    {}",
                    action.text(),
                    kind.text(),
                    path
                );
                if let Some(destination) = action.destination() {
                    text.push_str("\n\nTo:\n\n    ");
                    text.push_str(destination.as_str());
                }
                match cause {
                    FileIoCause::Other(error) => {
                        text.push_str(
                            "\n\nThe error message from the file IO library was:\n\n    ",
                        );
                        text.push_str(error.as_str());
                    }
                    FileIoCause::CacheMetadataFormatIncorrect => text.push_str(
                        "

The data in this file was not in the expected format, so it could
not be understood by the compiler.

This can happen if the data file was created with a different
version of i, or if the files have been corrupted in some way.",
                    ),
                };
                if cfg!(target_family = "windows") && action.is_link() {
                    text.push_str("

Windows does not support symbolic links without developer mode
or admin privileges. Please enable developer mode and try again.

https://learn.microsoft.com/en-us/windows/apps/get-started/enable-your-device-for-development#activate-developer-mode");
                }
                vec![Diagnostic {
                    title: "File IO failure".into(),
                    text,
                    hint: None,
                    level: Level::Error,
                    location: None,
                }]
            }

            Error::NonUtf8Path { path } => {
                let text = format!(
                    "Encountered a non UTF-8 path '{}', but only UTF-8 paths are supported.",
                    path.to_string_lossy()
                );
                vec![Diagnostic {
                    title: "Non UTF-8 Path Encountered".into(),
                    text,
                    level: Level::Error,
                    location: None,
                    hint: None,
                }]
            }

            Error::LspMessageSendFailed { error } => {
                let text = wrap_format!(
                    "An error occurred when the i language server \
attempted to send a message to the client:

    {error}

Is your editor still running and connected to the language server?"
                );
                vec![Diagnostic {
                    title: "Failed to send LSP message".into(),
                    text,
                    hint: None,
                    level: Level::Error,
                    location: None,
                }]
            }

            Error::LspMessageReceiveFailed { error } => {
                let text = wrap_format!(
                    "An error occurred when the i language server \
attempted to receive a message from the client:

    {error}

Is your editor still running and connected to the language server?"
                );
                vec![Diagnostic {
                    title: "Failed to send LSP message".into(),
                    text,
                    hint: None,
                    level: Level::Error,
                    location: None,
                }]
            }

            Error::Type {
                skipped_modules: _,
                failed_modules,
            } => failed_modules
                .values()
                .sorted_by_key(|failed_module| &failed_module.path)
                .flat_map(failed_module_diagnostics)
                .collect_vec(),

            Error::Parse { path, src, error } => {
                let location = if error.error == ParseErrorType::UnexpectedEof {
                    SrcSpan {
                        start: (src.len() - 1) as u32,
                        end: (src.len() - 1) as u32,
                    }
                } else {
                    error.location
                };

                let title = String::from("Syntax error");
                let ParseErrorDetails {
                    text,
                    label_text,
                    extra_labels,
                    hint,
                } = error.error.details();
                vec![Diagnostic {
                    title,
                    text,
                    level: Level::Error,
                    location: Some(Location {
                        src: src.clone(),
                        path: path.clone(),
                        label: Label {
                            text: Some(label_text.into()),
                            span: location,
                        },
                        extra_labels,
                    }),
                    hint,
                }]
            }

            Error::ImportCycle { modules } => {
                let first_location = &modules.first().1;
                let rest_locations = modules
                    .iter()
                    .skip(1)
                    .map(|(_, l)| ExtraLabel {
                        label: Label {
                            text: Some("Imported here".into()),
                            span: l.location,
                        },
                        src_info: Some((l.src.clone(), l.path.clone())),
                    })
                    .collect_vec();
                let mut text = "The import statements for these modules form a cycle:
"
                .into();
                let mod_names = modules.iter().map(|m| m.0.clone()).collect_vec();
                write_cycle(&mut text, &mod_names);
                text.push_str(&wrap(
                    "i doesn't support dependency cycles like these, \
please break the cycle to continue.",
                ));
                vec![Diagnostic {
                    title: "Import cycle".into(),
                    text,
                    hint: None,
                    level: Level::Error,
                    location: Some(Location {
                        label: Label {
                            text: Some("Imported here".into()),
                            span: first_location.location,
                        },
                        path: first_location.path.clone(),
                        src: first_location.src.clone(),
                        extra_labels: rest_locations,
                    }),
                }]
            }

            Error::PackageCycle { packages } => {
                let mut text = "The dependencies for these packages form a cycle:
"
                .into();
                write_cycle(&mut text, packages);
                text.push_str(&wrap(
                    "i doesn't support dependency cycles like these, \
please break the cycle to continue.",
                ));
                vec![Diagnostic {
                    title: "Dependency cycle".into(),
                    text,
                    hint: None,
                    level: Level::Error,
                    location: None,
                }]
            }

            Error::UnknownImport { import, details } => {
                let UnknownImportDetails {
                    module,
                    location,
                    path,
                    src,
                    modules,
                } = details.as_ref();
                let text = wrap(&format!(
                    "The module `{module}` is trying to import the module \
`{import}`, but it cannot be found."
                ));
                vec![Diagnostic {
                    title: "Unknown import".into(),
                    text,
                    hint: None,
                    level: Level::Error,
                    location: Some(Location {
                        label: Label {
                            text: did_you_mean(import, modules),
                            span: *location,
                        },
                        path: path.clone(),
                        src: src.clone(),
                        extra_labels: vec![],
                    }),
                }]
            }

            Error::StandardIo { action, err } => {
                let err = match err {
                    Some(e) => format!(
                        "\nThe error message from the stdio library was:\n\n    {}\n",
                        std_io_error_kind_text(e)
                    ),
                    None => "".into(),
                };
                vec![Diagnostic {
                    title: "Standard IO failure".into(),
                    text: format!(
                        "An error occurred while trying to {}:

{}",
                        action.text(),
                        err,
                    ),
                    hint: None,
                    location: None,
                    level: Level::Error,
                }]
            }

            Error::Format { problem_files } => {
                let files: Vec<_> = problem_files
                    .iter()
                    .map(|formatted| formatted.source.as_str())
                    .map(|p| format!("  - {p}"))
                    .sorted()
                    .collect();
                let mut text = files.iter().join("\n");
                text.push('\n');
                vec![Diagnostic {
                    title: "These files have not been formatted".into(),
                    text,
                    hint: None,
                    location: None,
                    level: Level::Error,
                }]
            }

            Error::ForbiddenWarnings { count } => {
                let word_warning = match count {
                    1 => "warning",
                    _ => "warnings",
                };
                let text = "Your project was compiled with the `--warnings-as-errors` flag.
Fix the warnings and try again."
                    .into();
                vec![Diagnostic {
                    title: format!("{count} {word_warning} generated."),
                    text,
                    hint: None,
                    location: None,
                    level: Level::Error,
                }]
            }

            Error::InvalidVersionFormat { input, error } => {
                let text = format!(
                    "I was unable to parse the version \"{input}\".
The error from the parser was:

    {error}"
                );
                vec![Diagnostic {
                    title: "Invalid version format".into(),
                    text,
                    hint: None,
                    location: None,
                    level: Level::Error,
                }]
            }

            Error::DuplicateDependency(name) => {
                let text = format!(
                    "The package `{name}` is specified in both the dependencies and
dev_dependencies sections of the i.toml file."
                );
                vec![Diagnostic {
                    title: "Dependency duplicated".into(),
                    text,
                    hint: None,
                    location: None,
                    level: Level::Error,
                }]
            }

            Error::UnsupportedBuildTool {
                package,
                build_tools,
            } => {
                let text = wrap_format!(
                    "The package `{}` declares unsupported build tools: {:?}.

iLang builds local i source packages for WebAssembly. Migrate native \
implementations to Wasm externals and use a local path dependency whose \
only build tool is `gleam`.",
                    package,
                    build_tools
                );
                vec![Diagnostic {
                    title: "Package requires migration to Wasm".into(),
                    text,
                    hint: None,
                    location: None,
                    level: Level::Error,
                }]
            }

            Error::IncompatibleCompilerVersion {
                package,
                required_version,
                ilang_version,
            } => {
                let text = format!(
                    "The package `{package}` requires an i version \
satisfying {required_version} but you are using v{ilang_version}.",
                );
                vec![Diagnostic {
                    title: "Incompatible i version".into(),
                    text,
                    hint: None,
                    location: None,
                    level: Level::Error,
                }]
            }
        }
    }
}

fn failed_module_diagnostics(failed_module: &FailedModule) -> impl Iterator<Item = Diagnostic> {
    use crate::type_::Error as TypeError;
    let FailedModule {
        path,
        src,
        errors,
        names,
    } = failed_module;

    errors.iter().map(|error| match error {
        TypeError::LiteralFloatOutOfRange { location, .. } => Diagnostic {
            title: "Float outside of valid range".into(),
            text: wrap(
                "This float value is too large to be represented by \
a floating point type: float values must be in the range -1.7976931348623157e308 \
- 1.7976931348623157e308.",
            ),
            hint: None,
            level: Level::Error,
            location: Some(Location {
                label: Label {
                    text: None,
                    span: *location,
                },
                path: path.clone(),
                src: src.clone(),
                extra_labels: vec![],
            }),
        },

        TypeError::InvalidImport {
            location,
            importing_module,
            imported_module,
            kind: InvalidImportKind::SrcImportingTest,
        } => {
            let text = wrap_format!(
                "The application module `{importing_module}` \
is importing the test module `{imported_module}`.

Test modules are not included in production builds so application \
modules cannot import them. Perhaps move the `{imported_module}` \
module to the src directory.",
            );

            Diagnostic {
                title: "App importing test module".into(),
                text,
                hint: None,
                level: Level::Error,
                location: Some(Location {
                    label: Label {
                        text: Some("Imported here".into()),
                        span: *location,
                    },
                    path: path.clone(),
                    src: src.clone(),
                    extra_labels: vec![],
                }),
            }
        }

        TypeError::InvalidImport {
            location,
            importing_module,
            imported_module,
            kind: InvalidImportKind::SrcImportingDev,
        } => {
            let text = wrap_format!(
                "The application module `{importing_module}` \
is importing the development module `{imported_module}`.

Development modules are not included in production builds so application \
modules cannot import them. Perhaps move the `{imported_module}` \
module to the src directory.",
            );

            Diagnostic {
                title: "App importing dev module".into(),
                text,
                hint: None,
                level: Level::Error,
                location: Some(Location {
                    label: Label {
                        text: Some("Imported here".into()),
                        span: *location,
                    },
                    path: path.clone(),
                    src: src.clone(),
                    extra_labels: vec![],
                }),
            }
        }

        TypeError::InvalidImport {
            location,
            importing_module,
            imported_module,
            kind: InvalidImportKind::DevImportingTest,
        } => {
            let text = wrap_format!(
                "The development module `{importing_module}` \
is importing the test module `{imported_module}`.

Test modules should only contain test-related code, and not general development \
code. Perhaps move the `{imported_module}` module to the dev directory.",
            );

            Diagnostic {
                title: "Dev importing test module".into(),
                text,
                hint: None,
                level: Level::Error,
                location: Some(Location {
                    label: Label {
                        text: Some("Imported here".into()),
                        span: *location,
                    },
                    path: path.clone(),
                    src: src.clone(),
                    extra_labels: vec![],
                }),
            }
        }

        TypeError::UnknownLabels {
            unknown,
            valid,
            supplied,
        } => {
            let other_labels: Vec<_> = valid
                .iter()
                .filter(|label| !supplied.contains(label))
                .cloned()
                .collect();

            let title = if unknown.len() > 1 {
                "Unknown labels"
            } else {
                "Unknown label"
            }
            .into();

            let mut labels = unknown.iter().map(|(label, location)| {
                let text =
                    did_you_mean(label, &other_labels).unwrap_or_else(|| "Unexpected label".into());
                Label {
                    text: Some(text),
                    span: *location,
                }
            });
            let label = labels.next().expect("Unknown labels first label");
            let extra_labels = labels
                .map(|label| ExtraLabel {
                    src_info: None,
                    label,
                })
                .collect();
            let text = if valid.is_empty() {
                "This constructor does not accept any labelled arguments.".into()
            } else if other_labels.is_empty() {
                "You have already supplied all the labelled arguments that this
constructor accepts."
                    .into()
            } else {
                let mut label_text = String::from("It accepts these labels:\n");
                for label in other_labels.iter().sorted() {
                    label_text.push_str("\n    ");
                    label_text.push_str(label);
                }
                label_text
            };
            Diagnostic {
                title,
                text,
                hint: None,
                level: Level::Error,
                location: Some(Location {
                    label,
                    path: path.clone(),
                    src: src.clone(),
                    extra_labels,
                }),
            }
        }

        TypeError::UnexpectedLabelledArg {
            location,
            label,
            kind,
        } => {
            let kind = match kind {
                UnexpectedLabelledArgKind::FunctionParameter => "function",
                UnexpectedLabelledArgKind::RecordConstructorArgument => "record constructor",
            };
            let text = format!(
                "This argument has been given a label but the {kind} does
not expect any. Please remove the label `{label}`."
            );
            Diagnostic {
                title: "Unexpected labelled argument".into(),
                text,
                hint: None,
                level: Level::Error,
                location: Some(Location {
                    label: Label {
                        text: None,
                        span: *location,
                    },
                    path: path.clone(),
                    src: src.clone(),
                    extra_labels: vec![],
                }),
            }
        }

        TypeError::PositionalArgumentAfterLabelled { location } => {
            let text = wrap(
                "This unlabeled argument has been \
supplied after a labelled argument.
Once a labelled argument has been supplied all following arguments must
also be labelled.",
            );

            Diagnostic {
                title: "Unexpected positional argument".into(),
                text,
                hint: None,
                level: Level::Error,
                location: Some(Location {
                    label: Label {
                        text: None,
                        span: *location,
                    },
                    path: path.clone(),
                    src: src.clone(),
                    extra_labels: vec![],
                }),
            }
        }

        TypeError::DuplicateImport {
            location,
            previous_location,
            name,
        } => {
            let text = format!(
                "`{name}` has been imported multiple times.
Names in an i module must be unique so one will need to be renamed."
            );
            Diagnostic {
                title: "Duplicate import".into(),
                text,
                hint: None,
                level: Level::Error,
                location: Some(Location {
                    label: Label {
                        text: Some("Reimported here".into()),
                        span: *location,
                    },
                    path: path.clone(),
                    src: src.clone(),
                    extra_labels: vec![ExtraLabel {
                        src_info: None,
                        label: Label {
                            text: Some("First imported here".into()),
                            span: *previous_location,
                        },
                    }],
                }),
            }
        }

        TypeError::DuplicateName {
            location_a,
            location_b,
            name,
            ..
        } => {
            let (first_location, second_location) = if location_a.start < location_b.start {
                (location_a, location_b)
            } else {
                (location_b, location_a)
            };
            let text = format!(
                "`{name}` has been defined multiple times.
Names in an i module must be unique so one will need to be renamed."
            );
            Diagnostic {
                title: "Duplicate definition".into(),
                text,
                hint: None,
                level: Level::Error,
                location: Some(Location {
                    label: Label {
                        text: Some("Redefined here".into()),
                        span: *second_location,
                    },
                    path: path.clone(),
                    src: src.clone(),
                    extra_labels: vec![ExtraLabel {
                        src_info: None,
                        label: Label {
                            text: Some("First defined here".into()),
                            span: *first_location,
                        },
                    }],
                }),
            }
        }

        TypeError::DuplicateTypeName {
            name,
            location,
            previous_location,
            ..
        } => {
            let text = format!(
                "The type `{name}` has been defined multiple times.
Names in an i module must be unique so one will need to be renamed."
            );
            Diagnostic {
                title: "Duplicate type definition".into(),
                text,
                hint: None,
                level: Level::Error,
                location: Some(Location {
                    label: Label {
                        text: Some("Redefined here".into()),
                        span: *location,
                    },
                    path: path.clone(),
                    src: src.clone(),
                    extra_labels: vec![ExtraLabel {
                        src_info: None,
                        label: Label {
                            text: Some("First defined here".into()),
                            span: *previous_location,
                        },
                    }],
                }),
            }
        }

        TypeError::DuplicateField { location, label } => {
            let text = format!("The label `{label}` has already been defined. Rename this label.");
            Diagnostic {
                title: "Duplicate label".into(),
                text,
                hint: None,
                level: Level::Error,
                location: Some(Location {
                    label: Label {
                        text: None,
                        span: *location,
                    },
                    path: path.clone(),
                    src: src.clone(),
                    extra_labels: vec![],
                }),
            }
        }

        TypeError::DuplicateArgument { location, label } => {
            let text = format!("The labelled argument `{label}` has already been supplied.");
            Diagnostic {
                title: "Duplicate argument".into(),
                text,
                hint: None,
                level: Level::Error,
                location: Some(Location {
                    label: Label {
                        text: None,
                        span: *location,
                    },
                    path: path.clone(),
                    src: src.clone(),
                    extra_labels: vec![],
                }),
            }
        }

        TypeError::RecursiveType { location } => {
            let text = wrap(
                "I don't know how to work out what type this \
value has. It seems to be defined in terms of itself.",
            );
            Diagnostic {
                title: "Recursive type".into(),
                text,
                hint: Some("Add some type annotations and try again.".into()),
                level: Level::Error,
                location: Some(Location {
                    label: Label {
                        text: None,
                        span: *location,
                    },
                    path: path.clone(),
                    src: src.clone(),
                    extra_labels: vec![],
                }),
            }
        }

        TypeError::NotFn { location, type_ } => {
            let mut printer = Printer::new(names);
            let text = format!(
                "This value is being called as a function but its type is:\n\n    {}",
                printer.print_type(type_)
            );
            Diagnostic {
                title: "Type mismatch".into(),
                text,
                hint: None,
                level: Level::Error,
                location: Some(Location {
                    label: Label {
                        text: None,
                        span: *location,
                    },
                    path: path.clone(),
                    src: src.clone(),
                    extra_labels: vec![],
                }),
            }
        }

        TypeError::UnknownRecordField {
            usage,
            location,
            type_,
            label,
            fields,
            unknown_field: variants,
        } => {
            let mut printer = Printer::new(names);

            // Give a hint about what type this value has.
            let mut text = format!(
                "The value being accessed has this type:\n\n    {}\n",
                printer.print_type(type_)
            );

            // Give a hint about what record fields this value has, if any.
            if fields.is_empty() {
                if variants == &UnknownField::NoFields {
                    text.push_str("\nIt does not have any fields.");
                } else {
                    text.push_str(
                        "\nIt does not have fields that are common \
across all variants.",
                    );
                }
            } else {
                text.push_str("\nIt has these accessible fields:\n");
            }
            for field in fields.iter().sorted() {
                text.push_str("\n    .");
                text.push_str(field);
            }

            match variants {
                UnknownField::AppearsInAVariant => {
                    let msg = wrap(
                        "Note: The field you are trying to access is \
not defined consistently across all variants of this custom type. To fix this, \
ensure that all variants include the field with the same name, position, and \
type.",
                    );
                    text.push_str("\n\n");
                    text.push_str(&msg);
                }
                UnknownField::AppearsInAnImpossibleVariant => {
                    let msg = wrap(
                        "Note: The field exists in this custom type \
but is not defined for the current variant. Ensure that you are accessing the \
field on a variant where it is valid.",
                    );
                    text.push_str("\n\n");
                    text.push_str(&msg);
                }
                UnknownField::TrulyUnknown => (),
                UnknownField::NoFields => (),
            }

            // Give a hint about i not having OOP methods if it
            // looks like they might be trying to call one.
            match usage {
                FieldAccessUsage::MethodCall => {
                    let msg = wrap(
                        "i is not object oriented, so if you are trying \
to call a method on this value you may want to use the function syntax instead.",
                    );
                    text.push_str("\n\n");
                    text.push_str(&msg);
                    text.push_str("\n\n    ");
                    text.push_str(label);
                    text.push_str("(value)");
                }
                FieldAccessUsage::Other | FieldAccessUsage::RecordUpdate => (),
            }

            let label =
                did_you_mean(label, fields).unwrap_or_else(|| "This field does not exist".into());
            Diagnostic {
                title: "Unknown record field".into(),
                text,
                hint: None,
                level: Level::Error,
                location: Some(Location {
                    label: Label {
                        text: Some(label),
                        span: *location,
                    },
                    path: path.clone(),
                    src: src.clone(),
                    extra_labels: vec![],
                }),
            }
        }

        TypeError::CouldNotUnify {
            location,
            expected,
            given,
            situation: Some(UnifyErrorSituation::Operator(op)),
        } => {
            let mut printer = Printer::new(names);
            let text = format!(
                "The {op} operator expects arguments of this type:

    {expected}

But this argument has this type:

    {given}",
                op = op.name(),
                expected = printer.print_type(expected),
                given = printer.print_type(given),
            );
            Diagnostic {
                title: "Type mismatch".into(),
                text,
                hint: hint_alternative_operator(op, given),
                level: Level::Error,
                location: Some(Location {
                    label: Label {
                        text: None,
                        span: *location,
                    },
                    path: path.clone(),
                    src: src.clone(),
                    extra_labels: vec![],
                }),
            }
        }

        TypeError::CouldNotUnify {
            location,
            expected,
            given,
            situation: Some(UnifyErrorSituation::PipeTypeMismatch),
        } => {
            // Remap the pipe function type into just the type expected by the pipe.
            let expected = expected
                .fn_types()
                .and_then(|(arguments, _)| arguments.first().cloned());

            // Remap the argument as well, if it's a function.
            let given = given
                .fn_types()
                .and_then(|(arguments, _)| arguments.first().cloned())
                .unwrap_or_else(|| given.clone());

            let mut printer = Printer::new(names);
            let text = format!(
                "The argument is:

    {given}

But function expects:

    {expected}",
                expected = expected
                    .map(|v| printer.print_type(&v))
                    .unwrap_or_else(|| "    No arguments".into()),
                given = printer.print_type(&given)
            );

            Diagnostic {
                title: "Type mismatch".into(),
                text,
                hint: None,
                level: Level::Error,
                location: Some(Location {
                    label: Label {
                        text: Some("This function does not accept the piped type".into()),
                        span: *location,
                    },
                    path: path.clone(),
                    src: src.clone(),
                    extra_labels: vec![],
                }),
            }
        }

        TypeError::CouldNotUnify {
            location,
            expected,
            given,
            situation,
        } => {
            let mut printer = Printer::new(names);
            let mut text =
                if let Some(description) = situation.as_ref().and_then(|s| s.description()) {
                    let mut text = description.to_string();
                    text.push('\n');
                    text.push('\n');
                    text
                } else {
                    "".into()
                };
            text.push_str("Expected type:\n\n    ");
            text.push_str(&printer.print_type(expected));
            text.push_str("\n\nFound type:\n\n    ");
            text.push_str(&printer.print_type(given));

            let (main_message_location, main_message_text, extra_labels) = match situation {
                // When the mismatch error comes from a case clause we want to highlight the
                // entire branch (pattern included) when reporting the error; in addition,
                // if the error could be resolved just by wrapping the value in an `Ok`
                // or `Error` we want to add an additional label with this hint below the
                // offending value.
                Some(UnifyErrorSituation::CaseClauseMismatch { clause_location }) => {
                    (clause_location, None, vec![])
                }
                // In all other cases we just highlight the offending expression, optionally
                // adding the wrapping hint if it makes sense.
                Some(_) | None => (location, hint_wrap_value_in_result(expected, given), vec![]),
            };

            Diagnostic {
                title: "Type mismatch".into(),
                text,
                hint: None,
                level: Level::Error,
                location: Some(Location {
                    label: Label {
                        text: main_message_text,
                        span: *main_message_location,
                    },
                    path: path.clone(),
                    src: src.clone(),
                    extra_labels,
                }),
            }
        }

        TypeError::IncorrectTypeArity {
            location,
            expected,
            given: given_number,
            name,
        } => {
            let expected = match expected {
                0 => "no type arguments".into(),
                1 => "1 type argument".into(),
                _ => format!("{expected} type arguments"),
            };
            let given = match given_number {
                0 => "none",
                _ => &format!("{given_number}"),
            };
            let text = wrap_format!(
                "`{name}` requires {expected} \
but {given} where provided."
            );
            Diagnostic {
                title: "Incorrect arity".into(),
                text,
                hint: None,
                level: Level::Error,
                location: Some(Location {
                    label: Label {
                        text: Some(format!("Expected {expected}, got {given_number}")),
                        span: *location,
                    },
                    path: path.clone(),
                    src: src.clone(),
                    extra_labels: vec![],
                }),
            }
        }

        TypeError::TypeUsedAsAConstructor { location, name } => {
            let text = wrap_format!(
                "`{name}` is a type with no parameters, but here it's \
being used as a type constructor."
            );

            Diagnostic {
                title: "Type used as a type constructor".into(),
                text,
                hint: None,
                level: Level::Error,
                location: Some(Location {
                    label: Label {
                        text: Some("You can remove this".into()),
                        span: *location,
                    },
                    path: path.clone(),
                    src: src.clone(),
                    extra_labels: vec![],
                }),
            }
        }

        TypeError::IncorrectArity {
            labels,
            location,
            context,
            expected,
            given,
        } => {
            let text = if labels.is_empty() {
                "".into()
            } else {
                let subject = match context {
                    IncorrectArityContext::Pattern => "pattern",
                    IncorrectArityContext::Function => "call",
                };
                let labels = labels
                    .iter()
                    .map(|p| format!("  - {p}"))
                    .sorted()
                    .join("\n");
                format!(
                    "This {subject} accepts these additional labelled \
                        arguments:\n\n{labels}",
                )
            };
            let expected = match expected {
                0 => "no arguments".into(),
                1 => "1 argument".into(),
                _ => format!("{expected} arguments"),
            };
            let label = format!("Expected {expected}, got {given}");
            Diagnostic {
                title: "Incorrect arity".into(),
                text,
                hint: None,
                level: Level::Error,
                location: Some(Location {
                    label: Label {
                        text: Some(label),
                        span: *location,
                    },
                    path: path.clone(),
                    src: src.clone(),
                    extra_labels: vec![],
                }),
            }
        }

        TypeError::UnnecessarySpreadOperator { location, arity } => {
            let text = wrap_format!(
                "This record has {arity} fields and you have already \
assigned variables to all of them."
            );
            Diagnostic {
                title: "Unnecessary spread operator".into(),
                text,
                hint: None,
                level: Level::Error,
                location: Some(Location {
                    label: Label {
                        text: None,
                        span: *location,
                    },
                    path: path.clone(),
                    src: src.clone(),
                    extra_labels: vec![],
                }),
            }
        }

        TypeError::UnsafeRecordUpdate { location, reason } => match reason {
            UnsafeRecordUpdateReason::UnknownVariant {
                constructed_variant,
            } => {
                let text = wrap_format!(
                    "This value cannot be used to build an updated \
`{constructed_variant}` as it could be some other variant.

Consider pattern matching on it with a case expression and then \
constructing a new record with its values."
                );

                Diagnostic {
                    title: "Unsafe record update".into(),
                    text,
                    hint: None,
                    level: Level::Error,
                    location: Some(Location {
                        label: Label {
                            text: Some(format!(
                                "I'm not sure this is always a `{constructed_variant}`"
                            )),
                            span: *location,
                        },
                        path: path.clone(),
                        src: src.clone(),
                        extra_labels: vec![],
                    }),
                }
            }
            UnsafeRecordUpdateReason::WrongVariant {
                constructed_variant,
                spread_variant,
            } => {
                let text = wrap_format!(
                    "This value is a `{spread_variant}` so \
it cannot be used to build a `{constructed_variant}`, even if they share some fields.

Note: If you want to change one variant of a type into another, you should \
specify all fields explicitly instead of using the record update syntax."
                );

                Diagnostic {
                    title: "Incorrect record update".into(),
                    text,
                    hint: None,
                    level: Level::Error,
                    location: Some(Location {
                        label: Label {
                            text: Some(format!("This is a `{spread_variant}`")),
                            span: *location,
                        },
                        path: path.clone(),
                        src: src.clone(),
                        extra_labels: vec![],
                    }),
                }
            }
            UnsafeRecordUpdateReason::IncompatibleFieldTypes {
                expected_field_type,
                record_field_type,
                record_variant,
                field,
                ..
            } => {
                let mut printer = Printer::new(names);
                let expected_field_type = printer.print_type(expected_field_type);
                let record_field_type = printer.print_type(record_field_type);
                let record_variant = printer.print_type(record_variant);
                let text = match field {
                    RecordField::Labelled(label) => wrap_format!(
                        "The `{label}` field \
of this value is a `{record_field_type}`, but the arguments given to the record \
update indicate that it should be a `{expected_field_type}`.

Note: If the same type variable is used for multiple fields, all those fields \
need to be updated at the same time if their type changes."
                    ),
                    RecordField::Unlabelled(index) => wrap_format!(
                        "The {} field \
of this value is a `{record_field_type}`, but the arguments given to the record \
update indicate that it should be a `{expected_field_type}`.

Note: Unlabelled fields cannot be updated in a record update, so either add \
a label or use a record constructor.",
                        to_ordinal(*index + 1),
                    ),
                };

                Diagnostic {
                    title: "Incomplete record update".into(),
                    text,
                    hint: None,
                    level: Level::Error,
                    location: Some(Location {
                        label: Label {
                            text: Some(format!("This is a `{record_variant}`")),
                            span: *location,
                        },
                        path: path.clone(),
                        src: src.clone(),
                        extra_labels: vec![],
                    }),
                }
            }
        },

        TypeError::QualifiedTypeMissingName { location } => Diagnostic {
            title: "Invalid type".into(),
            text: "".into(),
            hint: None,
            level: Level::Error,
            location: Some(Location {
                label: Label {
                    text: Some("This is not a valid type".into()),
                    span: *location,
                },
                path: path.clone(),
                src: src.clone(),
                extra_labels: vec![],
            }),
        },

        TypeError::UnknownType {
            location,
            name,
            hint,
            ..
        } => {
            let label_text = match hint {
                UnknownTypeHint::AlternativeTypes(types) => did_you_mean(name, types),
                UnknownTypeHint::ValueInScopeWithSameName => None,
            };

            let mut text =
                wrap_format!("The type `{name}` is not defined or imported in this module.");

            match hint {
                UnknownTypeHint::ValueInScopeWithSameName => {
                    let hint = wrap_format!(
                        "There is a value in scope with the name `{name}`, \
but no type in scope with that name."
                    );
                    text.push('\n');
                    text.push_str(hint.as_str());
                }
                UnknownTypeHint::AlternativeTypes(_) => {}
            }

            Diagnostic {
                title: "Unknown type".into(),
                text,
                hint: None,
                level: Level::Error,
                location: Some(Location {
                    label: Label {
                        text: label_text,
                        span: *location,
                    },
                    path: path.clone(),
                    src: src.clone(),
                    extra_labels: vec![],
                }),
            }
        }

        TypeError::UnknownVariable {
            location,
            variables,
            discarded_location,
            name,
            type_with_name_in_scope,
            possible_modules,
        } => {
            let title = String::from("Unknown variable");

            if let Some(ignored_location) = discarded_location {
                let location = Location {
                    label: Label {
                        text: Some("So this is not in scope".into()),
                        span: *location,
                    },
                    path: path.clone(),
                    src: src.clone(),
                    extra_labels: vec![ExtraLabel {
                        src_info: None,
                        label: Label {
                            text: Some("This value is discarded".into()),
                            span: *ignored_location,
                        },
                    }],
                };
                Diagnostic {
                    title,
                    text: "".into(),
                    hint: Some(wrap_format!(
                        "Change `_{name}` to `{name}` or reference another variable",
                    )),
                    level: Level::Error,
                    location: Some(location),
                }
            } else {
                let text = if *type_with_name_in_scope {
                    wrap_format!("`{name}` is a type, it cannot be used as a value.")
                } else {
                    let mut text = if name.starts_with(char::is_uppercase) {
                        wrap_format!(
                            "The custom type variant constructor `{name}` is not in scope here."
                        )
                    } else {
                        wrap_format!("The name `{name}` is not in scope here.")
                    };

                    // If there are some suggestions about public values in imported
                    // modules put a "did you mean" text after the main message
                    if !possible_modules.is_empty() {
                        let message = if possible_modules.len() == 1 {
                            "\nDid you mean:\n\n"
                        } else {
                            "\nDid you mean one of these:\n\n"
                        };
                        text.push_str(message);
                        for module_name in possible_modules {
                            text.push_str("  - ");
                            text.push_str(module_name);
                            text.push('.');
                            text.push_str(name);
                            text.push('\n');
                        }
                    }

                    text
                };

                Diagnostic {
                    title,
                    text,
                    hint: None,
                    level: Level::Error,
                    location: Some(Location {
                        label: Label {
                            text: did_you_mean(name, variables),
                            span: *location,
                        },
                        path: path.clone(),
                        src: src.clone(),
                        extra_labels: vec![],
                    }),
                }
            }
        }

        TypeError::PrivateTypeLeak { location, leaked } => {
            let mut printer = Printer::new(names);

            // TODO: be more precise.
            // - is being returned by this public function
            // - is taken as an argument by this public function
            // - is taken as an argument by this public enum constructor
            // etc
            let text = wrap_format!(
                "The following type is private, but is \
being used by this public export.

    {}

Private types can only be used within the module that defines them.",
                printer.print_type(leaked),
            );
            Diagnostic {
                title: "Private type used in public interface".into(),
                text,
                hint: None,
                level: Level::Error,
                location: Some(Location {
                    label: Label {
                        text: None,
                        span: *location,
                    },
                    path: path.clone(),
                    src: src.clone(),
                    extra_labels: vec![],
                }),
            }
        }

        TypeError::UnknownModule {
            location,
            name,
            suggestions,
        } => Diagnostic {
            title: "Unknown module".into(),
            text: format!("No module has been found with the name `{name}`."),
            hint: suggestions
                .first()
                .map(|suggestion| suggestion.suggestion(name)),
            level: Level::Error,
            location: Some(Location {
                label: Label {
                    text: None,
                    span: *location,
                },
                path: path.clone(),
                src: src.clone(),
                extra_labels: vec![],
            }),
        },

        TypeError::UnknownModuleType {
            location,
            name,
            module_name,
            type_constructors,
            value_with_same_name: imported_type_as_value,
        } => {
            let text = if *imported_type_as_value {
                format!("`{name}` is only a value, it cannot be imported as a type.")
            } else {
                format!("The module `{module_name}` does not have a `{name}` type.")
            };
            Diagnostic {
                title: "Unknown module type".into(),
                text,
                hint: None,
                level: Level::Error,
                location: Some(Location {
                    label: Label {
                        text: if *imported_type_as_value {
                            Some(format!("Did you mean `{name}`?"))
                        } else {
                            did_you_mean(name, type_constructors)
                        },
                        span: *location,
                    },
                    path: path.clone(),
                    src: src.clone(),
                    extra_labels: vec![],
                }),
            }
        }

        TypeError::UnknownModuleValue {
            location,
            name,
            module_name,
            value_constructors,
            type_with_same_name: imported_value_as_type,
            context,
        } => {
            let text = if *imported_value_as_type {
                match context {
                    ModuleValueUsageContext::UnqualifiedImport => {
                        wrap_format!("`{name}` is only a type, it cannot be imported as a value.")
                    }
                    ModuleValueUsageContext::ModuleAccess => wrap_format!(
                        "{module_name}.{name} is a type constructor, \
it cannot be used as a value"
                    ),
                }
            } else {
                wrap_format!("The module `{module_name}` does not have a `{name}` value.")
            };
            Diagnostic {
                title: "Unknown module value".into(),
                text,
                hint: None,
                level: Level::Error,
                location: Some(Location {
                    label: Label {
                        text: if *imported_value_as_type
                            && matches!(context, ModuleValueUsageContext::UnqualifiedImport)
                        {
                            Some(format!("Did you mean `type {name}`?"))
                        } else {
                            did_you_mean(name, value_constructors)
                        },
                        span: *location,
                    },
                    path: path.clone(),
                    src: src.clone(),
                    extra_labels: vec![],
                }),
            }
        }

        TypeError::ModuleAliasUsedAsName { location, name } => {
            let text = wrap(
                "Modules are not values, so you cannot assign them \
to variables, pass them to functions, or anything else that you would do with a value.",
            );
            Diagnostic {
                title: format!("Module `{name}` used as a value"),
                text,
                hint: None,
                level: Level::Error,
                location: Some(Location {
                    label: Label {
                        text: None,
                        span: *location,
                    },
                    path: path.clone(),
                    src: src.clone(),
                    extra_labels: vec![],
                }),
            }
        }

        TypeError::IncorrectNumClausePatterns {
            location,
            expected,
            given,
        } => {
            let subject = if *expected == 1 {
                "subject"
            } else {
                "subjects"
            };
            let pattern = if *expected == 1 {
                "pattern"
            } else {
                "patterns"
            };
            let text = wrap_format!(
                "This case expression has {expected} {subject}, \
but this pattern matches {given}.
Each clause must have a pattern for every subject value.",
            );
            let hint = if *expected == 1 {
                Some("Did you mean to match on alternative patterns using `|`?".into())
            } else {
                None
            };
            Diagnostic {
                title: "Incorrect number of patterns".into(),
                text,
                hint,
                level: Level::Error,
                location: Some(Location {
                    label: Label {
                        text: Some(format!("Expected {expected} {pattern}, got {given}")),
                        span: *location,
                    },
                    path: path.clone(),
                    src: src.clone(),
                    extra_labels: vec![],
                }),
            }
        }

        TypeError::NonLocalClauseGuardVariable { location, name } => {
            let text = wrap_format!(
                "Variables used in guards must be either defined in the \
function, or be an argument to the function. The variable \
`{name}` is not defined locally.",
            );
            Diagnostic {
                title: "Invalid guard variable".into(),
                text,
                hint: None,
                level: Level::Error,
                location: Some(Location {
                    label: Label {
                        text: Some("Is not locally defined".into()),
                        span: *location,
                    },
                    path: path.clone(),
                    src: src.clone(),
                    extra_labels: vec![],
                }),
            }
        }

        TypeError::ExtraVarInAlternativePattern { location, name } => {
            let text = wrap_format!(
                "All alternative patterns must define the same variables as \
the initial pattern. This variable `{name}` has not been previously defined.",
            );
            Diagnostic {
                title: "Extra alternative pattern variable".into(),
                text,
                hint: None,
                level: Level::Error,
                location: Some(Location {
                    label: Label {
                        text: Some("Has not been previously defined".into()),
                        span: *location,
                    },
                    path: path.clone(),
                    src: src.clone(),
                    extra_labels: vec![],
                }),
            }
        }

        TypeError::MissingVarInAlternativePattern { location, name } => {
            let text = wrap_format!(
                "All alternative patterns must define the same variables \
as the initial pattern, but the `{name}` variable is missing.",
            );
            Diagnostic {
                title: "Missing alternative pattern variable".into(),
                text,
                hint: None,
                level: Level::Error,
                location: Some(Location {
                    label: Label {
                        text: Some("This does not define all required variables".into()),
                        span: *location,
                    },
                    path: path.clone(),
                    src: src.clone(),
                    extra_labels: vec![],
                }),
            }
        }

        TypeError::DuplicateVarInPattern { location, name } => {
            let text = wrap_format!(
                "Variables can only be used once per pattern. This \
variable `{name}` appears multiple times.
If you used the same variable twice deliberately in order to check for equality \
please use a guard clause instead.
e.g. (x, y) if x == y -> ...",
            );
            Diagnostic {
                title: "Duplicate variable in pattern".into(),
                text,
                hint: None,
                level: Level::Error,
                location: Some(Location {
                    label: Label {
                        text: Some("This has already been used".into()),
                        span: *location,
                    },
                    path: path.clone(),
                    src: src.clone(),
                    extra_labels: vec![],
                }),
            }
        }

        TypeError::OutOfBoundsTupleIndex {
            location, size: 0, ..
        } => Diagnostic {
            title: "Out of bounds tuple index".into(),
            text: "This tuple has no elements so it cannot be indexed at all.".into(),
            hint: None,
            level: Level::Error,
            location: Some(Location {
                label: Label {
                    text: None,
                    span: *location,
                },
                path: path.clone(),
                src: src.clone(),
                extra_labels: vec![],
            }),
        },

        TypeError::OutOfBoundsTupleIndex {
            location,
            index,
            size,
        } => {
            let text = wrap_format!(
                "The index being accessed for this tuple is {}, but this \
tuple has {} elements so the highest valid index is {}.",
                index,
                size,
                size - 1,
            );
            Diagnostic {
                title: "Out of bounds tuple index".into(),
                text,
                hint: None,
                level: Level::Error,
                location: Some(Location {
                    label: Label {
                        text: Some("This index is too large".into()),
                        span: *location,
                    },
                    path: path.clone(),
                    src: src.clone(),
                    extra_labels: vec![],
                }),
            }
        }

        TypeError::NotATuple { location, given } => {
            let mut printer = Printer::new(names);
            let text = format!(
                "To index into this value it needs to be a tuple, \
however it has this type:

    {}",
                printer.print_type(given),
            );
            Diagnostic {
                title: "Type mismatch".into(),
                text,
                hint: None,
                level: Level::Error,
                location: Some(Location {
                    label: Label {
                        text: Some("This is not a tuple".into()),
                        span: *location,
                    },
                    path: path.clone(),
                    src: src.clone(),
                    extra_labels: vec![],
                }),
            }
        }

        TypeError::NotATupleUnbound { location } => {
            let text = wrap(
                "To index into a tuple we need to \
know its size, but we don't know anything about this type yet. \
Please add some type annotations so we can continue.",
            );
            Diagnostic {
                title: "Type mismatch".into(),
                text,
                hint: None,
                level: Level::Error,
                location: Some(Location {
                    label: Label {
                        text: Some("What type is this?".into()),
                        span: *location,
                    },
                    path: path.clone(),
                    src: src.clone(),
                    extra_labels: vec![],
                }),
            }
        }

        TypeError::RecordAccessUnknownType { location } => {
            let text = wrap(
                "In order to access a record field \
we need to know what type it is, but I can't tell \
the type here. Try adding type annotations to your \
function and try again.",
            );
            Diagnostic {
                title: "Unknown type for record access".into(),
                text,
                hint: None,
                level: Level::Error,
                location: Some(Location {
                    label: Label {
                        text: Some("I don't know what type this is".into()),
                        span: *location,
                    },
                    path: path.clone(),
                    src: src.clone(),
                    extra_labels: vec![],
                }),
            }
        }

        TypeError::BitArraySegmentError { error, location } => {
            let (label, mut extra) = match error {
                bit_array::ErrorType::ConflictingTypeOptions { existing_type } => (
                    "This is an extra type specifier",
                    vec![format!(
                        "Hint: This segment already has the type {existing_type}."
                    )],
                ),

                bit_array::ErrorType::ConflictingSignednessOptions { existing_signed } => (
                    "This is an extra signedness specifier",
                    vec![format!(
                        "Hint: This segment already has a \
signedness of {existing_signed}."
                    )],
                ),

                bit_array::ErrorType::ConflictingEndiannessOptions {
                    existing_endianness,
                } => (
                    "This is an extra endianness specifier",
                    vec![format!(
                        "Hint: This segment already has an \
endianness of {existing_endianness}."
                    )],
                ),

                bit_array::ErrorType::ConflictingSizeOptions => (
                    "This is an extra size specifier",
                    vec!["Hint: This segment already has a size.".into()],
                ),

                bit_array::ErrorType::ConflictingUnitOptions => (
                    "This is an extra unit specifier",
                    vec!["Hint: A BitArray segment can have at most 1 unit.".into()],
                ),

                bit_array::ErrorType::FloatWithSize => (
                    "Invalid float size",
                    vec!["Hint: floats have an exact size of 16/32/64 bits.".into()],
                ),

                bit_array::ErrorType::InvalidEndianness => (
                    "This option is invalid here",
                    vec![wrap(
                        "Hint: signed and unsigned \
can only be used with int, float, utf16 and utf32 types.",
                    )],
                ),

                bit_array::ErrorType::OptionNotAllowedInValue => (
                    "This option is only allowed in BitArray patterns",
                    vec!["Hint: This option has no effect in BitArray values.".into()],
                ),

                bit_array::ErrorType::SignednessUsedOnNonInt { type_ } => (
                    "Signedness is only valid with int types",
                    vec![format!("Hint: This segment has a type of {type_}")],
                ),
                bit_array::ErrorType::TypeDoesNotAllowSize { type_ } => (
                    "Size cannot be specified here",
                    vec![format!("Hint: {type_} segments have an automatic size.")],
                ),
                bit_array::ErrorType::TypeDoesNotAllowUnit { type_ } => (
                    "Unit cannot be specified here",
                    vec![wrap(&format!(
                        "Hint: {type_} segments \
are sized based on their value and cannot have a unit."
                    ))],
                ),
                bit_array::ErrorType::VariableUtfSegmentInPattern => (
                    "This cannot be a variable",
                    vec![wrap(
                        "Hint: in patterns utf8, utf16, and \
utf32  must be an exact string.",
                    )],
                ),
                bit_array::ErrorType::SegmentMustHaveSize => (
                    "This segment has no size",
                    vec![wrap(
                        "Hint: Bit array segments without \
a size are only allowed at the end of a bin pattern.",
                    )],
                ),
                bit_array::ErrorType::UnitMustHaveSize => (
                    "This needs an explicit size",
                    vec!["Hint: If you specify unit() you must also specify size().".into()],
                ),
                bit_array::ErrorType::ConstantSizeNotPositive => {
                    ("A constant size must be a positive number", vec![])
                }
            };
            let text = extra.join("\n");
            Diagnostic {
                title: "Invalid bit array segment".into(),
                text,
                hint: None,
                level: Level::Error,
                location: Some(Location {
                    label: Label {
                        text: Some(label.into()),
                        span: *location,
                    },
                    path: path.clone(),
                    src: src.clone(),
                    extra_labels: vec![],
                }),
            }
        }
        TypeError::RecordUpdateInvalidConstructor { location } => Diagnostic {
            title: "Invalid record constructor".into(),
            text: "Only record constructors can be used with the update syntax.".into(),
            hint: None,
            level: Level::Error,
            location: Some(Location {
                label: Label {
                    text: Some("This is not a record constructor".into()),
                    span: *location,
                },
                path: path.clone(),
                src: src.clone(),
                extra_labels: vec![],
            }),
        },

        TypeError::RecordUpdateVariantWithNoFields { location } => Diagnostic {
            title: "Invalid record constructor".into(),
            text: wrap(
                "Only constructors with at least one labelled \
field can be used with the update syntax.",
            ),
            hint: None,
            level: Level::Error,
            location: Some(Location {
                label: Label {
                    text: Some("This constructor has no labelled fields".into()),
                    span: *location,
                },
                path: path.clone(),
                src: src.clone(),
                extra_labels: vec![],
            }),
        },

        TypeError::UnexpectedTypeHole { location } => Diagnostic {
            title: "Unexpected type hole".into(),
            text: "We need to know the exact type here so type holes cannot be used.".into(),
            hint: None,
            level: Level::Error,
            location: Some(Location {
                label: Label {
                    text: Some("I need to know what this is".into()),
                    span: *location,
                },
                path: path.clone(),
                src: src.clone(),
                extra_labels: vec![],
            }),
        },

        TypeError::ReservedModuleName { name } => {
            let text = format!(
                "The module name `{name}` is reserved.
Try a different name for this module."
            );
            Diagnostic {
                title: "Reserved module name".into(),
                text,
                hint: None,
                location: None,
                level: Level::Error,
            }
        }

        TypeError::KeywordInModuleName { name, keyword } => {
            let text = wrap_format!(
                "The module name `{name}` contains the keyword `{keyword}`, \
so importing it would be a syntax error.
Try a different name for this module."
            );
            Diagnostic {
                title: "Invalid module name".into(),
                text,
                hint: None,
                location: None,
                level: Level::Error,
            }
        }

        TypeError::NotExhaustivePatternMatch {
            location,
            unmatched,
            kind,
        } => {
            let mut text = match kind {
                PatternMatchKind::Case => {
                    "This case expression does not match all possibilities.
Each constructor must have a pattern that matches it or
else it could crash."
                }
                PatternMatchKind::Assignment => {
                    "This assignment does not match all possibilities.
Either use a case expression with patterns for each possible
value, or use `let assert` rather than `let`."
                }
            }
            .to_string();

            text.push_str("\n\nThese values are not matched:\n\n");
            for unmatched in unmatched {
                text.push_str("  - ");
                text.push_str(unmatched);
                text.push('\n');
            }
            Diagnostic {
                title: "Not exhaustive pattern match".into(),
                text,
                hint: None,
                level: Level::Error,
                location: Some(Location {
                    label: Label {
                        text: None,
                        span: *location,
                    },
                    path: path.clone(),
                    src: src.clone(),
                    extra_labels: vec![],
                }),
            }
        }

        TypeError::ArgumentNameAlreadyUsed { location, name } => Diagnostic {
            title: "Argument name already used".into(),
            text: format!("Two `{name}` arguments have been defined for this function."),
            hint: None,
            level: Level::Error,
            location: Some(Location {
                label: Label {
                    text: None,
                    span: *location,
                },
                path: path.clone(),
                src: src.clone(),
                extra_labels: vec![],
            }),
        },

        TypeError::UnlabelledAfterlabelled { location } => Diagnostic {
            title: "Unlabelled argument after labelled argument".into(),
            text: wrap("All unlabelled arguments must come before any labelled arguments."),
            hint: None,
            level: Level::Error,
            location: Some(Location {
                label: Label {
                    text: None,
                    span: *location,
                },
                path: path.clone(),
                src: src.clone(),
                extra_labels: vec![],
            }),
        },

        TypeError::RecursiveTypeAlias { location, cycle } => {
            let mut text = "This type alias is defined in terms of itself.\n".into();
            write_cycle(&mut text, cycle);
            text.push_str(
                "If we tried to compile this recursive type it would expand
forever in a loop, and we'd never get the final type.",
            );
            Diagnostic {
                title: "Type cycle".into(),
                text,
                hint: None,
                level: Level::Error,
                location: Some(Location {
                    label: Label {
                        text: None,
                        span: *location,
                    },
                    path: path.clone(),
                    src: src.clone(),
                    extra_labels: vec![],
                }),
            }
        }

        TypeError::ExternalMissingAnnotation { location, kind } => {
            let kind = match kind {
                MissingAnnotation::Parameter => "parameter",
                MissingAnnotation::Return => "return",
            };
            let text = format!(
                "A {kind} annotation is missing from this function.

Functions with external implementations must have type annotations
so we can tell what type of values they accept and return.",
            );
            Diagnostic {
                title: "Missing type annotation".into(),
                text,
                hint: None,
                level: Level::Error,
                location: Some(Location {
                    label: Label {
                        text: None,
                        span: *location,
                    },
                    path: path.clone(),
                    src: src.clone(),
                    extra_labels: vec![],
                }),
            }
        }

        TypeError::NoImplementation { location } => {
            let text = "We can't compile this function as it doesn't have an
implementation. Add a body or an external implementation
using the `@external` attribute."
                .into();
            Diagnostic {
                title: "Function without an implementation".into(),
                text,
                hint: None,
                level: Level::Error,
                location: Some(Location {
                    label: Label {
                        text: None,
                        span: *location,
                    },
                    path: path.clone(),
                    src: src.clone(),
                    extra_labels: vec![],
                }),
            }
        }

        TypeError::InexhaustiveLetAssignment { location, missing } => {
            let mut text = wrap(
                "This assignment uses a pattern that does not \
match all possible values. If one of the other values \
is used then the assignment will crash.

The missing patterns are:\n",
            );
            for missing in missing {
                text.push_str("\n    ");
                text.push_str(missing);
            }

            Diagnostic {
                title: "Inexhaustive pattern".into(),
                text,
                hint: Some("Use a more general pattern or use `let assert` instead.".into()),
                level: Level::Error,
                location: Some(Location {
                    src: src.clone(),
                    path: path.to_path_buf(),
                    label: Label {
                        text: None,
                        span: *location,
                    },
                    extra_labels: Vec::new(),
                }),
            }
        }

        TypeError::InexhaustiveCaseExpression { location, missing } => {
            let mut text = wrap(
                "This case expression does not have a pattern \
for all possible values. If it is run on one of the \
values without a pattern then it will crash.

The missing patterns are:\n",
            );
            for missing in missing {
                text.push_str("\n    ");
                text.push_str(missing);
            }
            Diagnostic {
                title: "Inexhaustive patterns".into(),
                text,
                hint: None,
                level: Level::Error,
                location: Some(Location {
                    src: src.clone(),
                    path: path.to_path_buf(),
                    label: Label {
                        text: None,
                        span: *location,
                    },
                    extra_labels: Vec::new(),
                }),
            }
        }

        TypeError::MissingCaseBody { location } => {
            let text = wrap("This case expression is missing its body.");
            Diagnostic {
                title: "Missing case body".into(),
                text,
                hint: None,
                level: Level::Error,
                location: Some(Location {
                    src: src.clone(),
                    path: path.to_path_buf(),
                    label: Label {
                        text: None,
                        span: *location,
                    },
                    extra_labels: Vec::new(),
                }),
            }
        }

        TypeError::UnsupportedExpressionTarget {
            location,
            target: current_target,
        } => {
            let text = wrap_format!(
                "This value is not available as it is defined using externals, \
and there is no implementation for the {} target.",
                match current_target {
                    Target::Wasm => "Wasm",
                }
            );
            let hint = wrap("Provide an i implementation or a Wasm external.");
            Diagnostic {
                title: "Unsupported target".into(),
                text,
                hint: Some(hint),
                level: Level::Error,
                location: Some(Location {
                    path: path.clone(),
                    src: src.clone(),
                    label: Label {
                        text: None,
                        span: *location,
                    },
                    extra_labels: vec![],
                }),
            }
        }

        TypeError::UnsupportedPublicFunctionTarget {
            location,
            name,
            target,
        } => {
            let target = match target {
                Target::Wasm => "Wasm",
            };
            let text = wrap_format!(
                "The `{name}` function is public but doesn't have an \
implementation for the {target} target. All public functions of a package \
must be able to compile for a module to be valid."
            );
            Diagnostic {
                title: "Unsupported target".into(),
                text,
                hint: None,
                level: Level::Error,
                location: Some(Location {
                    path: path.clone(),
                    src: src.clone(),
                    label: Label {
                        text: None,
                        span: *location,
                    },
                    extra_labels: vec![],
                }),
            }
        }

        TypeError::UnusedTypeAliasParameter { location, name } => {
            let text =
                wrap_format!("The type variable `{name}` is unused. It can be safely removed.",);
            Diagnostic {
                title: "Unused type parameter".into(),
                text,
                hint: None,
                level: Level::Error,
                location: Some(Location {
                    path: path.clone(),
                    src: src.clone(),
                    label: Label {
                        text: None,
                        span: *location,
                    },
                    extra_labels: vec![],
                }),
            }
        }

        TypeError::DuplicateTypeParameter { location, name } => {
            let text = wrap_format!(
                "This definition has multiple type parameters named `{name}`.
Rename or remove one of them.",
            );
            Diagnostic {
                title: "Duplicate type parameter".into(),
                text,
                hint: None,
                level: Level::Error,
                location: Some(Location {
                    path: path.clone(),
                    src: src.clone(),
                    label: Label {
                        text: None,
                        span: *location,
                    },
                    extra_labels: vec![],
                }),
            }
        }

        TypeError::NotFnInUse { location, type_ } => {
            let mut printer = Printer::new(names);
            let text = wrap_format!(
                "In a use expression, there should be a function on \
the right hand side of `<-`, but this value has type:

    {}",
                printer.print_type(type_)
            );

            Diagnostic {
                title: "Type mismatch".into(),
                text,
                hint: None,
                level: Level::Error,
                location: Some(Location {
                    label: Label {
                        text: None,
                        span: *location,
                    },
                    path: path.clone(),
                    src: src.clone(),
                    extra_labels: vec![],
                }),
            }
        }

        TypeError::UseFnDoesntTakeCallback {
            location,
            actual_type: None,
        }
        | TypeError::UseFnIncorrectArity {
            location,
            expected: 0,
            given: 1,
        } => {
            let text = wrap(
                "The function on the right of `<-` here \
takes no arguments, but it has to take at least \
one argument, a callback function.",
            );
            Diagnostic {
                title: "Incorrect arity".into(),
                text,
                hint: None,
                level: Level::Error,
                location: Some(Location {
                    label: Label {
                        text: Some("Expected no arguments, got 1".into()),
                        span: *location,
                    },
                    path: path.clone(),
                    src: src.clone(),
                    extra_labels: vec![],
                }),
            }
        }

        TypeError::UseFnIncorrectArity {
            location,
            expected,
            given,
        } => {
            let expected_string = match expected {
                0 => "no arguments".into(),
                1 => "1 argument".into(),
                _ => format!("{expected} arguments"),
            };
            let supplied_arguments = given - 1;
            let supplied_arguments_string = match supplied_arguments {
                0 => "no arguments".into(),
                1 => "1 argument".into(),
                _ => format!("{given} arguments"),
            };
            let label = format!("Expected {expected_string}, got {given}");
            let mut text: String = format!(
                "The function on the right of `<-` \
here takes {expected_string}.\n"
            );

            if expected > given {
                if supplied_arguments == 0 {
                    text.push_str(
                        "The only argument that was supplied is \
the `use` callback function.\n",
                    );
                } else {
                    text.push_str("You supplied ");
                    text.push_str(&supplied_arguments_string);
                    text.push_str(" and the final one is the `use` callback function.\n");
                }
            } else {
                text.push_str(
                    "All the arguments have already been supplied, \
so it cannot take the `use` callback function as a final argument.\n",
                );
            }

            Diagnostic {
                title: "Incorrect arity".into(),
                text: wrap(text.trim_end()),
                hint: None,
                level: Level::Error,
                location: Some(Location {
                    label: Label {
                        text: Some(label),
                        span: *location,
                    },
                    path: path.clone(),
                    src: src.clone(),
                    extra_labels: vec![],
                }),
            }
        }

        TypeError::UseFnDoesntTakeCallback {
            location,
            actual_type: Some(actual),
        } => {
            let mut printer = Printer::new(names);
            let text = wrap_format!(
                "The function on the right hand side of `<-` \
has to take a callback function as its last argument. \
But the last argument of this function has type:

    {}",
                printer.print_type(actual)
            );
            Diagnostic {
                title: "Type mismatch".into(),
                text: wrap(&text),
                hint: None,
                level: Level::Error,
                location: Some(Location {
                    label: Label {
                        text: None,
                        span: *location,
                    },
                    path: path.clone(),
                    src: src.clone(),
                    extra_labels: vec![],
                }),
            }
        }

        TypeError::UseCallbackIncorrectArity {
            pattern_location,
            call_location,
            expected,
            given,
        } => {
            let expected = match expected {
                0 => "no arguments".into(),
                1 => "1 argument".into(),
                _ => format!("{expected} arguments"),
            };

            let specified = match given {
                0 => "none were provided".into(),
                1 => "1 was provided".into(),
                _ => format!("{given} were provided"),
            };

            let text = wrap_format!(
                "This function takes a callback that expects {expected}. \
But {specified} on the left hand side of `<-`."
            );
            Diagnostic {
                title: "Incorrect arity".into(),
                text,
                hint: None,
                level: Level::Error,
                location: Some(Location {
                    label: Label {
                        text: None,
                        span: *call_location,
                    },
                    path: path.clone(),
                    src: src.clone(),
                    extra_labels: vec![ExtraLabel {
                        src_info: None,
                        label: Label {
                            text: Some(format!("Expected {expected}, got {given}")),
                            span: *pattern_location,
                        },
                    }],
                }),
            }
        }

        TypeError::BadName {
            location,
            name,
            kind,
        } => {
            let kind_str = kind.as_str();
            let label = format!("This is not a valid {} name", kind_str.to_lowercase());
            let hint = match kind {
                Named::Type | Named::TypeAlias | Named::CustomTypeVariant => {
                    format!(
                        "{} names start with an uppercase \
letter and contain only lowercase letters, numbers, \
and uppercase letters.
Try: {}",
                        kind_str,
                        to_upper_camel_case(name)
                    )
                }
                Named::Variable
                | Named::TypeVariable
                | Named::Argument
                | Named::Label
                | Named::Constant
                | Named::Function => format!(
                    "{} names start with a lowercase letter \
and contain a-z, 0-9, or _.
Try: {}",
                    kind_str,
                    to_snake_case(name)
                ),
                Named::Discard => format!(
                    "{} names start with _ and contain \
a-z, 0-9, or _.
Try: _{}",
                    kind_str,
                    to_snake_case(name)
                ),
            };

            Diagnostic {
                title: format!("Invalid {} name", kind_str.to_lowercase()),
                text: "".into(),
                hint: Some(hint),
                level: Level::Error,
                location: Some(Location {
                    label: Label {
                        text: Some(label),
                        span: *location,
                    },
                    path: path.clone(),
                    src: src.clone(),
                    extra_labels: vec![],
                }),
            }
        }

        TypeError::AllVariantsDeprecated { location } => {
            let text = String::from(
                "Consider deprecating the type as a whole.

  @deprecated(\"message\")
  type Wibble {
    Wobble1
    Wobble2
  }
",
            );
            Diagnostic {
                title: "All variants of custom type deprecated.".into(),
                text,
                hint: None,
                level: Level::Error,
                location: Some(Location {
                    label: Label {
                        text: None,
                        span: *location,
                    },
                    path: path.clone(),
                    src: src.clone(),
                    extra_labels: vec![],
                }),
            }
        }
        TypeError::DeprecatedVariantOnDeprecatedType { location } => {
            let text = wrap(
                "This custom type has already been deprecated, so deprecating \
one of its variants does nothing.
Consider removing the deprecation attribute on the variant.",
            );

            Diagnostic {
                title: "Custom type already deprecated".into(),
                text,
                hint: None,
                level: Level::Error,
                location: Some(Location {
                    label: Label {
                        text: None,
                        span: *location,
                    },
                    path: path.clone(),
                    src: src.clone(),
                    extra_labels: vec![],
                }),
            }
        }

        TypeError::EchoWithNoFollowingExpression { location } => Diagnostic {
            title: "Invalid echo use".to_string(),
            text: wrap("The `echo` keyword should be followed by a value to print."),
            hint: None,
            level: Level::Error,
            location: Some(Location {
                label: Label {
                    text: Some("I was expecting a value after this".into()),
                    span: *location,
                },
                path: path.clone(),
                src: src.clone(),
                extra_labels: vec![],
            }),
        },

        TypeError::StringConcatenationWithAddInt { location } => Diagnostic {
            title: "Type mismatch".to_string(),
            text: wrap(
                "The + operator can only be used on Ints.
To join two strings together you can use the <> operator.",
            ),
            hint: None,
            level: Level::Error,
            location: Some(Location {
                label: Label {
                    text: Some("Use <> instead".into()),
                    span: *location,
                },
                path: path.clone(),
                src: src.clone(),
                extra_labels: vec![],
            }),
        },

        TypeError::IntOperatorOnFloats { location, operator } => Diagnostic {
            title: "Type mismatch".to_string(),
            text: wrap_format!("The {} operator can only be used on Ints.", operator.name()),
            hint: None,
            level: Level::Error,
            location: Some(Location {
                label: Label {
                    text: operator
                        .float_equivalent()
                        .map(|operator| format!("Use {} instead", operator.name())),
                    span: *location,
                },
                path: path.clone(),
                src: src.clone(),
                extra_labels: vec![],
            }),
        },

        TypeError::FloatOperatorOnInts { location, operator } => Diagnostic {
            title: "Type mismatch".to_string(),
            text: wrap_format!(
                "The {} operator can only be used on Floats.",
                operator.name()
            ),
            hint: None,
            level: Level::Error,
            location: Some(Location {
                label: Label {
                    text: operator
                        .int_equivalent()
                        .map(|operator| format!("Use {} instead", operator.name())),
                    span: *location,
                },
                path: path.clone(),
                src: src.clone(),
                extra_labels: vec![],
            }),
        },

        TypeError::DoubleVariableAssignmentInBitArray { location } => Diagnostic {
            title: "Double variable assignment".to_string(),
            text: wrap(
                "This pattern assigns to two different variables \
at once, which is not possible in bit arrays.",
            ),
            hint: Some(wrap("Remove the `as` assignment.")),
            level: Level::Error,
            location: Some(Location {
                label: Label {
                    text: None,
                    span: *location,
                },
                path: path.clone(),
                src: src.clone(),
                extra_labels: vec![],
            }),
        },

        TypeError::NonUtf8StringAssignmentInBitArray { location } => Diagnostic {
            title: "Non UTF-8 string assignment".to_string(),
            text: wrap(
                "This pattern assigns a non UTF-8 string to a \
variable in a bit array. This is planned to be supported in the future, but we are \
unsure of the desired behaviour. Please go to https://github.com/gleam-lang/gleam/issues/4566 \
and explain your usecase for this pattern, and how you would expect it to behave.",
            ),
            hint: None,
            level: Level::Error,
            location: Some(Location {
                label: Label {
                    text: None,
                    span: *location,
                },
                path: path.clone(),
                src: src.clone(),
                extra_labels: vec![],
            }),
        },

        TypeError::PrivateOpaqueType { location } => Diagnostic {
            title: "Private opaque type".to_string(),
            text: wrap("Only a public type can be opaque."),
            hint: None,
            level: Level::Error,
            location: Some(Location {
                label: Label {
                    text: Some("You can safely remove this.".to_string()),
                    span: *location,
                },
                path: path.clone(),
                src: src.clone(),
                extra_labels: vec![],
            }),
        },

        TypeError::SrcImportingDevDependency {
            location,
            importing_module,
            imported_module,
            package,
        } => Diagnostic {
            title: "App importing dev dependency".to_string(),
            text: wrap_format!(
                "The application module `{importing_module}` is \
importing the module `{imported_module}`, but `{package}`, the package it \
belongs to, is a dev dependency.

Dev dependencies are not included in production builds so application \
modules should not import them. Perhaps change `{package}` to a regular dependency."
            ),
            hint: None,
            level: Level::Error,
            location: Some(Location {
                label: Label {
                    text: None,
                    span: *location,
                },
                path: path.clone(),
                src: src.clone(),
                extra_labels: vec![],
            }),
        },

        TypeError::ExternalTypeWithConstructors { location } => Diagnostic {
            title: "External type with constructors".to_string(),
            text: wrap_format!(
                "This type is annotated with the `@external` annotation, \
but it has constructors. The `@external` annotation is only for external types \
with no constructors."
            ),
            hint: Some("Remove the `@external` annotation".into()),
            level: Level::Error,
            location: Some(Location {
                label: Label {
                    text: None,
                    span: *location,
                },
                path: path.clone(),
                src: src.clone(),
                extra_labels: vec![],
            }),
        },

        TypeError::LowercaseBoolPattern { location } => Diagnostic {
            title: "Lowercase bool pattern".to_string(),
            text: "".into(),
            hint: Some(
                "In i bool literals are `True` and `False`."
                    .into(),
            ),
            level: Level::Error,
            location: Some(Location {
                label: Label {
                    text: Some("This is not a bool".into()),
                    span: *location,
                },
                path: path.clone(),
                src: src.clone(),
                extra_labels: vec![],
            }),
        },

        TypeError::TodoConstant { location } => Diagnostic {
            title: "Constant todo found".to_string(),
            text: wrap(
                "This code will crash if it is run. \
Be sure to finish it before running your program.",
            ),
            hint: None,
            level: Level::Error,
            location: Some(Location {
                label: Label {
                    text: Some("This code is incomplete".into()),
                    span: *location,
                },
                path: path.clone(),
                src: src.clone(),
                extra_labels: vec![],
            }),
        },
        TypeError::InvalidConstantBinaryOperator {
            operator_start,
            operator,
        } => Diagnostic {
            title: "Unsupported operator in constant expression".into(),
            text: wrap("This operator is currently not supported in constants."),
            hint: None,
            level: Level::Error,
            location: Some(Location {
                label: Label {
                    text: None,
                    span: SrcSpan::new(*operator_start, operator_start + operator.size()),
                },
                path: path.clone(),
                src: src.clone(),
                extra_labels: vec![],
            }),
        },
        TypeError::PrivateValueUse {
            location,
            name,
            module_name,
        } => Diagnostic {
            title: "Use of private value".into(),
            text: wrap_format!(
                "`{module_name}` does define `{name}`, but it is private \
and cannot be accessed by other modules."
            ),
            hint: None,
            level: Level::Error,
            location: Some(Location {
                label: Label {
                    text: None,
                    span: *location,
                },
                path: path.clone(),
                src: src.clone(),
                extra_labels: vec![],
            }),
        },
        TypeError::PrivateTypeUse {
            location,
            name,
            module_name,
        } => Diagnostic {
            title: "Use of private type".into(),
            text: wrap_format!(
                "`{module_name}` does define `{name}`, but it is private \
and cannot be accessed by other modules."
            ),
            hint: None,
            level: Level::Error,
            location: Some(Location {
                label: Label {
                    text: None,
                    span: *location,
                },
                path: path.clone(),
                src: src.clone(),
                extra_labels: vec![],
            }),
        },
    })
}

fn std_io_error_kind_text(kind: &std::io::ErrorKind) -> String {
    use std::io::ErrorKind;
    match kind {
        ErrorKind::NotFound => "Could not find the stdio stream".into(),
        ErrorKind::PermissionDenied => "Permission was denied".into(),
        ErrorKind::ConnectionRefused => "Connection was refused".into(),
        ErrorKind::ConnectionReset => "Connection was reset".into(),
        ErrorKind::ConnectionAborted => "Connection was aborted".into(),
        ErrorKind::NotConnected => "Was not connected".into(),
        ErrorKind::AddrInUse => "The stream was already in use".into(),
        ErrorKind::AddrNotAvailable => "The stream was not available".into(),
        ErrorKind::BrokenPipe => "The pipe was broken".into(),
        ErrorKind::AlreadyExists => "A handle to the stream already exists".into(),
        ErrorKind::WouldBlock => "This operation would block when it was requested not to".into(),
        ErrorKind::InvalidInput => "Some parameter was invalid".into(),
        ErrorKind::InvalidData => "The data was invalid.  Check that the encoding is UTF-8".into(),
        ErrorKind::TimedOut => "The operation timed out".into(),
        ErrorKind::WriteZero => {
            "An attempt was made to write, but all bytes could not be written".into()
        }
        ErrorKind::Interrupted => "The operation was interrupted".into(),
        ErrorKind::UnexpectedEof => "The end of file was reached before it was expected".into(),
        ErrorKind::HostUnreachable
        | ErrorKind::NetworkUnreachable
        | ErrorKind::NetworkDown
        | ErrorKind::NotADirectory
        | ErrorKind::IsADirectory
        | ErrorKind::DirectoryNotEmpty
        | ErrorKind::ReadOnlyFilesystem
        | ErrorKind::StaleNetworkFileHandle
        | ErrorKind::StorageFull
        | ErrorKind::NotSeekable
        | ErrorKind::QuotaExceeded
        | ErrorKind::FileTooLarge
        | ErrorKind::ResourceBusy
        | ErrorKind::ExecutableFileBusy
        | ErrorKind::Deadlock
        | ErrorKind::CrossesDevices
        | ErrorKind::TooManyLinks
        | ErrorKind::InvalidFilename
        | ErrorKind::ArgumentListTooLong
        | ErrorKind::Unsupported
        | ErrorKind::OutOfMemory
        | ErrorKind::Other
        | _ => "An unknown error occurred".into(),
    }
}

fn write_cycle(buffer: &mut String, cycle: &[EcoString]) {
    buffer.push_str(
        "
    ┌─────┐\n",
    );
    for (index, name) in cycle.iter().enumerate() {
        if index != 0 {
            buffer.push_str("    │     ↓\n");
        }
        buffer.push_str("    │     ");
        buffer.push_str(name);
        buffer.push('\n');
    }
    buffer.push_str("    └─────┘\n");
}

fn hint_alternative_operator(op: &BinOp, given: &Type) -> Option<String> {
    match op {
        BinOp::AddInt if given.is_float() => Some(hint_numeric_message("+.", "Float")),
        BinOp::DivInt if given.is_float() => Some(hint_numeric_message("/.", "Float")),
        BinOp::GtEqInt if given.is_float() => Some(hint_numeric_message(">=.", "Float")),
        BinOp::GtInt if given.is_float() => Some(hint_numeric_message(">.", "Float")),
        BinOp::LtEqInt if given.is_float() => Some(hint_numeric_message("<=.", "Float")),
        BinOp::LtInt if given.is_float() => Some(hint_numeric_message("<.", "Float")),
        BinOp::MultInt if given.is_float() => Some(hint_numeric_message("*.", "Float")),
        BinOp::SubInt if given.is_float() => Some(hint_numeric_message("-.", "Float")),

        BinOp::AddFloat if given.is_int() => Some(hint_numeric_message("+", "Int")),
        BinOp::DivFloat if given.is_int() => Some(hint_numeric_message("/", "Int")),
        BinOp::GtEqFloat if given.is_int() => Some(hint_numeric_message(">=", "Int")),
        BinOp::GtFloat if given.is_int() => Some(hint_numeric_message(">", "Int")),
        BinOp::LtEqFloat if given.is_int() => Some(hint_numeric_message("<=", "Int")),
        BinOp::LtFloat if given.is_int() => Some(hint_numeric_message("<", "Int")),
        BinOp::MultFloat if given.is_int() => Some(hint_numeric_message("*", "Int")),
        BinOp::SubFloat if given.is_int() => Some(hint_numeric_message("-", "Int")),

        BinOp::AddInt if given.is_string() => Some(hint_string_message()),
        BinOp::AddFloat if given.is_string() => Some(hint_string_message()),

        BinOp::And
        | BinOp::Or
        | BinOp::Eq
        | BinOp::NotEq
        | BinOp::LtInt
        | BinOp::LtEqInt
        | BinOp::LtFloat
        | BinOp::LtEqFloat
        | BinOp::GtEqInt
        | BinOp::GtInt
        | BinOp::GtEqFloat
        | BinOp::GtFloat
        | BinOp::AddInt
        | BinOp::AddFloat
        | BinOp::SubInt
        | BinOp::SubFloat
        | BinOp::MultInt
        | BinOp::MultFloat
        | BinOp::DivInt
        | BinOp::DivFloat
        | BinOp::RemainderInt
        | BinOp::Concatenate => None,
    }
}

fn hint_wrap_value_in_result(expected: &Arc<Type>, given: &Arc<Type>) -> Option<String> {
    let expected = collapse_links(expected.clone());
    let (expected_ok_type, expected_error_type) = expected.result_types()?;

    // Unbound type variables are trivially matched by `same_as` to any other type,
    // leading to confusing hints. Any `given` type could match one of the `expected`
    // type variables, leading to false positives for the wrap hint.
    if !expected_ok_type.is_unbound() && given.same_as(expected_ok_type.as_ref()) {
        Some("Did you mean to wrap this in an `Ok`?".into())
    } else if !expected_error_type.is_unbound() && given.same_as(expected_error_type.as_ref()) {
        Some("Did you mean to wrap this in an `Error`?".into())
    } else {
        None
    }
}

fn hint_numeric_message(alt: &str, type_: &str) -> String {
    format!("The {alt} operator can be used with {type_}s\n")
}

fn hint_string_message() -> String {
    wrap("Strings can be joined using the `<>` operator.")
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unformatted {
    pub source: Utf8PathBuf,
    pub destination: Utf8PathBuf,
    pub input: EcoString,
    pub output: String,
}

pub fn wrap(text: &str) -> String {
    let mut result = String::with_capacity(text.len());

    for (i, line) in wrap_text(text, 75).iter().enumerate() {
        if i > 0 {
            result.push('\n');
        }
        result.push_str(line);
    }

    result
}

fn wrap_text(text: &str, width: usize) -> Vec<Cow<'_, str>> {
    let mut lines: Vec<Cow<'_, str>> = Vec::new();
    for line in text.split('\n') {
        // check if line needs to be broken
        match line.len() > width {
            false => lines.push(Cow::from(line)),
            true => {
                let mut new_lines = break_line(line, width);
                lines.append(&mut new_lines);
            }
        }
    }

    lines
}

fn break_line(line: &str, width: usize) -> Vec<Cow<'_, str>> {
    let mut lines: Vec<Cow<'_, str>> = Vec::new();
    let mut newline = String::from("");

    // split line by spaces
    for (i, word) in line.split(' ').enumerate() {
        let is_new_line = i < 1 || newline.is_empty();

        let can_add_word = match is_new_line {
            true => newline.len() + word.len() <= width,
            // +1 accounts for space added before word
            false => newline.len() + (word.len() + 1) <= width,
        };

        if can_add_word {
            if !is_new_line {
                newline.push(' ');
            }
            newline.push_str(word);
        } else {
            // word too big, save existing line if present
            if !newline.is_empty() {
                // save current line and reset it
                lines.push(Cow::from(newline.to_owned()));
                newline.clear();
            }

            // then save word to a new line or break it
            match word.len() > width {
                false => newline.push_str(word),
                true => {
                    let (mut newlines, remainder) = break_word(word, width);
                    lines.append(&mut newlines);
                    newline.push_str(remainder);
                }
            }
        }
    }

    // save last line after loop finishes
    if !newline.is_empty() {
        lines.push(Cow::from(newline));
    }

    lines
}

// breaks word into n lines based on width. Returns list of new lines and remainder
fn break_word(word: &str, width: usize) -> (Vec<Cow<'_, str>>, &str) {
    let mut new_lines: Vec<Cow<'_, str>> = Vec::new();
    // Split on a char boundary so a multi-byte UTF-8 char straddling `width`
    // doesn't cause a panic.
    let (first, mut remainder) = word.split_at(word.floor_char_boundary(width));
    new_lines.push(Cow::from(first));

    // split remainder until it's small enough
    while remainder.len() > width {
        let (first, second) = remainder.split_at(remainder.floor_char_boundary(width));
        new_lines.push(Cow::from(first));
        remainder = second;
    }

    (new_lines, remainder)
}
