use crate::{BitArray, HostImports, Value};
use futures_util::future::{Either, select};
use std::{process::Stdio, sync::Arc, time::Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    process::{Child, Command},
    sync::Mutex,
};

const NS: &str = "i:process/v1";
const KIND: &str = "process child";
struct Process {
    child: Mutex<Child>,
    stdin: Mutex<Option<tokio::process::ChildStdin>>,
    stdout: Mutex<Option<tokio::process::ChildStdout>>,
    stderr: Mutex<Option<tokio::process::ChildStderr>>,
}
fn code(s: &str) -> Value {
    Value::String(s.into())
}
fn error(e: std::io::Error) -> Value {
    code(match e.kind() {
        std::io::ErrorKind::NotFound => "not_found",
        std::io::ErrorKind::PermissionDenied => "denied",
        std::io::ErrorKind::InvalidInput | std::io::ErrorKind::InvalidData => "invalid",
        _ => "io",
    })
}
fn bits(bytes: Vec<u8>) -> Value {
    let len = bytes.len() as u32 * 8;
    Value::BitArray(BitArray::new(bytes, len).expect("whole bytes"))
}
fn encode(values: impl Iterator<Item = String>) -> Vec<u8> {
    let values: Vec<_> = values.collect();
    let mut out = Vec::new();
    out.extend_from_slice(&(values.len() as u32).to_le_bytes());
    for s in values {
        out.extend_from_slice(&(s.len() as u32).to_le_bytes());
        out.extend_from_slice(s.as_bytes());
    }
    out
}
fn decode(bytes: &[u8]) -> Option<Vec<String>> {
    let mut p = 0usize;
    let mut number = || {
        let end = p.checked_add(4)?;
        let n = u32::from_le_bytes(bytes.get(p..end)?.try_into().ok()?);
        p = end;
        Some(n as usize)
    };
    let count = number()?;
    let mut result = Vec::new();
    for _ in 0..count {
        let end = p.checked_add(4)?;
        let n = u32::from_le_bytes(bytes.get(p..end)?.try_into().ok()?) as usize;
        p = end;
        let end = p.checked_add(n)?;
        result.push(std::str::from_utf8(bytes.get(p..end)?).ok()?.to_owned());
        p = end;
    }
    (p == bytes.len()).then_some(result)
}
fn status(s: std::process::ExitStatus) -> i32 {
    if let Some(n) = s.code() {
        n.rem_euclid(256)
    } else {
        #[cfg(unix)]
        {
            use std::os::unix::process::ExitStatusExt;
            128 + s.signal().unwrap_or(127) % 128
        }
        #[cfg(not(unix))]
        {
            255
        }
    }
}
fn mode(s: &str) -> Option<Stdio> {
    match s {
        "inherit" => Some(Stdio::inherit()),
        "null" => Some(Stdio::null()),
        "piped" => Some(Stdio::piped()),
        _ => None,
    }
}
pub(super) fn register(imports: &mut HostImports, program_args: Vec<String>) {
    imports
        .async_service("i:arguments/v1", "args", move |_| {
            let values = program_args.clone();
            async move { Ok(Ok(bits(encode(values.into_iter())))) }
        })
        .expect(super::ONE);
    imports
        .async_service("i:environment/v1", "env", |args| {
            let [Value::String(name)] = args else {
                panic!("env arguments")
            };
            let name = name.clone();
            async move {
                Ok(if name.contains(['\0', '=']) || name.is_empty() {
                    Err(code("invalid"))
                } else {
                    std::env::var(name).map(Value::String).map_err(|e| {
                        code(match e {
                            std::env::VarError::NotPresent => "not_found",
                            std::env::VarError::NotUnicode(_) => "not_utf8",
                        })
                    })
                })
            }
        })
        .expect(super::ONE);
    imports
        .async_service("i:working-directory/v1", "cwd", |_| async {
            Ok(std::env::current_dir().map_err(error).and_then(|p| {
                p.into_os_string()
                    .into_string()
                    .map(Value::String)
                    .map_err(|_| code("not_utf8"))
            }))
        })
        .expect(super::ONE);
    imports
        .async_service("i:entropy/v1", "entropy", |args| {
            let [Value::Int(n)] = args else {
                panic!("entropy arguments")
            };
            let n = usize::try_from(n).ok().filter(|n| *n <= 1 << 24);
            async move {
                let Some(n) = n else {
                    return Ok(Err(code("invalid")));
                };
                let mut bytes = vec![0; n];
                Ok(getrandom::fill(&mut bytes)
                    .map(|()| bits(bytes))
                    .map_err(|_| code("io")))
            }
        })
        .expect(super::ONE);
    imports
        .context_async_service(NS, "spawn", |ctx, args| {
            let [Value::String(program), Value::BitArray(args)] = args else {
                panic!("spawn arguments")
            };
            let program = program.clone();
            let args = args.clone();
            async move {
                let Some(args) = (args.bit_length % 8 == 0)
                    .then(|| decode(&args.bytes))
                    .flatten()
                else {
                    return Ok(Err(code("invalid")));
                };
                let mut cmd = Command::new(program);
                cmd.args(args).kill_on_drop(true);
                let child = match cmd.spawn() {
                    Ok(c) => c,
                    Err(e) => return Ok(Err(error(e))),
                };
                let p = Process {
                    child: Mutex::new(child),
                    stdin: Mutex::new(None),
                    stdout: Mutex::new(None),
                    stderr: Mutex::new(None),
                };
                match ctx.insert(KIND, p, |p, done| {
                    tokio::spawn(async move {
                        let mut c = p.child.lock().await;
                        let _ = c.start_kill();
                        let _ = c.wait().await;
                        drop(done);
                    });
                }) {
                    Ok(id) => Ok(Ok(Value::Int(id))),
                    Err(e) => Ok(Err(code(&e.to_string()))),
                }
            }
        })
        .expect(super::ONE);

    imports
        .context_async_service(NS, "spawn_piped", |ctx, args| {
            let [
                Value::String(program),
                Value::BitArray(args),
                Value::String(input),
                Value::String(output),
                Value::String(errors),
            ] = args
            else {
                panic!("spawn_piped arguments")
            };
            let program = program.clone();
            let args = args.clone();
            let modes = (mode(input), mode(output), mode(errors));
            async move {
                let (Some(input), Some(output), Some(errors)) = modes else {
                    return Ok(Err(code("invalid")));
                };
                let Some(args) = (args.bit_length % 8 == 0)
                    .then(|| decode(&args.bytes))
                    .flatten()
                else {
                    return Ok(Err(code("invalid")));
                };
                let mut cmd = Command::new(program);
                cmd.args(args)
                    .stdin(input)
                    .stdout(output)
                    .stderr(errors)
                    .kill_on_drop(true);
                let mut child = match cmd.spawn() {
                    Ok(c) => c,
                    Err(e) => return Ok(Err(error(e))),
                };
                let p = Process {
                    stdin: Mutex::new(child.stdin.take()),
                    stdout: Mutex::new(child.stdout.take()),
                    stderr: Mutex::new(child.stderr.take()),
                    child: Mutex::new(child),
                };
                match ctx.insert(KIND, p, |p, done| {
                    tokio::spawn(async move {
                        let mut c = p.child.lock().await;
                        let _ = c.start_kill();
                        let _ = c.wait().await;
                        drop(done);
                    });
                }) {
                    Ok(id) => Ok(Ok(Value::Int(id))),
                    Err(e) => Ok(Err(code(&e.to_string()))),
                }
            }
        })
        .expect(super::ONE);
    imports
        .context_async_service(NS, "spawn_config", |ctx, args| {
            let [
                Value::String(program),
                Value::BitArray(args),
                Value::String(input),
                Value::String(output),
                Value::String(errors),
                Value::String(env_mode),
                Value::BitArray(env),
                Value::String(directory),
                Value::String(cwd_mode),
            ] = args
            else {
                panic!("spawn_piped arguments")
            };
            let program = program.clone();
            let args = args.clone();
            let modes = (mode(input), mode(output), mode(errors));
            let env_mode = env_mode.clone();
            let env = env.clone();
            let directory = directory.clone();
            let cwd_mode = cwd_mode.clone();
            async move {
                let (Some(input), Some(output), Some(errors)) = modes else {
                    return Ok(Err(code("invalid")));
                };
                let Some(args) = (args.bit_length % 8 == 0)
                    .then(|| decode(&args.bytes))
                    .flatten()
                else {
                    return Ok(Err(code("invalid")));
                };
                let Some(env) = (env.bit_length % 8 == 0).then(|| decode(&env.bytes)).flatten() else {
                    return Ok(Err(code("invalid")));
                };
                let mut cmd = Command::new(program);
                match env_mode.as_str() {
                    "replace" => { cmd.env_clear(); }
                    "inherit" => {}
                    _ => return Ok(Err(code("invalid"))),
                }
                for item in env {
                    if let Some((name, value)) = item.split_once('=') {
                        if name.is_empty() || name.contains('\0') || value.contains('\0') { return Ok(Err(code("invalid"))); }
                        cmd.env(name, value);
                    } else {
                        if item.is_empty() || item.contains('\0') { return Ok(Err(code("invalid"))); }
                        cmd.env_remove(item);
                    }
                }
                match cwd_mode.as_str() {
                    "set" => { cmd.current_dir(directory); }
                    "inherit" => {}
                    _ => return Ok(Err(code("invalid"))),
                }
                cmd.args(args)
                    .stdin(input)
                    .stdout(output)
                    .stderr(errors)
                    .kill_on_drop(true);
                let mut child = match cmd.spawn() {
                    Ok(c) => c,
                    Err(e) => return Ok(Err(error(e))),
                };
                let p = Process {
                    stdin: Mutex::new(child.stdin.take()),
                    stdout: Mutex::new(child.stdout.take()),
                    stderr: Mutex::new(child.stderr.take()),
                    child: Mutex::new(child),
                };
                match ctx.insert(KIND, p, |p, done| {
                    tokio::spawn(async move {
                        let mut c = p.child.lock().await;
                        let _ = c.start_kill();
                        let _ = c.wait().await;
                        drop(done);
                    });
                }) {
                    Ok(id) => Ok(Ok(Value::Int(id))),
                    Err(e) => Ok(Err(code(&e.to_string()))),
                }
            }
        })
        .expect(super::ONE);
    for op in ["read_stdout", "read_stderr", "write_stdin", "close_stdin"] {
        imports
            .context_async_service(NS, op, move |ctx, args| {
                let id = match args {
                    [Value::Int(id), ..] => id.clone(),
                    _ => panic!("process stream arguments"),
                };
                let arg = args.get(1).cloned();
                async move {
                    let lease = match ctx.acquire::<Process>(&id, KIND, op) {
                        Ok(l) => l,
                        Err(e) => return Ok(Err(code(&e.to_string()))),
                    };
                    let p = Arc::clone(lease.resource());
                    let work = async move {
                        match op {
                            "write_stdin" => {
                                let Some(Value::BitArray(b)) = arg else {
                                    panic!("write_stdin arguments")
                                };
                                if b.bit_length % 8 != 0 {
                                    return Err(code("invalid"));
                                }
                                let mut stream = p.stdin.lock().await;
                                match stream.as_mut() {
                                    Some(s) => s
                                        .write_all(&b.bytes)
                                        .await
                                        .map(|()| Value::Nil)
                                        .map_err(error),
                                    None => Err(code("closed")),
                                }
                            }
                            "close_stdin" => {
                                p.stdin.lock().await.take();
                                Ok(Value::Nil)
                            }
                            _ => {
                                let Some(Value::Int(n)) = arg else {
                                    panic!("read arguments")
                                };
                                let Some(n) = usize::try_from(&n).ok().filter(|n| *n > 0) else {
                                    return Err(code("invalid"));
                                };
                                let mut buf = vec![0; n.min(65536)];
                                let r = if op == "read_stdout" {
                                    let mut s = p.stdout.lock().await;
                                    match s.as_mut() {
                                        Some(s) => s.read(&mut buf).await.map_err(error),
                                        None => Err(code("closed")),
                                    }
                                } else {
                                    let mut s = p.stderr.lock().await;
                                    match s.as_mut() {
                                        Some(s) => s.read(&mut buf).await.map_err(error),
                                        None => Err(code("closed")),
                                    }
                                };
                                r.map(|n| {
                                    buf.truncate(n);
                                    bits(buf)
                                })
                            }
                        }
                    };
                    let result = match select(Box::pin(work), Box::pin(lease.closed())).await {
                        Either::Left((r, _)) => r,
                        Either::Right(_) => Err(code("closed")),
                    };
                    Ok(lease
                        .commit(result)
                        .unwrap_or_else(|e| Err(code(&e.to_string()))))
                }
            })
            .expect(super::ONE);
    }
    for operation in ["wait", "terminate"] {
        imports
            .context_async_service(NS, operation, move |ctx, args| {
                let [Value::Int(id)] = args else {
                    panic!("process operation arguments")
                };
                let id = id.clone();
                async move {
                    let lease = match ctx.acquire::<Process>(&id, KIND, operation) {
                        Ok(l) => l,
                        Err(e) => return Ok(Err(code(&e.to_string()))),
                    };
                    let p = Arc::clone(lease.resource());
                    let work = async move {
                        if operation == "terminate" {
                            let mut c = p.child.lock().await;
                            return match c.try_wait() {
                                Ok(Some(_)) => Ok(Value::Nil),
                                Ok(None) => c.start_kill().map(|()| Value::Nil).map_err(error),
                                Err(e) => Err(error(e)),
                            };
                        }
                        loop {
                            let result = p.child.lock().await.try_wait();
                            match result {
                                Ok(Some(s)) => return Ok(Value::Int(status(s).into())),
                                Ok(None) => tokio::time::sleep(Duration::from_millis(10)).await,
                                Err(e) => return Err(error(e)),
                            }
                        }
                    };
                    let result = match select(Box::pin(work), Box::pin(lease.closed())).await {
                        Either::Left((r, _)) => r,
                        Either::Right(_) => Err(code("closed")),
                    };
                    Ok(lease
                        .commit(result)
                        .unwrap_or_else(|e| Err(code(&e.to_string()))))
                }
            })
            .expect(super::ONE);
    }
    imports
        .context_async_service(NS, "close", |ctx, args| {
            let [Value::Int(id)] = args else {
                panic!("close arguments")
            };
            let receipt = ctx.close(id, KIND);
            async move {
                receipt.await;
                Ok(Ok(Value::Nil))
            }
        })
        .expect(super::ONE);
}
