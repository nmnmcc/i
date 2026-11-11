use crate::{HostImports, Value};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub(super) fn register(imports: &mut HostImports) {
    let origin = Instant::now();
    imports
        .async_service("i:clock/v1", "monotonic_ns", move |_| {
            let elapsed = origin.elapsed().as_nanos();
            async move { Ok(Ok(Value::Int(elapsed.into()))) }
        })
        .expect(super::ONE);
    imports
        .async_service("i:clock/v1", "unix_ns", |_| {
            let now = SystemTime::now();
            async move {
                let nanos = match now.duration_since(UNIX_EPOCH) {
                    Ok(d) => crate::BigInt::from(d.as_nanos()),
                    Err(e) => -crate::BigInt::from(e.duration().as_nanos()),
                };
                Ok(Ok(Value::Int(nanos)))
            }
        })
        .expect(super::ONE);
    imports
        .async_service("i:clock/v1", "sleep_ns", |args| {
            let [Value::Int(n)] = args else {
                panic!("clock sleep_ns takes an Int")
            };
            let deadline = if n < &0.into() {
                Err("invalid")
            } else {
                u128::try_from(n)
                    .ok()
                    .and_then(|n| {
                        let seconds = u64::try_from(n / 1_000_000_000).ok()?;
                        Instant::now()
                            .checked_add(Duration::new(seconds, (n % 1_000_000_000) as u32))
                    })
                    .ok_or("range")
            };
            async move {
                match deadline {
                    Ok(deadline) => {
                        tokio::time::sleep_until(tokio::time::Instant::from_std(deadline)).await;
                        Ok(Ok(Value::Nil))
                    }
                    Err(code) => Ok(Err(Value::String(code.into()))),
                }
            }
        })
        .expect(super::ONE);
}
