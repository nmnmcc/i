// spec: 8.3, 11.7
// expect: stop "bit array: length is negative"
// at: spec/stop_negative:11:7
// at: spec/stop_negative:17:3

make = (n: Int) {
  b = <<1:u1>>
  c = <<0:u7>>
  b2 = <<b:bits, c:bits>>
  // The position of `<<`.
  x = <<1:u(n)>>
  x
}

@main
main = () {
  make(-3)
  Nil
}
