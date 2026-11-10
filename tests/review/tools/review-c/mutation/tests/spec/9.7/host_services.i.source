// spec: 9.7, 10.2, B.2, B.4
// expect: pass
// output: hello
// output: i
// output: no x
// output: Error(Nil) Error(200) Error("negative") Ok(42)
import (i/core, i/fail)

// Host services that the host gives (`ilang_host::services::testing`): every
// field is the import `label` of the module `namespace`.
@host("test:io/v1")
Io { Io(print: (text: String) -> Nil) }

@host("test:kv/v1")
Kv { Kv(get: (key: String) -> String ? String, check: (n: Int) -> Int ? Nil | Int | String) }

say = (text: String, io ! Io) { io.print(text) }

checks = (kv ! Kv) {
  [0, 200, -1, 21]
  |> each((n) { core.inspect(fail.catch(() { kv.check(n) })) })
}

each = (xs: List(Int), f: (Int) -> String) -> String {
  case xs {
    [] -> ""
    [x] -> f(x)
    [x, ..rest] -> f(x) <> " " <> each(rest, f)
  }
}

@main
main = (kv ! Kv) {
  say("hello")
  say(kv.get("name"))
  // An error of the host goes through the program as an error (§9.3).
  case fail.catch(() { kv.get("x") }) {
    Ok(_) -> panic("no error")
    Error(e) -> say(e)
  }
  say(checks())
}

// A service is a normal value (§9.2).
@test
services_are_values = (kv ! Kv) {
  get = kv.get
  case get("lang") {
    "i language" -> Nil
    other -> panic(other)
  }
}
