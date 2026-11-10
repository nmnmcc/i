// spec: 4.8, 7.1
// expect: pass

// Rule 11: a constructor pattern with `..` counts as having a catch-all at
// the places of the fields that it leaves out. So the guard on the first
// clause does not leave `age` and `name` with no values.

User { User(name: String, age: Int) }

kind = (u: User) {
  case u {
    User(age, ..) if age > 10 -> 1
    User(..) -> 2
  }
}

label = (u: User) {
  case u {
    User(age, name) if age > 10 -> name
    User(name, ..) -> "young " <> name
  }
}

@test
spread_is_a_catch_all = () {
  case (kind(User("a", 20)), kind(User("b", 5))) {
    (1, 2) -> Nil
    _ -> panic("kind")
  }
  case (label(User("a", 20)), label(User("b", 5))) {
    ("a", "young b") -> Nil
    _ -> panic("label")
  }
}
