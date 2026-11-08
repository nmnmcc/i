//// Compatibility assertion helpers for migrated upstream tests.
//// Test discovery and failure reporting are provided by `i test`.

import i/option.{type Option, None, Some}

pub fn equal(actual: value, expected: value) -> Nil {
  assert actual == expected
}

pub fn not_equal(actual: value, unexpected: value) -> Nil {
  assert actual != unexpected
}

pub fn be_ok(value: Result(value, error)) -> value {
  let assert Ok(value) = value
  value
}

pub fn be_error(value: Result(value, error)) -> error {
  let assert Error(error) = value
  error
}

pub fn be_some(value: Option(value)) -> value {
  let assert Some(value) = value
  value
}

pub fn be_none(value: Option(value)) -> Nil {
  let assert None = value
  Nil
}

pub fn be_true(actual: Bool) -> Nil {
  assert actual
}

pub fn be_false(actual: Bool) -> Nil {
  assert !actual
}

pub fn fail() -> Nil {
  panic as "Explicit test failure"
}
