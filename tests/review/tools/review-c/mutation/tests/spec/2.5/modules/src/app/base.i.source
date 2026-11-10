export (start, Color(..), describe)

Color { Red Green Blue(level: Int) }

start = 40

describe = (c: Color) {
  case c {
    Red -> "red"
    Green -> "green"
    Blue(level) -> "blue " <> core_inspect(level)
  }
}

core_inspect = (n: Int) {
  case n { 0 -> "0"  _ -> "many" }
}
