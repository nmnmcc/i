import gleam/io

type Item { Number(Int) Text(String) Empty }

fn match_item(item) {
  case item {
    Number(value) if value > 10 -> value
    Number(_) | Empty -> 0
    Text("prefix:" <> tail) -> case tail { "ok" -> 42 _ -> 1 }
    Text(_) -> -1
  }
}

pub fn main() {
  let values = [1, 2, 3]
  let assert [head, ..tail] = values
  assert head == 1
  assert tail == [2, 3]
  assert match_item(Number(42)) == 42
  assert match_item(Empty) == 0
  assert match_item(Text("prefix:ok")) == 42
  let pair = #(1, "ok")
  assert pair.0 == 1
  assert case pair { #(1, text) as whole -> text == "ok" && whole == pair _ -> False }
  assert case 2, "ok" { 1, "ok" | 2, "ok" -> True _, _ -> False }
  io.println("PASS:patterns")
}
