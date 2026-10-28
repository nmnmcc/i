import gleam/io
import gleam/bit_array

pub fn main() {
  let unaligned = <<5:3, 17:5, 7:3>>
  let assert <<a:3, b:5, c:3>> = unaligned
  assert #(a, b, c) == #(5, 17, 7)
  assert bit_array.bit_size(unaligned) == 11
  let signed = <<-300:16-little, 70_000:32-big>>
  let assert <<negative:16-little-signed, positive:32-big>> = signed
  assert #(negative, positive) == #(-300, 70_000)
  let size = 5
  let assert <<n:size(size), tail:bits>> = <<19:5, 3:2>>
  assert n == 19
  assert tail == <<3:2>>
  let assert <<value:float-32-little>> = <<1.5:float-32-little>>
  assert value == 1.5
  let assert <<half:float-16>> = <<1.5:float-16>>
  assert half == 1.5
  assert <<"你好":utf8>> == <<228, 189, 160, 229, 165, 189>>
  io.println("PASS:bit_arrays")
}
