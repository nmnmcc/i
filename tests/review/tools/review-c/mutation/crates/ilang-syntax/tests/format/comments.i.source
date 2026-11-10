// A comment at the top of the file.

import (i/core) // the core

/// The documentation of a type.
/// It has two lines.
Shape {
  Circle(radius: Float) // round
  Square(side: Float)   // four sides
  // A shape with no area.
  Point
}

// A comment before the documentation.
/// The documentation of `area`.
@deprecated("use size")
// A comment after an attribute.
area = (shape) { // after the brace
  // the first statement
  x = 1 // one
  yy = 22 // twenty-two

  // after a blank line
  zzz = x + // in the middle of an operation
    yy
  call(x, // after the first argument
    yy)
  list = [
    // before the first part
    1
    2 // two
    // after the last part
  ]
  y = case shape {
    // before a clause
    Circle(r) -> r *. r // area of a circle
    Square(s) -> s *. s // area of a square
    Point -> 0.0
    // after the last clause
  }
  y
  // at the end of the block
}

empty = (x) {
  case x {
    // nothing here
  }
}

// At the end of the file.
