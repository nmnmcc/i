import gleam/int
import gleam/list
import gleam/order.{Eq, Gt, Lt}

pub fn negate_lt_test() -> Nil {
  assert order.negate(Lt) == Gt
  Nil
}

pub fn negate_eq_test() -> Nil {
  assert order.negate(Eq) == Eq
  Nil
}

pub fn negate_gt_test() -> Nil {
  assert order.negate(Gt) == Lt
  Nil
}

pub fn to_int_lt_test() -> Nil {
  assert order.to_int(Lt) == -1
  Nil
}

pub fn to_int_eq_test() -> Nil {
  assert order.to_int(Eq) == 0
  Nil
}

pub fn to_int_gt_test() -> Nil {
  assert order.to_int(Gt) == 1
  Nil
}

pub fn compare_lt_lt_test() -> Nil {
  assert order.compare(Lt, Lt) == Eq
  Nil
}

pub fn compare_lt_eq_test() -> Nil {
  assert order.compare(Lt, Eq) == Lt
  Nil
}

pub fn compare_lt_gt_test() -> Nil {
  assert order.compare(Lt, Gt) == Lt
  Nil
}

pub fn compare_eq_lt_test() -> Nil {
  assert order.compare(Eq, Lt) == Gt
  Nil
}

pub fn compare_eq_eq_test() -> Nil {
  assert order.compare(Eq, Eq) == Eq
  Nil
}

pub fn compare_eq_gt_test() -> Nil {
  assert order.compare(Eq, Gt) == Lt
  Nil
}

pub fn compare_gt_lt_test() -> Nil {
  assert order.compare(Gt, Lt) == Gt
  Nil
}

pub fn compare_gt_eq_test() -> Nil {
  assert order.compare(Gt, Eq) == Gt
  Nil
}

pub fn compare_gt_gt_test() -> Nil {
  assert order.compare(Gt, Gt) == Eq
  Nil
}

pub fn reverse_test() -> Nil {
  assert list.sort([4, 5, 1], by: order.reverse(int.compare)) == [5, 4, 1]
  Nil
}

pub fn break_tie_eq_lt_test() -> Nil {
  assert order.break_tie(in: Eq, with: Lt) == Lt
  Nil
}

pub fn break_tie_eq_gt_test() -> Nil {
  assert order.break_tie(in: Eq, with: Gt) == Gt
  Nil
}

pub fn break_tie_eq_eq_test() -> Nil {
  assert order.break_tie(in: Eq, with: Eq) == Eq
  Nil
}

pub fn break_tie_gt_lt_test() -> Nil {
  assert order.break_tie(in: Gt, with: Lt) == Gt
  Nil
}

pub fn break_tie_lt_gt_test() -> Nil {
  assert order.break_tie(in: Lt, with: Gt) == Lt
  Nil
}

pub fn lazy_break_tie_eq_lt_test() -> Nil {
  assert order.lazy_break_tie(in: Eq, with: fn() { Lt }) == Lt
  Nil
}

pub fn lazy_break_tie_eq_gt_test() -> Nil {
  assert order.lazy_break_tie(in: Eq, with: fn() { Gt }) == Gt
  Nil
}

pub fn lazy_break_tie_eq_eq_test() -> Nil {
  assert order.lazy_break_tie(in: Eq, with: fn() { Eq }) == Eq
  Nil
}

pub fn lazy_break_tie_gt_lazy_test() -> Nil {
  assert order.lazy_break_tie(in: Gt, with: fn() { panic }) == Gt
  Nil
}

pub fn lazy_break_tie_lt_lazy_test() -> Nil {
  assert order.lazy_break_tie(in: Lt, with: fn() { panic }) == Lt
  Nil
}
