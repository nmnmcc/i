(testlib = (test) = std/test)

test("force expansion", () {
    index = (r) { r.0 }
    multi = (r) { r.12 }
    underscored = (r) { r.1_2 }
    field = (r) { r.missing }
    method = (r) { r.missing(1) }
    Nil
})
