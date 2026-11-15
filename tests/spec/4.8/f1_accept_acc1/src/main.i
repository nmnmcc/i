(list = std/list)
total = (xs: List(Int)) { list.fold(xs, (count = 0, sum = 0), (acc, x) { (count = acc.count + 1, sum = acc.sum + x) }) }
