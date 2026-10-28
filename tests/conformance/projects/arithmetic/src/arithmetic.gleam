import gleam/io

const answer = 42

pub fn main() {
  assert answer == 0x2A
  assert 0b101010 == 0o52
  assert 7 / 2 == 3
  assert -7 / 2 == -3
  assert -7 % 2 == -1
  assert 7 % -2 == 1
  assert 123 / 0 == 0
  assert 123 % 0 == 0
  assert 1.0 /. 0.0 == 0.0
  assert 2.5 *. 4.0 == 10.0
  assert 3.0 -. 1.5 == 1.5
  assert !False && True
  assert { False && { panic as "short circuit" } } == False
  assert True || { panic as "short circuit" }
  io.println("PASS:arithmetic")
}
