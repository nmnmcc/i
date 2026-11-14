// spec: 2.8
// expect: pass
// MP01 payload: the package tests put this outside a located dependency
// and commit src/lib.i as a symbolic link to it. Loading must reject the link.
(value)
value = 1
main = () { Nil }
