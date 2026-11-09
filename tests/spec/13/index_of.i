// spec: 13
// expect: pass
import (i/core)

check = (got: Result(Int, Nil), want: Result(Int, Nil)) {
  case got == want {
    True -> Nil
    False -> panic(core.inspect(got) <> " is not " <> core.inspect(want))
  }
}

@test
found = () {
  check(core.index_of("hello", "l", 0), Ok(2))
  check(core.index_of("hello", "l", 3), Ok(3))
  check(core.index_of("hello", "lo", 0), Ok(3))
  check(core.index_of("hello", "hello", 0), Ok(0))
  check(core.index_of(in: "abcabcabd", find: "abcabd", from: 0), Ok(3))
  check(core.index_of("aaaaaaab", "aaab", 1), Ok(4))
}

@test
not_found = () {
  check(core.index_of("hello", "l", 4), Error(Nil))
  check(core.index_of("hello", "hello!", 0), Error(Nil))
  check(core.index_of("hello", "x", 0), Error(Nil))
  check(core.index_of("", "a", 0), Error(Nil))
}

@test
empty_find = () {
  check(core.index_of("hello", "", 0), Ok(0))
  check(core.index_of("hello", "", 2), Ok(2))
  check(core.index_of("", "", 0), Ok(0))
}

@test
from_at_the_end = () {
  check(core.index_of("hello", "", 5), Ok(5))
  check(core.index_of("hello", "o", 5), Error(Nil))
}

@test
from_outside_the_string = () {
  check(core.index_of("hello", "", 6), Error(Nil))
  check(core.index_of("hello", "h", -1), Error(Nil))
  check(core.index_of("hello", "", 100000000000000000000), Error(Nil))
}

@test
from_inside_a_character = () {
  // é is bytes 1 and 2.
  check(core.index_of("héé", "é", 0), Ok(1))
  check(core.index_of("héé", "é", 2), Error(Nil))
  check(core.index_of("héé", "", 2), Error(Nil))
  check(core.index_of("héé", "é", 3), Ok(3))
  check(core.index_of("😀😀", "😀", 1), Error(Nil))
  check(core.index_of("😀😀", "😀", 4), Ok(4))
}
