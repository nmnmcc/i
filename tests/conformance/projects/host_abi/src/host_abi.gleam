// SPDX-License-Identifier: Apache-2.0

/// Opaque host-owned reference. The compiler maps constructor-free types to
/// externref; no Gleam record or collection is exported through this interface.
pub type HostReference

@external(wasm, "custom", "echo_int")
fn host_echo_int(value: Int) -> Int

@external(wasm, "custom", "echo_string")
fn host_echo_string(value: String) -> String

@external(wasm, "custom", "echo_bits")
fn host_echo_bits(value: BitArray) -> BitArray

@external(wasm, "custom", "echo_ref")
fn host_echo_ref(value: HostReference) -> HostReference

@external(wasm, "custom", "echo_bool")
fn host_echo_bool(value: Bool) -> Bool

@external(wasm, "custom", "echo_float")
fn host_echo_float(value: Float) -> Float

@external(wasm, "custom", "echo_nil")
fn host_echo_nil(value: Nil) -> Nil

pub fn echo_int(value: Int) -> Int {
  host_echo_int(value)
}

pub fn echo_string(value: String) -> String {
  host_echo_string(value)
}

pub fn echo_bits(value: BitArray) -> BitArray {
  host_echo_bits(value)
}

pub fn echo_ref(value: HostReference) -> HostReference {
  host_echo_ref(value)
}

pub fn echo_bool(value: Bool) -> Bool {
  host_echo_bool(value)
}

pub fn echo_float(value: Float) -> Float {
  host_echo_float(value)
}

pub fn echo_nil(value: Nil) -> Nil {
  host_echo_nil(value)
}

pub fn reference_same(left: HostReference, right: HostReference) -> Bool {
  left == right
}

/// The returned slice has a three-bit offset internally. Its copied ABI value
/// must start at bit zero with bytes [255, 128] and a bit length of nine.
pub fn unaligned_bits() -> BitArray {
  let original = <<5:3, 511:9>>
  let assert <<_:3, tail:bits>> = original
  tail
}

pub fn panic_now() -> Nil {
  panic as "host_abi intentional guest panic"
}
