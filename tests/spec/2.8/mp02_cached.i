// spec: 2.8
// expect: pass
// MP02 payload: package tests replace a locked cached lib.i with these bytes,
// keeping its original .hash sidecar. The check must reject the copy.
(value)
value = "changed in cache"
main = () { Nil }
