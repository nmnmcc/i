// §7.5: known constructors, used by `.f` and record updates (§6.7).
Shape {
  Circle(radius: Float, label: String)
  Square(side: Float, label: String)
}

// Inside a clause `C(…)`, the subject name is known to be made by `C`.
radius = (s: Shape) {
  case s {
    Circle(..) -> s.radius
    Square(..) -> 0.0
  }
}
// Also with no written type.
side = (s) {
  case s {
    Square(..) -> s.side
    _ -> 0.0
  }
}
grow = (s: Shape) {
  case s {
    Circle(..) -> Circle(..s, radius: 2.0)
    _ -> s
  }
}
// After `x = C(…)`.
grow_named = (s: Shape) {
  case s {
    c = Circle(..) -> Circle(..c, radius: c.radius *. 2.0)
    other -> other
  }
}
from_binding = (s: Shape) {
  c = Circle(..) = s
  c.radius
}
// In a function made inside the clause.
later = (s: Shape) {
  case s {
    Circle(..) -> () { s.radius }
    _ -> () { 0.0 }
  }
}
// Every constructor has `label`.
label = (s: Shape) { s.label }
// Choices with two constructors: not known.
choice = (s: Shape) {
  case s {
    Circle(..) | Square(..) -> s.label
  }
}
// Problems.
not_known = (s: Shape) { Circle(..s, radius: 1.0) }
wrong_ctor = (s: Shape) {
  case s {
    Square(..) -> Circle(..s, radius: 1.0)
    _ -> s
  }
}
no_field = (s: Shape) {
  case s {
    Square(..) -> s.radius
    _ -> 0.0
  }
}
choice_field = (s: Shape) {
  case s {
    c = Circle(..) | c = Square(..) if True -> c.radius
   _ -> panic("unreachable") }
}
