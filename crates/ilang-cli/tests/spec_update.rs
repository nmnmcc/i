#[path = "common/spec_header.rs"]
mod spec_header;

#[test]
fn regeneration_preserves_crlf_and_every_byte_after_the_header() {
    // A1 §8: keep CRLF and the bare CR in the program; only stop/at lines change.
    let header = "// spec: 11.7\r\n// expect: stop \"old\"\r\n// at: spec/main:5:1\r\n// output: unchanged\r\n";
    let body = "@main\r\nmain = () { 1\r+ 2  Nil }\r\n// at: this is outside the header\r\n";
    let source = format!("{header}{body}");
    let out = spec_header::rewrite(&source, "//", "new", &["spec/main:6:2".into()]).unwrap();
    assert!(out.ends_with(body));
    assert_eq!(
        out.bytes().filter(|b| *b == b'\r').count(),
        source.bytes().filter(|b| *b == b'\r').count()
    );
    assert_eq!(
        out,
        format!(
            "// spec: 11.7\r\n// expect: stop \"new\"\r\n// at: spec/main:6:2\r\n// output: unchanged\r\n{body}"
        )
    );
}

#[test]
fn regeneration_rejects_trimmed_values_and_changed_frame_counts() {
    // A1 §8 requires a failure rather than silently trimming actual values.
    let source = "# spec: 11.7\n# expect: stop \"old\"\n# at: spec/main:1:1\nname = \"spec\"\n";
    for value in [" new", "new ", "new\nline", "new\rline"] {
        assert!(spec_header::rewrite(source, "#", value, &["spec/main:2:1".into()]).is_err());
    }
    assert!(spec_header::rewrite(source, "#", "new", &[]).is_err());
    assert!(
        spec_header::rewrite(
            source,
            "#",
            "new",
            &["spec/main:2:1".into(), "spec/main:3:1".into()]
        )
        .is_err()
    );
}
