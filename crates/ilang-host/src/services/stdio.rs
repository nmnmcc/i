use crate::{BitArray, HostImports, Value};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::Mutex;

fn error(e: std::io::Error) -> Value {
    let code = match e.kind() {
        std::io::ErrorKind::PermissionDenied => "denied",
        std::io::ErrorKind::Interrupted => "interrupted",
        std::io::ErrorKind::BrokenPipe => "closed",
        _ => "io",
    };
    Value::String(code.into())
}

pub(super) fn register(imports: &mut HostImports) {
    let input = Arc::new(Mutex::new(tokio::io::stdin()));
    imports
        .async_service("i:stdio/v1", "read", move |args| {
            let [Value::Int(n)] = args else {
                panic!("stdio read takes an Int")
            };
            let n = if n <= &0.into() {
                None
            } else if n > &65536.into() {
                Some(65536)
            } else {
                Some(usize::try_from(n).expect("positive bounded size"))
            };
            let input = input.clone();
            async move {
                let Some(n) = n else {
                    return Ok(Err(Value::String("invalid".into())));
                };
                let Ok(mut input) = input.try_lock() else {
                    return Ok(Err(Value::String("busy".into())));
                };
                let mut bytes = vec![0; n];
                Ok(match input.read(&mut bytes).await {
                    Ok(n) => {
                        bytes.truncate(n);
                        Ok(Value::BitArray(
                            BitArray::new(bytes, (n * 8) as u32).expect("whole bytes"),
                        ))
                    }
                    Err(e) => Err(error(e)),
                })
            }
        })
        .expect(super::ONE);
    let stdout = Arc::new(Mutex::new(tokio::io::stdout()));
    let stderr = Arc::new(Mutex::new(tokio::io::stderr()));
    macro_rules! stream {
        ($stream:expr, $write:literal, $flush:literal) => {{
            let stream = $stream.clone();
            imports
                .async_service("i:stdio/v1", $write, move |args| {
                    let [Value::BitArray(bits)] = args else {
                        panic!("stdio write takes a BitArray")
                    };
                    let bits = bits.clone();
                    let stream = stream.clone();
                    async move {
                        if bits.bit_length % 8 != 0 {
                            return Ok(Err(Value::String("invalid".into())));
                        }
                        let Ok(mut stream) = stream.try_lock() else {
                            return Ok(Err(Value::String("busy".into())));
                        };
                        Ok(stream
                            .write_all(&bits.bytes)
                            .await
                            .map(|()| Value::Nil)
                            .map_err(error))
                    }
                })
                .expect(super::ONE);
            let stream = $stream.clone();
            imports
                .async_service("i:stdio/v1", $flush, move |_| {
                    let stream = stream.clone();
                    async move {
                        let Ok(mut stream) = stream.try_lock() else {
                            return Ok(Err(Value::String("busy".into())));
                        };
                        Ok(stream.flush().await.map(|()| Value::Nil).map_err(error))
                    }
                })
                .expect(super::ONE);
        }};
    }
    stream!(stdout, "write", "flush");
    stream!(stderr, "write_error", "flush_error");
}
