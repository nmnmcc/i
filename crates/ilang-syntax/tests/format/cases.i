describe = (n) {
   n |> {
    0 -> "zero"
    1 -> "one"
    10 -> "ten"
    _ ? n < 0 -> "negative"
    _ ->  n % 2 |> { 0 -> "even" _ -> "odd" }
  }
}

shapes = (shape) {
   shape |> { Circle(r) -> 3 * r * r  Square(side) -> side * side
    Rectangle(width, height) -> width * height
    Polygon(points) -> {
      area = shoelace(points)
      area / 2
    }
    Triangle(base, height) -> base * height / 2


    Line | Point -> 0
  }
}

pairs = (a, b) {
   (a, b) |> {
    (Ok(x), Ok(y)) -> Ok((x, y))
    (Error(e), _) | (_, Error(e)) -> Error(e)
  }
}

typed = (v) {
   v |> {
    x: Int -> x
    s: String -> string.length(s)
    t: { (Int, Int) } -> t.0
    _ -> 0
  }
}
