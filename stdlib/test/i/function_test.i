import i/function

pub fn identity_int_test() -> Nil {
  assert function.identity(1) == 1
  Nil
}

pub fn identity_string_test() -> Nil {
  assert function.identity("") == ""
  Nil
}

pub fn identity_list_test() -> Nil {
  assert function.identity([]) == []
  Nil
}

pub fn identity_tuple_test() -> Nil {
  assert function.identity(#(1, 2.0)) == #(1, 2.0)
  Nil
}
