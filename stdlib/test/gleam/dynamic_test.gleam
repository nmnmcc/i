import gleam/dynamic

pub fn classify_true_test() -> Nil {
  assert dynamic.classify(dynamic.bool(True)) == "Bool"
  Nil
}

pub fn classify_false_test() -> Nil {
  assert dynamic.classify(dynamic.bool(False)) == "Bool"
  Nil
}

pub fn null_test() -> Nil {
  assert dynamic.classify(dynamic.nil()) == "Nil"
  Nil
}
