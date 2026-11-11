// spec: 4.9, 14
// expect: pass
(i/record)
(main)
@main
main = () {
  a = (name: "Ada", age: 36)
  assert(record.get(a, "name") == "Ada")
  b = record.put(a, "age", 37)
  assert(b.age == 37)
  assert(record.get(b, "name") == "Ada")
  c = record.remove(b, "age")
  assert(record.has(c, "name"))
  assert(record.has(c, "age") == False)
  assert(record.size(c) == 1)
  assert(record.keys(c) == ["name"])
  assert(record.texts(c) == [("name", "\"Ada\"")])
  Nil
}
