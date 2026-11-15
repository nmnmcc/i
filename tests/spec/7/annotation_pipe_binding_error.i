// spec: 3, 7.1
// expect: compile-error E0101
// old-syntax-check: E0101
// The old reading accepted a union annotation. The pipe now ends Annot.
v : Int | String = 1
