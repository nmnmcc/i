//! TLS transport for the standard host.
use crate::{BitArray, HostImports, ResourceContext, Value};
use futures_util::future::{Either, select};
use std::{
    io::Cursor,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, ReadHalf, WriteHalf},
    net::TcpStream,
    sync::Mutex,
};
use tokio_rustls::{
    TlsAcceptor, TlsConnector, TlsStream,
    rustls::{
        self,
        pki_types::{CertificateDer, PrivateKeyDer, ServerName},
    },
};

const NS: &str = "i:tls/v1";
const STREAM: &str = "tls stream";
struct Stream {
    read: Mutex<ReadHalf<TlsStream<TcpStream>>>,
    write: Mutex<WriteHalf<TlsStream<TcpStream>>>,
    shutdown: AtomicBool,
    alpn: String,
}
fn err(stage: &str, reason: impl std::fmt::Display) -> Value {
    Value::String(format!("{stage}:{reason}"))
}
fn resource_error(e: crate::resources::ResourceError) -> Value {
    err(
        "io",
        if e == crate::resources::ResourceError::Busy {
            "busy"
        } else {
            "closed"
        },
    )
}
fn certs(pem: &[u8]) -> Result<Vec<CertificateDer<'static>>, Value> {
    rustls_pemfile::certs(&mut Cursor::new(pem))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| err("certificate", e))
}
fn key(pem: &[u8]) -> Result<PrivateKeyDer<'static>, Value> {
    rustls_pemfile::private_key(&mut Cursor::new(pem))
        .map_err(|e| err("certificate", e))?
        .ok_or_else(|| err("certificate", "missing private key"))
}
fn protocols(s: &str) -> Vec<Vec<u8>> {
    if s.is_empty() {
        vec![]
    } else {
        s.split('\0').map(|p| p.as_bytes().to_vec()).collect()
    }
}
fn insert(
    ctx: &ResourceContext,
    stream: TlsStream<TcpStream>,
) -> anyhow::Result<Result<Value, Value>> {
    let alpn = match &stream {
        TlsStream::Client(s) => s.get_ref().1.alpn_protocol(),
        TlsStream::Server(s) => s.get_ref().1.alpn_protocol(),
    }
    .map(|p| String::from_utf8_lossy(p).into_owned())
    .unwrap_or_default();
    let (read, write) = tokio::io::split(stream);
    match ctx.insert(
        STREAM,
        Stream {
            read: Mutex::new(read),
            write: Mutex::new(write),
            shutdown: AtomicBool::new(false),
            alpn,
        },
        |_, done| drop(done),
    ) {
        Ok(id) => Ok(Ok(Value::Int(id))),
        Err(crate::resources::ResourceError::Memory) => {
            Err(crate::resources::ResourceError::Memory.into())
        }
        Err(e) => Ok(Err(resource_error(e))),
    }
}
fn handshake_error(e: std::io::Error) -> Value {
    let stage = if e
        .get_ref()
        .and_then(|e| e.downcast_ref::<rustls::Error>())
        .is_some_and(|e| {
            matches!(
                e,
                rustls::Error::InvalidCertificate(_) | rustls::Error::NoCertificatesPresented
            )
        }) {
        "certificate"
    } else {
        "handshake"
    };
    err(stage, e)
}
pub(super) fn register(imports: &mut HostImports) {
    imports
        .context_async_service(NS, "connect", |ctx, args| {
            let [
                Value::Int(id),
                Value::String(name),
                Value::String(alpn),
                Value::BitArray(roots),
                Value::BitArray(cert),
                Value::BitArray(private),
            ] = args
            else {
                panic!("tls connect arguments")
            };
            let (id, name, alpn, roots, cert, private) = (
                id.clone(),
                name.clone(),
                alpn.clone(),
                roots.clone(),
                cert.clone(),
                private.clone(),
            );
            async move {
                let config = (|| -> Result<_, Value> {
                    let mut store = rustls::RootCertStore::empty();
                    let native = rustls_native_certs::load_native_certs();
                    if !native.errors.is_empty() {
                        return Err(err("certificate", format!("{:?}", native.errors)));
                    }
                    for c in native.certs {
                        store.add(c).map_err(|e| err("certificate", e))?;
                    }
                    for c in certs(&roots.bytes)? {
                        store.add(c).map_err(|e| err("certificate", e))?;
                    }
                    let builder = rustls::ClientConfig::builder().with_root_certificates(store);
                    let mut config = if cert.bytes.is_empty() && private.bytes.is_empty() {
                        builder.with_no_client_auth()
                    } else {
                        builder
                            .with_client_auth_cert(certs(&cert.bytes)?, key(&private.bytes)?)
                            .map_err(|e| err("certificate", e))?
                    };
                    config.alpn_protocols = protocols(&alpn);
                    let name = ServerName::try_from(name).map_err(|e| err("certificate", e))?;
                    Ok((config, name))
                })();
                let (config, name) = match config {
                    Ok(v) => v,
                    Err(e) => return Ok(Err(e)),
                };
                let stream = match super::net::take_stream(&ctx, &id) {
                    Ok(s) => s,
                    Err(e) => return Ok(Err(resource_error(e))),
                };
                match TlsConnector::from(Arc::new(config))
                    .connect(name, stream)
                    .await
                {
                    Ok(s) => insert(&ctx, TlsStream::Client(s)),
                    Err(e) => Ok(Err(handshake_error(e))),
                }
            }
        })
        .expect(super::ONE);
    imports
        .context_async_service(NS, "accept", |ctx, args| {
            let [
                Value::Int(id),
                Value::BitArray(cert),
                Value::BitArray(private),
                Value::String(alpn),
            ] = args
            else {
                panic!("tls accept arguments")
            };
            let (id, cert, private, alpn) =
                (id.clone(), cert.clone(), private.clone(), alpn.clone());
            async move {
                let config = (|| -> Result<_, Value> {
                    let mut config = rustls::ServerConfig::builder()
                        .with_no_client_auth()
                        .with_single_cert(certs(&cert.bytes)?, key(&private.bytes)?)
                        .map_err(|e| err("certificate", e))?;
                    config.alpn_protocols = protocols(&alpn);
                    Ok(config)
                })();
                let config = match config {
                    Ok(v) => v,
                    Err(e) => return Ok(Err(e)),
                };
                let stream = match super::net::take_stream(&ctx, &id) {
                    Ok(s) => s,
                    Err(e) => return Ok(Err(resource_error(e))),
                };
                match TlsAcceptor::from(Arc::new(config)).accept(stream).await {
                    Ok(s) => insert(&ctx, TlsStream::Server(s)),
                    Err(e) => Ok(Err(handshake_error(e))),
                }
            }
        })
        .expect(super::ONE);
    imports
        .context_async_service(NS, "alpn", |ctx, args| {
            let [Value::Int(id)] = args else {
                panic!("tls alpn arguments")
            };
            let id = id.clone();
            async move {
                let lease = match ctx.acquire::<Stream>(&id, STREAM, "alpn") {
                    Ok(l) => l,
                    Err(e) => return Ok(Err(resource_error(e))),
                };
                let value = Value::String(lease.resource().alpn.clone());
                Ok(lease
                    .commit(Ok(value))
                    .unwrap_or_else(|e| Err(resource_error(e))))
            }
        })
        .expect(super::ONE);
    imports
        .context_async_service(NS, "read", |ctx, args| {
            let [Value::Int(id), Value::Int(n)] = args else {
                panic!("tls read arguments")
            };
            let (id, n) = (id.clone(), n.clone());
            async move {
                if n <= 0.into() {
                    return Ok(Err(err("io", "invalid")));
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
                        .map_err(|e| err("io", e))
                };
                let result = match select(Box::pin(work), Box::pin(lease.closed())).await {
                    Either::Left((r, _)) => r,
                    Either::Right(_) => Err(err("io", "closed")),
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
                panic!("tls write arguments")
            };
            let (id, bytes) = (id.clone(), bytes.clone());
            async move {
                if bytes.bit_length % 8 != 0 {
                    return Ok(Err(err("io", "invalid")));
                }
                let lease = match ctx.acquire::<Stream>(&id, STREAM, "write") {
                    Ok(l) => l,
                    Err(e) => return Ok(Err(resource_error(e))),
                };
                let stream = Arc::clone(lease.resource());
                let work = async move {
                    if stream.shutdown.load(Ordering::SeqCst) {
                        return Err(err("io", "closed"));
                    }
                    stream
                        .write
                        .lock()
                        .await
                        .write_all(&bytes.bytes)
                        .await
                        .map(|_| Value::Nil)
                        .map_err(|e| err("io", e))
                };
                let result = match select(Box::pin(work), Box::pin(lease.closed())).await {
                    Either::Left((r, _)) => r,
                    Either::Right(_) => Err(err("io", "closed")),
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
                panic!("tls shutdown arguments")
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
                        .map_err(|e| err("io", e))
                };
                let result = match select(Box::pin(work), Box::pin(lease.closed())).await {
                    Either::Left((r, _)) => r,
                    Either::Right(_) => Err(err("io", "closed")),
                };
                Ok(lease
                    .commit(result)
                    .unwrap_or_else(|e| Err(resource_error(e))))
            }
        })
        .expect(super::ONE);
    imports
        .context_async_service(NS, "close", |ctx, args| {
            let [Value::Int(id)] = args else {
                panic!("tls close arguments")
            };
            let receipt = ctx.close(id, STREAM);
            async move {
                receipt.await;
                Ok(Ok(Value::Nil))
            }
        })
        .expect(super::ONE);
}
