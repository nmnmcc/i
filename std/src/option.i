//// Optional values: `Some(value)` when there is a value, and `None` when
//// there is not.
////
//// In `std`, a function that may find nothing gives an `Option`, for
//// example `list.first` and `dict.get`. A function that may fail, such as
//// `int.parse`, gives a `Result`.

export (
  Option(..)
  is_some
  is_none
  map
  then
  flatten
  unwrap
  lazy_unwrap
  or
  lazy_or
  to_result
  from_result
  all
  values
  filter
  map2
)

/// A value that may be there (`Some`) or not (`None`).
Option(a) {
  Some(a)
  None
}

/// `True` when the option is `Some`.
is_some = (option: Option(a)) -> Bool {
  case option {
    Some(_) -> True
    None -> False
  }
}

/// `True` when the option is `None`.
is_none = (option: Option(a)) -> Bool {
  case option {
    Some(_) -> False
    None -> True
  }
}

/// Changes the value in a `Some` with a function. `None` stays `None`.
///
/// ```i
/// option.map(Some(1), (x) { x + 1 })  // Some(2)
/// ```
map = (over: Option(a), with: (a) -> b ? e ! n) -> Option(b) ? e ! n {
  case over {
    Some(value) -> Some(with(value))
    None -> None
  }
}

/// Calls a function that gives an option on the value in a `Some`. `None`
/// stays `None`. This puts together steps that may each find nothing.
///
/// ```i
/// option.then(Some(1), (x) { Some(x + 1) })  // Some(2)
/// ```
then = (option: Option(a), apply: (a) -> Option(b) ? e ! n) -> Option(b) ? e ! n {
  case option {
    Some(value) -> apply(value)
    None -> None
  }
}

/// Takes away one level of `Option`.
flatten = (option: Option(Option(a))) -> Option(a) {
  case option {
    Some(inner) -> inner
    None -> None
  }
}

/// The value in a `Some`, or `default` for `None`.
unwrap = (option: Option(a), or: a) -> a {
  case option {
    Some(value) -> value
    None -> or
  }
}

/// The value in a `Some`, or the value of `default()` for `None`.
lazy_unwrap = (option: Option(a), or: () -> a ? e ! n) -> a ? e ! n {
  case option {
    Some(value) -> value
    None -> or()
  }
}

/// The first option when it is `Some`, and the second when not.
or = (first: Option(a), second: Option(a)) -> Option(a) {
  case first {
    Some(_) -> first
    None -> second
  }
}

/// The first option when it is `Some`, and the value of `second()` when not.
lazy_or = (first: Option(a), second: () -> Option(a) ? e ! n) -> Option(a) ? e ! n {
  case first {
    Some(_) -> first
    None -> second()
  }
}

/// `Ok` with the value of a `Some`, or `Error(error)` for `None`.
to_result = (option: Option(a), error: e) -> Result(a, e) {
  case option {
    Some(value) -> Ok(value)
    None -> Error(error)
  }
}

/// `Some` with the value of an `Ok`, or `None` for an `Error`.
from_result = (result: Result(a, e)) -> Option(a) {
  case result {
    Ok(value) -> Some(value)
    Error(_) -> None
  }
}

/// `Some` with all the values when every option is `Some`, and `None` when
/// one of them is `None`.
///
/// ```i
/// option.all([Some(1), Some(2)])  // Some([1, 2])
/// option.all([Some(1), None])     // None
/// ```
all = (options: List(Option(a))) -> Option(List(a)) {
  go = (rest, acc) {
    case rest {
      [] -> Some(reverse(acc))
      [Some(x), ..xs] -> go(xs, [x, ..acc])
      [None, ..] -> None
    }
  }
  go(options, [])
}

/// The values of the `Some` options, in order.
///
/// ```i
/// option.values([Some(1), None, Some(3)])  // [1, 3]
/// ```
values = (options: List(Option(a))) -> List(a) {
  go = (rest, acc) {
    case rest {
      [] -> reverse(acc)
      [Some(x), ..xs] -> go(xs, [x, ..acc])
      [None, ..xs] -> go(xs, acc)
    }
  }
  go(options, [])
}

/// Keeps a `Some` only when its value passes the test.
filter = (option: Option(a), keeping: (a) -> Bool ? e ! n) -> Option(a) ? e ! n {
  case option {
    Some(value) -> case keeping(value) {
      True -> option
      False -> None
    }
    None -> None
  }
}

/// Puts the values of two options together when both are `Some`.
map2 = (a: Option(a), b: Option(b), with: (a, b) -> c ? e ! n) -> Option(c) ? e ! n {
  case (a, b) {
    (Some(x), Some(y)) -> Some(with(x, y))
    _ -> None
  }
}

reverse = (list) {
  go = (rest, acc) {
    case rest {
      [] -> acc
      [x, ..xs] -> go(xs, [x, ..acc])
    }
  }
  go(list, [])
}
