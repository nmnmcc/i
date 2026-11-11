use crate::{BitArray, HostImports, ResourceContext, Value};
use futures_util::future::{Either, select};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::Mutex;

const NS: &str = "i:fs/v1";
const KIND: &str = "file";

// A cancelled cursor operation may have detached blocking work. Retire that
// handle so no later operation can race the detached operation's cursor.
struct CursorGuard {
    context: ResourceContext,
    id: crate::BigInt,
    active: bool,
}
impl Drop for CursorGuard {
    fn drop(&mut self) {
        if self.active {
            self.context.close(&self.id, KIND);
        }
    }
}

fn error(e: std::io::Error) -> Value {
    let code = match e.kind() {
        std::io::ErrorKind::NotFound => "not_found",
        std::io::ErrorKind::PermissionDenied => "denied",
        std::io::ErrorKind::AlreadyExists => "exists",
        std::io::ErrorKind::InvalidInput | std::io::ErrorKind::InvalidData => "invalid",
        std::io::ErrorKind::Interrupted => "interrupted",
        std::io::ErrorKind::Unsupported => "unsupported",
        _ => "io",
    };
    Value::String(code.into())
}
fn resource_error(e: crate::resources::ResourceError) -> Value {
    Value::String(
        match e {
            crate::resources::ResourceError::Busy => "busy",
            _ => "closed",
        }
        .into(),
    )
}

pub(super) fn register(imports: &mut HostImports) {
    imports
        .context_async_service(NS, "open", |context, args| {
            let [Value::String(path), Value::String(mode)] = args else {
                panic!("fs open arguments")
            };
            let (path, mode) = (path.clone(), mode.clone());
            async move {
                let mut options = tokio::fs::OpenOptions::new();
                match mode.as_str() {
                    "read" => {
                        options.read(true);
                    }
                    "truncate" => {
                        options.write(true).create(true).truncate(true);
                    }
                    "append" => {
                        options.append(true).create(true);
                    }
                    "read_write" => {
                        options.read(true).write(true).create(true);
                    }
                    _ => return Ok(Err(Value::String("invalid".into()))),
                }
                let file = match options.open(path).await {
                    Ok(f) => f,
                    Err(e) => return Ok(Err(error(e))),
                };
                match context.insert(KIND, Mutex::new(file), |_, done| drop(done)) {
                    Ok(id) => Ok(Ok(Value::Int(id))),
                    Err(crate::resources::ResourceError::Memory) => {
                        Err(crate::resources::ResourceError::Memory.into())
                    }
                    Err(e) => Ok(Err(resource_error(e))),
                }
            }
        })
        .expect(super::ONE);
    imports
        .context_async_service(NS, "read", |context, args| {
            let [Value::Int(id), Value::Int(n)] = args else {
                panic!("fs read arguments")
            };
            let (id, n) = (id.clone(), n.clone());
            async move {
                if n <= 0.into() {
                    return Ok(Err(Value::String("invalid".into())));
                }
                let n = if n > 65536.into() {
                    65536
                } else {
                    usize::try_from(&n).expect("bounded read")
                };
                let lease = match context.acquire::<Mutex<tokio::fs::File>>(&id, KIND, "cursor") {
                    Ok(lease) => lease,
                    Err(e) => return Ok(Err(resource_error(e))),
                };
                let file = Arc::clone(lease.resource());
                let mut guard = CursorGuard {
                    context: context.clone(),
                    id,
                    active: true,
                };
                let mut bytes = vec![0; n];
                let work = async move {
                    file.lock()
                        .await
                        .read(&mut bytes)
                        .await
                        .map(|n| {
                            bytes.truncate(n);
                            Value::BitArray(
                                BitArray::new(bytes, (n * 8) as u32).expect("whole bytes"),
                            )
                        })
                        .map_err(error)
                };
                let result = match select(Box::pin(work), Box::pin(lease.closed())).await {
                    Either::Left((result, _)) => result,
                    Either::Right((_, _)) => Err(Value::String("closed".into())),
                };
                guard.active = false;
                Ok(lease
                    .commit(result)
                    .unwrap_or_else(|e| Err(resource_error(e))))
            }
        })
        .expect(super::ONE);
    imports
        .context_async_service(NS, "write", |context, args| {
            let [Value::Int(id), Value::BitArray(bytes)] = args else {
                panic!("fs write arguments")
            };
            let (id, bytes) = (id.clone(), bytes.clone());
            async move {
                if bytes.bit_length % 8 != 0 {
                    return Ok(Err(Value::String("invalid".into())));
                }
                let lease = match context.acquire::<Mutex<tokio::fs::File>>(&id, KIND, "cursor") {
                    Ok(lease) => lease,
                    Err(e) => return Ok(Err(resource_error(e))),
                };
                let file = Arc::clone(lease.resource());
                let mut guard = CursorGuard {
                    context: context.clone(),
                    id,
                    active: true,
                };
                let work = async move {
                    file.lock()
                        .await
                        .write_all(&bytes.bytes)
                        .await
                        .map(|()| Value::Nil)
                        .map_err(error)
                };
                let result = match select(Box::pin(work), Box::pin(lease.closed())).await {
                    Either::Left((result, _)) => result,
                    Either::Right((_, _)) => Err(Value::String("closed".into())),
                };
                guard.active = false;
                Ok(lease
                    .commit(result)
                    .unwrap_or_else(|e| Err(resource_error(e))))
            }
        })
        .expect(super::ONE);
    imports
        .context_async_service(NS, "flush", |context, args| {
            let [Value::Int(id)] = args else {
                panic!("fs flush arguments")
            };
            let id = id.clone();
            async move {
                let lease = match context.acquire::<Mutex<tokio::fs::File>>(&id, KIND, "cursor") {
                    Ok(lease) => lease,
                    Err(e) => return Ok(Err(resource_error(e))),
                };
                let file = Arc::clone(lease.resource());
                let mut guard = CursorGuard {
                    context: context.clone(),
                    id,
                    active: true,
                };
                let work = async move {
                    file.lock()
                        .await
                        .flush()
                        .await
                        .map(|()| Value::Nil)
                        .map_err(error)
                };
                let result = match select(Box::pin(work), Box::pin(lease.closed())).await {
                    Either::Left((result, _)) => result,
                    Either::Right((_, _)) => Err(Value::String("closed".into())),
                };
                guard.active = false;
                Ok(lease
                    .commit(result)
                    .unwrap_or_else(|e| Err(resource_error(e))))
            }
        })
        .expect(super::ONE);
    imports
        .context_async_service(NS, "close", |context: ResourceContext, args| {
            let [Value::Int(id)] = args else {
                panic!("fs close arguments")
            };
            let receipt = context.close(id, KIND);
            async move {
                receipt.await;
                Ok(Ok(Value::Nil))
            }
        })
        .expect(super::ONE);
    imports
        .async_service(NS, "remove", |args| {
            let [Value::String(path)] = args else {
                panic!("fs remove arguments")
            };
            let path = path.clone();
            async move {
                let metadata = match tokio::fs::symlink_metadata(&path).await {
                    Ok(m) => m,
                    Err(e) => return Ok(Err(error(e))),
                };
                let result = if metadata.is_dir() {
                    tokio::fs::remove_dir(path).await
                } else {
                    tokio::fs::remove_file(path).await
                };
                Ok(result.map(|()| Value::Nil).map_err(error))
            }
        })
        .expect(super::ONE);
    imports
        .async_service(NS, "rename", |args| {
            let [Value::String(from), Value::String(to)] = args else {
                panic!("fs rename arguments")
            };
            let (from, to) = (from.clone(), to.clone());
            async move {
                Ok(tokio::fs::rename(from, to)
                    .await
                    .map(|()| Value::Nil)
                    .map_err(error))
            }
        })
        .expect(super::ONE);
    imports
        .async_service(NS, "mkdir", |args| {
            let [Value::String(path)] = args else {
                panic!("fs mkdir arguments")
            };
            let path = path.clone();
            async move {
                Ok(tokio::fs::create_dir(path)
                    .await
                    .map(|()| Value::Nil)
                    .map_err(error))
            }
        })
        .expect(super::ONE);
    imports
        .async_service(NS, "list", |args| {
            let [Value::String(path)] = args else {
                panic!("fs list arguments")
            };
            let path = path.clone();
            async move {
                let mut dir = match tokio::fs::read_dir(path).await {
                    Ok(d) => d,
                    Err(e) => return Ok(Err(error(e))),
                };
                let mut names = Vec::new();
                loop {
                    match dir.next_entry().await {
                        Ok(Some(entry)) => match entry.file_name().into_string() {
                            Ok(name) => names.push(name),
                            Err(_) => return Ok(Err(Value::String("invalid".into()))),
                        },
                        Ok(None) => break,
                        Err(e) => return Ok(Err(error(e))),
                    }
                }
                names.sort();
                let mut result = String::new();
                for name in names {
                    result.push_str(&name);
                    result.push('\0');
                }
                Ok(Ok(Value::String(result)))
            }
        })
        .expect(super::ONE);
    imports
        .async_service(NS, "size", |args| {
            let [Value::String(path)] = args else {
                panic!("fs size arguments")
            };
            let path = path.clone();
            async move {
                Ok(tokio::fs::metadata(path)
                    .await
                    .map(|m| Value::Int(m.len().into()))
                    .map_err(error))
            }
        })
        .expect(super::ONE);
    imports
        .async_service(NS, "kind", |args| {
            let [Value::String(path)] = args else {
                panic!("fs kind arguments")
            };
            let path = path.clone();
            async move {
                Ok(tokio::fs::metadata(path)
                    .await
                    .map(|m| {
                        Value::String(
                            if m.is_file() {
                                "file"
                            } else if m.is_dir() {
                                "directory"
                            } else {
                                "other"
                            }
                            .into(),
                        )
                    })
                    .map_err(error))
            }
        })
        .expect(super::ONE);
    imports
        .async_service(NS, "temp_dir", |_| async {
            Ok(std::env::temp_dir()
                .into_os_string()
                .into_string()
                .map(Value::String)
                .map_err(|_| Value::String("invalid".into())))
        })
        .expect(super::ONE);
}
