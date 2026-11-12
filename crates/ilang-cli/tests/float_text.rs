//! The text form of a Float (§11.8) is i code, `lib/i/float_text.i`, which
//! the runtime calls for every inspected Float. These tests compile one
//! program at O0 and at O2 with scalar lowering, and compare its text with a
//! reference written independently in Rust: fixed presentations of the
//! specification, both sides of every limit of the long form, every power of
//! two and its neighbours, and pseudo-random bit patterns.
use ilang_host::{HostImports, HostOptions, Outcome, Program, Value};
use ilang_wasm::{Numeric, Options};
use num_bigint::BigInt;

// Fixed presentations, with hexadecimal IEEE encodings for subnormal
// endpoints and the neighbours of 1, and each side of every limit of the
// long form in spec §11.8.
const CASES: &[(u64, &str)] = &[
    (0x0000_0000_0000_0000, "0.0"),
    (0x8000_0000_0000_0000, "-0.0"),
    (0x3ff0_0000_0000_0000, "1.0"),
    (0xbff0_0000_0000_0000, "-1.0"),
    (0x3fb9_9999_9999_999a, "0.1"),
    (0x3ff1_9999_9999_999a, "1.1"),
    (0x3fef_ffff_ffff_ffff, "0.9999999999999999"),
    (0x3ff0_0000_0000_0001, "1.0000000000000002"),
    (0x0000_0000_0000_0001, "5.0e-324"),
    (0x8000_0000_0000_0001, "-5.0e-324"),
    (0x000f_ffff_ffff_ffff, "2.225073858507201e-308"),
    (0x0010_0000_0000_0000, "2.2250738585072014e-308"),
    (0x7fef_ffff_ffff_ffff, "1.7976931348623157e308"),
    (0xffef_ffff_ffff_ffff, "-1.7976931348623157e308"),
    (0x4340_0000_0000_0000, "9.007199254740992e15"),
    (0x433f_ffff_ffff_ffff, "9007199254740991.0"),
    (0x4059_0000_0000_0000, "100.0"),
    (0x408f_4000_0000_0000, "1.0e3"),
    (0x4415_af1d_78b5_8c40, "1.0e20"),
];

const DECIMAL_CASES: &[(f64, &str)] = &[
    (123.0, "123.0"),
    (12.67, "12.67"),
    (0.0123, "0.0123"),
    (3.0e26, "3.0e26"),
    (1.0e-7, "1.0e-7"),
    (0.0001, "0.0001"),
    (0.00001, "1.0e-5"),
    (0.0015, "0.0015"),
    (0.00015, "1.5e-4"),
    (-0.00123, "-0.00123"),
    (0.000123, "1.23e-4"),
    (120.0, "120.0"),
    (1200.0, "1.2e3"),
    (1230.0, "1230.0"),
    (12300.0, "1.23e4"),
    (12345.0, "12345.0"),
    (123456.0, "123456.0"),
    (1.0e10, "1.0e10"),
    (12345678901.0, "12345678901.0"),
    (123456789010.0, "123456789010.0"),
    (1234567890100.0, "1234567890100.0"),
    (12345678901000.0, "1.2345678901e13"),
    (123400000000.0, "1.234e11"),
    (10000000000.5, "10000000000.5"),
    (2251799813685248.5, "2251799813685248.5"),
    (9007199254740994.0, "9.007199254740994e15"),
    (1.5e300, "1.5e300"),
    (5.0, "5.0"),
    (-2.5e-3, "-0.0025"),
    (0.3, "0.3"),
    (2.0 / 3.0, "0.6666666666666666"),
    (100.5, "100.5"),
];

/// |digits × 10^t − value|, scaled by a positive factor that is the same
/// for every candidate of one value.
fn distance(digits: &BigInt, t: i32, value: f64) -> BigInt {
    let bits = value.abs().to_bits();
    let exponent = ((bits >> 52) & 0x7ff) as i32;
    let fraction = bits & ((1 << 52) - 1);
    let (mantissa, q) = if exponent == 0 {
        (fraction, -1074)
    } else {
        (fraction | (1 << 52), exponent - 1075)
    };
    // Multiply both by 2^-q (when q < 0) and 10^-t (when t < 0).
    let mut candidate = digits.clone();
    let mut exact = BigInt::from(mantissa);
    if t >= 0 {
        candidate *= BigInt::from(10).pow(t as u32);
    } else {
        exact *= BigInt::from(10).pow((-t) as u32);
    }
    if q >= 0 {
        exact <<= q as usize;
    } else {
        candidate <<= (-q) as usize;
    }
    let difference = candidate - exact;
    if difference.sign() == num_bigint::Sign::Minus {
        -difference
    } else {
        difference
    }
}

/// The shortest digits that read back as `value`; of those, the nearest,
/// and of two equally near, the one with an even last digit.
fn shortest(value: f64) -> (String, i32) {
    let scientific = format!("{:e}", value.abs());
    let (mantissa, exponent) = scientific.split_once('e').unwrap();
    let digits: String = mantissa.chars().filter(|c| *c != '.').collect();
    let digits = digits.trim_end_matches('0').to_owned();
    let n = digits.len() as i32;
    let k = exponent.parse::<i32>().unwrap() + 1;
    let t = k - n;
    let found: BigInt = digits.parse().unwrap();
    let mut best: Option<(BigInt, BigInt)> = None;
    for candidate in [&found - 1, found.clone(), &found + 1] {
        let text = candidate.to_string();
        if text.len() != digits.len() || text.ends_with('0') {
            continue;
        }
        if format!("{text}e{t}").parse::<f64>().unwrap() != value.abs() {
            continue;
        }
        let far = distance(&candidate, t, value);
        let better = match &best {
            None => true,
            Some((_, best_far)) => {
                far < *best_far || (far == *best_far && text.ends_with(['0', '2', '4', '6', '8']))
            }
        };
        if better {
            best = Some((candidate, far));
        }
    }
    (best.unwrap().0.to_string(), k)
}

/// The text form of spec §11.8, written independently of the runtime.
fn reference(value: f64) -> String {
    if value == 0.0 {
        return if value.is_sign_negative() {
            "-0.0"
        } else {
            "0.0"
        }
        .into();
    }
    let (digits, k) = shortest(value);
    let digits = digits.as_str();
    let n = digits.len() as i32;
    let p = k - n;
    let lower = if n == 1 { -4 } else { -2 - n };
    let upper = if n == 1 || k >= 11 { 2 } else { 1 };
    let long = p >= lower && p <= upper && !(p >= 0 && value.abs() >= 2_f64.powi(53));
    let mut out = String::from(if value < 0.0 { "-" } else { "" });
    if long {
        if k <= 0 {
            out.push_str("0.");
            out.push_str(&"0".repeat((-k) as usize));
            out.push_str(digits);
        } else if k >= n {
            out.push_str(digits);
            out.push_str(&"0".repeat((k - n) as usize));
            out.push_str(".0");
        } else {
            out.push_str(&digits[..k as usize]);
            out.push('.');
            out.push_str(&digits[k as usize..]);
        }
    } else {
        out.push_str(&digits[..1]);
        out.push('.');
        out.push_str(if n == 1 { "0" } else { &digits[1..] });
        out.push('e');
        out.push_str(&(k - 1).to_string());
    }
    out
}

fn programs() -> Vec<(u8, Program)> {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("src")).unwrap();
    std::fs::write(
        dir.path().join("i.toml"),
        "name = \"text\"\n[wasm]\nexports = [\"text/main.show\", \"text/main.nested\"]\n",
    )
    .unwrap();
    std::fs::write(
        dir.path().join("src/main.i"),
        "(i/core)\n(show, nested)\n\
         show = (x: Float) -> String { core.inspect(x) }\n\
         nested = (x: Float) -> String { core.inspect([Ok(x), Error((x, \"x\"))]) }\n",
    )
    .unwrap();
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

fn text(instance: &mut ilang_host::Instance, export: &str, value: f64) -> String {
    match instance.call(export, &[Value::Float(value)]).unwrap() {
        Outcome::Ok(Value::String(text)) => text,
        other => panic!("{value:e}: {other:?}"),
    }
}

fn check_all(values: &[(f64, String)]) {
    for (opt, program) in programs() {
        let mut instance = program
            .instantiate(HostOptions::new(HostImports::default()))
            .unwrap();
        for (value, expected) in values {
            assert_eq!(
                text(&mut instance, "text/main.show", *value),
                *expected,
                "O{opt}: {value:e} ({:#x})",
                value.to_bits()
            );
        }
    }
}

#[test]
fn reference_follows_the_examples_of_the_specification() {
    for (bits, expected) in CASES {
        assert_eq!(reference(f64::from_bits(*bits)), *expected);
    }
    for (value, expected) in DECIMAL_CASES {
        assert_eq!(reference(*value), *expected);
    }
    // 2^-25 is exactly halfway between two 17-digit candidates.
    assert_eq!(reference(2_f64.powi(-25)), "2.9802322387695312e-8");
}

#[test]
fn shortest_formatter_matches_fixed_values() {
    let mut values = Vec::new();
    for (bits, expected) in CASES {
        values.push((f64::from_bits(*bits), expected.to_string()));
    }
    for (value, expected) in DECIMAL_CASES {
        values.push((*value, expected.to_string()));
        values.push((
            -*value,
            match expected.strip_prefix('-') {
                Some(positive) => positive.to_string(),
                None => format!("-{expected}"),
            },
        ));
    }
    values.push((2_f64.powi(-25), "2.9802322387695312e-8".into()));
    check_all(&values);
}

#[test]
fn shortest_formatter_matches_the_reference_on_many_floats() {
    let mut values = Vec::new();
    for exponent in -325..=308 {
        values.push(format!("1e{exponent}").parse::<f64>().unwrap());
        values.push(format!("1.5e{exponent}").parse::<f64>().unwrap());
        values.push(
            format!("9.999999999999999e{exponent}")
                .parse::<f64>()
                .unwrap(),
        );
    }
    for exponent in -1074..=1023 {
        let power = 2_f64.powi(exponent);
        values.extend([power, power.next_up(), power.next_down()]);
    }
    let mut state = 0x9e37_79b9_7f4a_7c15_u64;
    for _ in 0..3000 {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        values.push(f64::from_bits(state));
    }
    let values: Vec<_> = values
        .into_iter()
        .filter(|value| value.is_finite())
        .map(|value| (value, reference(value)))
        .collect();
    check_all(&values);
}

#[test]
fn floats_inside_inspected_values_use_the_same_text() {
    for (opt, program) in programs() {
        let mut instance = program
            .instantiate(HostOptions::new(HostImports::default()))
            .unwrap();
        for value in [0.1, -0.0, 1.0e20, 5.0e-324, 123456.0] {
            let one = reference(value);
            assert_eq!(
                text(&mut instance, "text/main.nested", value),
                format!("[Ok({one}), Error(({one}, \"x\"))]"),
                "O{opt}"
            );
        }
    }
}
