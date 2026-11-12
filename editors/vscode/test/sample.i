// old-syntax-check: E0101,E0101,E0003,E0001,E0102,E0001,E0001,E0101,E0101
//// The module documentation.

(
  i/fail
  decode = json/decode
  opt = (Option, Some) = std/option
)

(main, User(..), Shape)

/// A user.
User { User(name: String, age: Int) }

Shape(A) = (A, A) -> A | Nil

@host("console")
Console { Console(print: (String) -> Nil) }

@external("env", "now")
now: () -> Float

@deprecated("use add")
plus = (a, b) { a + b }

/// Every kind of number.
numbers = () {
  ints = [0, 1_000, 0xFF_ff, 0o17, 0b1010_0101]
  floats = [1.0, 2., 3.5e10, 6.25e-3, 1_0.0_1]
  t = ((1, 2), 3)
  t.0.1 + ints.0
}

strings = "tab\t quote\" slash\\ \u{1F600} bad\q
over two lines"

floats = (x: Float, y) -> Float ^ fail.Fail ! Nil {
  z = -.x *. y +. 1.5 /. 2.0 -. 0.5
  z <. 1.0 || z >=. 2.0 && z <=. 3.0 || z >. 4.0 |> {
    True -> z
    False -> 0.0
  }
}

main = (io ! Console) {
  bits = <<1, 2: u16be, s: utf8, f: f64le, rest: bits>>
  [first, ..rest] = [1, 2, 3]
  user = User(..base, name: "x")
  label = user.name <> "!"
  n = first |> {
    0 | 1 -> 1
    n ? n > 10 -> n % 3
    _ -> __ignored
  }
  { 1 |> plus(2) } == 3 != False
  opt.map(Some(1), (x) { x * 2 })
}

test("adds", () { assert(plus(1, 2) == 3) })

identity(A) = (x: A) { x }
import = (x) { x }
export = import
case = export
if = case
