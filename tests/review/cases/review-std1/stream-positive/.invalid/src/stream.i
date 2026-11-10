// old-syntax-check: E0101
import (i/fail, std/list)
export (Stream, Step(..), new, wrap, make, pull, map, append, attempt, map_environment, to_list)
Stream(a, e, n) { Stream(() -> Step(a, e, n) ? e ! n, (n) -> Nil) }
Step(a, e, n) { Done Next(a, Stream(a, e, n)) }
make = (step: () -> Step(a, e, n) ? e ! n) -> Stream(a, e, n) { Stream(step, (_: n) { Nil }) }
pull = (stream: Stream(a, e, n)) -> Step(a, e, n) ? e ! n {
 Stream(step, _) = stream
 step()
}
new = () -> Stream(a, Never, n) { Stream(() -> Step(a, Never, n) ! n { Done }, (_: n) { Nil }) }
wrap = (item: a) -> Stream(a, Never, n) { Stream(() -> Step(a, Never, n) ! n { Next(item, new()) }, (_: n) { Nil }) }
map = (over: Stream(a, e, n), with: (a) -> b ? f ! n) -> Stream(b, e | f, n) {
 make(() {
  case pull(over) {
   Done -> Done
   Next(x, tail) -> Next(with(x), map(tail, with))
  }
 })
}
append = (first: Stream(a, e, n), second: Stream(a, f, n)) -> Stream(a, e | f, n) {
 make(() {
  case pull(first) {
   Done -> pull(second)
   Next(x, tail) -> Next(x, append(tail, second))
  }
 })
}
attempt = (stream: Stream(a, e, n)) -> Stream(Result(a, e), Never, n) {
 make(() {
  case fail.catch(() { pull(stream) }) {
   Ok(Done) -> Done
   Ok(Next(x, tail)) -> Next(Ok(x), attempt(tail))
   Error(error) -> Next(Error(error), new())
  }
 })
}
map_environment = (stream: Stream(a, e, n), run: { (body: () -> Step(a, e, n) ? e ! n) -> Step(a, e, n) ? e ! m }) -> Stream(a, e, m) {
 make(() {
  case run(() { pull(stream) }) {
   Done -> Done
   Next(x, tail) -> Next(x, map_environment(tail, run))
  }
 })
}
collect = (stream: Stream(a, e, n), acc: List(a)) -> List(a) ? e ! n {
 case pull(stream) {
  Done -> list.reverse(acc)
  Next(x, tail) -> collect(tail, [x, ..acc])
 }
}
to_list = (stream: Stream(a, e, n)) -> List(a) ? e ! n { collect(stream, []) }
