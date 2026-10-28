import gleam/io
import local_math

pub fn main() {
  assert local_math.answer() == 42
  io.println("PASS:local_packages")
}
