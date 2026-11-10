// spec: 9.4, 9.2, 9.3, 9.5, 11.6
// expect: pass
import (i/assertion, i/control, i/core, i/fail, i/resource)

check = (got: String, want: String) {
  case got == want {
    True -> Nil
    False -> panic(got <> " is not " <> want)
  }
}

show = (x) { core.inspect(x) }

length = (xs, n) {
  case xs {
    [] -> n
    [_, ..rest] -> length(rest, n + 1)
  }
}

// The example of §9.4.
Yield(a) { Yield(emit: (value: a) -> Nil) }

collect = (body: () -> Nil ! Yield(a) | n) -> List(a) ! n {
  control.boundary((b) {
    ! Yield(a) = Yield((x) { control.suspend(b, (k) { [x, ..control.resume(k, Nil)] }) })
    body()
    []
  })
}

emit = (x, y ! Yield(a)) { y.emit(x) }

count = (from, to) {
  case from > to {
    True -> Nil
    False -> {
      emit(from)
      count(from + 1, to)
    }
  }
}

@test
generator = () {
  check(show(collect(() { emit(1)  emit(2)  emit(3) })), "[1, 2, 3]")
  check(show(collect(() { Nil })), "[]")
  check(show(collect(() { count(1, 5) })), "[1, 2, 3, 4, 5]")
  // Generators inside generators.
  nested = collect(() {
    emit(collect(() { emit("a")  emit("b") }))
    emit(collect(() { emit("c") }))
  })
  check(show(nested), "[[\"a\", \"b\"], [\"c\"]]")
}

// §9.4 rules 2 and 3: `f(k)` gives the value of the boundary, and the
// value that the resumed computation has at the boundary is the value of
// `resume`.
@test
values = () {
  r = control.boundary((b) {
    x = control.suspend(b, (k) { control.resume(k, 10) * 2 })
    x + 1
  })
  check(show(r), "22")
  // `f` need not resume.
  check(show(control.boundary((b) { control.suspend(b, (k) { 5 }) + 1 })), "5")
  // Without a suspend, the value of the body.
  check(show(control.boundary((b) { 7 })), "7")
}

// After `resume`, the computation may suspend again: each time, `f` is done
// in the place of the boundary, which is now the place of `resume`.
@test
again = () {
  r = control.boundary((b) {
    a = control.suspend(b, (k) { control.resume(k, 1) + 100 })
    c = control.suspend(b, (k) { control.resume(k, a + 1) + 1000 })
    d = control.suspend(b, (k) { control.resume(k, c + 1) + 10000 })
    a + c + d
  })
  check(show(r), "11106")
}

// A suspend to an outer boundary goes through the inner one, which is
// paused with it and active again after `resume`.
@test
nested_boundaries = () {
  r = control.boundary((outer) {
    v = control.boundary((inner) {
      x = control.suspend(outer, (k) { control.resume(k, 5) * 10 })
      y = control.suspend(inner, (k) { control.resume(k, x + 1) + 1 })
      x + y
    })
    v + 100
  })
  check(show(r), "1120")
  // The inner boundary takes its own suspends.
  s = control.boundary((outer) {
    control.boundary((inner) { control.suspend(inner, (k) { 3 }) }) + 1
  })
  check(show(s), "4")
}

// §9.4 rule 4: `cancel` gives `Nil`.
@test
cancel = () {
  r = control.boundary((b) {
    x = control.suspend(b, (k) { show(control.cancel(k)) <> "!" })
    "not " <> x
  })
  check(r, "Nil!")
}

// `cancel` runs the final functions of the paused computation (§9.5). Here
// the final function suspends to an outer boundary, which shows that it
// runs, and goes on when it is resumed.
@test
cancel_runs_final_functions = () {
  r = control.boundary((outer) {
    control.boundary((b) {
      resource.ensure(
        () { control.suspend(b, (k) { control.cancel(k)  "cancelled" }) },
        () { control.suspend(outer, (k) { "final, " <> control.resume(k, Nil) }) },
      )
    })
  })
  check(r, "final, cancelled")
  // Without `cancel`, the final function runs when the resumed body ends.
  s = control.boundary((outer) {
    control.boundary((b) {
      resource.ensure(
        () { control.suspend(b, (k) { control.resume(k, "resumed") }) },
        () { control.suspend(outer, (k) { "final, " <> control.resume(k, Nil) }) },
      )
    })
  })
  check(s, "final, resumed")
}

// §9.4 rule 3: the paused part keeps the environment statements that were
// inside it; for the rest, it gets the environment of the place of `resume`.
Name { Name(text: String) }

name = (n ! Name) { n.text }

deep = (b) {
  ! Name = Name("deep")
  control.suspend(b, (k) {
    ! Name = Name("resumer")
    control.resume(k, Nil)
  })
  name()
}

@test
environment = () {
  ! Name = Name("outside")
  r = control.boundary((b) {
    before = name()
    inner = {
      ! Name = Name("inside")
      control.suspend(b, (k) {
        ! Name = Name("resumer")
        control.resume(k, Nil)
      })
      name()
    }
    (before, inner, name())
  })
  check(show(r), "(\"outside\", \"inside\", \"resumer\")")
  s = control.boundary((b) { (deep(b), name()) })
  check(show(s), "(\"deep\", \"resumer\")")
  // `f(k)` gets the environment of the place of the boundary.
  t = control.boundary((b) {
    ! Name = Name("inside")
    control.suspend(b, (k) { name() })
  })
  check(t, "outside")
}

// A paused `fail.catch` takes the errors raised after `resume`, and an
// error that no paused handler takes goes out of `resume`.
@test
errors = () {
  r = control.boundary((b) {
    fail.catch(() {
      x = control.suspend(b, (k) { control.resume(k, 1) })
      fail.raise(x + 1)
    })
  })
  check(show(r), "Error(2)")
  e = fail.catch(() {
    control.boundary((b) {
      x = control.suspend(b, (k) { control.resume(k, 3) + 1 })
      fail.raise(x)
    })
  })
  check(show(e), "Error(3)")
}

// A generator that is pulled one value at a time: the continuation is a
// value, and it is resumed later from other places.
Step { Done  More(Int, Continuation(Nil, Step, Never, Never)) }

numbers = (from, to) {
  control.boundary((b) {
    each = (n) {
      case n > to {
        True -> Done
        False -> {
          control.suspend(b, (k) { More(n, k) })
          each(n + 1)
        }
      }
    }
    each(from)
  })
}

take = (step, n, acc) {
  case n {
    0 -> acc
    _ -> case step {
      Done -> acc
      More(v, k) -> take(control.resume(k, Nil), n - 1, [v, ..acc])
    }
  }
}

sum = (step, acc) {
  case step {
    Done -> acc
    More(v, k) -> sum(control.resume(k, Nil), acc + v)
  }
}

@test
pulled = () {
  check(show(take(numbers(1, 10), 3, [])), "[3, 2, 1]")
  check(show(take(numbers(1, 2), 5, [])), "[2, 1]")
  // The last continuation is dropped, or cancelled.
  case numbers(1, 10) {
    More(_, k) -> check(show(control.cancel(k)), "Nil")
    Done -> panic("no value")
  }
}

loop = (n, acc) {
  case n {
    0 -> acc
    _ -> loop(n - 1, acc + 1)
  }
}

// Tail calls stay tail calls inside a boundary, and in a resumed
// computation (§11.6). A million calls do not fit on the stack.
@test
tail_calls = () {
  check(show(control.boundary((b) { loop(1000000, 0) })), "1000000")
  r = control.boundary((b) {
    x = control.suspend(b, (k) { control.resume(k, 1000000) })
    loop(x, 0)
  })
  check(show(r), "1000000")
}

// Deep generators: 100000 values. The generator of §9.4 needs one frame of
// `f` for each value; a pulled generator needs none.
@test
deep_generators = () {
  check(show(length(collect(() { count(1, 100000) }), 0)), "100000")
  check(show(sum(numbers(1, 100000), 0)), "5000050000")
}

// Suspends in the clauses of `case`, in guards, and on the right of `&&`:
// the rest of the function goes on after each of them.
@test
in_branches = () {
  r = control.boundary((b) {
    x = case 1 {
      1 -> control.suspend(b, (k) { control.resume(k, 10) })
      _ -> 0
    }
    y = case x {
      n if control.suspend(b, (k) { control.resume(k, n == 10) }) -> {
        z = control.suspend(b, (k) { control.resume(k, 20) })
        z + 1
      }
      _ -> 0
    }
    w = x == 10 && control.suspend(b, (k) { control.resume(k, True) })
    (x, y, w)
  })
  check(show(r), "(10, 21, True)")
  // Closures made before a suspend, and functions that call themselves.
  s = control.boundary((b) {
    add = (n) { n + 1 }
    fact = (n) {
      case n {
        0 -> 1
        _ -> n * fact(n - 1)
      }
    }
    x = control.suspend(b, (k) { control.resume(k, 4) })
    add(fact(x))
  })
  check(show(s), "25")
}

// Frames that are rebuilt by `resume` may be paused again: code, `fail.catch`,
// `resource.ensure`, and an inner boundary.
@test
paused_again = () {
  a = control.boundary((b) {
    fail.catch(() {
      x = control.suspend(b, (k) { control.resume(k, 1) })
      y = control.suspend(b, (k) { control.resume(k, x + 1) })
      fail.raise(x + y)
    })
  })
  check(show(a), "Error(3)")
  e = control.boundary((b) {
    resource.ensure(() {
      x = control.suspend(b, (k) { control.resume(k, 1) })
      y = control.suspend(b, (k) { control.resume(k, x + 1) })
      x + y
    }, () { Nil })
  })
  check(show(e), "3")
  n = control.boundary((outer) {
    control.boundary((inner) {
      x = control.suspend(outer, (k) { control.resume(k, 1) + 100 })
      y = control.suspend(outer, (k) { control.resume(k, x + 1) + 1000 })
      z = control.suspend(inner, (k) { control.resume(k, y + 1) + 10000 })
      x + y + z
    })
  })
  check(show(n), "11106")
}

// A producer that is not tail recursive: the paused part has many frames.
walk = (n) {
  case n {
    0 -> Nil
    _ -> {
      walk(n - 1)
      emit(n)
      walk(n - 1)
    }
  }
}

@test
deep_paused_part = () {
  check(show(collect(() { walk(3) })), "[1, 2, 1, 3, 1, 2, 1]")
  check(show(length(collect(() { walk(12) }), 0)), "4095")
}

// `cancel` unwinds through an inner boundary, and an error goes out
// through one.
@test
through_inner_boundaries = () {
  r = control.boundary((outer) {
    control.boundary((inner) { control.suspend(outer, (k) { control.cancel(k)  5 }) })
  })
  check(show(r), "5")
  e = control.boundary((outer) {
    fail.catch(() {
      control.boundary((inner) {
        x = control.suspend(outer, (k) { control.resume(k, 7) })
        fail.raise(x)
      })
    })
  })
  check(show(e), "Error(7)")
}

// `assertion.catch` is written with a boundary (§9.6): a generator inside
// it suspends through it, and its failures still go to it after `resume`.
@test
assertions_in_generators = () {
  r = collect(() {
    a = assertion.catch(() {
      emit(1)
      assert(False)
    })
    emit(show(a) == "Error(AssertionFailure(\"assert\"))")
  })
  check(show(r), "[1, True]")
  c = assertion.catch(() { collect(() { emit(1)  assert(False)  emit(2) }) })
  check(show(c), "Error(AssertionFailure(\"assert\"))")
}
