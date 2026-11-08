import i/io
import i/int

pub fn main() {
  let boundary = 1_073_741_824
  assert boundary - 1 + 1 == boundary
  assert -boundary - 1 + 1 == -boundary
  let huge = 340_282_366_920_938_463_463_374_607_431_768_211_457
  assert huge * huge / huge == huge
  assert huge * huge % huge == 0
  assert -huge * huge / huge == -huge
  assert huge - huge == 0
  assert int.to_string(huge) == "340282366920938463463374607431768211457"
  assert int.parse(int.to_string(huge)) == Ok(huge)
  assert int.bitwise_shift_right(int.bitwise_shift_left(huge, 200), 200) == huge
  assert int.bitwise_not(-1) == 0
  assert int.bitwise_and(-2, 3) == 2
  io.println("PASS:bigint")
}
