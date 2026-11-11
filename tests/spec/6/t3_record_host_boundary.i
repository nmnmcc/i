// spec: 4.9, 9.7, 10.3
// expect: compile-error E0320
@host("test:records/v1")
Records { Records(get: () -> (name: String)) }
@test
check = () { Nil }
