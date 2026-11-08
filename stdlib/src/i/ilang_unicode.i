//// Unicode 17.0.0 algorithms shared by the Wasm string modules.
////
//// Grapheme boundaries follow UAX #29 revision 47, extended grapheme clusters.
//// Case mapping uses the Unicode default (locale-independent) full mappings.
//// The only runtime primitives decode and encode Unicode scalar values in UTF-8.

import i/ilang_unicode_tables as tables

@external(wasm, "ilang:runtime/v1", "string_codepoints")
pub fn codepoints(string: String) -> List(Int)

@external(wasm, "ilang:runtime/v1", "string_from_codepoints")
pub fn from_codepoints(codepoints: List(Int)) -> String

pub fn is_pattern_whitespace(codepoint: Int) -> Bool {
  tables.pattern_whitespace(codepoint)
}

// Property numbers are fixed by unicode/generate.py's GCB and INCB tables.
type BoundaryState {
  BoundaryState(
    previous: Int,
    regional_count: Int,
    pictographic_suffix: Bool,
    pictographic_zwj: Bool,
    conjunct: Int,
  )
}

fn initial_state() -> BoundaryState {
  BoundaryState(0, 0, False, False, 0)
}

/// Split into extended grapheme clusters, preserving the original scalar values.
pub fn graphemes(string: String) -> List(String) {
  case codepoints(string) {
    [] -> []
    [first, ..rest] -> {
      let state = advance(initial_state(), first, tables.grapheme_break(first))
      grapheme_loop(rest, state, [first], [])
    }
  }
}

fn grapheme_loop(
  rest: List(Int),
  state: BoundaryState,
  cluster: List(Int),
  clusters: List(String),
) -> List(String) {
  case rest {
    [] -> reverse([from_codepoints(reverse(cluster, [])), ..clusters], [])
    [codepoint, ..rest] -> {
      let property = tables.grapheme_break(codepoint)
      let conjunct = tables.indic_conjunct_break(codepoint)
      let pictographic = tables.extended_pictographic(codepoint)
      case breaks_before(state, property, conjunct, pictographic) {
        True -> {
          let clusters = [from_codepoints(reverse(cluster, [])), ..clusters]
          let state = advance(initial_state(), codepoint, property)
          grapheme_loop(rest, state, [codepoint], clusters)
        }
        False -> {
          let state = advance(state, codepoint, property)
          grapheme_loop(rest, state, [codepoint, ..cluster], clusters)
        }
      }
    }
  }
}

fn breaks_before(
  state: BoundaryState,
  next: Int,
  next_conjunct: Int,
  next_pictographic: Bool,
) -> Bool {
  case state.previous, next {
    // GB3: CR × LF.
    1, 2 -> False
    // GB4 / GB5: boundaries around controls, before extending rules.
    1, _ | 2, _ | 3, _ -> True
    _, 1 | _, 2 | _, 3 -> True
    // GB6–GB8: Hangul syllable sequences.
    9, 9 | 9, 10 | 9, 12 | 9, 13 -> False
    10, 10 | 10, 11 | 12, 10 | 12, 11 -> False
    11, 11 | 13, 11 -> False
    // GB9 / GB9a / GB9b.
    _, 4 | _, 5 | _, 8 -> False
    7, _ -> False
    // GB9c: consonant followed by InCB extenders including a linker.
    _, _ if state.conjunct == 2 && next_conjunct == 1 -> False
    // GB11: Extended_Pictographic Extend* ZWJ × Extended_Pictographic.
    _, _ if state.pictographic_zwj && next_pictographic -> False
    // GB12 / GB13: pair regional indicators from the start of their run.
    6, 6 if state.regional_count % 2 == 1 -> False
    // GB999, with start/end boundaries handled by grapheme_loop.
    _, _ -> True
  }
}

fn advance(
  state: BoundaryState,
  codepoint: Int,
  property: Int,
) -> BoundaryState {
  let regional_count = case property {
    6 -> state.regional_count + 1
    _ -> 0
  }
  let pictographic_zwj = property == 5 && state.pictographic_suffix
  let pictographic_suffix = case tables.extended_pictographic(codepoint) {
    True -> True
    False -> property == 4 && state.pictographic_suffix
  }
  let conjunct = case tables.indic_conjunct_break(codepoint) {
    1 -> 1
    2 -> state.conjunct
    3 if state.conjunct != 0 -> 2
    _ -> 0
  }
  BoundaryState(
    property,
    regional_count,
    pictographic_suffix,
    pictographic_zwj,
    conjunct,
  )
}

/// Unicode default full lowercase, including the context-sensitive final sigma.
pub fn lowercase(string: String) -> String {
  string
  |> codepoints
  |> lowercase_loop(False, [])
  |> from_codepoints
}

fn lowercase_loop(
  input: List(Int),
  preceded_by_cased: Bool,
  output: List(Int),
) -> List(Int) {
  case input {
    [] -> reverse(output, [])
    [codepoint, ..rest] -> {
      let mapping = case codepoint == 931 && preceded_by_cased {
        True -> {
          case followed_by_cased(rest) {
            True -> [963]
            False -> [962]
          }
        }
        False -> tables.lowercase_mapping(codepoint)
      }
      let preceded_by_cased = case tables.case_ignorable(codepoint) {
        True -> preceded_by_cased
        False -> tables.cased(codepoint)
      }
      lowercase_loop(rest, preceded_by_cased, reverse(mapping, output))
    }
  }
}

fn followed_by_cased(input: List(Int)) -> Bool {
  case input {
    [] -> False
    [codepoint, ..rest] -> {
      case tables.case_ignorable(codepoint) {
        True -> followed_by_cased(rest)
        False -> tables.cased(codepoint)
      }
    }
  }
}

/// Unicode default full uppercase (including one-to-many expansions).
pub fn uppercase(string: String) -> String {
  string
  |> codepoints
  |> uppercase_loop([])
  |> from_codepoints
}

fn uppercase_loop(input: List(Int), output: List(Int)) -> List(Int) {
  case input {
    [] -> reverse(output, [])
    [codepoint, ..rest] ->
      uppercase_loop(rest, reverse(tables.uppercase_mapping(codepoint), output))
  }
}

fn reverse(input: List(a), output: List(a)) -> List(a) {
  case input {
    [] -> output
    [first, ..rest] -> reverse(rest, [first, ..output])
  }
}
