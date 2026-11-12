User { User(name: String) }
f = () { (u) { u.name } |> { g: (User) -> String -> g(User(name: "yes")) } }


main = () { assert(f() == "yes") Nil }
