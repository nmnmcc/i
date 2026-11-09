//! Host services that come with the host (spec §9.7, §10.1).
//!
//! - [`standard`] is what `i run` and `i test` give every program: the
//!   standard service of spec §9.7, `Io` of the library module `i/io`,
//!   which a program imports like this:
//!
//!   ```text
//!   import ((Io) = i/io)
//!   ```
//!
//!   `print` writes the text and a line feed. Another embedder gives it like
//!   any host service, for example with this function.
//! - [`testing`] is what the spec suite gives: services and external
//!   functions under `test:` namespaces, which keep a [`Log`].
//!
//! A host gives only the functions that the metadata of a program lists
//! (B.6), so a program that does not declare a service does not need it. To
//! put a fake in the place of a host service in a test, i code uses an
//! environment statement (§9.7); a Rust host gives its own [`HostImports`].

use crate::{BigInt, HostImports, Value};
use anyhow::bail;
use std::sync::{Arc, Mutex};

const ONE: &str = "each function is given once";

/// The namespace of the standard service `Io` of `i/io`.
pub const IO: &str = "i:io/v1";

/// The standard services, which print with `print`.
pub fn standard(print: impl Fn(&str) + Send + Sync + 'static) -> HostImports {
    let mut imports = HostImports::default();
    imports
        .service(IO, "print", move |arguments| {
            let [Value::String(text)] = arguments else {
                bail!("{IO} print takes one String");
            };
            print(text);
            Ok(Ok(Value::Nil))
        })
        .expect("a new set of imports");
    imports
}

/// The standard services, which print to the standard output.
pub fn stdout() -> HostImports {
    standard(|text| {
        use std::io::Write;
        let mut out = std::io::stdout().lock();
        let _ = writeln!(out, "{text}");
        let _ = out.flush();
    })
}

/// What the services of [`testing`] were given, in order.
pub type Log = Arc<Mutex<Vec<String>>>;

/// The services and external functions of the spec suite:
///
/// | Namespace | Function | |
/// |---|---|---|
/// | `test:io/v1` | `print: (text: String) -> Nil` | logs `text` |
/// | `test:math/v1` | external `add: (a: Int, b: Int) -> Int` | |
/// | `test:math/v1` | external `hypot: (a: Float, b: Float) -> Float` | |
/// | `test:kv/v1` | `get: (key: String) -> String ? String` | `name` is `"i"`, `lang` is `"i language"`; other keys give the error `"no <key>"` |
/// | `test:kv/v1` | `check: (n: Int) -> Int ? Nil \| Int \| String` | `2 * n`; the error `Nil` for 0, `n` for more than 100, and `"negative"` for less than 0 |
/// | `test:async/v1`, async | `double: (n: Int) -> Int ? String` | `2 * n`, or the error `"negative"` for less than 0; logs `double <n>` |
/// | `test:async/v1`, async | `echo: (text: String) -> String` | logs `echo <text>` |
/// | `test:async/v1`, async | `nan: () -> Float` | a Float that is not finite |
/// | `test:bad/v1` | `nan: () -> Float` | a Float that is not finite |
/// | `test:bad/v1` | `bits: () -> BitArray` | 1 bit with bits after the end that are not 0 |
pub fn testing(log: Log) -> HostImports {
    let mut imports = HostImports::default();
    let printed = Arc::clone(&log);
    imports
        .service("test:io/v1", "print", move |arguments| {
            let [Value::String(text)] = arguments else {
                bail!("print takes one String");
            };
            printed.lock().unwrap().push(text.clone());
            Ok(Ok(Value::Nil))
        })
        .expect(ONE);
    imports
        .external("test:math/v1", "add", |arguments| {
            let [Value::Int(a), Value::Int(b)] = arguments else {
                bail!("add takes two Ints");
            };
            Ok(Value::Int(a + b))
        })
        .expect(ONE);
    imports
        .external("test:math/v1", "hypot", |arguments| {
            let [Value::Float(a), Value::Float(b)] = arguments else {
                bail!("hypot takes two Floats");
            };
            Ok(Value::Float(a.hypot(*b)))
        })
        .expect(ONE);
    imports
        .service("test:kv/v1", "get", |arguments| {
            let [Value::String(key)] = arguments else {
                bail!("get takes one String");
            };
            Ok(match key.as_str() {
                "name" => Ok(Value::String("i".into())),
                "lang" => Ok(Value::String("i language".into())),
                _ => Err(Value::String(format!("no {key}"))),
            })
        })
        .expect(ONE);
    imports
        .service("test:kv/v1", "check", |arguments| {
            let [Value::Int(n)] = arguments else {
                bail!("check takes one Int");
            };
            Ok(if *n < BigInt::from(0) {
                Err(Value::String("negative".into()))
            } else if *n == BigInt::from(0) {
                Err(Value::Nil)
            } else if *n > BigInt::from(100) {
                Err(Value::Int(n.clone()))
            } else {
                Ok(Value::Int(n * 2))
            })
        })
        .expect(ONE);
    let doubled = Arc::clone(&log);
    imports
        .async_service("test:async/v1", "double", move |arguments| {
            let n = match arguments {
                [Value::Int(n)] => Some(n.clone()),
                _ => None,
            };
            if let Some(n) = &n {
                doubled.lock().unwrap().push(format!("double {n}"));
            }
            async move {
                let Some(n) = n else {
                    bail!("double takes one Int");
                };
                Ok(if n < BigInt::from(0) {
                    Err(Value::String("negative".into()))
                } else {
                    Ok(Value::Int(n * 2))
                })
            }
        })
        .expect(ONE);
    let echoed = Arc::clone(&log);
    imports
        .async_service("test:async/v1", "echo", move |arguments| {
            let text = match arguments {
                [Value::String(text)] => Some(text.clone()),
                _ => None,
            };
            if let Some(text) = &text {
                echoed.lock().unwrap().push(format!("echo {text}"));
            }
            async move {
                match text {
                    Some(text) => Ok(Ok(Value::String(text))),
                    None => bail!("echo takes one String"),
                }
            }
        })
        .expect(ONE);
    imports
        .async_service("test:async/v1", "nan", |_| async {
            Ok(Ok(Value::Float(f64::NAN)))
        })
        .expect(ONE);
    imports
        .service("test:bad/v1", "nan", |_| Ok(Ok(Value::Float(f64::NAN))))
        .expect(ONE);
    imports
        .service("test:bad/v1", "bits", |_| {
            Ok(Ok(Value::BitArray(crate::BitArray {
                bytes: vec![0xff],
                bit_length: 1,
            })))
        })
        .expect(ONE);
    imports
}
