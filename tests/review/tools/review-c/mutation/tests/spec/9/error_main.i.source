// spec: 9.3, 10.2, 11.7
// expect: stop "error: HttpError(500, 2)"
// at: spec/error_main:13:1
import (i/fail)

HttpError { HttpError(Int, Int) }

get = (n) {
  case n { 0 -> fail.raise(HttpError(500, 2))  _ -> get(n - 1) + 1 }
}

@main
main = () {
  get(10)
}
