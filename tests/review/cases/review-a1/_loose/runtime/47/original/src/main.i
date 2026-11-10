User { User(name: String) }
f = (n: Int) { case n { 0 -> User("yes") _ -> User("no") }.name }

@main
main = () { assert(f(0) == "yes" && f(1) == "no") Nil }
