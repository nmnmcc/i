//// Lexical POSIX paths. Only `/` separates names; no function consults the
//// filesystem, a current directory, or a platform's drive conventions.
//// Unicode, NUL, backslashes, and percent escapes are ordinary characters.
(
	list = std/list
	string = std/string
)

(
	RelativeError(..)
	clean
	join
	split
	base
	dir
	extension
	is_absolute
	relative
)

DifferentRoots = ()
UnknownParent = ()
/// Why a lexical relative path cannot be found.
RelativeError = DifferentRoots | UnknownParent

/// Removes empty and dot segments and resolves lexical parents. Empty relative
/// paths become `.`, and parents cannot climb above an absolute root.
clean = (path : String) -> String {
	rooted = is_absolute(path)
	go = (rest : List(String), acc : List(String)) -> String {
		rest |>
			([]) { render(list.reverse(acc), rooted) }
			| ([part, ..parts]) {
				part |>
					(p ? p == "" || p == ".") { go(parts, acc) }
					| (p ? p == "..") {
						acc |>
							([x, ..xs] ? x != "..") { go(parts, xs) }
							| (_) {
								rooted |>
									(True) { go(parts, acc) }
									| (False) { go(parts, ["..", ..acc]) }
							}
					}
					| (_) { go(parts, [part, ..acc]) }
			}
	}
	go(segments(path), [])
}

/// Joins nonempty pieces with `/`, then cleans. All-empty input gives `""`.
/// A slash at the start of a later piece does not reset earlier pieces.
join = (parts : List(String)) -> String {
	list.filter(parts, (p) { p != "" }) |>
		([]) { "" }
		| (xs) { clean(string.join(xs, "/")) }
}

/// Cuts raw input after its last slash, keeping that slash in the first part.
split = (path : String) -> (String, String) {
	go = (rest, part, directory) {
		rest |>
			([]) {
				(
					string.from_codepoints(list.reverse(directory)),
					string.from_codepoints(list.reverse(part)),
				)
			}
			| ([c, ..cs]) {
				string.utf_codepoint_to_int(c) |>
					(47) { go(cs, [], [c, ..list.append(part, directory)]) }
					| (_) { go(cs, [c, ..part], directory) }
			}
	}
	go(string.to_codepoints(path), [], [])
}

/// The final raw name after stripping trailing slashes. Empty input gives `.`,
/// and an all-slash input gives `/`. Dots are not resolved.
base = (path : String) -> String {
	go = (rest) {
		rest |>
			([]) { path |> ("") { "." } | (_) { "/" } }
			| (["", ..xs]) { go(xs) }
			| ([x, .._]) { x }
	}
	go(
		list.reverse(segments(path)),
	)
}

/// The cleaned directory part of `split`.
dir = (path : String) -> String {
	(directory, _) = split(path)
	clean(directory)
}

/// The suffix from the last dot in the final raw segment. A dotfile's leading
/// dot counts, and a trailing slash gives an empty extension.
extension = (path : String) -> String {
	(_, name) = split(path)
	go = (rest, suffix) {
		rest |>
			([]) { "" }
			| ([c, ..cs]) {
				string.utf_codepoint_to_int(c) |>
					(46) { string.from_codepoints([c, ..suffix]) }
					| (_) { go(cs, [c, ..suffix]) }
			}
	}
	go(
		list.reverse(string.to_codepoints(name)),
		[],
	)
}

/// Tests only for a leading slash.
is_absolute = (path : String) -> Bool {
	path |> ("/" <> _) { True } | (_) { False }
}

/// A lexical route from one location to another. Different roots and unmatched
/// parents in the base are errors. Neither location is implicitly a file.
relative = (from : String, to : String) -> Result(String, RelativeError) {
	a = clean(from)
	b = clean(to)
	{ is_absolute(a) != is_absolute(b) } |>
		(True) { Error(DifferentRoots) }
		| (False) {
			go = (left, right) {
				(left, right) |>
					(([x, ..xs], [y, ..ys]) ? x == y) { go(xs, ys) }
					| (_) {
						list.contains(left, "..") |>
							(True) { Error(UnknownParent) }
							| (False) {
								Ok(
									render(
										list.append(
											list.map(left, (_) { ".." }),
											right,
										),
										False,
									),
								)
							}
					}
			}
			go(clean_segments(a), clean_segments(b))
		}
}

// Convert once, then finish each segment once. Reversed codepoints and segments
// keep long names and long paths linear in the input size.
segments = (path : String) -> List(String) {
	go = (rest, part, acc) {
		rest |>
			([]) {
				list.reverse(
					[string.from_codepoints(list.reverse(part)), ..acc],
				)
			}
			| ([c, ..cs]) {
				string.utf_codepoint_to_int(c) |>
					(47) {
						go(
							cs,
							[],
							[string.from_codepoints(list.reverse(part)), ..acc],
						)
					}
					| (_) { go(cs, [c, ..part], acc) }
			}
	}
	go(string.to_codepoints(path), [], [])
}

render = (parts : List(String), rooted : Bool) -> String {
	(rooted, parts) |>
		((False, [])) { "." }
		| ((True, _)) { string.concat(["/", string.join(parts, "/")]) }
		| ((False, _)) { string.join(parts, "/") }
}

clean_segments = (path : String) -> List(String) {
	path |> (".") { [] } | (_) { list.filter(segments(path), (p) { p != "" }) }
}
