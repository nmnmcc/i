Clock = (now = () -> Int)

Log = (write = (String) -> Nil)

State(S) = (get = () -> S)

Id(X) = X

Svc = Clock

K2 = Id(Clock)

// An alias with parameters is expanded with its arguments (§9.2).
f = () <- (c: Id(Clock)) { c.now() }

g = () <- (c: Svc, l: Id(Id(Log))) { c.now() }

h = () { 1 <- (Clock(now = () { 1 }), Clock(now = () { 1 })) }

// Still not one named type.
bad1 = () <- (c: Id(Never)) { 1 }

bad2 = () <- (c: Id(Clock | Log)) { 1 }

bad3 = () { 1 <- (panic("x")) }

// Two environment parameters with one type constructor (§9.2).
twice = () <- (a: Clock, b: Clock) { 1 }

twice_alias = () <- (a: Clock, b: K2, c: Svc) { 1 }

twice_args = () <- (a: State(Int), b: State(String)) { 1 }
