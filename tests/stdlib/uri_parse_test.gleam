import gleam/option.{None, Some}
import gleam/uri

pub fn absolute_components_test() -> Nil {
  assert uri.parse("HTTPS://weebl:bob@EXAMPLE.com:1234/a/b?x=1#fragment")
    == Ok(uri.Uri(
      scheme: Some("https"),
      userinfo: Some("weebl:bob"),
      host: Some("EXAMPLE.com"),
      port: Some(1234),
      path: "/a/b",
      query: Some("x=1"),
      fragment: Some("fragment"),
    ))
}

pub fn relative_path_is_not_scheme_test() -> Nil {
  assert uri.parse("a/b?x#y")
    == Ok(uri.Uri(
      scheme: None,
      userinfo: None,
      host: None,
      port: None,
      path: "a/b",
      query: Some("x"),
      fragment: Some("y"),
    ))
}

pub fn colon_after_first_path_segment_test() -> Nil {
  assert uri.parse("a/b:c") == Ok(uri.Uri(..uri.empty, path: "a/b:c"))
  assert uri.parse("/a:b") == Ok(uri.Uri(..uri.empty, path: "/a:b"))
}

pub fn scheme_without_authority_test() -> Nil {
  assert uri.parse("mailto:friend@example.com")
    == Ok(
      uri.Uri(..uri.empty, scheme: Some("mailto"), path: "friend@example.com"),
    )
  assert uri.parse("a+1.-:path")
    == Ok(uri.Uri(..uri.empty, scheme: Some("a+1.-"), path: "path"))
}

pub fn empty_components_remain_present_test() -> Nil {
  assert uri.parse("") == Ok(uri.empty)
  assert uri.parse("?#")
    == Ok(uri.Uri(..uri.empty, query: Some(""), fragment: Some("")))
  assert uri.parse("//:?#")
    == Ok(
      uri.Uri(..uri.empty, host: Some(""), query: Some(""), fragment: Some("")),
    )
}

pub fn empty_query_before_fragment_test() -> Nil {
  let assert Ok(parsed) = uri.parse("https://example.com/path?#fragment")
  assert parsed.query == Some("")
  assert parsed.fragment == Some("fragment")
  assert uri.to_string(parsed) == "https://example.com/path?#fragment"
}

pub fn file_uri_and_empty_authority_test() -> Nil {
  assert uri.parse("file:///one/two")
    == Ok(
      uri.Uri(
        ..uri.empty,
        scheme: Some("file"),
        host: Some(""),
        path: "/one/two",
      ),
    )
  assert uri.parse("//") == Ok(uri.Uri(..uri.empty, host: Some("")))
}

pub fn ipv6_host_roundtrip_test() -> Nil {
  let assert Ok(parsed) = uri.parse("https://[::1]:8443/path?x#y")
  assert parsed.host == Some("::1")
  assert parsed.port == Some(8443)
  assert parsed.path == "/path"
  assert uri.to_string(parsed) == "https://[::1]:8443/path?x#y"
}

pub fn ipv6_full_and_mapped_address_test() -> Nil {
  let assert Ok(full) = uri.parse("//[2001:0db8:0000:0000:0000:0000:1428:07ab]")
  assert full.host == Some("2001:0db8:0000:0000:0000:0000:1428:07ab")
  let assert Ok(mapped) = uri.parse("//[::ffff:192.0.2.1]/")
  assert mapped.host == Some("::ffff:192.0.2.1")
  assert uri.to_string(mapped) == "//[::ffff:192.0.2.1]/"
}

pub fn invalid_ip_literal_test() -> Nil {
  assert uri.parse("http://[::1") == Error(Nil)
  assert uri.parse("http://[::1]suffix") == Error(Nil)
  assert uri.parse("http://[::G]") == Error(Nil)
  assert uri.parse("http://[1:2:3:4:5:6:7:8:9]") == Error(Nil)
  assert uri.parse("http://[::ffff:999.0.0.1]") == Error(Nil)
}

pub fn invalid_scheme_test() -> Nil {
  assert uri.parse("1http://example.com") == Error(Nil)
  assert uri.parse("+http://example.com") == Error(Nil)
  assert uri.parse(":path") == Error(Nil)
}

pub fn invalid_port_test() -> Nil {
  assert uri.parse("http://example.com:-1/") == Error(Nil)
  assert uri.parse("http://example.com:12x/") == Error(Nil)
  assert uri.parse("http://example.com:1:2/") == Error(Nil)
}

pub fn port_is_an_integer_not_a_socket_validation_test() -> Nil {
  let assert Ok(parsed) = uri.parse("//example.com:123456/")
  assert parsed.port == Some(123_456)
  assert uri.parse("//example.com:/")
    == Ok(uri.Uri(..uri.empty, host: Some("example.com"), path: "/"))
}

pub fn uri_components_require_encoded_characters_test() -> Nil {
  assert uri.parse("https://example.com/a b") == Error(Nil)
  assert uri.parse("https://example.com/örebro") == Error(Nil)
  assert uri.parse("https://example.com/a\nb") == Error(Nil)
  assert uri.parse("https://example.com/?x=\t") == Error(Nil)
  let assert Ok(encoded) = uri.parse("https://example.com/%C3%B6rebro?x=a%20b")
  assert encoded.path == "/%C3%B6rebro"
  assert encoded.query == Some("x=a%20b")
}

pub fn query_can_contain_question_marks_test() -> Nil {
  let assert Ok(parsed) =
    uri.parse("ldap:///dc=example,dc=com??sub?(givenName=John)")
  assert parsed.query == Some("?sub?(givenName=John)")
}

pub fn path_segments_resolve_dot_segments_test() -> Nil {
  assert uri.path_segments("/a//./b/../c/") == ["a", "c"]
  assert uri.path_segments("../../a") == ["a"]
  assert uri.path_segments("/") == []
}
