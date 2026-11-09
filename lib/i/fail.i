//// Errors (§9.3).

export (raise, catch, from_result, map_error)

/// Ends the computation with `error`.
@external("ilang:fail", "raise")
raise: (error: e) -> Never ? e

/// Changes every error into `Error(…)`.
@external("ilang:fail", "catch")
catch: (body: () -> a ? e ! n) -> Result(a, e) ! n

/// Gives the error `x` for `Error(x)`.
from_result = (result: Result(a, e)) -> a ? e {
  case result {
    Ok(value) -> value
    Error(error) -> raise(error)
  }
}

/// Changes the errors.
map_error = (body: () -> a ? e ! n, with: (e) -> e2 ! n) -> a ? e2 ! n {
  case catch(body) {
    Ok(value) -> value
    Error(error) -> raise(with(error))
  }
}
