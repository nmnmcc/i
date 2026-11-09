//// A first program in the pure core of i. It has no output yet: host
//// services, which print, come later. So `i run` is quiet when the program
//// works, and shows a stop message when a check fails.

import (i/core)

export (Greeting(..), greet, repeat)

Greeting { Greeting(name: String, times: Int) }

/// `text` `times` times, with `sep` between.
repeat = (text: String, times: Int, sep: String) -> String {
  case times {
    0 -> ""
    1 -> text
    _ -> text <> sep <> repeat(text, times - 1, sep)
  }
}

greet = (g: Greeting) { repeat("hello " <> g.name <> "!", g.times, " ") }

@main
main = () {
  text = greet(Greeting(name: "world", times: 3))
  case text == "hello world! hello world! hello world!" {
    True -> core.inspect([1, 2, 3])
    False -> panic(text)
  }
}
