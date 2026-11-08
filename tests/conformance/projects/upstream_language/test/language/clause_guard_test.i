// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 The Gleam contributors

import importable

// Constructor functions are used rather than literals to stop the Erlang
// compiler being clever and complaining about the guards always having the
// same result

fn true() {
  True
}

fn false() {
  False
}

fn make_int_zero() {
  0
}

fn make_float_zero() {
  0.0
}

fn make_pair(a, b) {
  #(a, b)
}

fn make_ok(value) {
  Ok(value)
}

fn make_error(reason) {
  Error(reason)
}

pub fn var_true_test() -> Nil {
  let true_ = true()
  assert 0
    == case Nil {
      _ if true_ -> 0
      _ -> 1
    }
  Nil
}

pub fn var_false_test() -> Nil {
  let false_ = false()
  assert 1
    == case Nil {
      _ if false_ -> 0
      _ -> 1
    }
  Nil
}

pub fn int_equals_match_test() -> Nil {
  let int_zero = make_int_zero()
  assert 0
    == case Nil {
      _ if int_zero == int_zero -> 0
      _ -> 1
    }
  Nil
}

pub fn int_equals_nomatch_test() -> Nil {
  let int_zero = make_int_zero()
  let int_one = make_int_zero() + 1
  assert 1
    == case Nil {
      _ if int_zero == int_one -> 0
      _ -> 1
    }
  Nil
}

pub fn int_not_equals_match_test() -> Nil {
  let int_zero = make_int_zero()
  let int_one = make_int_zero() + 1
  assert 0
    == case Nil {
      _ if int_zero != int_one -> 0
      _ -> 1
    }
  Nil
}

pub fn int_not_equals_nomatch_test() -> Nil {
  let int_zero = make_int_zero()
  assert 1
    == case Nil {
      _ if int_zero != int_zero -> 0
      _ -> 1
    }
  Nil
}

pub fn record_equals_match_test() -> Nil {
  let ok = make_ok(1)
  assert 0
    == case Nil {
      _ if ok == ok -> 0
      _ -> 1
    }
  Nil
}

pub fn record_equals_nomatch_test() -> Nil {
  let ok = make_ok(1)
  let error = make_error(1)
  assert 1
    == case Nil {
      _ if ok == error -> 0
      _ -> 1
    }
  Nil
}

pub fn record_not_equals_match_test() -> Nil {
  let ok = make_ok(1)
  let error = make_error(1)
  assert 0
    == case Nil {
      _ if ok != error -> 0
      _ -> 1
    }
  Nil
}

pub fn record_not_equals_nomatch_test() -> Nil {
  let error = make_error(1)
  assert 1
    == case Nil {
      _ if error != error -> 0
      _ -> 1
    }
  Nil
}

pub fn and_true_true_test() -> Nil {
  let true_ = true()
  assert 0
    == case Nil {
      _ if true_ && true_ -> 0
      _ -> 1
    }
  Nil
}

pub fn and_true_false_test() -> Nil {
  let true_ = true()
  let false_ = false()
  assert 1
    == case Nil {
      _ if true_ && false_ -> 0
      _ -> 1
    }
  Nil
}

pub fn and_false_true_test() -> Nil {
  let true_ = true()
  let false_ = false()
  assert 1
    == case Nil {
      _ if false_ && true_ -> 0
      _ -> 1
    }
  Nil
}

pub fn and_false_false_test() -> Nil {
  let false_ = false()
  assert 1
    == case Nil {
      _ if false_ && false_ -> 0
      _ -> 1
    }
  Nil
}

pub fn or_true_true_test() -> Nil {
  let true_ = true()
  assert 0
    == case Nil {
      _ if true_ || true_ -> 0
      _ -> 1
    }
  Nil
}

pub fn or_true_false_test() -> Nil {
  let true_ = true()
  let false_ = false()
  assert 0
    == case Nil {
      _ if true_ || false_ -> 0
      _ -> 1
    }
  Nil
}

pub fn or_false_true_test() -> Nil {
  let true_ = true()
  let false_ = false()
  assert 0
    == case Nil {
      _ if false_ || true_ -> 0
      _ -> 1
    }
  Nil
}

pub fn or_false_false_test() -> Nil {
  let false_ = false()
  assert 1
    == case Nil {
      _ if false_ || false_ -> 0
      _ -> 1
    }
  Nil
}

pub fn float_gt_1_test() -> Nil {
  let float_zero = make_float_zero()
  let float_one = make_float_zero() +. 1.0
  assert 0
    == case Nil {
      _ if float_one >. float_zero -> 0
      _ -> 1
    }
  Nil
}

pub fn float_gt_2_test() -> Nil {
  let float_zero = make_float_zero()
  assert 1
    == case Nil {
      _ if float_zero >. float_zero -> 0
      _ -> 1
    }
  Nil
}

pub fn float_gte_1_test() -> Nil {
  let float_zero = make_float_zero()
  let float_one = make_float_zero() +. 1.0
  assert 0
    == case Nil {
      _ if float_one >=. float_zero -> 0
      _ -> 1
    }
  Nil
}

pub fn float_gte_2_test() -> Nil {
  let float_zero = make_float_zero()
  assert 0
    == case Nil {
      _ if float_zero >=. float_zero -> 0
      _ -> 1
    }
  Nil
}

pub fn float_gte_3_test() -> Nil {
  let float_zero = make_float_zero()
  let float_one = make_float_zero() +. 1.0
  assert 1
    == case Nil {
      _ if float_zero >=. float_one -> 0
      _ -> 1
    }
  Nil
}

pub fn float_lt_1_test() -> Nil {
  let float_zero = make_float_zero()
  let float_one = make_float_zero() +. 1.0
  assert 0
    == case Nil {
      _ if float_zero <. float_one -> 0
      _ -> 1
    }
  Nil
}

pub fn float_lt_2_test() -> Nil {
  let float_zero = make_float_zero()
  assert 1
    == case Nil {
      _ if float_zero <. float_zero -> 0
      _ -> 1
    }
  Nil
}

pub fn float_lte_1_test() -> Nil {
  let float_zero = make_float_zero()
  let float_one = make_float_zero() +. 1.0
  assert 0
    == case Nil {
      _ if float_zero <=. float_one -> 0
      _ -> 1
    }
  Nil
}

pub fn float_lte_2_test() -> Nil {
  let float_zero = make_float_zero()
  assert 0
    == case Nil {
      _ if float_zero <=. float_zero -> 0
      _ -> 1
    }
  Nil
}

pub fn float_lte_3_test() -> Nil {
  let float_zero = make_float_zero()
  let float_one = make_float_zero() +. 1.0
  assert 1
    == case Nil {
      _ if float_one <=. float_zero -> 0
      _ -> 1
    }
  Nil
}

pub fn int_gt_1_test() -> Nil {
  let int_zero = make_int_zero()
  let int_one = make_int_zero() + 1
  assert 0
    == case Nil {
      _ if int_one > int_zero -> 0
      _ -> 1
    }
  Nil
}

pub fn int_gt_2_test() -> Nil {
  let int_zero = make_int_zero()
  assert 1
    == case Nil {
      _ if int_zero > int_zero -> 0
      _ -> 1
    }
  Nil
}

pub fn int_gte_1_test() -> Nil {
  let int_zero = make_int_zero()
  let int_one = make_int_zero() + 1
  assert 0
    == case Nil {
      _ if int_one >= int_zero -> 0
      _ -> 1
    }
  Nil
}

pub fn int_gte_2_test() -> Nil {
  let int_zero = make_int_zero()
  assert 0
    == case Nil {
      _ if int_zero >= int_zero -> 0
      _ -> 1
    }
  Nil
}

pub fn int_gte_3_test() -> Nil {
  let int_zero = make_int_zero()
  let int_one = make_int_zero() + 1
  assert 1
    == case Nil {
      _ if int_zero >= int_one -> 0
      _ -> 1
    }
  Nil
}

pub fn int_lt_1_test() -> Nil {
  let int_zero = make_int_zero()
  let int_one = make_int_zero() + 1
  assert 0
    == case Nil {
      _ if int_zero < int_one -> 0
      _ -> 1
    }
  Nil
}

pub fn int_lt_2_test() -> Nil {
  let int_zero = make_int_zero()
  assert 1
    == case Nil {
      _ if int_zero < int_zero -> 0
      _ -> 1
    }
  Nil
}

pub fn int_lte_1_test() -> Nil {
  let int_zero = make_int_zero()
  let int_one = make_int_zero() + 1
  assert 0
    == case Nil {
      _ if int_zero <= int_one -> 0
      _ -> 1
    }
  Nil
}

pub fn int_lte_2_test() -> Nil {
  let int_zero = make_int_zero()
  assert 0
    == case Nil {
      _ if int_zero <= int_zero -> 0
      _ -> 1
    }
  Nil
}

pub fn int_lte_3_test() -> Nil {
  let int_zero = make_int_zero()
  let int_one = make_int_zero() + 1
  assert 1
    == case Nil {
      _ if int_one <= int_zero -> 0
      _ -> 1
    }
  Nil
}

pub fn arithmetic_1_test() -> Nil {
  assert 0
    == case Nil {
      _ if 1 + 1 == 2 -> 0
      _ -> 1
    }
  Nil
}

pub fn arithmetic_2_test() -> Nil {
  assert 0
    == case Nil {
      _ if 47 % 5 == 2 -> 0
      _ -> 1
    }
  Nil
}

pub fn arithmetic_3_test() -> Nil {
  assert 0
    == case Nil {
      _ if 3 * 5 == 15 -> 0
      _ -> 1
    }
  Nil
}

pub fn arithmetic_4_test() -> Nil {
  assert 0
    == case Nil {
      _ if 3 * 5 + 1 == 16 -> 0
      _ -> 1
    }
  Nil
}

pub fn arithmetic_5_test() -> Nil {
  assert 0
    == case Nil {
      _ if 1 + 3 * 5 == 16 -> 0
      _ -> 1
    }
  Nil
}

pub fn arithmetic_6_test() -> Nil {
  assert 0
    == case Nil {
      _ if 1 - 15 / 5 == -2 -> 0
      _ -> 1
    }
  Nil
}

pub fn arithmetic_7_test() -> Nil {
  assert 0
    == case Nil {
      _ if 15 / 5 - 1 == 2 -> 0
      _ -> 1
    }
  Nil
}

pub fn tuple_access_1_test() -> Nil {
  let tuple_true_false = make_pair(True, False)
  assert 0
    == case Nil {
      _ if tuple_true_false.0 -> 0
      _ -> 1
    }
  Nil
}

pub fn tuple_access_2_test() -> Nil {
  let tuple_true_false = make_pair(True, False)
  assert 1
    == case Nil {
      _ if tuple_true_false.1 -> 0
      _ -> 1
    }
  Nil
}

pub fn const_1_test() -> Nil {
  let int_zero = make_int_zero()
  assert 0
    == case Nil {
      _ if int_zero == 0 -> 0
      _ -> 1
    }
  Nil
}

pub fn const_2_test() -> Nil {
  let int_zero = make_int_zero()
  assert 1
    == case Nil {
      _ if int_zero == 1 -> 0
      _ -> 1
    }
  Nil
}

pub fn const_ok_test() -> Nil {
  let ok = make_ok(1)
  assert 0
    == case Nil {
      _ if ok == Ok(1) -> 0
      _ -> 1
    }
  Nil
}

pub fn const_error_test() -> Nil {
  let ok = make_ok(1)
  assert 1
    == case Nil {
      _ if ok == Error(1) -> 0
      _ -> 1
    }
  Nil
}

pub fn tuple_with_pattern_var_test() -> Nil {
  let x = True
  let int = case x {
    a if #(a) == #(True) -> 0
    _ -> 1
  }
  assert 0 == int
  Nil
}

pub fn module_access_string_const_matches_test() -> Nil {
  let string = "gleam"
  assert True
    == case string {
      lang if lang == importable.language -> True
      _ -> False
    }
  Nil
}

pub fn module_access_string_const_nomatch_test() -> Nil {
  let string = "python"
  assert False
    == case string {
      lang if lang == importable.language -> True
      _ -> False
    }
  Nil
}

pub fn module_access_custom_type_const_matches_test() -> Nil {
  let string = "WarGames"
  assert True
    == case string {
      movie if movie == importable.war_games.title -> True
      _ -> False
    }
  Nil
}

pub fn module_access_custom_type_const_nomatch_test() -> Nil {
  let string = "Gattaca"
  assert False
    == case string {
      movie if movie == importable.war_games.title -> True
      _ -> False
    }
  Nil
}

// https://github.com/gleam-lang/gleam/issues/5283
pub fn case_with_guard_does_not_pollute_outer_scope_test() -> Nil {
  let a = case 1337 {
    n if n == 1347 -> 1
    _ -> 2
  }
  let b = case 1337 {
    n -> 2
  }
  assert a == b
  Nil
}

// https://github.com/gleam-lang/gleam/issues/5641
pub fn tail_of_list_in_clause_guard_is_not_ignored_test() -> Nil {
  assert go([2, 3, 4], [1, 2, 3, 4])
  assert !go([2, 3], [1, 2])
  Nil
}

pub fn go(x: List(Int), y: List(Int)) {
  case x {
    [_, ..x] if [1, 2, ..x] == y -> True
    _ -> False
  }
}

pub fn or_guard_does_not_escape_its_pattern_test() -> Nil {
  let flag = false()
  let other = true()

  let value = [1, 2]
  assert 0
    == case value {
      [a] if flag || other -> a
      _ -> 0
    }

  let value = 7
  assert 7
    == case value {
      a if flag || other -> a
      _ -> 0
    }
  Nil
}

pub fn guard_binding_in_directly_matching_case_is_not_redeclared_test() -> Nil {
  let value = make_pair(1, 2)
  case value {
    #(_, b) if b == 2 -> b
    #(_, b) -> b
  }
  let b = 5
  assert b == 5
  Nil
}
