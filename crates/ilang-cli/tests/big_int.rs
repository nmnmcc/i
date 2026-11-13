//! Large Int arithmetic is i code, `lib/i/big.i`, which the runtime calls
//! when an operand is not small. These tests compile one program at O0 and
//! at O2 with scalar lowering and compare `+ - * / %`, the comparisons and
//! the text form with `num-bigint`, on the edges of the small range, of 32
//! and 64 bits, and on pseudo-random values of up to 40 limbs. The bitwise
//! operations, shifts and Float conversions stay in the runtime, but they
//! use the arithmetic, so they are compared here too.
use ilang_host::{HostImports, HostOptions, Outcome, Program, Value};
use ilang_wasm::{Numeric, Options};
use num_bigint::BigInt;

const SOURCE: &str = "(\n\
\ti/core\n\
)\n\
\n\
(\n\
\tapply\n\
\tbits\n\
\tfloat_of\n\
\twhole\n\
\tsegment\n\
\tread_u\n\
\tread_i\n\
\tle72\n\
\tle72_i\n\
\tle512\n\
\tle512_u\n\
)\n\
apply = (a: Int, b: Int) -> String {\n\
\tcore.inspect(\n\
\t\t(a + b, a - b, a * b, a / b, a % b, a < b, a == b, a > b, a <= b),\n\
\t)\n\
}\n\
bits = (a: Int, b: Int, n: Int) -> String {\n\
\tcore.inspect(\n\
\t\t(\n\
\t\t\tcore.bit_and(a, b),\n\
\t\t\tcore.bit_or(a, b),\n\
\t\t\tcore.bit_xor(a, b),\n\
\t\t\tcore.bit_not(a),\n\
\t\t\tcore.shift_left(a, n),\n\
\t\t\tcore.shift_right(a, n),\n\
\t\t),\n\
\t)\n\
}\n\
float_of = (a: Int) -> Float { core.to_float(a) }\n\
whole = (x: Float) -> String { core.inspect(core.truncate(x)) }\n\
segment = (v: Int, n: Int) -> BitArray { <<v:u(n)>> }\n\
read_u = (b: BitArray, n: Int) -> Int { b |> (<<x:u(n)>>) { x } | (_) { -1 } }\n\
read_i = (b: BitArray, n: Int) -> Int { b |> (<<x:i(n)>>) { x } | (_) { -1 } }\n\
le72 = (v: Int) -> BitArray { <<v:u72le>> }\n\
le72_i = (b: BitArray) -> Int { b |> (<<x:i72le>>) { x } | (_) { -1 } }\n\
le512 = (v: Int) -> BitArray { <<v:u512le>> }\n\
le512_u = (b: BitArray) -> Int { b |> (<<x:u512le>>) { x } | (_) { -1 } }\n\
";

fn programs() -> Vec<(u8, Program)> {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("src")).unwrap();
    std::fs::write(
        dir.path().join("i.toml"),
        format!(
            "name = \"big\"\n[wasm]\nexports = {:?}\n",
            [
                "apply", "bits", "float_of", "whole", "segment", "read_u", "read_i", "le72",
                "le72_i", "le512", "le512_u"
            ]
            .map(|name| format!("big/main.{name}"))
        ),
    )
    .unwrap();
    std::fs::write(dir.path().join("src/main.i"), SOURCE).unwrap();
    let (graph, diagnostics) = ilang_package::load(dir.path());
    assert!(
        !diagnostics.iter().any(ilang_package::Diagnostic::is_error),
        "{diagnostics:?}"
    );
    let checked = ilang_check::check(&graph);
    assert!(
        !checked
            .diags
            .iter()
            .any(ilang_package::Diagnostic::is_error),
        "{:?}",
        graph.sources.render_all(&checked.diags)
    );
    let program = checked.program.as_ref().unwrap();
    let typed = checked.types.as_ref().unwrap();
    [(0, Numeric::Reference), (2, Numeric::Scalars)]
        .into_iter()
        .map(|(opt, numeric)| {
            let (wasm, _) = ilang_wasm::compile_with_bindings_numeric(
                &graph,
                program,
                typed,
                Options::default().numeric(numeric),
            )
            .unwrap();
            let wasm = ilang_cli::optimize(&wasm, ilang_cli::level(opt).unwrap()).unwrap();
            (opt, Program::from_bytes(&wasm).unwrap())
        })
        .collect()
}

/// The text of §11.8 for the tuple of `apply`, worked out with `num-bigint`.
fn expected(a: &BigInt, b: &BigInt) -> String {
    let zero = BigInt::from(0);
    let (q, r) = if *b == zero {
        (zero.clone(), zero.clone())
    } else {
        // Truncating division; the remainder has the sign of `a`.
        let q = a / b;
        let r = a - &q * b;
        (q, r)
    };
    let bool_text = |b: bool| if b { "True" } else { "False" };
    format!(
        "({}, {}, {}, {q}, {r}, {}, {}, {}, {})",
        a + b,
        a - b,
        a * b,
        bool_text(a < b),
        bool_text(a == b),
        bool_text(a > b),
        bool_text(a <= b),
    )
}

fn values() -> Vec<BigInt> {
    let mut values: Vec<BigInt> = [
        "0",
        "1",
        "-1",
        "1073741823",
        "1073741824",
        "-1073741824",
        "-1073741825",
        "4294967295",
        "4294967296",
        "16383",
        "16384",
        "268435456",
        "9223372036854775807",
        "9223372036854775808",
        "-9223372036854775808",
        "18446744073709551616",
        "340282366920938463463374607431768211455",
        "-340282366920938463463374607431768211456",
        "100000000000000000000",
        "99999999999999999999999999999999999999",
        // The boundaries of the former runtime-alone JavaScript BigInt test.
        "10",
        "-10",
        "31",
        "-33",
        "36",
        "-36",
        "-4294967295",
        "2147483647",
        "2147483648",
        "-1073741823",
    ]
    .iter()
    .map(|t| t.parse().unwrap())
    .collect();
    let mut state = 0x2545_f491_4f6c_dd1d_u64;
    let mut next = || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    for _ in 0..60 {
        let limbs = (next() % 40 + 1) as usize;
        let mut v = BigInt::from(0);
        for _ in 0..limbs {
            // Runs of all-ones and zero limbs stress carries and estimates.
            let limb = match next() % 4 {
                0 => u32::MAX,
                1 => 0,
                _ => next() as u32,
            };
            v = (v << 32) + BigInt::from(limb);
        }
        if next() % 2 == 0 {
            v = -v;
        }
        values.push(v);
    }
    values
}

#[test]
fn large_int_arithmetic_matches_num_bigint() {
    let values = values();
    let mut pairs = Vec::new();
    for (i, a) in values.iter().enumerate() {
        for (j, b) in values.iter().enumerate() {
            // Every fixed pair, and a sample of the random ones.
            if i < 20 || j < 20 || (i * 7 + j) % 5 == 0 {
                pairs.push((a.clone(), b.clone()));
            }
        }
        // Exact multiples and near multiples divide with no remainder or
        // with a small one.
        if let Some(b) = values.get(i + 1)
            && *b != BigInt::from(0)
        {
            let product = a * b;
            let step = BigInt::from(if *b > BigInt::from(0) { 1 } else { -1 });
            pairs.push((product.clone(), b.clone()));
            pairs.push((&product - step, b.clone()));
        }
    }
    // Indexed reads cross 30-bit digit and 32-bit storage boundaries. Include
    // the measured 8500-bit size and long quotients, with both signs.
    for width in [100_usize, 200, 300, 479, 480, 481, 959, 960, 961, 8500] {
        let a = (BigInt::from(1) << width) - 1_u8;
        for b in [
            BigInt::from(1073741823),
            (BigInt::from(1) << (width / 2)) + 123_u8,
            &a - 37_u8,
        ] {
            for (x, y) in [(&a, &b), (&(-&a), &b), (&a, &(-&b))] {
                pairs.push((x.clone(), y.clone()));
            }
        }
    }
    // Karatsuba must also handle an all-zero lower half without copying
    // or accidentally reading the adjacent range of the original number.
    for width in [480_usize, 510, 960, 990, 8500] {
        let power = BigInt::from(1) << width;
        pairs.push((power.clone(), &power - 1_u8));
        pairs.push((&power - 1_u8, power.clone()));
        pairs.push((power.clone(), &power + 1_u8));
        pairs.push((-&power, &power - 1_u8));
    }
    for (opt, program) in programs() {
        let mut instance = program
            .instantiate(HostOptions::new(HostImports::default()))
            .unwrap();
        for (a, b) in &pairs {
            let got = match instance
                .call(
                    "big/main.apply",
                    &[Value::Int(a.clone()), Value::Int(b.clone())],
                )
                .unwrap()
            {
                Outcome::Ok(Value::String(text)) => text,
                other => panic!("level {opt} {a} {b}: {other:?}"),
            };
            assert_eq!(got, expected(a, b), "level {opt}: a = {a}, b = {b}");
        }
    }
}

fn text(instance: &mut ilang_host::Instance, export: &str, args: &[Value]) -> String {
    match instance.call(export, args).unwrap() {
        Outcome::Ok(Value::String(text)) => text,
        other => panic!("{export} {args:?}: {other:?}"),
    }
}

#[test]
fn bitwise_operations_shifts_and_floats_match_num_bigint() {
    let values = values();
    for (opt, program) in programs() {
        let mut instance = program
            .instantiate(HostOptions::new(HostImports::default()))
            .unwrap();
        for (i, a) in values.iter().enumerate() {
            for (j, b) in values.iter().enumerate() {
                if !(i < 20 || j < 20 || (i * 3 + j) % 7 == 0) {
                    continue;
                }
                let n = ((i * 31 + j * 17) % 300) as u32;
                let expected = format!(
                    "({}, {}, {}, {}, {}, {})",
                    a & b,
                    a | b,
                    a ^ b,
                    !a.clone(),
                    a << n,
                    a >> n
                );
                let got = text(
                    &mut instance,
                    "big/main.bits",
                    &[
                        Value::Int(a.clone()),
                        Value::Int(b.clone()),
                        Value::Int(BigInt::from(n)),
                    ],
                );
                assert_eq!(got, expected, "level {opt}: bits of {a}, {b}, {n}");
            }
            // The nearest Float, read from the exact decimal text.
            if a.bits() < 1024 {
                let expected: f64 = a.to_string().parse().unwrap();
                match instance
                    .call("big/main.float_of", &[Value::Int(a.clone())])
                    .unwrap()
                {
                    Outcome::Ok(Value::Float(got)) => {
                        assert_eq!(
                            got.to_bits(),
                            expected.to_bits(),
                            "level {opt}: to_float {a}"
                        )
                    }
                    other => panic!("level {opt}: to_float {a}: {other:?}"),
                }
            }
        }
        for x in [
            0.0,
            1.999,
            -1.999,
            2f64.powi(30),
            -(2f64.powi(30)),
            2f64.powi(53) + 2.0,
            2f64.powi(63),
            -(2f64.powi(63)),
            1e100,
            -1e100,
            f64::MIN_POSITIVE,
            f64::MAX,
            -f64::MAX,
        ] {
            let expected = if x.trunc() == 0.0 {
                "0".to_string()
            } else {
                format!("{:.0}", x.trunc())
            };
            assert_eq!(
                text(&mut instance, "big/main.whole", &[Value::Float(x)]),
                expected,
                "level {opt}: truncate {x}"
            );
        }
    }
}

/// The `width` low bits of `value` in two's complement, as a number from 0.
fn unsigned(value: &BigInt, width: usize) -> BigInt {
    let modulus = BigInt::from(1) << width;
    ((value % &modulus) + &modulus) % &modulus
}

/// The bytes of a segment: big endian with the last byte filled from its
/// highest bit, or little endian for a multiple of 8.
fn encoding(value: &BigInt, width: usize, little: bool) -> Vec<u8> {
    let length = width.div_ceil(8);
    let u = unsigned(value, width);
    let mut bytes = if little {
        u.to_bytes_le().1
    } else {
        (u << ((8 - width % 8) % 8)).to_bytes_be().1
    };
    if bytes == [0] {
        bytes.clear();
    }
    if little {
        bytes.resize(length, 0);
    } else {
        let mut padded = vec![0; length - bytes.len()];
        padded.extend(bytes);
        bytes = padded;
    }
    bytes
}

fn int_of(value: Value) -> BigInt {
    match value {
        Value::Int(n) => n,
        other => panic!("not an Int: {other:?}"),
    }
}

#[test]
fn wide_integer_segments_match_their_encoding() {
    let mut cases: Vec<(BigInt, usize)> = Vec::new();
    for width in [64_usize, 65, 100, 127, 128, 129, 255, 512] {
        let boundary: BigInt = BigInt::from(1) << width;
        for value in [
            BigInt::from(0),
            BigInt::from(1),
            BigInt::from(-1),
            &boundary - 1_u8,
            -(&boundary - 1_u8),
            boundary.clone(),
            -&boundary,
            BigInt::from(1) << (width - 1),
        ] {
            cases.push((value, width));
        }
    }
    for value in values() {
        cases.push((value.clone(), 64 + (value.bits() as usize % 449)));
    }
    for (opt, program) in programs() {
        let mut instance = program
            .instantiate(HostOptions::new(HostImports::default()))
            .unwrap();
        let mut run = |export: &str, args: Vec<Value>| match instance.call(export, &args).unwrap() {
            Outcome::Ok(value) => value,
            other => panic!("level {opt} {export} {args:?}: {other:?}"),
        };
        for (value, width) in &cases {
            let n = Value::Int(BigInt::from(*width));
            let bytes = encoding(value, *width, false);
            match run(
                "big/main.segment",
                vec![Value::Int(value.clone()), n.clone()],
            ) {
                Value::BitArray(b) => {
                    assert_eq!(
                        (b.bytes, b.bit_length as usize),
                        (bytes.clone(), *width),
                        "level {opt}: u({width}) of {value}"
                    )
                }
                other => panic!("{other:?}"),
            }
            let bits = Value::BitArray(ilang_host::BitArray::new(bytes, *width as u32).unwrap());
            assert_eq!(
                int_of(run("big/main.read_u", vec![bits.clone(), n.clone()])),
                unsigned(value, *width),
                "level {opt}: read u({width}) of {value}"
            );
            let half = BigInt::from(1) << (width - 1);
            let u = unsigned(value, *width);
            let signed = if u >= half {
                u - (BigInt::from(1) << *width)
            } else {
                u
            };
            assert_eq!(
                int_of(run("big/main.read_i", vec![bits, n])),
                signed,
                "level {opt}: read i({width}) of {value}"
            );
        }
        for value in values() {
            for (width, write, read, signed) in [
                (72, "big/main.le72", "big/main.le72_i", true),
                (512, "big/main.le512", "big/main.le512_u", false),
            ] {
                let bytes = encoding(&value, width, true);
                match run(write, vec![Value::Int(value.clone())]) {
                    Value::BitArray(b) => {
                        assert_eq!(b.bytes, bytes, "level {opt}: le{width} of {value}")
                    }
                    other => panic!("{other:?}"),
                }
                let bits = Value::BitArray(ilang_host::BitArray::new(bytes, width as u32).unwrap());
                let u = unsigned(&value, width);
                let expected = if signed && u >= BigInt::from(1) << (width - 1) {
                    u - (BigInt::from(1) << width)
                } else {
                    u
                };
                assert_eq!(
                    int_of(run(read, vec![bits])),
                    expected,
                    "level {opt}: read le{width} of {value}"
                );
            }
        }
    }
}
