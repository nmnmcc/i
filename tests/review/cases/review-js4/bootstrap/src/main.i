Config = (label = String)
Await = (wait = () -> String)
make_config = () { Config(label = "configured") }
make_await = () <- (c : Config) { Await(wait = () { c.label }) }
body = () <- (a : Await) { a.wait() }
fixed_order = () { body() <- (make_config()) <- (make_await()) }
dependencies_first = () { body() <- (make_await()) <- (make_config()) }

main = () { assert(dependencies_first() == "configured") }
