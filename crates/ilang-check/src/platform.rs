//! Entries and the platform. At an entry (the program
//! entry, a test, a host export) the target platform gives its own
//! services: the host services of the target's table, `Assert`, `Fail`,
//! `Root`, `Owner`, `Await`, `Runner` and `Scheduler`. Every other service is
//! given by i code in the entry. The one check: the need of an entry has only
//! services of the platform of the target.

use crate::hir::{DefId, Program, TypeId};
use crate::types::Typed;
use ilang_package::{Diagnostic, ModuleKind};

/// "`T` is not provided by the target `t`".
pub const NOT_PROVIDED: &str = "E0523";

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Target {
    Js,
    Nodejs,
    Native,
    Wasi,
    Host,
}

impl Target {
    pub fn name(self) -> &'static str {
        match self {
            Target::Js => "js",
            Target::Nodejs => "nodejs",
            Target::Native => "native",
            Target::Wasi => "wasi",
            Target::Host => "host",
        }
    }
}

use Target::*;

/// The host services of the library and the targets whose platform gives
/// them. A host service declared outside this table (a program's own host
/// service, given by the embedding host) is given on every target.
pub const HOST_SERVICES: &[(&str, &str, &[Target])] = &[
    ("i/io", "Io", &[Native, Nodejs, Js, Wasi]),
    ("i/clock", "Clock", &[Native, Nodejs, Js, Wasi]),
    ("i/stdio", "Stdio", &[Native, Nodejs, Js, Wasi]),
    ("i/fs", "Fs", &[Native, Nodejs, Wasi]),
    ("i/net", "Net", &[Native, Nodejs]),
    ("i/tls", "Tls", &[Native, Nodejs]),
    ("i/process", "Arguments", &[Native, Nodejs, Wasi]),
    ("i/process", "Environment", &[Native, Nodejs, Wasi]),
    ("i/process", "WorkingDirectory", &[Native, Nodejs]),
    ("i/process", "Entropy", &[Native, Nodejs, Js, Wasi]),
    ("i/process", "Process", &[Native, Nodejs]),
    ("i/fetch", "Fetch", &[Nodejs, Js]),
];

/// What the entry stub of one entry gives.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Stub {
    pub entry: DefId,
    pub target: Target,
    /// The host services that the entry needs.
    pub hosts: Vec<TypeId>,
    pub root: bool,
    pub owner: bool,
    pub await_: bool,
    /// `Runner` or `Scheduler`: the entry runs inside `task.run`.
    pub tasks: bool,
    /// The entry waits for the host: it needs a request-mode host service, or
    /// tasks (and the target is not WASI).
    pub is_async: bool,
}

impl Stub {
    /// Whether the stub opens the request root (`Root`, `Owner`, `Await`, tasks).
    pub fn requests(&self) -> bool {
        self.root || self.owner || self.await_ || self.tasks
    }
}

/// WASI emits one command; test bodies are checked, but are not entries on
/// that target.
pub fn entry_is_eligible(p: &Program, entry: DefId, target: Target) -> bool {
    target != Wasi || p.module(entry.module).kind != ModuleKind::Test
}

fn find(p: &Program, module: &str, name: &str) -> Option<TypeId> {
    p.types
        .iter()
        .position(|t| t.name == name && p.module(t.module).path == module)
        .map(|i| TypeId(i as u32))
}

/// The stub of `entry` for `target`, or the error for each service of its
/// need that the platform does not give.
pub fn stub(p: &Program, t: &Typed, entry: DefId, target: Target) -> Result<Stub, Vec<Diagnostic>> {
    let needs = t.entry_services.get(&entry).cloned().unwrap_or_default();
    let mut stub = Stub {
        entry,
        target,
        hosts: Vec::new(),
        root: false,
        owner: false,
        await_: false,
        tasks: false,
        is_async: false,
    };
    let mut errors = Vec::new();
    for id in needs {
        let ty = p.ty(id);
        let module = p.module(ty.module);
        let path = module.path.as_str();
        match (path, ty.name.as_str()) {
            (_, "Assert" | "Fail") if module.kind == ModuleKind::Prelude => {}
            ("i/request", "Root") => stub.root = true,
            ("i/request", "Owner") => stub.owner = true,
            ("i/request", "Await") => stub.await_ = true,
            ("i/task", "Runner" | "Scheduler") => stub.tasks = true,
            _ if ty.host.is_some() => {
                let given = HOST_SERVICES
                    .iter()
                    .find(|(m, n, _)| *m == path && *n == ty.name)
                    .is_none_or(|(_, _, targets)| targets.contains(&target));
                if given {
                    stub.hosts.push(id);
                    let host = ty.host.as_ref().expect("host");
                    stub.is_async |= host.is_request || host.is_async;
                } else {
                    errors.push(not_provided(
                        p,
                        entry,
                        &format!("{path}.{}", ty.name),
                        target,
                    ));
                }
            }
            _ => errors.push(not_provided(
                p,
                entry,
                &format!("{path}.{}", ty.name),
                target,
            )),
        }
    }
    stub.is_async = target != Wasi && (stub.is_async || stub.tasks);
    if errors.is_empty() {
        Ok(stub)
    } else {
        Err(errors)
    }
}

/// The stubs of every eligible entry.
pub fn stubs(
    p: &Program,
    t: &Typed,
    target: Target,
) -> std::collections::BTreeMap<DefId, Result<Stub, Vec<Diagnostic>>> {
    t.entry_services
        .keys()
        .filter(|&&e| entry_is_eligible(p, e, target))
        .map(|&e| (e, stub(p, t, e, target)))
        .collect()
}

/// The type ids of the services that the stub provides by i code.
pub fn request_types(p: &Program) -> [Option<TypeId>; 5] {
    [
        find(p, "i/request", "Root"),
        find(p, "i/request", "Owner"),
        find(p, "i/request", "Await"),
        find(p, "i/task", "Runner"),
        find(p, "i/task", "Scheduler"),
    ]
}

fn not_provided(p: &Program, entry: DefId, service: &str, target: Target) -> Diagnostic {
    Diagnostic::error(
        NOT_PROVIDED,
        Some(p.module(entry.module).source),
        p.def(entry).span,
        format!(
            "`{service}` is not provided by the target `{}`: the entry `{}.{}` needs it, so give it in the entry with i code",
            target.name(),
            p.module(entry.module).path,
            p.def(entry).name
        ),
    )
}
