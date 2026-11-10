(
	i/core
	f = i/fail
	shapes = (Shape, Circle, area) = app/geometry/shapes
	(Point) = app/geometry/points
	u = (double) = util/math
)

(
	main
	Pair(..)
	Kind
)

Pair(A, B) {
	Pair(left: A, right: B)
}

Kind {
	Big
	Small
}

Size = Int

Both(A) = Pair(A, A)

// Top-level names may be used before they are defined (§2.5).
main = () {
	p: Both(Size) = Pair(left: 1, right: 2)
	q = Pair(..p, left: 3)
	c = Circle(radius: 1.0)
	s = shapes.Square(2.0)
	total = area(c) +. shapes.area(s)
	n = double(u.double(p.left))
	pt: Point = (1, 2)
	check(q, total, n, pt)
}

check = (q, total, n, pt) {
	q |> {
		Pair(left = 1, right) | Pair(right = 1, left) -> core.inspect((
			left
			right
			total
		))
		Pair(left, right) ? left > right -> core.inspect(n)
		_                                -> core.inspect(pt)
	}
}

safe = (x) {
	f.catch(() { x }) |> {
		Ok(v)    -> v
		Error(v) -> v
	}
}

// A local name may cover a module name and a built-in name (§2.3, §5).
covers = (shapes, core) {
	panic = shapes.width
	(panic, core.size)
}
