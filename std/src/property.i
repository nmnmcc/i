//// Reproducible pure generators with lazy, ordered shrink trees.
//// Shrinking is structural: a mapped value need not get numerically smaller.
//// Replay requires the same library version, generator code, captured values,
//// and deterministic, terminating callbacks. Store the lock file with reports.
//// Pure types alone do not prove these callback laws; function identities and
//// hidden host effects are not reproducible data.
////
//// Each attempt has its own path-derived stream. Shrinking greedily follows
//// the first failing child; its budget counts predicate calls, not time or
//// child-list construction. `smallest` is the best value on that path, not a
//// global minimum. User thunks may diverge. Integer children cost logarithmic
//// space; list children can require quadratic copying. Ancestors may remain
//// reachable through closures. See docs/design/std-3.md P1–P9 for the protocol.

(
	i/core
	i/random
	i/fail
	i/assertion
	lists = std/list
	(Option, Some, None) = std/option
)

(
	Gen
	Tree(..)
	Config(..)
	BuildError(..)
	Verdict(..)
	Failure(..)
	Report(..)
	defaults
	make
	sample
	tree
	constant
	map
	flat_map
	pair
	filter
	sized
	resize
	int
	bool
	element
	list
	float
	utf_codepoint
	string
	check
	replay
	format
	test
)

Gen(A) {
	Gen((Int, random.Generator) -> Option(Tree(A)))
}

Tree(A) {
	Tree(value: A, children: () -> List(Tree(A)))
}

Config {
	Config(
		seed:         Int
		cases:        Int
		max_size:     Int
		max_discards: Int
		max_shrinks:  Int
	)
}

BuildError {
	InvalidRange(Int, Int)
	InvalidLength(Int, Int)
	EmptyChoice
}

Verdict {
	Pass
	Fail(String)
	Discard
}

Failure(A) {
	Failure(
		seed:     Int
		attempt:  Int
		size:     Int
		original: A
		smallest: A
		reason:   String
		path:     List(Int)
		checked:  Int
		limited:  Bool
	)
}

Report(A) {
	Passed(successes: Int, discards: Int)
	GaveUp(successes: Int, discards: Int)
	Falsified(Failure(A))
	InvalidConfig(String)
}

/// The default seed, success count, size, discard budget, and shrink budget.
defaults = () -> Config { Config(0, 100, 100, 1000, 1000) }

/// Stores a pure draw. None discards one attempt; there are no hidden retries.
make(A) = (draw: (Int, random.Generator) -> Option(Tree(A))) -> Gen(A) {
	Gen(draw)
}

/// Calls the draw once, clamping negative size to zero. Random is immutable.
sample(A) = (
	gen: Gen(A)
	size: Int
	random: random.Generator
) -> Option(Tree(A)) {
	Gen(draw) = gen
	draw(nonnegative(size), random)
}

nonnegative = (n: Int) -> Int {
	{ n < 0 } |> {
		True  -> 0
		False -> n
	}
}

/// The root is computed now; children are deferred until requested.
tree(A) = (value: A, children: () -> List(Tree(A))) -> Tree(A) {
	Tree(value, children)
}

/// A leaf, independent of size and random input.
constant(A) = (value: A) -> Gen(A) {
	Gen((_, _) { Some(Tree(value, () { [] })) })
}

map_tree(A, B) = (t: Tree(A), with: (A) -> B) -> Tree(B) {
	Tree(with(t.value), () {
		lists.map(t.children(), (c) { map_tree(c, with) })
	})
}

/// Changes values, retaining ordered shape and duplicates, lazily below root.
map(A, B) = (gen: Gen(A), with: (A) -> B) -> Gen(B) {
	Gen((s, g) {
		sample(gen, s, g) |> {
			None    -> None
			Some(t) -> Some(map_tree(t, with))
		}
	})
}

bind_node(A, B) = (
	outer: Tree(A)
	inner: Tree(B)
	with: (A) -> Gen(B)
	size: Int
	right: random.Generator
) -> Tree(B) {
	Tree(inner.value, () {
		outer_children = lists.filter_map(outer.children(), (c) {
			bind_outer(c, with, size, right)
		})
		inner_children = lists.map(inner.children(), (c) {
			bind_node(outer, c, with, size, right)
		})
		lists.append(outer_children, inner_children)
	})
}

bind_outer(A, B) = (
	outer: Tree(A)
	with: (A) -> Gen(B)
	size: Int
	right: random.Generator
) -> Option(Tree(B)) {
	sample(with(outer.value), size, right) |> {
		None        -> None
		Some(inner) -> Some(bind_node(outer, inner, with, size, right))
	}
}

/// Splits once. Outer shrinks regenerate the dependent tree on the same right
/// stream and size. Outer alternatives precede inner alternatives; discarded
/// branches are pruned, including descendants. Seeded associativity is no law.
flat_map(A, B) = (gen: Gen(A), with: (A) -> Gen(B)) -> Gen(B) {
	Gen((s, g) {
		(left, right) = random.split(g)
		sample(gen, s, left) |> {
			None    -> None
			Some(t) -> bind_outer(t, with, s, right)
		}
	})
}

/// Exactly flat_map followed by map, including random stream allocation.
pair(A, B) = (first: Gen(A), second: Gen(B)) -> Gen((A, B)) {
	flat_map(first, (x) { map(second, (y) { (x, y) }) })
}

filter_tree(A) = (t: Tree(A), keeping: (A) -> Bool) -> Option(Tree(A)) {
	keeping(t.value) |> {
		False -> None
		True -> Some(Tree(t.value, () {
			lists.filter_map(t.children(), (c) { filter_tree(c, keeping) })
		}))
	}
}

/// One root draw. Failing children and their whole subtrees are removed.
filter(A) = (gen: Gen(A), keeping: (A) -> Bool) -> Gen(A) {
	Gen((s, g) {
		sample(gen, s, g) |> {
			None    -> None
			Some(t) -> filter_tree(t, keeping)
		}
	})
}

/// Selects a generator using the current size, with the same random stream.
sized(A) = (with: (Int) -> Gen(A)) -> Gen(A) {
	Gen((s, g) { sample(with(s), s, g) })
}

/// Replaces size, including the size captured by dependent shrink trees.
resize(A) = (gen: Gen(A), size: Int) -> Gen(A) {
	Gen((_, g) { sample(gen, nonnegative(size), g) })
}

// Bounds and path indices below are valid by construction. Exhaustive fallbacks
// demote RandomError rather than giving stored generator callbacks an effect.
draw_int = (g: random.Generator, below: Int) -> Option(Int) {
	fail.catch(() { random.int(g, below) }) |> {
		Ok((value, _)) -> Some(value)
		Error(_)       -> None
	}
}

at_path = (g: random.Generator, path: List(Int)) -> Option(random.Generator) {
	fail.catch(() { random.at_path(g, path) }) |> {
		Ok(next) -> Some(next)
		Error(_) -> None
	}
}

integer_tree = (x: Int, target: Int) -> Tree(Int) {
	Tree(x, () {
		{ x == target } |> {
			True -> []
			False -> {
				halves = (q: Int, acc: List(Int)) -> List(Int) {
					q |> {
						0 -> lists.reverse(acc)
						_ -> halves(q / 2, [x - q, ..acc])
					}
				}
				values = lists.unique([
					target
					..halves({ x - target } / 2, [])
				])
				lists.map(values, (v) { integer_tree(v, target) })
			}
		}
	})
}

/// Uniform inclusive range under the uniform-word model. Shrinks toward zero
/// or its nearest endpoint, then by halving distance, with duplicates removed.
int = (min: Int, max: Int) -> Result(Gen(Int), BuildError) {
	{ min > max } |> {
		True -> Error(InvalidRange(min, max))
		False -> {
			target = { min > 0 } |> {
				True -> min
				False -> { max < 0 } |> {
					True  -> max
					False -> 0
				}
			}
			Ok(Gen((_, g) {
				draw_int(g, max - min + 1) |> {
					None        -> None
					Some(value) -> Some(integer_tree(min + value, target))
				}
			}))
		}
	}
}

// Internal known-valid factories are matched exhaustively, without Assert.
range = (min: Int, max: Int) -> Gen(Int) {
	int(min, max) |> {
		Ok(gen)  -> gen
		Error(_) -> Gen((_, _) { None })
	}
}

/// True shrinks to False.
bool = () -> Gen(Bool) { map(range(0, 1), (n) { n == 1 }) }

/// Draws and shrinks an index toward zero. Duplicates keep their multiplicity.
element(A) = (choices: List(A)) -> Result(Gen(A), BuildError) {
	choices |> {
		[] -> Error(EmptyChoice)
		[first, .._] -> Ok(map(range(0, lists.length(choices) - 1), (index) {
			lists.at(choices, index) |> {
				Some(value) -> value
				None        -> first
			}
		}))
	}
}

list_tree(A) = (items: List(Tree(A)), min: Int) -> Tree(List(A)) {
	Tree(lists.map(items, (t) { t.value }), () {
		n = lists.length(items)
		chunks = (k: Int, acc: List(Tree(List(A)))) -> List(Tree(List(A))) {
			{ k == 0 } |> {
				True -> lists.reverse(acc)
				False -> {
					starts = (
						start: Int
						out: List(Tree(List(A)))
					) -> List(Tree(List(A))) {
						{ start + k > n || n - k < min } |> {
							True -> out
							False -> starts(
								start + k
								[
									list_tree(
										lists.append(
											lists.take(items, start)
											lists.drop(items, start + k)
										)
										min
									)
									..out
								]
							)
						}
					}
					chunks(k / 2, starts(0, acc))
				}
			}
		}
		elements = (
			rest: List(Tree(A))
			before: List(Tree(A))
		) -> List(Tree(List(A))) {
			rest |> {
				[] -> []
				[t, ..tail] -> lists.append(
					lists.map(t.children(), (c) {
						list_tree(
							lists.append(lists.reverse(before), [c, ..tail])
							min
						)
					})
					elements(tail, [t, ..before])
				)
			}
		}
		lists.append(chunks(n, []), elements(items, []))
	})
}

draw_items(A) = (
	of: Gen(A)
	n: Int
	size: Int
	g: random.Generator
	index: Int
	acc: List(Tree(A))
) -> Option(List(Tree(A))) {
	{ index == n } |> {
		True -> Some(lists.reverse(acc))
		False -> at_path(g, [index + 1]) |> {
			None -> None
			Some(next) -> sample(of, size, next) |> {
				None    -> None
				Some(t) -> draw_items(of, n, size, g, index + 1, [t, ..acc])
			}
		}
	}
}

/// Length is uniform in min..min(max,max(min,size)). Deletions of full chunks
/// n,n/2,...,1 precede left-to-right element shrinks. Surviving trees are kept.
list(A) = (of: Gen(A), min: Int, max: Int) -> Result(Gen(List(A)), BuildError) {
	{ min < 0 || max < min } |> {
		True -> Error(InvalidLength(min, max))
		False -> Ok(Gen((s, g) {
			upper = { s < min } |> {
				True -> min
				False -> { s > max } |> {
					True  -> max
					False -> s
				}
			}
			(left, right) = random.split(g)
			draw_int(left, upper - min + 1) |> {
				None -> None
				Some(n) -> draw_items(of, n + min, s, right, 0, []) |> {
					None        -> None
					Some(items) -> Some(list_tree(items, min))
				}
			}
		}))
	}
}

/// Equal weight for finite bit patterns under the uniform-word model, not
/// uniform over reals or a guarantee about finite seed coverage. Both zeros
/// are included. Sign and magnitude shrink independently toward positive zero.
float = () -> Gen(Float) {
	map(pair(range(0, 1), range(0, 0x7fefffffffffffff)), (parts) {
		core.float_from_bits(parts.0 * 0x8000000000000000 + parts.1)
	})
}

/// Uniform scalar rank, skipping the surrogate block; shrinks rank toward NUL.
utf_codepoint = () -> Gen(UtfCodepoint) {
	Gen((s, g) {
		sample(range(0, 0x10f7ff), s, g) |> {
			None -> None
			Some(t) -> {
				convert = (rank: Int) -> UtfCodepoint {
					n = { rank >= 0xd800 } |> {
						True  -> rank + 0x800
						False -> rank
					}
					core.int_to_codepoint(n) |> {
						Ok(c) -> c
						Error(_) -> core.int_to_codepoint(0) |> {
							Ok(c)    -> c
							Error(_) -> convert(0)
						}
					}
				}
				Some(map_tree(t, convert))
			}
		}
	})
}

/// Scalar lists of length 0..max_length, preserving valid UTF-8 while shrinking.
string = (max_length: Int) -> Result(Gen(String), BuildError) {
	list(utf_codepoint(), 0, max_length) |> {
		Error(e) -> Error(e)
		Ok(gen)  -> Ok(map(gen, core.from_codepoints))
	}
}

shrink(A) = (
	current: Tree(A)
	reason: String
	left: Int
	path: List(Int)
	checked: Int
	property: (A) -> Verdict
) -> (Tree(A), String, List(Int), Int, Bool) {
	{ left == 0 } |> {
		True -> (current, reason, lists.reverse(path), checked, True)
		False -> scan(
			current
			reason
			current.children()
			left
			path
			checked
			0
			property
		)
	}
}

scan(A) = (
	current: Tree(A)
	reason: String
	children: List(Tree(A))
	left: Int
	path: List(Int)
	checked: Int
	index: Int
	property: (A) -> Verdict
) -> (Tree(A), String, List(Int), Int, Bool) {
	{ left == 0 } |> {
		True -> (current, reason, lists.reverse(path), checked, True)
		False -> children |> {
			[] -> (current, reason, lists.reverse(path), checked, False)
			[c, ..rest] -> property(c.value) |> {
				Fail(next_reason) -> shrink(
					c
					next_reason
					left - 1
					[index, ..path]
					checked + 1
					property
				)
				_ -> scan(
					current
					reason
					rest
					left - 1
					path
					checked + 1
					index + 1
					property
				)
			}
		}
	}
}

attempts(A) = (
	gen: Gen(A)
	config: Config
	property: (A) -> Verdict
	seed: Int
	attempt: Int
	successes: Int
	discards: Int
) -> Report(A) {
	{ successes == config.cases } |> {
		True -> Passed(successes, discards)
		False -> { discards > config.max_discards } |> {
			True -> GaveUp(successes, discards)
			False -> {
				size = attempt % { config.max_size + 1 }
				drawn = at_path(random.from_seed(seed), [attempt + 1]) |> {
					None    -> None
					Some(g) -> sample(gen, size, g)
				}
				drawn |> {
					None -> attempts(
						gen
						config
						property
						seed
						attempt + 1
						successes
						discards + 1
					)
					Some(t) -> property(t.value) |> {
						Pass -> attempts(
							gen
							config
							property
							seed
							attempt + 1
							successes + 1
							discards
						)
						Discard -> attempts(
							gen
							config
							property
							seed
							attempt + 1
							successes
							discards + 1
						)
						Fail(reason) -> {
							(small, why, path, checked, limited) = shrink(
								t
								reason
								config.max_shrinks
								[]
								0
								property
							)
							Falsified(
								Failure(
									seed
									attempt
									size
									t.value
									small.value
									why
									path
									checked
									limited
								)
							)
						}
					}
				}
			}
		}
	}
}

/// Validates before any callback. Attempts j use path [j+1], size j%(max_size+1).
/// None and Discard each consume one discard. Exhausting the candidate budget
/// sets limited even when the next child list would be empty; zero never forces
/// children. The first failing child is adopted and scanning restarts at zero.
check(A) = (
	gen: Gen(A)
	config: Config
	property: (A) -> Verdict
) -> Report(A) {
	{ config.cases <= 0 } |> {
		True -> InvalidConfig("cases must be positive")
		False -> { config.max_size < 0 } |> {
			True -> InvalidConfig("max_size must be nonnegative")
			False -> { config.max_discards < 0 } |> {
				True -> InvalidConfig("max_discards must be nonnegative")
				False -> { config.max_shrinks < 0 } |> {
					True -> InvalidConfig("max_shrinks must be nonnegative")
					False -> attempts(
						gen
						config
						property
						core.bit_and(config.seed, 0xffffffffffffffff)
						0
						0
						0
					)
				}
			}
		}
	}
}

follow(A) = (t: Tree(A), path: List(Int)) -> Option(A) {
	path |> {
		[] -> Some(t.value)
		[index, ..rest] -> lists.at(t.children(), index) |> {
			None    -> None
			Some(c) -> follow(c, rest)
		}
	}
}

/// Follows retained child positions without invoking a predicate. Invalid
/// attempt, size or path entries, discarded draws and missing children miss.
replay(A) = (
	gen: Gen(A)
	seed: Int
	attempt: Int
	size: Int
	path: List(Int)
) -> Option(A) {
	{ attempt < 0 || size < 0 || lists.any(path, (i) { i < 0 }) } |> {
		True -> None
		False -> at_path(random.from_seed(seed), [attempt + 1]) |> {
			None -> None
			Some(g) -> sample(gen, size, g) |> {
				None    -> None
				Some(t) -> follow(t, path)
			}
		}
	}
}

/// Protocol v1, decimal integers and LF separators with no final LF. Calls
/// show twice only for Falsified; includes multiline text literally. No check
/// or predicate is rerun, and arbitrary identities are not serialized.
format(A) = (report: Report(A), show: (A) -> String) -> String {
	report |> {
		Passed(n = successes, d = discards) -> "std/property v1: passed; successes: " <>
			core.inspect(n) <>
			"; discards: " <>
			core.inspect(d)
		GaveUp(n = successes, d = discards) -> "std/property v1: gave up; successes: " <>
			core.inspect(n) <>
			"; discards: " <>
			core.inspect(d)
		InvalidConfig(message) -> "std/property v1: invalid config; " <> message
		Falsified(f) -> core.concat([
			"std/property v1: falsified\nseed: "
			core.inspect(f.seed)
			"\nattempt: "
			core.inspect(f.attempt)
			"\nsize: "
			core.inspect(f.size)
			"\npath: "
			core.inspect(f.path)
			"\nchecked: "
			core.inspect(f.checked)
			"\nlimited: "
			core.inspect(f.limited)
			"\noriginal: "
			show(f.original)
			"\nsmallest: "
			show(f.smallest)
			"\nreason: "
			f.reason
		])
	}
}

/// Catches each case's Assert into Fail(message). The final single assertion
/// is sent to the caller's Assert outside that catch. Stops remain stops; the
/// original source stack is not retained, only its message. No runner hook.
test(A) = (
	gen: Gen(A)
	config: Config
	property: (A) -> Nil ! Assert
	show: (A) -> String
	a ! Assert
) -> Nil ! Assert {
	report = check(gen, config, (value) {
		assertion.catch(() { property(value) }) |> {
			Ok(Nil)        -> Pass
			Error(failure) -> Fail(failure.message)
		}
	})
	report |> {
		Passed(..) -> Nil
		_          -> { a.fail(AssertionFailure(format(report, show))) }
	}
}
