import (u = app/util/list, (Pair) = app/util/list)

lengths = (xs) { u.map(xs, (s) { s <> "!" }) }
numbers = u.map([1, 2], (n) { n * 2 })
flipped = u.swap(Pair(left: 1, right: "a"))
by_label = u.map(f: (n) { n + 1 }, xs: [1])
