import i/io

fn identity(value) { value }
fn apply(value, function) { function(value) }
fn add(first first: Int, second second: Int) { first + second }
fn continuation(value, then) { then(value) }
fn make_adder(base) { fn(value) { value + base } }

pub fn main() {
  let add_two = make_adder(2)
  let add_three = make_adder(3)
  assert add_two(40) == 42
  assert add_three(40) == 43
  assert identity("polymorphic") == "polymorphic"
  assert identity(42) == 42
  assert apply(40, add(_, second: 2)) == 42
  assert add(first: 40, second: 2) == 42
  let result = {
    use value <- continuation(40)
    value |> add_two |> identity
  }
  assert result == 42
  io.println("PASS:functions")
}
