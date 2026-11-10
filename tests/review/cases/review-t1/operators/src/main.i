a = 1 + 2
b = "a" <> "b"
c = -{1}
d = -{-1}
x = -0
y: 16 = 0x10
s: "é" = "\u{e9}"
@main
main = () { assert(a == 3) assert(b == "ab") Nil }
