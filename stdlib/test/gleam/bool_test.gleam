import gleam/bool

pub fn and_true_true_test() -> Nil {
  assert bool.and(True, True)
  Nil
}

pub fn and_false_true_test() -> Nil {
  assert !bool.and(False, True)
  Nil
}

pub fn and_true_true_2_test() -> Nil {
  assert bool.and(True, True)
  Nil
}

pub fn and_true_false_test() -> Nil {
  assert !bool.and(True, False)
  Nil
}

pub fn or_true_true_test() -> Nil {
  assert bool.or(True, True)
  Nil
}

pub fn or_false_true_test() -> Nil {
  assert bool.or(False, True)
  Nil
}

pub fn or_true_false_test() -> Nil {
  assert bool.or(True, False)
  Nil
}

pub fn or_true_false_2_test() -> Nil {
  assert bool.or(True, False)
  Nil
}

pub fn negate_true_test() -> Nil {
  assert !bool.negate(True)
  Nil
}

pub fn negate_false_test() -> Nil {
  assert bool.negate(False)
  Nil
}

pub fn nor_false_false_test() -> Nil {
  assert bool.nor(False, False)
  Nil
}

pub fn nor_false_true_test() -> Nil {
  assert !bool.nor(False, True)
  Nil
}

pub fn nor_true_false_test() -> Nil {
  assert !bool.nor(True, False)
  Nil
}

pub fn nor_true_true_test() -> Nil {
  assert !bool.nor(True, True)
  Nil
}

pub fn nand_false_false_test() -> Nil {
  assert bool.nand(False, False)
  Nil
}

pub fn nand_false_true_test() -> Nil {
  assert bool.nand(False, True)
  Nil
}

pub fn nand_true_false_test() -> Nil {
  assert bool.nand(True, False)
  Nil
}

pub fn nand_true_true_test() -> Nil {
  assert !bool.nand(True, True)
  Nil
}

pub fn exclusive_or_true_true_test() -> Nil {
  assert !bool.exclusive_or(True, True)
  Nil
}

pub fn exclusive_or_false_false_test() -> Nil {
  assert !bool.exclusive_or(False, False)
  Nil
}

pub fn exclusive_or_true_false_test() -> Nil {
  assert bool.exclusive_or(True, False)
  Nil
}

pub fn exclusive_or_false_true_test() -> Nil {
  assert bool.exclusive_or(False, True)
  Nil
}

pub fn exclusive_nor_false_false_test() -> Nil {
  assert bool.exclusive_nor(False, False)
  Nil
}

pub fn exclusive_nor_false_true_test() -> Nil {
  assert !bool.exclusive_nor(False, True)
  Nil
}

pub fn exclusive_nor_true_false_test() -> Nil {
  assert !bool.exclusive_nor(True, False)
  Nil
}

pub fn exclusive_nor_true_true_test() -> Nil {
  assert bool.exclusive_nor(True, True)
  Nil
}

pub fn to_string_true_test() -> Nil {
  assert bool.to_string(True) == "True"
  Nil
}

pub fn to_string_false_test() -> Nil {
  assert bool.to_string(False) == "False"
  Nil
}

pub fn guard_when_true_test() -> Nil {
  let assert 2 = {
    use <- bool.guard(when: True, return: 2)
    1
  }
  Nil
}

pub fn guard_when_false_test() -> Nil {
  let assert 1 = {
    use <- bool.guard(when: False, return: 2)
    1
  }
  Nil
}

pub fn lazy_guard_when_true_test() -> Nil {
  let oops = fn() { panic as "this shouldn't run!" }

  let assert 2 = {
    use <- bool.lazy_guard(when: True, otherwise: oops)
    2
  }
  Nil
}

pub fn lazy_guard_when_false_test() -> Nil {
  let oops = fn() { panic as "this shouldn't run!" }

  let assert 1 = {
    use <- bool.lazy_guard(when: False, return: oops)
    1
  }
  Nil
}
