import gleam/dict

/// `Dynamic` data is data that we don't know the type of yet.
/// We likely get data like this from host interoperation or IO with the
/// outside world.
///
/// This module contains code for forming dynamic data, and the
/// `gleam/dynamic/decode` module contains code for turning dynamic data back
/// into Gleam data with known types. You will likely mostly use the other
/// module in your projects.
///
/// Dynamic values retain the runtime type of the original WasmGC value.
///
pub type Dynamic

/// Return a string indicating the type of the dynamic value.
///
/// This function may be useful for constructing error messages or logs. If you
/// want to turn dynamic data into well typed data then you want the
/// `gleam/dynamic/decode` module.
///
/// ```gleam
/// assert dynamic.classify(dynamic.string("Hello")) == "String"
/// ```
///
@external(wasm, "wleam:runtime/v1", "dynamic_classify")
pub fn classify(data: Dynamic) -> String

/// Create a dynamic value from a bool.
///
pub fn bool(a: Bool) -> Dynamic {
  cast(a)
}

/// Create a dynamic value from a string.
///
/// The value retains its UTF-8 string representation.
///
pub fn string(a: String) -> Dynamic {
  cast(a)
}

/// Create a dynamic value from a float.
///
pub fn float(a: Float) -> Dynamic {
  cast(a)
}

/// Create a dynamic value from an int.
///
pub fn int(a: Int) -> Dynamic {
  cast(a)
}

/// Create a dynamic value from a bit array.
///
pub fn bit_array(a: BitArray) -> Dynamic {
  cast(a)
}

/// Create a dynamic value from a list.
///
pub fn list(a: List(Dynamic)) -> Dynamic {
  cast(a)
}

/// Create a dynamic value from a list, converting it to a sequential runtime
/// format rather than the regular list format.
///
/// The result is a dynamically sized array, distinct from a linked list.
///
@external(wasm, "wleam:runtime/v1", "dynamic_array")
pub fn array(a: List(Dynamic)) -> Dynamic

/// Create a dynamic value made of an unordered series of keys and values, where
/// the keys are unique.
///
/// The result uses the same persistent dictionary as `gleam/dict`.
///
pub fn properties(entries: List(#(Dynamic, Dynamic))) -> Dynamic {
  cast(dict.from_list(entries))
}

/// A dynamic value representing nothing.
///
/// This is the Gleam `Nil` value with its static type erased.
///
pub fn nil() -> Dynamic {
  cast(Nil)
}

@external(wasm, "wleam:runtime/v1", "dynamic_cast")
fn cast(a: anything) -> Dynamic
