User { User(name: String) }
f = () { (u) { u.name } |> { g: (User) -> String -> g(User("yes")) } }


main = () { assert(f() == "yes") Nil }
