import i/io
import box

type Person { Person(name: String, age: Int) }
type Wrap(a) { Wrap(value: a) }

pub fn main() {
  let person = Person(name: "Ada", age: 36)
  let older = Person(..person, age: 37)
  assert person.age == 36
  assert older.name == "Ada"
  assert older.age == 37
  assert older != person
  assert Wrap(value: [1, 2]) == Wrap([1, 2])
  let private = box.new(42)
  assert box.value(private) == 42
  io.println("PASS:records")
}
