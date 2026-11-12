Config { Config(label: String) }
Await { Await(wait: () -> String) }
make_config = () { Config(label: "configured") }
make_await = (c ! Config) { Await(wait: () { c.label }) }
body = (a ! Await) { a.wait() }
fixed_order = () {
  ! Await = make_await()
  ! Config = make_config()
  body()
}
dependencies_first = () {
  ! Config = make_config()
  ! Await = make_await()
  body()
}

main = () { assert(dependencies_first() == "configured") }
