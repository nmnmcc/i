// spec: 9.1, 9.3
// expect: pass
import (i/core, i/fail)

check = (got: String, want: String) {
  case got == want {
    True -> Nil
    False -> panic(got <> " is not " <> want)
  }
}

show = (x) { core.inspect(x) }

@test
raise_and_catch = () {
  check(show(fail.catch(() { 1 })), "Ok(1)")
  check(show(fail.catch(() { fail.raise("e") })), "Error(\"e\")")
  check(show(fail.catch(() { 1 + fail.raise(2) })), "Error(2)")
  // The error goes out through the calls until it is taken.
  inner = (n) { case n { 0 -> fail.raise("bottom")  _ -> n + inner(n - 1) } }
  check(show(fail.catch(() { inner(100) })), "Error(\"bottom\")")
  // Nested: the inner `catch` takes the error.
  r = fail.catch(() {
    a = fail.catch(() { fail.raise(1) })
    (a, 2)
  })
  check(show(r), "Ok((Error(1), 2))")
  // Raised again from a handler.
  again = fail.catch(() {
    case fail.catch(() { fail.raise("x") }) {
      Ok(_) -> "no"
      Error(e) -> fail.raise(e <> "y")
    }
  })
  check(show(again), "Error(\"xy\")")
}

@test
from_result = () {
  check(show(fail.catch(() { fail.from_result(Ok(3)) })), "Ok(3)")
  check(show(fail.catch(() { fail.from_result(Error("no")) })), "Error(\"no\")")
}

@test
map_error = () {
  m = fail.catch(() { fail.map_error(() { fail.raise(2) }, (e) { e * 10 }) })
  check(show(m), "Error(20)")
  ok = fail.catch(() { fail.map_error(() { 5 }, (e) { e }) })
  check(show(ok), "Ok(5)")
}

// §9.1: retrying is a normal function.
retry = (times: Int, body: () -> a ? e ! n) -> a ? e ! n {
  case fail.catch(body) {
    Ok(value) -> value
    Error(error) -> case times > 1 {
      True -> retry(times - 1, body)
      False -> fail.raise(error)
    }
  }
}

@test
retrying = () {
  check(show(fail.catch(() { retry(3, () { fail.raise("down") }) })), "Error(\"down\")")
  check(show(fail.catch(() { retry(3, () { "up" }) })), "Ok(\"up\")")
}

// The example of §9.3, with `http_get` and `parse_user` given by a service.
User { User(name: String) }
HttpError { HttpError(status: Int) }
ParseError { ParseError(at: Int) }
Http { Http(get: (id: String) -> String ? HttpError) }

default_user = User("default")

http_get = (id: String, http ! Http) { http.get(id) }

parse_user = (text: String) {
  case text {
    "user " <> name -> User(name)
    _ -> fail.raise(ParseError(0))
  }
}

load = (id) { parse_user(http_get(id)) }

safe = (id) {
  case fail.catch(() { load(id) }) {
    Ok(user) -> user
    Error(_: ParseError) -> default_user
    Error(other) -> fail.raise(other)
  }
}

fake = Http((id) {
  case id {
    "1" -> "user ann"
    "2" -> "garbage"
    _ -> fail.raise(HttpError(404))
  }
})

@test
safe_example = () {
  ! Http = fake
  check(show(fail.catch(() { safe("1") })), "Ok(User(\"ann\"))")
  check(show(fail.catch(() { safe("2") })), "Ok(User(\"default\"))")
  check(show(fail.catch(() { safe("3") })), "Error(HttpError(404))")
}

// A deep error goes out through many frames, and the frames are gone.
@test
deep_unwinding = () {
  down = (n) { case n { 0 -> fail.raise(n)  _ -> 1 + down(n - 1) } }
  each = (k) {
    case k {
      0 -> Nil
      _ -> {
        check(show(fail.catch(() { down(1000) })), "Error(0)")
        each(k - 1)
      }
    }
  }
  each(1000)
}
