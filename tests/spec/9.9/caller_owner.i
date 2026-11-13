// spec: 9.9, 9.11
// expect: pass

// C05: scopes and combinators preserve their caller's request Owner.
(
	i/task
	i/random
	i/request
)

same = (before: request.Owner) <- (inside: request.Owner) { before == inside }

main = () {
	task.test(random.from_seed(7), [], () <- (before: request.Owner) {
		assert(
			task.both(() { same(before) }, () { same(before) }) == (True, True),
		)
		assert(task.scope((_) { same(before) }))
		assert(task.local_scope((_) { same(before) }))
		assert(task.race(() { same(before) }, () { same(before) }))
		assert(
			task.map_concurrently([1, 2], (_) { same(before) }) == [True, True],
		)
		request.within(() <- (inner: request.Owner) {
			assert(inner != before)
			assert(task.scope((_) { same(inner) }))
			assert(
				task.both(() { same(inner) }, () { same(inner) }) ==
					(True, True),
			)
		})
	})
}
