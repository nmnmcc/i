//// A program built by the compiler, for the tests of the JavaScript host
//// (`bindings/javascript/test`) and of the browsers (`tests/browser`). It
//// uses the host functions of `tests/browser/fixture-abi.mjs`, and the
//// async `tick` of `app:clock/v1`.

import (i/core)

export (Console, Storage, Clock, greet, lookup, distance, wait_sum)

@host("app:console/v1")
Console { Console(print: (text: String) -> Nil) }

@host("app:storage/v1")
Storage { Storage(read: (key: String) -> String ? String) }

@host("app:clock/v1", "async")
Clock { Clock(later: (x: Float) -> Float ? String, tick: (n: Int) -> Int) }

@external("app:math/v1", "hypot")
hypot: (a: Float, b: Float) -> Float

/// A sync host export with a sync service: prints, and gives the size.
greet = (name: String, console ! Console) {
  console.print("hello " <> name)
  core.byte_size(name)
}

/// A sync host export whose service gives errors.
lookup = (key: String, storage ! Storage) { storage.read(key) }

/// A host export that calls an external function.
distance = (a: Float, b: Float) { hypot(a, b) }

/// An async host export: it waits for the host two times.
wait_sum = (a: Float, b: Float, clock ! Clock) { clock.later(a) +. clock.later(b) }

/// An async program entry.
@main
main = (console ! Console, clock ! Clock) {
  n = clock.tick(41)
  x = clock.later(1.5)
  console.print("main " <> core.inspect(n) <> " " <> core.inspect(x))
}
