Target = Js | NodeJS | Wasi | Native | Host
Js { Js(Int) }
NodeJS { NodeJS(Int) }
Wasi { Wasi(Int) }
Native { Native(Int) }
Host { Host }
f = (t: Target) {
  case t {
    Js(j) -> Nil
    Wasi(w) -> Nil
    Host -> Nil
  }
}

main = () { f(Host) }
