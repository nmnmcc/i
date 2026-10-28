const unary = (type) => ({ params: [type], result: type });

export const fixtureAbi = {
  version: 1,
  entry: "main",
  tests: ["passing_test", "failing_test", "fresh_instance_test", "fresh_instance_test"],
  exports: {
    string_roundtrip: unary("string"),
    int_roundtrip: unary("int"),
    bit_array_roundtrip: unary("bit_array"),
    float_roundtrip: unary("float"),
    bool_roundtrip: unary("bool"),
    nil_roundtrip: { params: [], result: "nil" },
    opaque_roundtrip: unary("externref"),
    fail: { params: ["string"], result: "nil" },
    trap_with_string: { params: ["string"], result: "nil" },
  },
};

export const customAbi = {
  version: 1,
  exports: {
    leaking_throw: unary("string"),
    echo_string: unary("string"),
    echo_int: unary("int"),
    echo_bits: unary("bit_array"),
    echo_ref: unary("externref"),
    echo_bool: unary("bool"),
    echo_float: unary("float"),
    echo_nil: { params: [], result: "nil" },
  },
  imports: {
    custom: {
      echo_string: unary("string"),
      echo_int: unary("int"),
      echo_bits: unary("bit_array"),
      echo_ref: unary("externref"),
      echo_bool: unary("bool"),
      echo_float: unary("float"),
      echo_nil: { params: [], result: "nil" },
    },
  },
};

export function identityImports(overrides = {}) {
  return { custom: Object.fromEntries(Object.keys(customAbi.imports.custom).map((name) => [name, overrides[name] ?? ((value) => value)])) };
}
