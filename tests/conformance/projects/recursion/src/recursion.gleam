import gleam/io

fn sum(n, acc) {
  case n { 0 -> acc _ -> sum(n - 1, acc + n) }
}
fn even(n) { case n { 0 -> True _ -> odd(n - 1) } }
fn odd(n) { case n { 0 -> False _ -> even(n - 1) } }
fn factorial(n) { case n { 0 -> 1 _ -> n * factorial(n - 1) } }

pub fn main() {
  assert sum(100_000, 0) == 5_000_050_000
  assert even(100_000)
  assert odd(100_001)
  assert factorial(25) == 15_511_210_043_330_985_984_000_000
  io.println("PASS:recursion")
}
