// spec: 9.7, 9.3, 11.7
// expect: stop "error: "no missing""
// at: spec/host_error_main:12:1

// An error of a host function that comes to the program entry stops it.
@host("test:kv/v1")
Kv { Kv(get: (key: String) -> String ? String) }

lookup = (key: String, kv ! Kv) { kv.get(key) }

@main
main = () {
  lookup("missing")
  Nil
}
