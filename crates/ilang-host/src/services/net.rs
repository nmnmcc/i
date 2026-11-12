use crate::{BitArray, HostImports, ResourceContext, Value};
use futures_util::future::{Either, select};
use std::{
    net::{IpAddr, SocketAddr},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream, UdpSocket},
    sync::Mutex,
};

const NS: &str = "i:net/v1";
const LISTENER: &str = "net listener";
const DATAGRAM: &str = "net datagram";
const STREAM: &str = "net stream";
struct Stream {
    read: Mutex<tokio::net::tcp::OwnedReadHalf>,
    write: Mutex<tokio::net::tcp::OwnedWriteHalf>,
    shutdown: AtomicBool,
}
fn code(s: &str) -> Value {
    Value::String(s.into())
}
fn error(e: std::io::Error) -> Value {
    code(match e.kind() {
        std::io::ErrorKind::InvalidInput | std::io::ErrorKind::InvalidData => "invalid",
        std::io::ErrorKind::PermissionDenied => "denied",
        std::io::ErrorKind::AddrInUse => "address_in_use",
        std::io::ErrorKind::ConnectionRefused => "refused",
        std::io::ErrorKind::ConnectionReset
        | std::io::ErrorKind::BrokenPipe
        | std::io::ErrorKind::ConnectionAborted => "reset",
        std::io::ErrorKind::NetworkUnreachable | std::io::ErrorKind::HostUnreachable => {
            "unreachable"
        }
        std::io::ErrorKind::NotFound => "not_found",
        std::io::ErrorKind::Interrupted => "interrupted",
        std::io::ErrorKind::Unsupported => "unsupported",
        _ => "io",
    })
}
fn resource_error(e: crate::resources::ResourceError) -> Value {
    code(if e == crate::resources::ResourceError::Busy {
        "busy"
    } else {
        "closed"
    })
}
fn insert<T: Send + Sync + 'static>(
    context: &ResourceContext,
    kind: &str,
    value: T,
) -> anyhow::Result<Result<Value, Value>> {
    match context.insert(kind, value, |_, done| drop(done)) {
        Ok(id) => Ok(Ok(Value::Int(id))),
        Err(crate::resources::ResourceError::Memory) => {
            Err(crate::resources::ResourceError::Memory.into())
        }
        Err(e) => Ok(Err(resource_error(e))),
    }
}
fn insert_stream(
    context: &ResourceContext,
    stream: TcpStream,
) -> anyhow::Result<Result<Value, Value>> {
    let (read, write) = stream.into_split();
    insert(
        context,
        STREAM,
        Stream {
            read: Mutex::new(read),
            write: Mutex::new(write),
            shutdown: AtomicBool::new(false),
        },
    )
}

/// Consume an idle TCP stream so a transport such as TLS can own the socket.
pub(super) fn take_stream(
    context: &ResourceContext,
    id: &crate::BigInt,
) -> Result<TcpStream, crate::resources::ResourceError> {
    let stream = context.take_idle::<Stream>(id, STREAM)?;
    let stream = Arc::try_unwrap(stream).unwrap_or_else(|_| panic!("idle TCP stream is shared"));
    let read = stream.read.into_inner();
    let write = stream.write.into_inner();
    Ok(read
        .reunite(write)
        .expect("TCP halves belong to the same stream"))
}
fn port(n: &crate::BigInt, zero: bool) -> Option<u16> {
    u16::try_from(n).ok().filter(|p| zero || *p != 0)
}
pub(super) fn register(imports: &mut HostImports) {
    register_datagrams(imports);
    imports
        .context_async_service(NS, "listen", |ctx, args| {
            let [Value::String(address), Value::Int(n)] = args else {
                panic!("net listen arguments")
            };
            let address = address.parse::<IpAddr>();
            let n = port(n, true);
            async move {
                let (Ok(address), Some(n)) = (address, n) else {
                    return Ok(Err(code("invalid")));
                };
                match TcpListener::bind(SocketAddr::new(address, n)).await {
                    Ok(l) => insert(&ctx, LISTENER, l),
                    Err(e) => Ok(Err(error(e))),
                }
            }
        })
        .expect(super::ONE);
    imports
        .context_async_service(NS, "connect", |ctx, args| {
            let [Value::String(address), Value::Int(n)] = args else {
                panic!("net connect arguments")
            };
            let address = address.clone();
            let n = port(n, false);
            async move {
                let Some(n) = n.filter(|_| !address.is_empty()) else {
                    return Ok(Err(code("invalid")));
                };
                let addresses = match tokio::net::lookup_host((address.as_str(), n)).await {
                    Ok(a) => a,
                    Err(e) => return Ok(Err(error(e))),
                };
                let mut last = code("io");
                for addr in addresses {
                    match TcpStream::connect(addr).await {
                        Ok(s) => return insert_stream(&ctx, s),
                        Err(e) => last = error(e),
                    }
                }
                Ok(Err(last))
            }
        })
        .expect(super::ONE);
    imports
        .async_service(NS, "resolve", |args| {
            let [Value::String(name)] = args else {
                panic!("net resolve arguments")
            };
            let name = name.clone();
            async move {
                if name.is_empty() {
                    return Ok(Err(code("invalid")));
                }
                Ok(tokio::net::lookup_host((name.as_str(), 0))
                    .await
                    .map(|addrs| Value::String(addrs.map(|a| format!("{}\0", a.ip())).collect()))
                    .map_err(error))
            }
        })
        .expect(super::ONE);
    imports
        .context_async_service(NS, "local_port", |ctx, args| {
            let [Value::Int(id)] = args else {
                panic!("net local_port arguments")
            };
            let id = id.clone();
            async move {
                let lease = match ctx.acquire::<TcpListener>(&id, LISTENER, "port") {
                    Ok(l) => l,
                    Err(e) => return Ok(Err(resource_error(e))),
                };
                let result = lease
                    .resource()
                    .local_addr()
                    .map(|a| Value::Int(a.port().into()))
                    .map_err(error);
                Ok(lease
                    .commit(result)
                    .unwrap_or_else(|e| Err(resource_error(e))))
            }
        })
        .expect(super::ONE);
    imports
        .context_async_service(NS, "accept", |ctx, args| {
            let [Value::Int(id)] = args else {
                panic!("net accept arguments")
            };
            let id = id.clone();
            async move {
                let lease = match ctx.acquire::<TcpListener>(&id, LISTENER, "accept") {
                    Ok(l) => l,
                    Err(e) => return Ok(Err(resource_error(e))),
                };
                let listener = Arc::clone(lease.resource());
                let result =
                    match select(Box::pin(listener.accept()), Box::pin(lease.closed())).await {
                        Either::Left((r, _)) => r.map(|(s, _)| s).map_err(error),
                        Either::Right(_) => Err(code("closed")),
                    };
                match lease
                    .commit(result)
                    .unwrap_or_else(|e| Err(resource_error(e)))
                {
                    Ok(s) => insert_stream(&ctx, s),
                    Err(e) => Ok(Err(e)),
                }
            }
        })
        .expect(super::ONE);
    imports
        .context_async_service(NS, "read", |ctx, args| {
            let [Value::Int(id), Value::Int(n)] = args else {
                panic!("net read arguments")
            };
            let id = id.clone();
            let n = n.clone();
            async move {
                if n <= 0.into() {
                    return Ok(Err(code("invalid")));
                }
                let n = if n > 65536.into() {
                    65536
                } else {
                    usize::try_from(&n).expect("bounded read")
                };
                let lease = match ctx.acquire::<Stream>(&id, STREAM, "read") {
                    Ok(l) => l,
                    Err(e) => return Ok(Err(resource_error(e))),
                };
                let stream = Arc::clone(lease.resource());
                let work = async move {
                    let mut bytes = vec![0; n];
                    stream
                        .read
                        .lock()
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
                    Either::Left((r, _)) => r,
                    Either::Right(_) => Err(code("closed")),
                };
                Ok(lease
                    .commit(result)
                    .unwrap_or_else(|e| Err(resource_error(e))))
            }
        })
        .expect(super::ONE);
    imports
        .context_async_service(NS, "write", |ctx, args| {
            let [Value::Int(id), Value::BitArray(bytes)] = args else {
                panic!("net write arguments")
            };
            let id = id.clone();
            let bytes = bytes.clone();
            async move {
                if bytes.bit_length % 8 != 0 {
                    return Ok(Err(code("invalid")));
                }
                let lease = match ctx.acquire::<Stream>(&id, STREAM, "write") {
                    Ok(l) => l,
                    Err(e) => return Ok(Err(resource_error(e))),
                };
                let stream = Arc::clone(lease.resource());
                let work = async move {
                    if stream.shutdown.load(Ordering::SeqCst) {
                        return Err(code("closed"));
                    }
                    stream
                        .write
                        .lock()
                        .await
                        .write_all(&bytes.bytes)
                        .await
                        .map(|_| Value::Nil)
                        .map_err(error)
                };
                let result = match select(Box::pin(work), Box::pin(lease.closed())).await {
                    Either::Left((r, _)) => r,
                    Either::Right(_) => Err(code("closed")),
                };
                Ok(lease
                    .commit(result)
                    .unwrap_or_else(|e| Err(resource_error(e))))
            }
        })
        .expect(super::ONE);
    imports
        .context_async_service(NS, "shutdown_write", |ctx, args| {
            let [Value::Int(id)] = args else {
                panic!("net shutdown arguments")
            };
            let id = id.clone();
            async move {
                let lease = match ctx.acquire::<Stream>(&id, STREAM, "write") {
                    Ok(l) => l,
                    Err(e) => return Ok(Err(resource_error(e))),
                };
                let stream = Arc::clone(lease.resource());
                let work = async move {
                    if stream.shutdown.load(Ordering::SeqCst) {
                        return Ok(Value::Nil);
                    }
                    stream
                        .write
                        .lock()
                        .await
                        .shutdown()
                        .await
                        .map(|_| {
                            stream.shutdown.store(true, Ordering::SeqCst);
                            Value::Nil
                        })
                        .map_err(error)
                };
                let result = match select(Box::pin(work), Box::pin(lease.closed())).await {
                    Either::Left((r, _)) => r,
                    Either::Right(_) => Err(code("closed")),
                };
                Ok(lease
                    .commit(result)
                    .unwrap_or_else(|e| Err(resource_error(e))))
            }
        })
        .expect(super::ONE);
    for (name, kind) in [
        ("close_stream", STREAM),
        ("close_listener", LISTENER),
        ("close_datagram", DATAGRAM),
    ] {
        imports
            .context_async_service(NS, name, move |ctx, args| {
                let [Value::Int(id)] = args else {
                    panic!("net close arguments")
                };
                let receipt = ctx.close(id, kind);
                async move {
                    receipt.await;
                    Ok(Ok(Value::Nil))
                }
            })
            .expect(super::ONE);
    }
}

fn register_datagrams(imports: &mut HostImports) {
    imports
        .context_async_service(NS, "bind", |ctx, args| {
            let [Value::String(address), Value::Int(n)] = args else {
                panic!("net bind arguments")
            };
            let address = address.clone();
            let n = port(n, true);
            async move {
                let Some(n) = n.filter(|_| !address.is_empty()) else {
                    return Ok(Err(code("invalid")));
                };
                match UdpSocket::bind((address.as_str(), n)).await {
                    Ok(socket) => insert(&ctx, DATAGRAM, socket),
                    Err(e) => Ok(Err(error(e))),
                }
            }
        })
        .expect(super::ONE);
    for name in ["local_address", "datagram_port"] {
        imports
            .context_async_service(NS, name, move |ctx, args| {
                let [Value::Int(id)] = args else {
                    panic!("net datagram address arguments")
                };
                let id = id.clone();
                async move {
                    let lease = match ctx.acquire::<UdpSocket>(&id, DATAGRAM, "address") {
                        Ok(l) => l,
                        Err(e) => return Ok(Err(resource_error(e))),
                    };
                    let result = lease
                        .resource()
                        .local_addr()
                        .map(|a| {
                            if name == "local_address" {
                                Value::String(a.ip().to_string())
                            } else {
                                Value::Int(a.port().into())
                            }
                        })
                        .map_err(error);
                    Ok(lease
                        .commit(result)
                        .unwrap_or_else(|e| Err(resource_error(e))))
                }
            })
            .expect(super::ONE);
    }
    imports
        .context_async_service(NS, "send_to", |ctx, args| {
            let [
                Value::Int(id),
                Value::String(address),
                Value::Int(n),
                Value::BitArray(bytes),
            ] = args
            else {
                panic!("net send_to arguments")
            };
            let (id, address, n, bytes) =
                (id.clone(), address.clone(), port(n, false), bytes.clone());
            async move {
                let Some(n) = n.filter(|_| !address.is_empty() && bytes.bit_length % 8 == 0) else {
                    return Ok(Err(code("invalid")));
                };
                let lease = match ctx.acquire::<UdpSocket>(&id, DATAGRAM, "write") {
                    Ok(l) => l,
                    Err(e) => return Ok(Err(resource_error(e))),
                };
                let socket = Arc::clone(lease.resource());
                let work = async move {
                    let addresses = tokio::net::lookup_host((address.as_str(), n))
                        .await
                        .map_err(error)?;
                    let mut last = code("invalid");
                    for address in addresses {
                        match socket.send_to(&bytes.bytes, address).await {
                            Ok(n) => {
                                assert_eq!(n, bytes.bytes.len(), "UDP send is atomic");
                                return Ok(Value::Nil);
                            }
                            Err(e) => last = error(e),
                        }
                    }
                    Err(last)
                };
                let result = match select(Box::pin(work), Box::pin(lease.closed())).await {
                    Either::Left((r, _)) => r,
                    Either::Right(_) => Err(code("closed")),
                };
                Ok(lease
                    .commit(result)
                    .unwrap_or_else(|e| Err(resource_error(e))))
            }
        })
        .expect(super::ONE);
    imports
        .context_async_service(NS, "receive_from", |ctx, args| {
            let [Value::Int(id), Value::Int(n)] = args else {
                panic!("net receive_from arguments")
            };
            let (id, n) = (id.clone(), n.clone());
            async move {
                if n <= 0.into() {
                    return Ok(Err(code("invalid")));
                }
                let n = if n > 65536.into() {
                    65536
                } else {
                    usize::try_from(&n).expect("bounded datagram")
                };
                let lease = match ctx.acquire::<UdpSocket>(&id, DATAGRAM, "read") {
                    Ok(l) => l,
                    Err(e) => return Ok(Err(resource_error(e))),
                };
                let socket = Arc::clone(lease.resource());
                let work = async move {
                    let mut data = vec![0; n];
                    let (n, sender) = socket.recv_from(&mut data).await.map_err(error)?;
                    let address = sender.ip().to_string();
                    let mut reply = sender.port().to_le_bytes().to_vec();
                    reply.push(u8::try_from(address.len()).expect("numeric IP length"));
                    reply.extend_from_slice(address.as_bytes());
                    reply.extend_from_slice(&data[..n]);
                    let length = reply.len() as u32 * 8;
                    Ok(Value::BitArray(
                        BitArray::new(reply, length).expect("whole datagram bytes"),
                    ))
                };
                let result = match select(Box::pin(work), Box::pin(lease.closed())).await {
                    Either::Left((r, _)) => r,
                    Either::Right(_) => Err(code("closed")),
                };
                Ok(lease
                    .commit(result)
                    .unwrap_or_else(|e| Err(resource_error(e))))
            }
        })
        .expect(super::ONE);
}
