//// Unicode scalar regular expressions: ordered Thompson NFA / PikeVM.
//// Searches are O(program states * input scalars), with no backtracking.
//// Offsets are zero-based scalar offsets; captures include group zero.
//// Repeated searches (find_all, split, replacements) have Rust regex's
//// iterator cost: O(m*n^2) in the worst case, O(m*n) for bounded matches.
(
	i/core
	i/fail
	std/list
	std/dict
	std/string
	std/int
	std/bit_array
	(Option, Some, None) = std/option
)

(
	Regex
	RegexError(..)
	Match(..)
	Captures
	compile
	is_match
	find
	find_all
	captures
	group
	named
	replace
	replace_all
	split
)

RegexError = (position = Int, reason = String)

Match = (start = Int, end = Int, text = String)

Flags = (insensitive = Bool, multiline = Bool, dotall = Bool)

Literal = (Int)
Range = (Int, Int)
Digit = ()
Space = ()
Word = ()
Any = (Bool)
Union = (List(Class))
Negate = (Class)
Class = Literal | Range | Digit | Space | Word | Any | Union | Negate

Empty = ()
Atom = (Class, Bool)
Look = (Int)
Sequence = (List(Ast))
Alternate = (Ast, Ast)
Repeat = (Ast, Int, Int, Bool)
Capture = (Int, Ast)
Ast = Empty | Atom | Look | Sequence | Alternate | Repeat | Capture

Parser = (
	rest = List(Int),
	position = Int,
	flags = Flags,
	count = Int,
	names = dict.Dict(String, Int),
)

bump = (p : Parser) -> Parser {
	p.rest |>
		([_, ..tail]) { Parser(..p, rest = tail, position = p.position + 1) }
		| ([]) { panic("regex: parser advance at end") }
}

syntax_error = (p : Parser, reason : String) -> Never {
	fail.raise(RegexError(position = p.position, reason = reason))
}

peek = (p : Parser) -> Int { p.rest |> ([]) { -1 } | ([c, .._]) { c } }

digit = (c : Int) -> Bool { c >= 48 && c <= 57 }

name_char = (c : Int) -> Bool {
	{ c >= 65 && c <= 90 } || { c >= 97 && c <= 122 } || digit(c) || c == 95
}

scalar = (n : Int) -> String {
	<<n:u(32)>> |>
		(<<c:utf32be_codepoint>>) { core.from_codepoints([c]) }
		| (_) { panic("regex: scalar invariant") }
}

parse_alternation = (p : Parser) -> (Ast, Parser) {
	(left, tail) = parse_sequence(p, [])
	{ peek(tail) == 124 } |>
		(True) {
			(right, end) = parse_alternation(bump(tail))
			(Alternate(left, right), end)
		}
		| (False) { (left, tail) }
}

parse_sequence = (p : Parser, acc : List(Ast)) -> (Ast, Parser) {
	peek(p) |>
		(-1 | 41 | 124) { (Sequence(list.reverse(acc)), p) }
		| (_) {
			(a, tail) = parse_atom(p)
			(b, end) = parse_repeat(a, tail)
			parse_sequence(end, [b, ..acc])
		}
}

parse_atom = (p : Parser) -> (Ast, Parser) {
	c = peek(p)
	tail = bump(p)
	c |>
		(46) { (Atom(Any(p.flags.dotall), False), tail) }
		| (94) {
			(Look(p.flags.multiline |> (True) { 2 } | (False) { 0 }), tail)
		}
		| (36) {
			(Look(p.flags.multiline |> (True) { 3 } | (False) { 1 }), tail)
		}
		| (91) {
			(cls, end) = parse_class(tail)
			(Atom(cls, p.flags.insensitive), end)
		}
		| (92) { escape_atom(tail) }
		| (40) { parse_group(tail) }
		| (42 | 43 | 63 | 123) {
			syntax_error(p, "repetition has no expression")
		}
		| (_) { (Atom(Literal(c), p.flags.insensitive), tail) }
}

escape_class = (p : Parser) -> (Class, Parser) {
	c = peek(p)
	{ c == -1 } |>
		(True) { syntax_error(p, "trailing escape") }
		| (False) {
			tail = bump(p)
			c |>
				(100) { (Digit, tail) }
				| (68) { (Negate(Digit), tail) }
				| (119) { (Word, tail) }
				| (87) { (Negate(Word), tail) }
				| (115) { (Space, tail) }
				| (83) { (Negate(Space), tail) }
				| (110) { (Literal(10), tail) }
				| (114) { (Literal(13), tail) }
				| (116) { (Literal(9), tail) }
				| (102) { (Literal(12), tail) }
				| (118) { (Literal(11), tail) }
				| (97) { (Literal(7), tail) }
				| (120 | 117) { hex_escape(tail, c) }
				| (49 | 50 | 51 | 52 | 53 | 54 | 55 | 56 | 57 | 107 | 103) {
					syntax_error(p, "backreferences are not supported")
				}
				| (_) {
					name_char(c) |>
						(True) { syntax_error(p, "unknown escape") }
						| (False) { (Literal(c), tail) }
				}
		}
}

hex_value = (c : Int) -> Int {
	{ c >= 48 && c <= 57 } |>
		(True) { c - 48 }
		| (False) {
			{ c >= 65 && c <= 70 } |>
				(True) { c - 55 }
				| (False) {
					{ c >= 97 && c <= 102 } |>
						(True) { c - 87 }
						| (False) { -1 }
				}
		}
}

hex_digits = (p : Parser, n : Int, value : Int, braced : Bool, seen : Bool) -> (
	Class,
	Parser,
) {
	c = peek(p)
	{ { braced && c == 125 && seen } || { core.not(braced) && n == 0 } } |>
		(True) {
			{ value > 1114111 || { value >= 55296 && value <= 57343 } } |>
				(True) { syntax_error(p, "escape is not a Unicode scalar") }
				| (False) {
					(
						Literal(value),
						braced |> (True) { bump(p) } | (False) { p },
					)
				}
		}
		| (False) {
			h = hex_value(c)
			{ h < 0 || n == 0 } |>
				(True) { syntax_error(p, "invalid hex escape") }
				| (False) {
					hex_digits(bump(p), n - 1, value * 16 + h, braced, True)
				}
		}
}

hex_escape = (p : Parser, kind : Int) -> (Class, Parser) {
	{ peek(p) == 123 } |>
		(True) { hex_digits(bump(p), 6, 0, True, False) }
		| (False) {
			{ kind == 120 } |>
				(True) { hex_digits(p, 2, 0, False, False) }
				| (False) { syntax_error(p, "Unicode escape requires braces") }
		}
}

escape_atom = (p : Parser) -> (Ast, Parser) {
	peek(p) |>
		(98) { (Look(4), bump(p)) }
		| (66) { (Look(5), bump(p)) }
		| (65) { (Look(0), bump(p)) }
		| (122) { (Look(1), bump(p)) }
		| (_) {
			(c, tail) = escape_class(p)
			(Atom(c, p.flags.insensitive), tail)
		}
}

class_item = (p : Parser) -> (Class, Parser) {
	peek(p) |>
		(-1) { syntax_error(p, "unclosed character class") }
		| (92) { escape_class(bump(p)) }
		| (c) { (Literal(c), bump(p)) }
}

class_items = (p : Parser, acc : List(Class)) -> (Class, Parser) {
	peek(p) |>
		(-1) { syntax_error(p, "unclosed character class") }
		| (93) {
			acc |>
				([]) { syntax_error(p, "empty character class") }
				| (_) { (Union(list.reverse(acc)), bump(p)) }
		}
		| (_) {
			(first, tail) = class_item(p)
			tail.rest |>
				([45, next, .._] ? next != 93) {
					(last, end) = class_item(bump(tail))
					(first, last) |>
						((Literal(a), Literal(b)) ? a <= b) {
							class_items(end, [Range(a, b), ..acc])
						}
						| (_) { syntax_error(tail, "invalid character range") }
				}
				| (_) { class_items(tail, [first, ..acc]) }
		}
}

parse_class = (p : Parser) -> (Class, Parser) {
	{ peek(p) == 94 } |>
		(True) {
			(c, tail) = class_items(bump(p), [])
			(Negate(c), tail)
		}
		| (False) { class_items(p, []) }
}

read_name = (p : Parser, acc : List(String)) -> (String, Parser) {
	peek(p) |>
		(62) {
			acc |>
				([]) { syntax_error(p, "empty capture name") }
				| (_) { (core.concat(list.reverse(acc)), bump(p)) }
		}
		| (c) {
			name_char(c) |>
				(True) { read_name(bump(p), [scalar(c), ..acc]) }
				| (False) { syntax_error(p, "invalid capture name") }
		}
}

finish_group = (p : Parser, outer : Flags, id : Int) -> (Ast, Parser) {
	(body, tail) = parse_alternation(p)
	{ peek(tail) == 41 } |>
		(False) { syntax_error(tail, "unclosed group") }
		| (True) {
			a = { id < 0 } |> (True) { body } | (False) { Capture(id, body) }
			(a, Parser(..bump(tail), flags = outer))
		}
}

flag_group = (p : Parser, outer : Flags, enabled : Bool, seen : Bool) -> (
	Ast,
	Parser,
) {
	peek(p) |>
		(105) {
			flag_group(
				Parser(
					..bump(p),
					flags = Flags(..p.flags, insensitive = enabled),
				),
				outer,
				enabled,
				True,
			)
		}
		| (109) {
			flag_group(
				Parser(
					..bump(p),
					flags = Flags(..p.flags, multiline = enabled),
				),
				outer,
				enabled,
				True,
			)
		}
		| (115) {
			flag_group(
				Parser(..bump(p), flags = Flags(..p.flags, dotall = enabled)),
				outer,
				enabled,
				True,
			)
		}
		| (45) { flag_group(bump(p), outer, False, seen) }
		| (41 ? seen) { (Empty, bump(p)) }
		| (58 ? seen) { finish_group(bump(p), outer, -1) }
		| (_) {
			syntax_error(
				p,
				"unknown group or flag; lookaround is not supported",
			)
		}
}

parse_group = (p : Parser) -> (Ast, Parser) {
	{ peek(p) == 63 } |>
		(False) {
			finish_group(Parser(..p, count = p.count + 1), p.flags, p.count + 1)
		}
		| (True) {
			tail = bump(p)
			peek(tail) |>
				(58) { finish_group(bump(tail), p.flags, -1) }
				| (60) {
					start = bump(tail)
					{ peek(start) == 61 || peek(start) == 33 } |>
						(True) {
							syntax_error(start, "lookaround is not supported")
						}
						| (False) {
							(name, end) = read_name(start, [])
							{ digit(peek(start)) || dict.has(p.names, name) } |>
								(True) {
									syntax_error(
										start,
										"duplicate or invalid capture name",
									)
								}
								| (False) {
									finish_group(
										Parser(
											..end,
											count = p.count + 1,
											names = dict.insert(
												p.names,
												name,
												p.count + 1,
											),
										),
										p.flags,
										p.count + 1,
									)
								}
						}
				}
				| (_) { flag_group(tail, p.flags, True, False) }
		}
}

number = (p : Parser, value : Int, seen : Bool) -> (Int, Parser) {
	c = peek(p)
	digit(c) |>
		(True) {
			{ value > 1000 } |>
				(True) { syntax_error(p, "repetition count exceeds 1000") }
				| (False) { number(bump(p), value * 10 + c - 48, True) }
		}
		| (False) {
			{ core.not(seen) || value > 1000 } |>
				(True) {
					syntax_error(p, "invalid repetition count (maximum 1000)")
				}
				| (False) { (value, p) }
		}
}

bounds = (p : Parser) -> (Int, Int, Parser) {
	(low, tail) = number(p, 0, False)
	peek(tail) |>
		(125) { (low, low, bump(tail)) }
		| (44) {
			after = bump(tail)
			{ peek(after) == 125 } |>
				(True) { (low, -1, bump(after)) }
				| (False) {
					(high, end) = number(after, 0, False)
					{ peek(end) == 125 && high >= low } |>
						(True) { (low, high, bump(end)) }
						| (False) {
							syntax_error(end, "invalid repetition bounds")
						}
				}
		}
		| (_) { syntax_error(tail, "unclosed repetition") }
}

parse_repeat = (a : Ast, p : Parser) -> (Ast, Parser) {
	peek(p) |>
		(42 | 43 | 63 | 123) {
			(low, high, tail) = peek(p) |>
				(42) { (0, -1, bump(p)) }
				| (43) { (1, -1, bump(p)) }
				| (63) { (0, 1, bump(p)) }
				| (_) { bounds(bump(p)) }
			lazy = peek(tail) == 63
			end = lazy |> (True) { bump(tail) } | (False) { tail }
			peek(end) |>
				(42 | 43 | 63 | 123) {
					syntax_error(end, "nested repetition operator")
				}
				| (_) { (Repeat(a, low, high, lazy), end) }
		}
		| (_) { (a, p) }
}

Accept = ()
Eat = (Class, Bool, Int)
Fork = (Int, Int)
Save = (Int, Int)
Check = (Int, Int)
Inst = Accept | Eat | Fork | Save | Check

Build = (code = dict.Dict(Int, Inst), size = Int)

Regex = (
	code = dict.Dict(Int, Inst),
	start = Int,
	names = dict.Dict(String, Int),
)

emit = (inst : Inst, b : Build) -> (Int, Build) {
	(b.size, Build(code = dict.insert(b.code, b.size, inst), size = b.size + 1))
}

sequence_code = (xs : List(Ast), next : Int, b : Build) -> (Int, Build) {
	xs |>
		([]) { (next, b) }
		| ([a, ..tail]) {
			(pc, end) = lower(a, next, b)
			sequence_code(tail, pc, end)
		}
}

optional_code = (a : Ast, n : Int, lazy : Bool, next : Int, b : Build) -> (
	Int,
	Build,
) {
	{ n == 0 } |>
		(True) { (next, b) }
		| (False) {
			(body, tail) = lower(a, next, b)
			fork = lazy |>
				(True) { Fork(next, body) }
				| (False) { Fork(body, next) }
			(pc, end) = emit(fork, tail)
			optional_code(a, n - 1, lazy, pc, end)
		}
}

required_code = (a : Ast, n : Int, next : Int, b : Build) -> (Int, Build) {
	{ n == 0 } |>
		(True) { (next, b) }
		| (False) {
			(pc, end) = lower(a, next, b)
			required_code(a, n - 1, pc, end)
		}
}

nullable = (a : Ast) -> Bool {
	a |>
		(Empty | Look(_)) { True }
		| (Atom(_, _)) { False }
		| (Sequence(xs)) { list.all(xs, nullable) }
		| (Alternate(a, b)) { nullable(a) || nullable(b) }
		| (Capture(_, body)) { nullable(body) }
		| (Repeat(body, low, _, _)) { low == 0 || nullable(body) }
}

at_least_code = (body : Ast, low : Int, lazy : Bool, next : Int, b : Build) -> (
	Int,
	Build,
) {
	loop = b.size
	(child, end) = lower(body, loop, Build(code = b.code, size = b.size + 1))
	fork = lazy |> (True) { Fork(next, child) } | (False) { Fork(child, next) }
	tail = Build(..end, code = dict.insert(end.code, loop, fork))
	{ low > 0 } |>
		(True) { required_code(body, low - 1, child, tail) }
		| (False) {
			nullable(body) |>
				// Rust regex compiler c_at_least: nullable x* must be (x+)? to
				// preserve first-path priority when an epsilon loop is deduplicated.
				(True) { emit(fork, tail) }
				| (False) { (loop, tail) }
		}
}

lower = (a : Ast, next : Int, b : Build) -> (Int, Build) {
	a |>
		(Empty) { (next, b) }
		| (Atom(c, i)) {
			folded = i |> (True) { fold_class(c) } | (False) { c }
			emit(Eat(folded, False, next), b)
		}
		| (Look(k)) { emit(Check(k, next), b) }
		| (Sequence(xs)) { sequence_code(list.reverse(xs), next, b) }
		| (Alternate(left, right)) {
			(l, tail) = lower(left, next, b)
			(r, end) = lower(right, next, tail)
			emit(Fork(l, r), end)
		}
		| (Capture(id, body)) {
			(end, tail) = emit(Save(id * 2 + 1, next), b)
			(pc, more) = lower(body, end, tail)
			emit(Save(id * 2, pc), more)
		}
		| (Repeat(body, low, high, lazy)) {
			{ high == -1 } |>
				(True) { at_least_code(body, low, lazy, next, b) }
				| (False) {
					(pc, tail) = optional_code(body, high - low, lazy, next, b)
					required_code(body, low, pc, tail)
				}
		}
}

/// Compiles to an immutable NFA, or an error at a scalar position.
/// `regex.compile("(?<word>\\w+)")`.
compile = (pattern : String) -> Result(Regex, RegexError) {
	fail.catch(() {
		p = Parser(
			rest = list.map(core.codepoints(pattern), core.codepoint_to_int),
			position = 0,
			flags = Flags(
				insensitive = False,
				multiline = False,
				dotall = False,
			),
			count = 0,
			names = dict.new(),
		)
		(a, end) = parse_alternation(p)
		{ peek(end) != -1 } |>
			(True) { syntax_error(end, "unexpected closing group") }
			| (False) { Nil }
		(accept, b) = emit(Accept, Build(code = dict.new(), size = 0))
		(pc, tail) = lower(Capture(0, a), accept, b)
		Regex(code = tail.code, start = pc, names = end.names)
	})
}

// Unicode 16 property tables and simple folds follow regex-syntax's UCD data.
// Binary search over packed range endpoints is logarithmic in table size.
in_ranges = (table : BitArray, c : Int) -> Bool {
	go = (low, high) {
		{ low >= high } |>
			(True) { False }
			| (False) {
				mid = { low + high } / 2
				table |>
					(<<_:bytes(mid * 6), a:u(24), b:u(24), _:bits>>) {
						{ c < a } |>
							(True) { go(low, mid) }
							| (False) {
								{ c > b } |>
									(True) { go(mid + 1, high) }
									| (False) { True }
							}
					}
					| (_) { panic("regex: property table index") }
			}
	}
	go(0, core.bit_size(table) / 48)
}

class_has = (cls : Class, c : Int) -> Bool {
	cls |>
		(Literal(n)) { n == c }
		| (Range(low, high)) { c >= low && c <= high }
		| (Digit) { in_ranges(unicode_digit, c) }
		| (Word) { in_ranges(unicode_word, c) }
		| (Space) { in_ranges(unicode_space, c) }
		| (Any(dotall)) { dotall || c != 10 }
		| (Union(xs)) { list.any(xs, (x) { class_has(x, c) }) }
		| (Negate(x)) { core.not(class_has(x, c)) }
}

// Fold classes at compile time, before applying their negation. This matters
// for e.g. (?i)[^a], which must reject both a and A.
fold_class = (cls : Class) -> Class {
	cls |>
		(Negate(x)) { Negate(fold_class(x)) }
		| (Union(xs)) { Union(list.map(xs, fold_class)) }
		| (_) {
			collect = (table, acc) {
				table |>
					(<<>>) { list.reverse(acc) }
					| (<<c:u(24), a:u(24), b:u(24), d:u(24), tail:bits>>) {
						has = class_has(cls, a) ||
							{ b != 16777215 && class_has(cls, b) } ||
							{ d != 16777215 && class_has(cls, d) }
						has |>
							(True) { collect(tail, [Literal(c), ..acc]) }
							| (False) { collect(tail, acc) }
					}
					| (_) { panic("regex: simple fold table") }
			}
			extras = collect(fold_pairs, [])
			Union([cls, ..extras])
		}
}

Offset = (scalar = Int, byte = Int)

Input = (text = String, rest = List(Int), previous = Int, at = Offset)

Thread = (Int, dict.Dict(Int, Offset))

Captures = (
	slots = dict.Dict(Int, Offset),
	names = dict.Dict(String, Int),
	text = String,
)

input = (text : String) -> Input {
	Input(
		text = text,
		rest = list.map(core.codepoints(text), core.codepoint_to_int),
		previous = -1,
		at = Offset(scalar = 0, byte = 0),
	)
}

byte_width = (c : Int) -> Int {
	{ c < 128 } |>
		(True) { 1 }
		| (False) {
			{ c < 2048 } |>
				(True) { 2 }
				| (False) { { c < 65536 } |> (True) { 3 } | (False) { 4 } }
		}
}

advance = (i : Input) -> Input {
	i.rest |>
		([]) { panic("regex: advance after end") }
		| ([c, ..tail]) {
			Input(
				text = i.text,
				rest = tail,
				previous = c,
				at = Offset(
					scalar = i.at.scalar + 1,
					byte = i.at.byte + byte_width(c),
				),
			)
		}
}

at_end = (i : Input) -> Bool { i.rest == [] }

word_at = (c : Int) -> Bool { c >= 0 && in_ranges(unicode_word, c) }

assertion = (kind : Int, i : Input) -> Bool {
	following = i.rest |> ([]) { -1 } | ([c, .._]) { c }
	kind |>
		(0) { i.at.scalar == 0 }
		| (1) { at_end(i) }
		| (2) { i.at.scalar == 0 || i.previous == 10 }
		| (3) { following == -1 || following == 10 }
		| (4) { word_at(i.previous) != word_at(following) }
		| (5) { word_at(i.previous) == word_at(following) }
		| (_) { panic("regex: assertion opcode") }
}

instruction = (re : Regex, pc : Int) -> Inst {
	dict.get(re.code, pc) |>
		(Some(i)) { i }
		| (None) { panic("regex: instruction address") }
}

// Priority-ordered depth-first epsilon closure. Mark before following a Fork
// to terminate empty cycles and retain the first path's capture slots.
closure = (
	re : Regex,
	pending : List(Thread),
	i : Input,
	seen : dict.Dict(Int, Bool),
	acc : List(Thread)
) -> List(Thread) {
	pending |>
		([]) { list.reverse(acc) }
		| ([thread, ..rest]) {
			Thread(pc, slots) = thread
			dict.has(seen, pc) |>
				(True) { closure(re, rest, i, seen, acc) }
				| (False) {
					visited = dict.insert(seen, pc, True)
					instruction(re, pc) |>
						(Fork(a, b)) {
							closure(
								re,
								[Thread(a, slots), Thread(b, slots), ..rest],
								i,
								visited,
								acc,
							)
						}
						| (Save(slot, next)) {
							closure(
								re,
								[
									Thread(
										next,
										dict.insert(slots, slot, i.at),
									),
									..rest,
								],
								i,
								visited,
								acc,
							)
						}
						| (Check(kind, next)) {
							assertion(kind, i) |>
								(True) {
									closure(
										re,
										[Thread(next, slots), ..rest],
										i,
										visited,
										acc,
									)
								}
								| (False) { closure(re, rest, i, visited, acc) }
						}
						| (_) { closure(re, rest, i, visited, [thread, ..acc]) }
				}
		}
}

step_threads = (
	re : Regex,
	threads : List(Thread),
	i : Input,
	acc : List(Thread),
	best : Option(Captures)
) -> (List(Thread), Option(Captures)) {
	threads |>
		([]) { (list.reverse(acc), best) }
		| ([thread, ..tail]) {
			Thread(pc, slots) = thread
			instruction(re, pc) |>
				(Accept) {
					(
						list.reverse(acc),
						Some(
							Captures(
								slots = slots,
								names = re.names,
								text = i.text,
							),
						),
					)
				}
				| (Eat(cls, _, next)) {
					i.rest |>
						([c, .._] ? class_has(cls, c)) {
							step_threads(
								re,
								tail,
								i,
								[Thread(next, slots), ..acc],
								best,
							)
						}
						| (_) { step_threads(re, tail, i, acc, best) }
				}
				| (_) { panic("regex: epsilon instruction in terminal set") }
		}
}

search_loop = (
	re : Regex,
	i : Input,
	threads : List(Thread),
	best : Option(Captures)
) -> Option(Captures) {
	active = best |>
		(None) { list.append(threads, [Thread(re.start, dict.new())]) }
		| (Some(_)) { threads }
	expanded = closure(re, active, i, dict.new(), [])
	(next, found) = step_threads(re, expanded, i, [], best)
	{ at_end(i) || { next == [] && found != None } } |>
		(True) { found }
		| (False) { search_loop(re, advance(i), next, found) }
}

bytes = (text : String, start : Int, end : Int) -> String {
	core.byte_slice(text, start, end - start) |>
		(Ok(s)) { s }
		| (Error(_)) { panic("regex: capture byte boundaries") }
}

/// Capture by index; zero is the full match. An unmatched group is None.
/// `regex.group(captures, 1)`.
group = (captures : Captures, index : Int) -> Option(Match) {
	{ index < 0 } |>
		(True) { None }
		| (False) {
			(
				dict.get(captures.slots, index * 2),
				dict.get(captures.slots, index * 2 + 1),
			) |>
				((Some(start), Some(end))) {
					Some(
						Match(
							start = start.scalar,
							end = end.scalar,
							text = bytes(captures.text, start.byte, end.byte),
						),
					)
				}
				| (_) { None }
		}
}

/// Capture by name, or None for an unknown/unmatched name.
named = (captures : Captures, name : String) -> Option(Match) {
	dict.get(captures.names, name) |>
		(None) { None }
		| (Some(id)) { group(captures, id) }
}

whole = (captures : Captures) -> Match {
	group(captures, 0) |>
		(Some(m)) { m }
		| (None) { panic("regex: full match capture") }
}

/// The leftmost-first capture set, including all participating groups.
captures = (regex : Regex, input : String) -> Option(Captures) {
	search_loop(regex, regex_input(input), [], None)
}

regex_input = input

/// Finds the leftmost-first match: `regex.find(re, "abc")`.
find = (regex : Regex, input : String) -> Option(Match) {
	captures(regex, input) |> (None) { None } | (Some(c)) { Some(whole(c)) }
}

/// Whether any match exists, including an empty match.
is_match = (regex : Regex, input : String) -> Bool {
	captures(regex, input) != None
}

move_to = (i : Input, scalar : Int) -> Input {
	{ i.at.scalar == scalar } |>
		(True) { i }
		| (False) { move_to(advance(i), scalar) }
}

// Rust's empty-match iteration rule: suppress an empty match immediately
// after a nonempty match, and otherwise advance one complete scalar.
scan_all = (
	re : Regex,
	i : Input,
	last_end : Int,
	acc : List(Captures)
) -> List(Captures) {
	search_loop(re, i, [], None) |>
		(None) { list.reverse(acc) }
		| (Some(c)) {
			m = whole(c)
			empty = m.start == m.end
			end = move_to(i, m.end)
			{ empty && m.end == last_end } |>
				(True) {
					at_end(end) |>
						(True) { list.reverse(acc) }
						| (False) { scan_all(re, advance(end), last_end, acc) }
				}
				| (False) {
					empty |>
						(True) {
							at_end(end) |>
								(True) { list.reverse([c, ..acc]) }
								| (False) {
									scan_all(
										re,
										advance(end),
										m.end,
										[c, ..acc],
									)
								}
						}
						| (False) { scan_all(re, end, m.end, [c, ..acc]) }
				}
		}
}

/// Non-overlapping matches, advancing by scalars after empty matches.
find_all = (regex : Regex, input : String) -> List(Match) {
	list.map(
		scan_all(regex, regex_input(input), -1, []),
		whole,
	)
}

slot = (c : Captures, n : Int) -> Offset {
	dict.get(c.slots, n) |>
		(Some(x)) { x }
		| (None) { panic("regex: whole capture slot") }
}

replacement_value = (c : Captures, name : String) -> String {
	m = int.parse(name) |>
		(Ok(n)) { group(c, n) }
		| (Error(_)) { named(c, name) }
	m |> (Some(v)) { v.text } | (None) { "" }
}

replacement_name = (xs : List(Int), braced : Bool, acc : List(String)) -> (
	String,
	List(Int),
	Bool,
) {
	xs |>
		([125, ..tail] ? braced) {
			(core.concat(list.reverse(acc)), tail, True)
		}
		| ([c, ..tail] ? name_char(c)) {
			replacement_name(tail, braced, [scalar(c), ..acc])
		}
		| (_) { (core.concat(list.reverse(acc)), xs, core.not(braced)) }
}

expand_replacement = (
	xs : List(Int),
	c : Captures,
	acc : List(String)
) -> String {
	xs |>
		([]) { core.concat(list.reverse(acc)) }
		| ([36, 36, ..tail]) { expand_replacement(tail, c, ["$", ..acc]) }
		| ([36, ..tail]) {
			braced = tail |> ([123, .._]) { True } | (_) { False }
			start = braced |> (True) { list.drop(tail, 1) } | (False) { tail }
			(name, end, closed) = replacement_name(start, braced, [])
			{ name != "" && closed } |>
				(True) {
					expand_replacement(
						end,
						c,
						[replacement_value(c, name), ..acc],
					)
				}
				| (False) { expand_replacement(tail, c, ["$", ..acc]) }
		}
		| ([x, ..tail]) { expand_replacement(tail, c, [scalar(x), ..acc]) }
}

replace_captures = (
	matches : List(Captures),
	text : String,
	replacement : List(Int),
	at : Int,
	acc : List(String)
) -> String {
	matches |>
		([]) {
			core.concat(
				list.reverse(
					[bytes(text, at, core.byte_size(text)), ..acc],
				),
			)
		}
		| ([c, ..tail]) {
			replace_captures(
				tail,
				text,
				replacement,
				slot(c, 1).byte,
				[
					expand_replacement(replacement, c, []),
					bytes(text, at, slot(c, 0).byte),
					..acc,
				],
			)
		}
}

/// Replaces the first match; `$1`, `${name}`, and `$$` expand captures/dollar.
/// Missing groups expand to empty text. `$1x` names group `1x`; use `${1}x`.
replace = (regex : Regex, input : String, with : String) -> String {
	matches = captures(regex, input) |> (None) { [] } | (Some(c)) { [c] }
	replace_captures(
		matches,
		input,
		list.map(core.codepoints(with), core.codepoint_to_int),
		0,
		[],
	)
}

/// Replaces all non-overlapping matches with capture expansion.
replace_all = (regex : Regex, input : String, with : String) -> String {
	replace_captures(
		scan_all(regex, regex_input(input), -1, []),
		input,
		list.map(core.codepoints(with), core.codepoint_to_int),
		0,
		[],
	)
}

/// Splits around every non-overlapping match, preserving empty pieces.
split = (regex : Regex, input : String) -> List(String) {
	go = (matches, at, acc) {
		matches |>
			([]) {
				list.reverse(
					[bytes(input, at, core.byte_size(input)), ..acc],
				)
			}
			| ([c, ..tail]) {
				go(
					tail,
					slot(c, 1).byte,
					[bytes(input, at, slot(c, 0).byte), ..acc],
				)
			}
	}
	go(
		scan_all(regex, regex_input(input), -1, []),
		0,
		[],
	)
}

// Derived Unicode data. See docs/design/std-4.md and the Unicode license.
unicode_word = table(
	"00003000003900004100005a00005f00005f00006100007a0000aa0000aa0000b50000b50000ba0000ba0000c00000d60000d80000f60000f80002c10002c60002d10002e00002e40002ec0002ec0002ee0002ee00030000037400037600037700037a00037d00037f00037f00038600038600038800038a00038c00038c00038e0003a10003a30003f50003f700048100048300052f0005310005560005590005590005600005880005910005bd0005bf0005bf0005c10005c20005c40005c50005c70005c70005d00005ea0005ef0005f200061000061a00062000066900066e0006d30006d50006dc0006df0006e80006ea0006fc0006ff0006ff00071000074a00074d0007b10007c00007f50007fa0007fa0007fd0007fd00080000082d00084000085b00086000086a00087000088700088900088e0008970008e10008e300096300096600096f00097100098300098500098c00098f0009900009930009a80009aa0009b00009b20009b20009b60009b90009bc0009c40009c70009c80009cb0009ce0009d70009d70009dc0009dd0009df0009e30009e60009f10009fc0009fc0009fe0009fe000a01000a03000a05000a0a000a0f000a10000a13000a28000a2a000a30000a32000a33000a35000a36000a38000a39000a3c000a3c000a3e000a42000a47000a48000a4b000a4d000a51000a51000a59000a5c000a5e000a5e000a66000a75000a81000a83000a85000a8d000a8f000a91000a93000aa8000aaa000ab0000ab2000ab3000ab5000ab9000abc000ac5000ac7000ac9000acb000acd000ad0000ad0000ae0000ae3000ae6000aef000af9000aff000b01000b03000b05000b0c000b0f000b10000b13000b28000b2a000b30000b32000b33000b35000b39000b3c000b44000b47000b48000b4b000b4d000b55000b57000b5c000b5d000b5f000b63000b66000b6f000b71000b71000b82000b83000b85000b8a000b8e000b90000b92000b95000b99000b9a000b9c000b9c000b9e000b9f000ba3000ba4000ba8000baa000bae000bb9000bbe000bc2000bc6000bc8000bca000bcd000bd0000bd0000bd7000bd7000be6000bef000c00000c0c000c0e000c10000c12000c28000c2a000c39000c3c000c44000c46000c48000c4a000c4d000c55000c56000c58000c5a000c5d000c5d000c60000c63000c66000c6f000c80000c83000c85000c8c000c8e000c90000c92000ca8000caa000cb3000cb5000cb9000cbc000cc4000cc6000cc8000cca000ccd000cd5000cd6000cdd000cde000ce0000ce3000ce6000cef000cf1000cf3000d00000d0c000d0e000d10000d12000d44000d46000d48000d4a000d4e000d54000d57000d5f000d63000d66000d6f000d7a000d7f000d81000d83000d85000d96000d9a000db1000db3000dbb000dbd000dbd000dc0000dc6000dca000dca000dcf000dd4000dd6000dd6000dd8000ddf000de6000def000df2000df3000e01000e3a000e40000e4e000e50000e59000e81000e82000e84000e84000e86000e8a000e8c000ea3000ea5000ea5000ea7000ebd000ec0000ec4000ec6000ec6000ec8000ece000ed0000ed9000edc000edf000f00000f00000f18000f19000f20000f29000f35000f35000f37000f37000f39000f39000f3e000f47000f49000f6c000f71000f84000f86000f97000f99000fbc000fc6000fc600100000104900105000109d0010a00010c50010c70010c70010cd0010cd0010d00010fa0010fc00124800124a00124d00125000125600125800125800125a00125d00126000128800128a00128d0012900012b00012b20012b50012b80012be0012c00012c00012c20012c50012c80012d60012d800131000131200131500131800135a00135d00135f00138000138f0013a00013f50013f80013fd00140100166c00166f00167f00168100169a0016a00016ea0016ee0016f800170000171500171f00173400174000175300176000176c00176e0017700017720017730017800017d30017d70017d70017dc0017dd0017e00017e900180b00180d00180f0018190018200018780018800018aa0018b00018f500190000191e00192000192b00193000193b00194600196d0019700019740019800019ab0019b00019c90019d00019d9001a00001a1b001a20001a5e001a60001a7c001a7f001a89001a90001a99001aa7001aa7001ab0001ace001b00001b4c001b50001b59001b6b001b73001b80001bf3001c00001c37001c40001c49001c4d001c7d001c80001c8a001c90001cba001cbd001cbf001cd0001cd2001cd4001cfa001d00001f15001f18001f1d001f20001f45001f48001f4d001f50001f57001f59001f59001f5b001f5b001f5d001f5d001f5f001f7d001f80001fb4001fb6001fbc001fbe001fbe001fc2001fc4001fc6001fcc001fd0001fd3001fd6001fdb001fe0001fec001ff2001ff4001ff6001ffc00200c00200d00203f00204000205400205400207100207100207f00207f00209000209c0020d00020f000210200210200210700210700210a00211300211500211500211900211d00212400212400212600212600212800212800212a00212d00212f00213900213c00213f00214500214900214e00214e0021600021880024b60024e9002c00002ce4002ceb002cf3002d00002d25002d27002d27002d2d002d2d002d30002d67002d6f002d6f002d7f002d96002da0002da6002da8002dae002db0002db6002db8002dbe002dc0002dc6002dc8002dce002dd0002dd6002dd8002dde002de0002dff002e2f002e2f00300500300700302100302f00303100303500303800303c00304100309600309900309a00309d00309f0030a10030fa0030fc0030ff00310500312f00313100318e0031a00031bf0031f00031ff003400004dbf004e0000a48c00a4d000a4fd00a50000a60c00a61000a62b00a64000a67200a67400a67d00a67f00a6f100a71700a71f00a72200a78800a78b00a7cd00a7d000a7d100a7d300a7d300a7d500a7dc00a7f200a82700a82c00a82c00a84000a87300a88000a8c500a8d000a8d900a8e000a8f700a8fb00a8fb00a8fd00a92d00a93000a95300a96000a97c00a98000a9c000a9cf00a9d900a9e000a9fe00aa0000aa3600aa4000aa4d00aa5000aa5900aa6000aa7600aa7a00aac200aadb00aadd00aae000aaef00aaf200aaf600ab0100ab0600ab0900ab0e00ab1100ab1600ab2000ab2600ab2800ab2e00ab3000ab5a00ab5c00ab6900ab7000abea00abec00abed00abf000abf900ac0000d7a300d7b000d7c600d7cb00d7fb00f90000fa6d00fa7000fad900fb0000fb0600fb1300fb1700fb1d00fb2800fb2a00fb3600fb3800fb3c00fb3e00fb3e00fb4000fb4100fb4300fb4400fb4600fbb100fbd300fd3d00fd5000fd8f00fd9200fdc700fdf000fdfb00fe0000fe0f00fe2000fe2f00fe3300fe3400fe4d00fe4f00fe7000fe7400fe7600fefc00ff1000ff1900ff2100ff3a00ff3f00ff3f00ff4100ff5a00ff6600ffbe00ffc200ffc700ffca00ffcf00ffd200ffd700ffda00ffdc01000001000b01000d01002601002801003a01003c01003d01003f01004d01005001005d0100800100fa0101400101740101fd0101fd01028001029c0102a00102d00102e00102e001030001031f01032d01034a01035001037a01038001039d0103a00103c30103c80103cf0103d10103d501040001049d0104a00104a90104b00104d30104d80104fb01050001052701053001056301057001057a01057c01058a01058c0105920105940105950105970105a10105a30105b10105b30105b90105bb0105bc0105c00105f30106000107360107400107550107600107670107800107850107870107b00107b20107ba01080001080501080801080801080a01083501083701083801083c01083c01083f01085501086001087601088001089e0108e00108f20108f40108f50109000109150109200109390109800109b70109be0109bf010a00010a03010a05010a06010a0c010a13010a15010a17010a19010a35010a38010a3a010a3f010a3f010a60010a7c010a80010a9c010ac0010ac7010ac9010ae6010b00010b35010b40010b55010b60010b72010b80010b91010c00010c48010c80010cb2010cc0010cf2010d00010d27010d30010d39010d40010d65010d69010d6d010d6f010d85010e80010ea9010eab010eac010eb0010eb1010ec2010ec4010efc010f1c010f27010f27010f30010f50010f70010f85010fb0010fc4010fe0010ff601100001104601106601107501107f0110ba0110c20110c20110d00110e80110f00110f901110001113401113601113f0111440111470111500111730111760111760111800111c40111c90111cc0111ce0111da0111dc0111dc01120001121101121301123701123e01124101128001128601128801128801128a01128d01128f01129d01129f0112a80112b00112ea0112f00112f901130001130301130501130c01130f01131001131301132801132a01133001133201133301133501133901133b01134401134701134801134b01134d01135001135001135701135701135d01136301136601136c01137001137401138001138901138b01138b01138e01138e0113900113b50113b70113c00113c20113c20113c50113c50113c70113ca0113cc0113d30113e10113e201140001144a01145001145901145e0114610114800114c50114c70114c70114d00114d90115800115b50115b80115c00115d80115dd0116000116400116440116440116500116590116800116b80116c00116c90116d00116e301170001171a01171d01172b01173001173901174001174601180001183a0118a00118e90118ff01190601190901190901190c01191301191501191601191801193501193701193801193b0119430119500119590119a00119a70119aa0119d70119da0119e10119e30119e4011a00011a3e011a47011a47011a50011a99011a9d011a9d011ab0011af8011bc0011be0011bf0011bf9011c00011c08011c0a011c36011c38011c40011c50011c59011c72011c8f011c92011ca7011ca9011cb6011d00011d06011d08011d09011d0b011d36011d3a011d3a011d3c011d3d011d3f011d47011d50011d59011d60011d65011d67011d68011d6a011d8e011d90011d91011d93011d98011da0011da9011ee0011ef6011f00011f10011f12011f3a011f3e011f42011f50011f5a011fb0011fb001200001239901240001246e012480012543012f90012ff001300001342f0134400134550134600143fa014400014646016100016139016800016a38016a40016a5e016a60016a69016a70016abe016ac0016ac9016ad0016aed016af0016af4016b00016b36016b40016b43016b50016b59016b63016b77016b7d016b8f016d40016d6c016d70016d79016e40016e7f016f00016f4a016f4f016f87016f8f016f9f016fe0016fe1016fe3016fe4016ff0016ff10170000187f7018800018cd5018cff018d0801aff001aff301aff501affb01affd01affe01b00001b12201b13201b13201b15001b15201b15501b15501b16401b16701b17001b2fb01bc0001bc6a01bc7001bc7c01bc8001bc8801bc9001bc9901bc9d01bc9e01ccf001ccf901cf0001cf2d01cf3001cf4601d16501d16901d16d01d17201d17b01d18201d18501d18b01d1aa01d1ad01d24201d24401d40001d45401d45601d49c01d49e01d49f01d4a201d4a201d4a501d4a601d4a901d4ac01d4ae01d4b901d4bb01d4bb01d4bd01d4c301d4c501d50501d50701d50a01d50d01d51401d51601d51c01d51e01d53901d53b01d53e01d54001d54401d54601d54601d54a01d55001d55201d6a501d6a801d6c001d6c201d6da01d6dc01d6fa01d6fc01d71401d71601d73401d73601d74e01d75001d76e01d77001d78801d78a01d7a801d7aa01d7c201d7c401d7cb01d7ce01d7ff01da0001da3601da3b01da6c01da7501da7501da8401da8401da9b01da9f01daa101daaf01df0001df1e01df2501df2a01e00001e00601e00801e01801e01b01e02101e02301e02401e02601e02a01e03001e06d01e08f01e08f01e10001e12c01e13001e13d01e14001e14901e14e01e14e01e29001e2ae01e2c001e2f901e4d001e4f901e5d001e5fa01e7e001e7e601e7e801e7eb01e7ed01e7ee01e7f001e7fe01e80001e8c401e8d001e8d601e90001e94b01e95001e95901ee0001ee0301ee0501ee1f01ee2101ee2201ee2401ee2401ee2701ee2701ee2901ee3201ee3401ee3701ee3901ee3901ee3b01ee3b01ee4201ee4201ee4701ee4701ee4901ee4901ee4b01ee4b01ee4d01ee4f01ee5101ee5201ee5401ee5401ee5701ee5701ee5901ee5901ee5b01ee5b01ee5d01ee5d01ee5f01ee5f01ee6101ee6201ee6401ee6401ee6701ee6a01ee6c01ee7201ee7401ee7701ee7901ee7c01ee7e01ee7e01ee8001ee8901ee8b01ee9b01eea101eea301eea501eea901eeab01eebb01f13001f14901f15001f16901f17001f18901fbf001fbf902000002a6df02a70002b73902b74002b81d02b82002cea102ceb002ebe002ebf002ee5d02f80002fa1d03000003134a0313500323af0e01000e01ef",
)

unicode_digit = table(
	"0000300000390006600006690006f00006f90007c00007c900096600096f0009e60009ef000a66000a6f000ae6000aef000b66000b6f000be6000bef000c66000c6f000ce6000cef000d66000d6f000de6000def000e50000e59000ed0000ed9000f20000f290010400010490010900010990017e00017e900181000181900194600194f0019d00019d9001a80001a89001a90001a99001b50001b59001bb0001bb9001c40001c49001c50001c5900a62000a62900a8d000a8d900a90000a90900a9d000a9d900a9f000a9f900aa5000aa5900abf000abf900ff1000ff190104a00104a9010d30010d39010d40010d4901106601106f0110f00110f901113601113f0111d00111d90112f00112f90114500114590114d00114d90116500116590116c00116c90116d00116e30117300117390118e00118e9011950011959011bf0011bf9011c50011c59011d50011d59011da0011da9011f50011f59016130016139016a60016a69016ac0016ac9016b50016b59016d70016d7901ccf001ccf901d7ce01d7ff01e14001e14901e2f001e2f901e4f001e4f901e5f101e5fa01e95001e95901fbf001fbf9",
)

unicode_space = table(
	"00000900000d0000200000200000850000850000a00000a000168000168000200000200a00202800202900202f00202f00205f00205f003000003000",
)

fold_pairs = table(
	"000041000061ffffffffffff000042000062ffffffffffff000043000063ffffffffffff000044000064ffffffffffff000045000065ffffffffffff000046000066ffffffffffff000047000067ffffffffffff000048000068ffffffffffff000049000069ffffffffffff00004a00006affffffffffff00004b00006b00212affffff00004c00006cffffffffffff00004d00006dffffffffffff00004e00006effffffffffff00004f00006fffffffffffff000050000070ffffffffffff000051000071ffffffffffff000052000072ffffffffffff00005300007300017fffffff000054000074ffffffffffff000055000075ffffffffffff000056000076ffffffffffff000057000077ffffffffffff000058000078ffffffffffff000059000079ffffffffffff00005a00007affffffffffff000061000041ffffffffffff000062000042ffffffffffff000063000043ffffffffffff000064000044ffffffffffff000065000045ffffffffffff000066000046ffffffffffff000067000047ffffffffffff000068000048ffffffffffff000069000049ffffffffffff00006a00004affffffffffff00006b00004b00212affffff00006c00004cffffffffffff00006d00004dffffffffffff00006e00004effffffffffff00006f00004fffffffffffff000070000050ffffffffffff000071000051ffffffffffff000072000052ffffffffffff00007300005300017fffffff000074000054ffffffffffff000075000055ffffffffffff000076000056ffffffffffff000077000057ffffffffffff000078000058ffffffffffff000079000059ffffffffffff00007a00005affffffffffff0000b500039c0003bcffffff0000c00000e0ffffffffffff0000c10000e1ffffffffffff0000c20000e2ffffffffffff0000c30000e3ffffffffffff0000c40000e4ffffffffffff0000c50000e500212bffffff0000c60000e6ffffffffffff0000c70000e7ffffffffffff0000c80000e8ffffffffffff0000c90000e9ffffffffffff0000ca0000eaffffffffffff0000cb0000ebffffffffffff0000cc0000ecffffffffffff0000cd0000edffffffffffff0000ce0000eeffffffffffff0000cf0000efffffffffffff0000d00000f0ffffffffffff0000d10000f1ffffffffffff0000d20000f2ffffffffffff0000d30000f3ffffffffffff0000d40000f4ffffffffffff0000d50000f5ffffffffffff0000d60000f6ffffffffffff0000d80000f8ffffffffffff0000d90000f9ffffffffffff0000da0000faffffffffffff0000db0000fbffffffffffff0000dc0000fcffffffffffff0000dd0000fdffffffffffff0000de0000feffffffffffff0000df001e9effffffffffff0000e00000c0ffffffffffff0000e10000c1ffffffffffff0000e20000c2ffffffffffff0000e30000c3ffffffffffff0000e40000c4ffffffffffff0000e50000c500212bffffff0000e60000c6ffffffffffff0000e70000c7ffffffffffff0000e80000c8ffffffffffff0000e90000c9ffffffffffff0000ea0000caffffffffffff0000eb0000cbffffffffffff0000ec0000ccffffffffffff0000ed0000cdffffffffffff0000ee0000ceffffffffffff0000ef0000cfffffffffffff0000f00000d0ffffffffffff0000f10000d1ffffffffffff0000f20000d2ffffffffffff0000f30000d3ffffffffffff0000f40000d4ffffffffffff0000f50000d5ffffffffffff0000f60000d6ffffffffffff0000f80000d8ffffffffffff0000f90000d9ffffffffffff0000fa0000daffffffffffff0000fb0000dbffffffffffff0000fc0000dcffffffffffff0000fd0000ddffffffffffff0000fe0000deffffffffffff0000ff000178ffffffffffff000100000101ffffffffffff000101000100ffffffffffff000102000103ffffffffffff000103000102ffffffffffff000104000105ffffffffffff000105000104ffffffffffff000106000107ffffffffffff000107000106ffffffffffff000108000109ffffffffffff000109000108ffffffffffff00010a00010bffffffffffff00010b00010affffffffffff00010c00010dffffffffffff00010d00010cffffffffffff00010e00010fffffffffffff00010f00010effffffffffff000110000111ffffffffffff000111000110ffffffffffff000112000113ffffffffffff000113000112ffffffffffff000114000115ffffffffffff000115000114ffffffffffff000116000117ffffffffffff000117000116ffffffffffff000118000119ffffffffffff000119000118ffffffffffff00011a00011bffffffffffff00011b00011affffffffffff00011c00011dffffffffffff00011d00011cffffffffffff00011e00011fffffffffffff00011f00011effffffffffff000120000121ffffffffffff000121000120ffffffffffff000122000123ffffffffffff000123000122ffffffffffff000124000125ffffffffffff000125000124ffffffffffff000126000127ffffffffffff000127000126ffffffffffff000128000129ffffffffffff000129000128ffffffffffff00012a00012bffffffffffff00012b00012affffffffffff00012c00012dffffffffffff00012d00012cffffffffffff00012e00012fffffffffffff00012f00012effffffffffff000132000133ffffffffffff000133000132ffffffffffff000134000135ffffffffffff000135000134ffffffffffff000136000137ffffffffffff000137000136ffffffffffff00013900013affffffffffff00013a000139ffffffffffff00013b00013cffffffffffff00013c00013bffffffffffff00013d00013effffffffffff00013e00013dffffffffffff00013f000140ffffffffffff00014000013fffffffffffff000141000142ffffffffffff000142000141ffffffffffff000143000144ffffffffffff000144000143ffffffffffff000145000146ffffffffffff000146000145ffffffffffff000147000148ffffffffffff000148000147ffffffffffff00014a00014bffffffffffff00014b00014affffffffffff00014c00014dffffffffffff00014d00014cffffffffffff00014e00014fffffffffffff00014f00014effffffffffff000150000151ffffffffffff000151000150ffffffffffff000152000153ffffffffffff000153000152ffffffffffff000154000155ffffffffffff000155000154ffffffffffff000156000157ffffffffffff000157000156ffffffffffff000158000159ffffffffffff000159000158ffffffffffff00015a00015bffffffffffff00015b00015affffffffffff00015c00015dffffffffffff00015d00015cffffffffffff00015e00015fffffffffffff00015f00015effffffffffff000160000161ffffffffffff000161000160ffffffffffff000162000163ffffffffffff000163000162ffffffffffff000164000165ffffffffffff000165000164ffffffffffff000166000167ffffffffffff000167000166ffffffffffff000168000169ffffffffffff000169000168ffffffffffff00016a00016bffffffffffff00016b00016affffffffffff00016c00016dffffffffffff00016d00016cffffffffffff00016e00016fffffffffffff00016f00016effffffffffff000170000171ffffffffffff000171000170ffffffffffff000172000173ffffffffffff000173000172ffffffffffff000174000175ffffffffffff000175000174ffffffffffff000176000177ffffffffffff000177000176ffffffffffff0001780000ffffffffffffff00017900017affffffffffff00017a000179ffffffffffff00017b00017cffffffffffff00017c00017bffffffffffff00017d00017effffffffffff00017e00017dffffffffffff00017f000053000073ffffff000180000243ffffffffffff000181000253ffffffffffff000182000183ffffffffffff000183000182ffffffffffff000184000185ffffffffffff000185000184ffffffffffff000186000254ffffffffffff000187000188ffffffffffff000188000187ffffffffffff000189000256ffffffffffff00018a000257ffffffffffff00018b00018cffffffffffff00018c00018bffffffffffff00018e0001ddffffffffffff00018f000259ffffffffffff00019000025bffffffffffff000191000192ffffffffffff000192000191ffffffffffff000193000260ffffffffffff000194000263ffffffffffff0001950001f6ffffffffffff000196000269ffffffffffff000197000268ffffffffffff000198000199ffffffffffff000199000198ffffffffffff00019a00023dffffffffffff00019b00a7dcffffffffffff00019c00026fffffffffffff00019d000272ffffffffffff00019e000220ffffffffffff00019f000275ffffffffffff0001a00001a1ffffffffffff0001a10001a0ffffffffffff0001a20001a3ffffffffffff0001a30001a2ffffffffffff0001a40001a5ffffffffffff0001a50001a4ffffffffffff0001a6000280ffffffffffff0001a70001a8ffffffffffff0001a80001a7ffffffffffff0001a9000283ffffffffffff0001ac0001adffffffffffff0001ad0001acffffffffffff0001ae000288ffffffffffff0001af0001b0ffffffffffff0001b00001afffffffffffff0001b100028affffffffffff0001b200028bffffffffffff0001b30001b4ffffffffffff0001b40001b3ffffffffffff0001b50001b6ffffffffffff0001b60001b5ffffffffffff0001b7000292ffffffffffff0001b80001b9ffffffffffff0001b90001b8ffffffffffff0001bc0001bdffffffffffff0001bd0001bcffffffffffff0001bf0001f7ffffffffffff0001c40001c50001c6ffffff0001c50001c40001c6ffffff0001c60001c40001c5ffffff0001c70001c80001c9ffffff0001c80001c70001c9ffffff0001c90001c70001c8ffffff0001ca0001cb0001ccffffff0001cb0001ca0001ccffffff0001cc0001ca0001cbffffff0001cd0001ceffffffffffff0001ce0001cdffffffffffff0001cf0001d0ffffffffffff0001d00001cfffffffffffff0001d10001d2ffffffffffff0001d20001d1ffffffffffff0001d30001d4ffffffffffff0001d40001d3ffffffffffff0001d50001d6ffffffffffff0001d60001d5ffffffffffff0001d70001d8ffffffffffff0001d80001d7ffffffffffff0001d90001daffffffffffff0001da0001d9ffffffffffff0001db0001dcffffffffffff0001dc0001dbffffffffffff0001dd00018effffffffffff0001de0001dfffffffffffff0001df0001deffffffffffff0001e00001e1ffffffffffff0001e10001e0ffffffffffff0001e20001e3ffffffffffff0001e30001e2ffffffffffff0001e40001e5ffffffffffff0001e50001e4ffffffffffff0001e60001e7ffffffffffff0001e70001e6ffffffffffff0001e80001e9ffffffffffff0001e90001e8ffffffffffff0001ea0001ebffffffffffff0001eb0001eaffffffffffff0001ec0001edffffffffffff0001ed0001ecffffffffffff0001ee0001efffffffffffff0001ef0001eeffffffffffff0001f10001f20001f3ffffff0001f20001f10001f3ffffff0001f30001f10001f2ffffff0001f40001f5ffffffffffff0001f50001f4ffffffffffff0001f6000195ffffffffffff0001f70001bfffffffffffff0001f80001f9ffffffffffff0001f90001f8ffffffffffff0001fa0001fbffffffffffff0001fb0001faffffffffffff0001fc0001fdffffffffffff0001fd0001fcffffffffffff0001fe0001ffffffffffffff0001ff0001feffffffffffff000200000201ffffffffffff000201000200ffffffffffff000202000203ffffffffffff000203000202ffffffffffff000204000205ffffffffffff000205000204ffffffffffff000206000207ffffffffffff000207000206ffffffffffff000208000209ffffffffffff000209000208ffffffffffff00020a00020bffffffffffff00020b00020affffffffffff00020c00020dffffffffffff00020d00020cffffffffffff00020e00020fffffffffffff00020f00020effffffffffff000210000211ffffffffffff000211000210ffffffffffff000212000213ffffffffffff000213000212ffffffffffff000214000215ffffffffffff000215000214ffffffffffff000216000217ffffffffffff000217000216ffffffffffff000218000219ffffffffffff000219000218ffffffffffff00021a00021bffffffffffff00021b00021affffffffffff00021c00021dffffffffffff00021d00021cffffffffffff00021e00021fffffffffffff00021f00021effffffffffff00022000019effffffffffff000222000223ffffffffffff000223000222ffffffffffff000224000225ffffffffffff000225000224ffffffffffff000226000227ffffffffffff000227000226ffffffffffff000228000229ffffffffffff000229000228ffffffffffff00022a00022bffffffffffff00022b00022affffffffffff00022c00022dffffffffffff00022d00022cffffffffffff00022e00022fffffffffffff00022f00022effffffffffff000230000231ffffffffffff000231000230ffffffffffff000232000233ffffffffffff000233000232ffffffffffff00023a002c65ffffffffffff00023b00023cffffffffffff00023c00023bffffffffffff00023d00019affffffffffff00023e002c66ffffffffffff00023f002c7effffffffffff000240002c7fffffffffffff000241000242ffffffffffff000242000241ffffffffffff000243000180ffffffffffff000244000289ffffffffffff00024500028cffffffffffff000246000247ffffffffffff000247000246ffffffffffff000248000249ffffffffffff000249000248ffffffffffff00024a00024bffffffffffff00024b00024affffffffffff00024c00024dffffffffffff00024d00024cffffffffffff00024e00024fffffffffffff00024f00024effffffffffff000250002c6fffffffffffff000251002c6dffffffffffff000252002c70ffffffffffff000253000181ffffffffffff000254000186ffffffffffff000256000189ffffffffffff00025700018affffffffffff00025900018fffffffffffff00025b000190ffffffffffff00025c00a7abffffffffffff000260000193ffffffffffff00026100a7acffffffffffff000263000194ffffffffffff00026400a7cbffffffffffff00026500a78dffffffffffff00026600a7aaffffffffffff000268000197ffffffffffff000269000196ffffffffffff00026a00a7aeffffffffffff00026b002c62ffffffffffff00026c00a7adffffffffffff00026f00019cffffffffffff000271002c6effffffffffff00027200019dffffffffffff00027500019fffffffffffff00027d002c64ffffffffffff0002800001a6ffffffffffff00028200a7c5ffffffffffff0002830001a9ffffffffffff00028700a7b1ffffffffffff0002880001aeffffffffffff000289000244ffffffffffff00028a0001b1ffffffffffff00028b0001b2ffffffffffff00028c000245ffffffffffff0002920001b7ffffffffffff00029d00a7b2ffffffffffff00029e00a7b0ffffffffffff0003450003990003b9001fbe000370000371ffffffffffff000371000370ffffffffffff000372000373ffffffffffff000373000372ffffffffffff000376000377ffffffffffff000377000376ffffffffffff00037b0003fdffffffffffff00037c0003feffffffffffff00037d0003ffffffffffffff00037f0003f3ffffffffffff0003860003acffffffffffff0003880003adffffffffffff0003890003aeffffffffffff00038a0003afffffffffffff00038c0003ccffffffffffff00038e0003cdffffffffffff00038f0003ceffffffffffff000390001fd3ffffffffffff0003910003b1ffffffffffff0003920003b20003d0ffffff0003930003b3ffffffffffff0003940003b4ffffffffffff0003950003b50003f5ffffff0003960003b6ffffffffffff0003970003b7ffffffffffff0003980003b80003d10003f40003990003450003b9001fbe00039a0003ba0003f0ffffff00039b0003bbffffffffffff00039c0000b50003bcffffff00039d0003bdffffffffffff00039e0003beffffffffffff00039f0003bfffffffffffff0003a00003c00003d6ffffff0003a10003c10003f1ffffff0003a30003c20003c3ffffff0003a40003c4ffffffffffff0003a50003c5ffffffffffff0003a60003c60003d5ffffff0003a70003c7ffffffffffff0003a80003c8ffffffffffff0003a90003c9002126ffffff0003aa0003caffffffffffff0003ab0003cbffffffffffff0003ac000386ffffffffffff0003ad000388ffffffffffff0003ae000389ffffffffffff0003af00038affffffffffff0003b0001fe3ffffffffffff0003b1000391ffffffffffff0003b20003920003d0ffffff0003b3000393ffffffffffff0003b4000394ffffffffffff0003b50003950003f5ffffff0003b6000396ffffffffffff0003b7000397ffffffffffff0003b80003980003d10003f40003b9000345000399001fbe0003ba00039a0003f0ffffff0003bb00039bffffffffffff0003bc0000b500039cffffff0003bd00039dffffffffffff0003be00039effffffffffff0003bf00039fffffffffffff0003c00003a00003d6ffffff0003c10003a10003f1ffffff0003c20003a30003c3ffffff0003c30003a30003c2ffffff0003c40003a4ffffffffffff0003c50003a5ffffffffffff0003c60003a60003d5ffffff0003c70003a7ffffffffffff0003c80003a8ffffffffffff0003c90003a9002126ffffff0003ca0003aaffffffffffff0003cb0003abffffffffffff0003cc00038cffffffffffff0003cd00038effffffffffff0003ce00038fffffffffffff0003cf0003d7ffffffffffff0003d00003920003b2ffffff0003d10003980003b80003f40003d50003a60003c6ffffff0003d60003a00003c0ffffff0003d70003cfffffffffffff0003d80003d9ffffffffffff0003d90003d8ffffffffffff0003da0003dbffffffffffff0003db0003daffffffffffff0003dc0003ddffffffffffff0003dd0003dcffffffffffff0003de0003dfffffffffffff0003df0003deffffffffffff0003e00003e1ffffffffffff0003e10003e0ffffffffffff0003e20003e3ffffffffffff0003e30003e2ffffffffffff0003e40003e5ffffffffffff0003e50003e4ffffffffffff0003e60003e7ffffffffffff0003e70003e6ffffffffffff0003e80003e9ffffffffffff0003e90003e8ffffffffffff0003ea0003ebffffffffffff0003eb0003eaffffffffffff0003ec0003edffffffffffff0003ed0003ecffffffffffff0003ee0003efffffffffffff0003ef0003eeffffffffffff0003f000039a0003baffffff0003f10003a10003c1ffffff0003f20003f9ffffffffffff0003f300037fffffffffffff0003f40003980003b80003d10003f50003950003b5ffffff0003f70003f8ffffffffffff0003f80003f7ffffffffffff0003f90003f2ffffffffffff0003fa0003fbffffffffffff0003fb0003faffffffffffff0003fd00037bffffffffffff0003fe00037cffffffffffff0003ff00037dffffffffffff000400000450ffffffffffff000401000451ffffffffffff000402000452ffffffffffff000403000453ffffffffffff000404000454ffffffffffff000405000455ffffffffffff000406000456ffffffffffff000407000457ffffffffffff000408000458ffffffffffff000409000459ffffffffffff00040a00045affffffffffff00040b00045bffffffffffff00040c00045cffffffffffff00040d00045dffffffffffff00040e00045effffffffffff00040f00045fffffffffffff000410000430ffffffffffff000411000431ffffffffffff000412000432001c80ffffff000413000433ffffffffffff000414000434001c81ffffff000415000435ffffffffffff000416000436ffffffffffff000417000437ffffffffffff000418000438ffffffffffff000419000439ffffffffffff00041a00043affffffffffff00041b00043bffffffffffff00041c00043cffffffffffff00041d00043dffffffffffff00041e00043e001c82ffffff00041f00043fffffffffffff000420000440ffffffffffff000421000441001c83ffffff000422000442001c84001c85000423000443ffffffffffff000424000444ffffffffffff000425000445ffffffffffff000426000446ffffffffffff000427000447ffffffffffff000428000448ffffffffffff000429000449ffffffffffff00042a00044a001c86ffffff00042b00044bffffffffffff00042c00044cffffffffffff00042d00044dffffffffffff00042e00044effffffffffff00042f00044fffffffffffff000430000410ffffffffffff000431000411ffffffffffff000432000412001c80ffffff000433000413ffffffffffff000434000414001c81ffffff000435000415ffffffffffff000436000416ffffffffffff000437000417ffffffffffff000438000418ffffffffffff000439000419ffffffffffff00043a00041affffffffffff00043b00041bffffffffffff00043c00041cffffffffffff00043d00041dffffffffffff00043e00041e001c82ffffff00043f00041fffffffffffff000440000420ffffffffffff000441000421001c83ffffff000442000422001c84001c85000443000423ffffffffffff000444000424ffffffffffff000445000425ffffffffffff000446000426ffffffffffff000447000427ffffffffffff000448000428ffffffffffff000449000429ffffffffffff00044a00042a001c86ffffff00044b00042bffffffffffff00044c00042cffffffffffff00044d00042dffffffffffff00044e00042effffffffffff00044f00042fffffffffffff000450000400ffffffffffff000451000401ffffffffffff000452000402ffffffffffff000453000403ffffffffffff000454000404ffffffffffff000455000405ffffffffffff000456000406ffffffffffff000457000407ffffffffffff000458000408ffffffffffff000459000409ffffffffffff00045a00040affffffffffff00045b00040bffffffffffff00045c00040cffffffffffff00045d00040dffffffffffff00045e00040effffffffffff00045f00040fffffffffffff000460000461ffffffffffff000461000460ffffffffffff000462000463001c87ffffff000463000462001c87ffffff000464000465ffffffffffff000465000464ffffffffffff000466000467ffffffffffff000467000466ffffffffffff000468000469ffffffffffff000469000468ffffffffffff00046a00046bffffffffffff00046b00046affffffffffff00046c00046dffffffffffff00046d00046cffffffffffff00046e00046fffffffffffff00046f00046effffffffffff000470000471ffffffffffff000471000470ffffffffffff000472000473ffffffffffff000473000472ffffffffffff000474000475ffffffffffff000475000474ffffffffffff000476000477ffffffffffff000477000476ffffffffffff000478000479ffffffffffff000479000478ffffffffffff00047a00047bffffffffffff00047b00047affffffffffff00047c00047dffffffffffff00047d00047cffffffffffff00047e00047fffffffffffff00047f00047effffffffffff000480000481ffffffffffff000481000480ffffffffffff00048a00048bffffffffffff00048b00048affffffffffff00048c00048dffffffffffff00048d00048cffffffffffff00048e00048fffffffffffff00048f00048effffffffffff000490000491ffffffffffff000491000490ffffffffffff000492000493ffffffffffff000493000492ffffffffffff000494000495ffffffffffff000495000494ffffffffffff000496000497ffffffffffff000497000496ffffffffffff000498000499ffffffffffff000499000498ffffffffffff00049a00049bffffffffffff00049b00049affffffffffff00049c00049dffffffffffff00049d00049cffffffffffff00049e00049fffffffffffff00049f00049effffffffffff0004a00004a1ffffffffffff0004a10004a0ffffffffffff0004a20004a3ffffffffffff0004a30004a2ffffffffffff0004a40004a5ffffffffffff0004a50004a4ffffffffffff0004a60004a7ffffffffffff0004a70004a6ffffffffffff0004a80004a9ffffffffffff0004a90004a8ffffffffffff0004aa0004abffffffffffff0004ab0004aaffffffffffff0004ac0004adffffffffffff0004ad0004acffffffffffff0004ae0004afffffffffffff0004af0004aeffffffffffff0004b00004b1ffffffffffff0004b10004b0ffffffffffff0004b20004b3ffffffffffff0004b30004b2ffffffffffff0004b40004b5ffffffffffff0004b50004b4ffffffffffff0004b60004b7ffffffffffff0004b70004b6ffffffffffff0004b80004b9ffffffffffff0004b90004b8ffffffffffff0004ba0004bbffffffffffff0004bb0004baffffffffffff0004bc0004bdffffffffffff0004bd0004bcffffffffffff0004be0004bfffffffffffff0004bf0004beffffffffffff0004c00004cfffffffffffff0004c10004c2ffffffffffff0004c20004c1ffffffffffff0004c30004c4ffffffffffff0004c40004c3ffffffffffff0004c50004c6ffffffffffff0004c60004c5ffffffffffff0004c70004c8ffffffffffff0004c80004c7ffffffffffff0004c90004caffffffffffff0004ca0004c9ffffffffffff0004cb0004ccffffffffffff0004cc0004cbffffffffffff0004cd0004ceffffffffffff0004ce0004cdffffffffffff0004cf0004c0ffffffffffff0004d00004d1ffffffffffff0004d10004d0ffffffffffff0004d20004d3ffffffffffff0004d30004d2ffffffffffff0004d40004d5ffffffffffff0004d50004d4ffffffffffff0004d60004d7ffffffffffff0004d70004d6ffffffffffff0004d80004d9ffffffffffff0004d90004d8ffffffffffff0004da0004dbffffffffffff0004db0004daffffffffffff0004dc0004ddffffffffffff0004dd0004dcffffffffffff0004de0004dfffffffffffff0004df0004deffffffffffff0004e00004e1ffffffffffff0004e10004e0ffffffffffff0004e20004e3ffffffffffff0004e30004e2ffffffffffff0004e40004e5ffffffffffff0004e50004e4ffffffffffff0004e60004e7ffffffffffff0004e70004e6ffffffffffff0004e80004e9ffffffffffff0004e90004e8ffffffffffff0004ea0004ebffffffffffff0004eb0004eaffffffffffff0004ec0004edffffffffffff0004ed0004ecffffffffffff0004ee0004efffffffffffff0004ef0004eeffffffffffff0004f00004f1ffffffffffff0004f10004f0ffffffffffff0004f20004f3ffffffffffff0004f30004f2ffffffffffff0004f40004f5ffffffffffff0004f50004f4ffffffffffff0004f60004f7ffffffffffff0004f70004f6ffffffffffff0004f80004f9ffffffffffff0004f90004f8ffffffffffff0004fa0004fbffffffffffff0004fb0004faffffffffffff0004fc0004fdffffffffffff0004fd0004fcffffffffffff0004fe0004ffffffffffffff0004ff0004feffffffffffff000500000501ffffffffffff000501000500ffffffffffff000502000503ffffffffffff000503000502ffffffffffff000504000505ffffffffffff000505000504ffffffffffff000506000507ffffffffffff000507000506ffffffffffff000508000509ffffffffffff000509000508ffffffffffff00050a00050bffffffffffff00050b00050affffffffffff00050c00050dffffffffffff00050d00050cffffffffffff00050e00050fffffffffffff00050f00050effffffffffff000510000511ffffffffffff000511000510ffffffffffff000512000513ffffffffffff000513000512ffffffffffff000514000515ffffffffffff000515000514ffffffffffff000516000517ffffffffffff000517000516ffffffffffff000518000519ffffffffffff000519000518ffffffffffff00051a00051bffffffffffff00051b00051affffffffffff00051c00051dffffffffffff00051d00051cffffffffffff00051e00051fffffffffffff00051f00051effffffffffff000520000521ffffffffffff000521000520ffffffffffff000522000523ffffffffffff000523000522ffffffffffff000524000525ffffffffffff000525000524ffffffffffff000526000527ffffffffffff000527000526ffffffffffff000528000529ffffffffffff000529000528ffffffffffff00052a00052bffffffffffff00052b00052affffffffffff00052c00052dffffffffffff00052d00052cffffffffffff00052e00052fffffffffffff00052f00052effffffffffff000531000561ffffffffffff000532000562ffffffffffff000533000563ffffffffffff000534000564ffffffffffff000535000565ffffffffffff000536000566ffffffffffff000537000567ffffffffffff000538000568ffffffffffff000539000569ffffffffffff00053a00056affffffffffff00053b00056bffffffffffff00053c00056cffffffffffff00053d00056dffffffffffff00053e00056effffffffffff00053f00056fffffffffffff000540000570ffffffffffff000541000571ffffffffffff000542000572ffffffffffff000543000573ffffffffffff000544000574ffffffffffff000545000575ffffffffffff000546000576ffffffffffff000547000577ffffffffffff000548000578ffffffffffff000549000579ffffffffffff00054a00057affffffffffff00054b00057bffffffffffff00054c00057cffffffffffff00054d00057dffffffffffff00054e00057effffffffffff00054f00057fffffffffffff000550000580ffffffffffff000551000581ffffffffffff000552000582ffffffffffff000553000583ffffffffffff000554000584ffffffffffff000555000585ffffffffffff000556000586ffffffffffff000561000531ffffffffffff000562000532ffffffffffff000563000533ffffffffffff000564000534ffffffffffff000565000535ffffffffffff000566000536ffffffffffff000567000537ffffffffffff000568000538ffffffffffff000569000539ffffffffffff00056a00053affffffffffff00056b00053bffffffffffff00056c00053cffffffffffff00056d00053dffffffffffff00056e00053effffffffffff00056f00053fffffffffffff000570000540ffffffffffff000571000541ffffffffffff000572000542ffffffffffff000573000543ffffffffffff000574000544ffffffffffff000575000545ffffffffffff000576000546ffffffffffff000577000547ffffffffffff000578000548ffffffffffff000579000549ffffffffffff00057a00054affffffffffff00057b00054bffffffffffff00057c00054cffffffffffff00057d00054dffffffffffff00057e00054effffffffffff00057f00054fffffffffffff000580000550ffffffffffff000581000551ffffffffffff000582000552ffffffffffff000583000553ffffffffffff000584000554ffffffffffff000585000555ffffffffffff000586000556ffffffffffff0010a0002d00ffffffffffff0010a1002d01ffffffffffff0010a2002d02ffffffffffff0010a3002d03ffffffffffff0010a4002d04ffffffffffff0010a5002d05ffffffffffff0010a6002d06ffffffffffff0010a7002d07ffffffffffff0010a8002d08ffffffffffff0010a9002d09ffffffffffff0010aa002d0affffffffffff0010ab002d0bffffffffffff0010ac002d0cffffffffffff0010ad002d0dffffffffffff0010ae002d0effffffffffff0010af002d0fffffffffffff0010b0002d10ffffffffffff0010b1002d11ffffffffffff0010b2002d12ffffffffffff0010b3002d13ffffffffffff0010b4002d14ffffffffffff0010b5002d15ffffffffffff0010b6002d16ffffffffffff0010b7002d17ffffffffffff0010b8002d18ffffffffffff0010b9002d19ffffffffffff0010ba002d1affffffffffff0010bb002d1bffffffffffff0010bc002d1cffffffffffff0010bd002d1dffffffffffff0010be002d1effffffffffff0010bf002d1fffffffffffff0010c0002d20ffffffffffff0010c1002d21ffffffffffff0010c2002d22ffffffffffff0010c3002d23ffffffffffff0010c4002d24ffffffffffff0010c5002d25ffffffffffff0010c7002d27ffffffffffff0010cd002d2dffffffffffff0010d0001c90ffffffffffff0010d1001c91ffffffffffff0010d2001c92ffffffffffff0010d3001c93ffffffffffff0010d4001c94ffffffffffff0010d5001c95ffffffffffff0010d6001c96ffffffffffff0010d7001c97ffffffffffff0010d8001c98ffffffffffff0010d9001c99ffffffffffff0010da001c9affffffffffff0010db001c9bffffffffffff0010dc001c9cffffffffffff0010dd001c9dffffffffffff0010de001c9effffffffffff0010df001c9fffffffffffff0010e0001ca0ffffffffffff0010e1001ca1ffffffffffff0010e2001ca2ffffffffffff0010e3001ca3ffffffffffff0010e4001ca4ffffffffffff0010e5001ca5ffffffffffff0010e6001ca6ffffffffffff0010e7001ca7ffffffffffff0010e8001ca8ffffffffffff0010e9001ca9ffffffffffff0010ea001caaffffffffffff0010eb001cabffffffffffff0010ec001cacffffffffffff0010ed001cadffffffffffff0010ee001caeffffffffffff0010ef001cafffffffffffff0010f0001cb0ffffffffffff0010f1001cb1ffffffffffff0010f2001cb2ffffffffffff0010f3001cb3ffffffffffff0010f4001cb4ffffffffffff0010f5001cb5ffffffffffff0010f6001cb6ffffffffffff0010f7001cb7ffffffffffff0010f8001cb8ffffffffffff0010f9001cb9ffffffffffff0010fa001cbaffffffffffff0010fd001cbdffffffffffff0010fe001cbeffffffffffff0010ff001cbfffffffffffff0013a000ab70ffffffffffff0013a100ab71ffffffffffff0013a200ab72ffffffffffff0013a300ab73ffffffffffff0013a400ab74ffffffffffff0013a500ab75ffffffffffff0013a600ab76ffffffffffff0013a700ab77ffffffffffff0013a800ab78ffffffffffff0013a900ab79ffffffffffff0013aa00ab7affffffffffff0013ab00ab7bffffffffffff0013ac00ab7cffffffffffff0013ad00ab7dffffffffffff0013ae00ab7effffffffffff0013af00ab7fffffffffffff0013b000ab80ffffffffffff0013b100ab81ffffffffffff0013b200ab82ffffffffffff0013b300ab83ffffffffffff0013b400ab84ffffffffffff0013b500ab85ffffffffffff0013b600ab86ffffffffffff0013b700ab87ffffffffffff0013b800ab88ffffffffffff0013b900ab89ffffffffffff0013ba00ab8affffffffffff0013bb00ab8bffffffffffff0013bc00ab8cffffffffffff0013bd00ab8dffffffffffff0013be00ab8effffffffffff0013bf00ab8fffffffffffff0013c000ab90ffffffffffff0013c100ab91ffffffffffff0013c200ab92ffffffffffff0013c300ab93ffffffffffff0013c400ab94ffffffffffff0013c500ab95ffffffffffff0013c600ab96ffffffffffff0013c700ab97ffffffffffff0013c800ab98ffffffffffff0013c900ab99ffffffffffff0013ca00ab9affffffffffff0013cb00ab9bffffffffffff0013cc00ab9cffffffffffff0013cd00ab9dffffffffffff0013ce00ab9effffffffffff0013cf00ab9fffffffffffff0013d000aba0ffffffffffff0013d100aba1ffffffffffff0013d200aba2ffffffffffff0013d300aba3ffffffffffff0013d400aba4ffffffffffff0013d500aba5ffffffffffff0013d600aba6ffffffffffff0013d700aba7ffffffffffff0013d800aba8ffffffffffff0013d900aba9ffffffffffff0013da00abaaffffffffffff0013db00ababffffffffffff0013dc00abacffffffffffff0013dd00abadffffffffffff0013de00abaeffffffffffff0013df00abafffffffffffff0013e000abb0ffffffffffff0013e100abb1ffffffffffff0013e200abb2ffffffffffff0013e300abb3ffffffffffff0013e400abb4ffffffffffff0013e500abb5ffffffffffff0013e600abb6ffffffffffff0013e700abb7ffffffffffff0013e800abb8ffffffffffff0013e900abb9ffffffffffff0013ea00abbaffffffffffff0013eb00abbbffffffffffff0013ec00abbcffffffffffff0013ed00abbdffffffffffff0013ee00abbeffffffffffff0013ef00abbfffffffffffff0013f00013f8ffffffffffff0013f10013f9ffffffffffff0013f20013faffffffffffff0013f30013fbffffffffffff0013f40013fcffffffffffff0013f50013fdffffffffffff0013f80013f0ffffffffffff0013f90013f1ffffffffffff0013fa0013f2ffffffffffff0013fb0013f3ffffffffffff0013fc0013f4ffffffffffff0013fd0013f5ffffffffffff001c80000412000432ffffff001c81000414000434ffffff001c8200041e00043effffff001c83000421000441ffffff001c84000422000442001c85001c85000422000442001c84001c8600042a00044affffff001c87000462000463ffffff001c8800a64a00a64bffffff001c89001c8affffffffffff001c8a001c89ffffffffffff001c900010d0ffffffffffff001c910010d1ffffffffffff001c920010d2ffffffffffff001c930010d3ffffffffffff001c940010d4ffffffffffff001c950010d5ffffffffffff001c960010d6ffffffffffff001c970010d7ffffffffffff001c980010d8ffffffffffff001c990010d9ffffffffffff001c9a0010daffffffffffff001c9b0010dbffffffffffff001c9c0010dcffffffffffff001c9d0010ddffffffffffff001c9e0010deffffffffffff001c9f0010dfffffffffffff001ca00010e0ffffffffffff001ca10010e1ffffffffffff001ca20010e2ffffffffffff001ca30010e3ffffffffffff001ca40010e4ffffffffffff001ca50010e5ffffffffffff001ca60010e6ffffffffffff001ca70010e7ffffffffffff001ca80010e8ffffffffffff001ca90010e9ffffffffffff001caa0010eaffffffffffff001cab0010ebffffffffffff001cac0010ecffffffffffff001cad0010edffffffffffff001cae0010eeffffffffffff001caf0010efffffffffffff001cb00010f0ffffffffffff001cb10010f1ffffffffffff001cb20010f2ffffffffffff001cb30010f3ffffffffffff001cb40010f4ffffffffffff001cb50010f5ffffffffffff001cb60010f6ffffffffffff001cb70010f7ffffffffffff001cb80010f8ffffffffffff001cb90010f9ffffffffffff001cba0010faffffffffffff001cbd0010fdffffffffffff001cbe0010feffffffffffff001cbf0010ffffffffffffff001d7900a77dffffffffffff001d7d002c63ffffffffffff001d8e00a7c6ffffffffffff001e00001e01ffffffffffff001e01001e00ffffffffffff001e02001e03ffffffffffff001e03001e02ffffffffffff001e04001e05ffffffffffff001e05001e04ffffffffffff001e06001e07ffffffffffff001e07001e06ffffffffffff001e08001e09ffffffffffff001e09001e08ffffffffffff001e0a001e0bffffffffffff001e0b001e0affffffffffff001e0c001e0dffffffffffff001e0d001e0cffffffffffff001e0e001e0fffffffffffff001e0f001e0effffffffffff001e10001e11ffffffffffff001e11001e10ffffffffffff001e12001e13ffffffffffff001e13001e12ffffffffffff001e14001e15ffffffffffff001e15001e14ffffffffffff001e16001e17ffffffffffff001e17001e16ffffffffffff001e18001e19ffffffffffff001e19001e18ffffffffffff001e1a001e1bffffffffffff001e1b001e1affffffffffff001e1c001e1dffffffffffff001e1d001e1cffffffffffff001e1e001e1fffffffffffff001e1f001e1effffffffffff001e20001e21ffffffffffff001e21001e20ffffffffffff001e22001e23ffffffffffff001e23001e22ffffffffffff001e24001e25ffffffffffff001e25001e24ffffffffffff001e26001e27ffffffffffff001e27001e26ffffffffffff001e28001e29ffffffffffff001e29001e28ffffffffffff001e2a001e2bffffffffffff001e2b001e2affffffffffff001e2c001e2dffffffffffff001e2d001e2cffffffffffff001e2e001e2fffffffffffff001e2f001e2effffffffffff001e30001e31ffffffffffff001e31001e30ffffffffffff001e32001e33ffffffffffff001e33001e32ffffffffffff001e34001e35ffffffffffff001e35001e34ffffffffffff001e36001e37ffffffffffff001e37001e36ffffffffffff001e38001e39ffffffffffff001e39001e38ffffffffffff001e3a001e3bffffffffffff001e3b001e3affffffffffff001e3c001e3dffffffffffff001e3d001e3cffffffffffff001e3e001e3fffffffffffff001e3f001e3effffffffffff001e40001e41ffffffffffff001e41001e40ffffffffffff001e42001e43ffffffffffff001e43001e42ffffffffffff001e44001e45ffffffffffff001e45001e44ffffffffffff001e46001e47ffffffffffff001e47001e46ffffffffffff001e48001e49ffffffffffff001e49001e48ffffffffffff001e4a001e4bffffffffffff001e4b001e4affffffffffff001e4c001e4dffffffffffff001e4d001e4cffffffffffff001e4e001e4fffffffffffff001e4f001e4effffffffffff001e50001e51ffffffffffff001e51001e50ffffffffffff001e52001e53ffffffffffff001e53001e52ffffffffffff001e54001e55ffffffffffff001e55001e54ffffffffffff001e56001e57ffffffffffff001e57001e56ffffffffffff001e58001e59ffffffffffff001e59001e58ffffffffffff001e5a001e5bffffffffffff001e5b001e5affffffffffff001e5c001e5dffffffffffff001e5d001e5cffffffffffff001e5e001e5fffffffffffff001e5f001e5effffffffffff001e60001e61001e9bffffff001e61001e60001e9bffffff001e62001e63ffffffffffff001e63001e62ffffffffffff001e64001e65ffffffffffff001e65001e64ffffffffffff001e66001e67ffffffffffff001e67001e66ffffffffffff001e68001e69ffffffffffff001e69001e68ffffffffffff001e6a001e6bffffffffffff001e6b001e6affffffffffff001e6c001e6dffffffffffff001e6d001e6cffffffffffff001e6e001e6fffffffffffff001e6f001e6effffffffffff001e70001e71ffffffffffff001e71001e70ffffffffffff001e72001e73ffffffffffff001e73001e72ffffffffffff001e74001e75ffffffffffff001e75001e74ffffffffffff001e76001e77ffffffffffff001e77001e76ffffffffffff001e78001e79ffffffffffff001e79001e78ffffffffffff001e7a001e7bffffffffffff001e7b001e7affffffffffff001e7c001e7dffffffffffff001e7d001e7cffffffffffff001e7e001e7fffffffffffff001e7f001e7effffffffffff001e80001e81ffffffffffff001e81001e80ffffffffffff001e82001e83ffffffffffff001e83001e82ffffffffffff001e84001e85ffffffffffff001e85001e84ffffffffffff001e86001e87ffffffffffff001e87001e86ffffffffffff001e88001e89ffffffffffff001e89001e88ffffffffffff001e8a001e8bffffffffffff001e8b001e8affffffffffff001e8c001e8dffffffffffff001e8d001e8cffffffffffff001e8e001e8fffffffffffff001e8f001e8effffffffffff001e90001e91ffffffffffff001e91001e90ffffffffffff001e92001e93ffffffffffff001e93001e92ffffffffffff001e94001e95ffffffffffff001e95001e94ffffffffffff001e9b001e60001e61ffffff001e9e0000dfffffffffffff001ea0001ea1ffffffffffff001ea1001ea0ffffffffffff001ea2001ea3ffffffffffff001ea3001ea2ffffffffffff001ea4001ea5ffffffffffff001ea5001ea4ffffffffffff001ea6001ea7ffffffffffff001ea7001ea6ffffffffffff001ea8001ea9ffffffffffff001ea9001ea8ffffffffffff001eaa001eabffffffffffff001eab001eaaffffffffffff001eac001eadffffffffffff001ead001eacffffffffffff001eae001eafffffffffffff001eaf001eaeffffffffffff001eb0001eb1ffffffffffff001eb1001eb0ffffffffffff001eb2001eb3ffffffffffff001eb3001eb2ffffffffffff001eb4001eb5ffffffffffff001eb5001eb4ffffffffffff001eb6001eb7ffffffffffff001eb7001eb6ffffffffffff001eb8001eb9ffffffffffff001eb9001eb8ffffffffffff001eba001ebbffffffffffff001ebb001ebaffffffffffff001ebc001ebdffffffffffff001ebd001ebcffffffffffff001ebe001ebfffffffffffff001ebf001ebeffffffffffff001ec0001ec1ffffffffffff001ec1001ec0ffffffffffff001ec2001ec3ffffffffffff001ec3001ec2ffffffffffff001ec4001ec5ffffffffffff001ec5001ec4ffffffffffff001ec6001ec7ffffffffffff001ec7001ec6ffffffffffff001ec8001ec9ffffffffffff001ec9001ec8ffffffffffff001eca001ecbffffffffffff001ecb001ecaffffffffffff001ecc001ecdffffffffffff001ecd001eccffffffffffff001ece001ecfffffffffffff001ecf001eceffffffffffff001ed0001ed1ffffffffffff001ed1001ed0ffffffffffff001ed2001ed3ffffffffffff001ed3001ed2ffffffffffff001ed4001ed5ffffffffffff001ed5001ed4ffffffffffff001ed6001ed7ffffffffffff001ed7001ed6ffffffffffff001ed8001ed9ffffffffffff001ed9001ed8ffffffffffff001eda001edbffffffffffff001edb001edaffffffffffff001edc001eddffffffffffff001edd001edcffffffffffff001ede001edfffffffffffff001edf001edeffffffffffff001ee0001ee1ffffffffffff001ee1001ee0ffffffffffff001ee2001ee3ffffffffffff001ee3001ee2ffffffffffff001ee4001ee5ffffffffffff001ee5001ee4ffffffffffff001ee6001ee7ffffffffffff001ee7001ee6ffffffffffff001ee8001ee9ffffffffffff001ee9001ee8ffffffffffff001eea001eebffffffffffff001eeb001eeaffffffffffff001eec001eedffffffffffff001eed001eecffffffffffff001eee001eefffffffffffff001eef001eeeffffffffffff001ef0001ef1ffffffffffff001ef1001ef0ffffffffffff001ef2001ef3ffffffffffff001ef3001ef2ffffffffffff001ef4001ef5ffffffffffff001ef5001ef4ffffffffffff001ef6001ef7ffffffffffff001ef7001ef6ffffffffffff001ef8001ef9ffffffffffff001ef9001ef8ffffffffffff001efa001efbffffffffffff001efb001efaffffffffffff001efc001efdffffffffffff001efd001efcffffffffffff001efe001effffffffffffff001eff001efeffffffffffff001f00001f08ffffffffffff001f01001f09ffffffffffff001f02001f0affffffffffff001f03001f0bffffffffffff001f04001f0cffffffffffff001f05001f0dffffffffffff001f06001f0effffffffffff001f07001f0fffffffffffff001f08001f00ffffffffffff001f09001f01ffffffffffff001f0a001f02ffffffffffff001f0b001f03ffffffffffff001f0c001f04ffffffffffff001f0d001f05ffffffffffff001f0e001f06ffffffffffff001f0f001f07ffffffffffff001f10001f18ffffffffffff001f11001f19ffffffffffff001f12001f1affffffffffff001f13001f1bffffffffffff001f14001f1cffffffffffff001f15001f1dffffffffffff001f18001f10ffffffffffff001f19001f11ffffffffffff001f1a001f12ffffffffffff001f1b001f13ffffffffffff001f1c001f14ffffffffffff001f1d001f15ffffffffffff001f20001f28ffffffffffff001f21001f29ffffffffffff001f22001f2affffffffffff001f23001f2bffffffffffff001f24001f2cffffffffffff001f25001f2dffffffffffff001f26001f2effffffffffff001f27001f2fffffffffffff001f28001f20ffffffffffff001f29001f21ffffffffffff001f2a001f22ffffffffffff001f2b001f23ffffffffffff001f2c001f24ffffffffffff001f2d001f25ffffffffffff001f2e001f26ffffffffffff001f2f001f27ffffffffffff001f30001f38ffffffffffff001f31001f39ffffffffffff001f32001f3affffffffffff001f33001f3bffffffffffff001f34001f3cffffffffffff001f35001f3dffffffffffff001f36001f3effffffffffff001f37001f3fffffffffffff001f38001f30ffffffffffff001f39001f31ffffffffffff001f3a001f32ffffffffffff001f3b001f33ffffffffffff001f3c001f34ffffffffffff001f3d001f35ffffffffffff001f3e001f36ffffffffffff001f3f001f37ffffffffffff001f40001f48ffffffffffff001f41001f49ffffffffffff001f42001f4affffffffffff001f43001f4bffffffffffff001f44001f4cffffffffffff001f45001f4dffffffffffff001f48001f40ffffffffffff001f49001f41ffffffffffff001f4a001f42ffffffffffff001f4b001f43ffffffffffff001f4c001f44ffffffffffff001f4d001f45ffffffffffff001f51001f59ffffffffffff001f53001f5bffffffffffff001f55001f5dffffffffffff001f57001f5fffffffffffff001f59001f51ffffffffffff001f5b001f53ffffffffffff001f5d001f55ffffffffffff001f5f001f57ffffffffffff001f60001f68ffffffffffff001f61001f69ffffffffffff001f62001f6affffffffffff001f63001f6bffffffffffff001f64001f6cffffffffffff001f65001f6dffffffffffff001f66001f6effffffffffff001f67001f6fffffffffffff001f68001f60ffffffffffff001f69001f61ffffffffffff001f6a001f62ffffffffffff001f6b001f63ffffffffffff001f6c001f64ffffffffffff001f6d001f65ffffffffffff001f6e001f66ffffffffffff001f6f001f67ffffffffffff001f70001fbaffffffffffff001f71001fbbffffffffffff001f72001fc8ffffffffffff001f73001fc9ffffffffffff001f74001fcaffffffffffff001f75001fcbffffffffffff001f76001fdaffffffffffff001f77001fdbffffffffffff001f78001ff8ffffffffffff001f79001ff9ffffffffffff001f7a001feaffffffffffff001f7b001febffffffffffff001f7c001ffaffffffffffff001f7d001ffbffffffffffff001f80001f88ffffffffffff001f81001f89ffffffffffff001f82001f8affffffffffff001f83001f8bffffffffffff001f84001f8cffffffffffff001f85001f8dffffffffffff001f86001f8effffffffffff001f87001f8fffffffffffff001f88001f80ffffffffffff001f89001f81ffffffffffff001f8a001f82ffffffffffff001f8b001f83ffffffffffff001f8c001f84ffffffffffff001f8d001f85ffffffffffff001f8e001f86ffffffffffff001f8f001f87ffffffffffff001f90001f98ffffffffffff001f91001f99ffffffffffff001f92001f9affffffffffff001f93001f9bffffffffffff001f94001f9cffffffffffff001f95001f9dffffffffffff001f96001f9effffffffffff001f97001f9fffffffffffff001f98001f90ffffffffffff001f99001f91ffffffffffff001f9a001f92ffffffffffff001f9b001f93ffffffffffff001f9c001f94ffffffffffff001f9d001f95ffffffffffff001f9e001f96ffffffffffff001f9f001f97ffffffffffff001fa0001fa8ffffffffffff001fa1001fa9ffffffffffff001fa2001faaffffffffffff001fa3001fabffffffffffff001fa4001facffffffffffff001fa5001fadffffffffffff001fa6001faeffffffffffff001fa7001fafffffffffffff001fa8001fa0ffffffffffff001fa9001fa1ffffffffffff001faa001fa2ffffffffffff001fab001fa3ffffffffffff001fac001fa4ffffffffffff001fad001fa5ffffffffffff001fae001fa6ffffffffffff001faf001fa7ffffffffffff001fb0001fb8ffffffffffff001fb1001fb9ffffffffffff001fb3001fbcffffffffffff001fb8001fb0ffffffffffff001fb9001fb1ffffffffffff001fba001f70ffffffffffff001fbb001f71ffffffffffff001fbc001fb3ffffffffffff001fbe0003450003990003b9001fc3001fccffffffffffff001fc8001f72ffffffffffff001fc9001f73ffffffffffff001fca001f74ffffffffffff001fcb001f75ffffffffffff001fcc001fc3ffffffffffff001fd0001fd8ffffffffffff001fd1001fd9ffffffffffff001fd3000390ffffffffffff001fd8001fd0ffffffffffff001fd9001fd1ffffffffffff001fda001f76ffffffffffff001fdb001f77ffffffffffff001fe0001fe8ffffffffffff001fe1001fe9ffffffffffff001fe30003b0ffffffffffff001fe5001fecffffffffffff001fe8001fe0ffffffffffff001fe9001fe1ffffffffffff001fea001f7affffffffffff001feb001f7bffffffffffff001fec001fe5ffffffffffff001ff3001ffcffffffffffff001ff8001f78ffffffffffff001ff9001f79ffffffffffff001ffa001f7cffffffffffff001ffb001f7dffffffffffff001ffc001ff3ffffffffffff0021260003a90003c9ffffff00212a00004b00006bffffff00212b0000c50000e5ffffff00213200214effffffffffff00214e002132ffffffffffff002160002170ffffffffffff002161002171ffffffffffff002162002172ffffffffffff002163002173ffffffffffff002164002174ffffffffffff002165002175ffffffffffff002166002176ffffffffffff002167002177ffffffffffff002168002178ffffffffffff002169002179ffffffffffff00216a00217affffffffffff00216b00217bffffffffffff00216c00217cffffffffffff00216d00217dffffffffffff00216e00217effffffffffff00216f00217fffffffffffff002170002160ffffffffffff002171002161ffffffffffff002172002162ffffffffffff002173002163ffffffffffff002174002164ffffffffffff002175002165ffffffffffff002176002166ffffffffffff002177002167ffffffffffff002178002168ffffffffffff002179002169ffffffffffff00217a00216affffffffffff00217b00216bffffffffffff00217c00216cffffffffffff00217d00216dffffffffffff00217e00216effffffffffff00217f00216fffffffffffff002183002184ffffffffffff002184002183ffffffffffff0024b60024d0ffffffffffff0024b70024d1ffffffffffff0024b80024d2ffffffffffff0024b90024d3ffffffffffff0024ba0024d4ffffffffffff0024bb0024d5ffffffffffff0024bc0024d6ffffffffffff0024bd0024d7ffffffffffff0024be0024d8ffffffffffff0024bf0024d9ffffffffffff0024c00024daffffffffffff0024c10024dbffffffffffff0024c20024dcffffffffffff0024c30024ddffffffffffff0024c40024deffffffffffff0024c50024dfffffffffffff0024c60024e0ffffffffffff0024c70024e1ffffffffffff0024c80024e2ffffffffffff0024c90024e3ffffffffffff0024ca0024e4ffffffffffff0024cb0024e5ffffffffffff0024cc0024e6ffffffffffff0024cd0024e7ffffffffffff0024ce0024e8ffffffffffff0024cf0024e9ffffffffffff0024d00024b6ffffffffffff0024d10024b7ffffffffffff0024d20024b8ffffffffffff0024d30024b9ffffffffffff0024d40024baffffffffffff0024d50024bbffffffffffff0024d60024bcffffffffffff0024d70024bdffffffffffff0024d80024beffffffffffff0024d90024bfffffffffffff0024da0024c0ffffffffffff0024db0024c1ffffffffffff0024dc0024c2ffffffffffff0024dd0024c3ffffffffffff0024de0024c4ffffffffffff0024df0024c5ffffffffffff0024e00024c6ffffffffffff0024e10024c7ffffffffffff0024e20024c8ffffffffffff0024e30024c9ffffffffffff0024e40024caffffffffffff0024e50024cbffffffffffff0024e60024ccffffffffffff0024e70024cdffffffffffff0024e80024ceffffffffffff0024e90024cfffffffffffff002c00002c30ffffffffffff002c01002c31ffffffffffff002c02002c32ffffffffffff002c03002c33ffffffffffff002c04002c34ffffffffffff002c05002c35ffffffffffff002c06002c36ffffffffffff002c07002c37ffffffffffff002c08002c38ffffffffffff002c09002c39ffffffffffff002c0a002c3affffffffffff002c0b002c3bffffffffffff002c0c002c3cffffffffffff002c0d002c3dffffffffffff002c0e002c3effffffffffff002c0f002c3fffffffffffff002c10002c40ffffffffffff002c11002c41ffffffffffff002c12002c42ffffffffffff002c13002c43ffffffffffff002c14002c44ffffffffffff002c15002c45ffffffffffff002c16002c46ffffffffffff002c17002c47ffffffffffff002c18002c48ffffffffffff002c19002c49ffffffffffff002c1a002c4affffffffffff002c1b002c4bffffffffffff002c1c002c4cffffffffffff002c1d002c4dffffffffffff002c1e002c4effffffffffff002c1f002c4fffffffffffff002c20002c50ffffffffffff002c21002c51ffffffffffff002c22002c52ffffffffffff002c23002c53ffffffffffff002c24002c54ffffffffffff002c25002c55ffffffffffff002c26002c56ffffffffffff002c27002c57ffffffffffff002c28002c58ffffffffffff002c29002c59ffffffffffff002c2a002c5affffffffffff002c2b002c5bffffffffffff002c2c002c5cffffffffffff002c2d002c5dffffffffffff002c2e002c5effffffffffff002c2f002c5fffffffffffff002c30002c00ffffffffffff002c31002c01ffffffffffff002c32002c02ffffffffffff002c33002c03ffffffffffff002c34002c04ffffffffffff002c35002c05ffffffffffff002c36002c06ffffffffffff002c37002c07ffffffffffff002c38002c08ffffffffffff002c39002c09ffffffffffff002c3a002c0affffffffffff002c3b002c0bffffffffffff002c3c002c0cffffffffffff002c3d002c0dffffffffffff002c3e002c0effffffffffff002c3f002c0fffffffffffff002c40002c10ffffffffffff002c41002c11ffffffffffff002c42002c12ffffffffffff002c43002c13ffffffffffff002c44002c14ffffffffffff002c45002c15ffffffffffff002c46002c16ffffffffffff002c47002c17ffffffffffff002c48002c18ffffffffffff002c49002c19ffffffffffff002c4a002c1affffffffffff002c4b002c1bffffffffffff002c4c002c1cffffffffffff002c4d002c1dffffffffffff002c4e002c1effffffffffff002c4f002c1fffffffffffff002c50002c20ffffffffffff002c51002c21ffffffffffff002c52002c22ffffffffffff002c53002c23ffffffffffff002c54002c24ffffffffffff002c55002c25ffffffffffff002c56002c26ffffffffffff002c57002c27ffffffffffff002c58002c28ffffffffffff002c59002c29ffffffffffff002c5a002c2affffffffffff002c5b002c2bffffffffffff002c5c002c2cffffffffffff002c5d002c2dffffffffffff002c5e002c2effffffffffff002c5f002c2fffffffffffff002c60002c61ffffffffffff002c61002c60ffffffffffff002c6200026bffffffffffff002c63001d7dffffffffffff002c6400027dffffffffffff002c6500023affffffffffff002c6600023effffffffffff002c67002c68ffffffffffff002c68002c67ffffffffffff002c69002c6affffffffffff002c6a002c69ffffffffffff002c6b002c6cffffffffffff002c6c002c6bffffffffffff002c6d000251ffffffffffff002c6e000271ffffffffffff002c6f000250ffffffffffff002c70000252ffffffffffff002c72002c73ffffffffffff002c73002c72ffffffffffff002c75002c76ffffffffffff002c76002c75ffffffffffff002c7e00023fffffffffffff002c7f000240ffffffffffff002c80002c81ffffffffffff002c81002c80ffffffffffff002c82002c83ffffffffffff002c83002c82ffffffffffff002c84002c85ffffffffffff002c85002c84ffffffffffff002c86002c87ffffffffffff002c87002c86ffffffffffff002c88002c89ffffffffffff002c89002c88ffffffffffff002c8a002c8bffffffffffff002c8b002c8affffffffffff002c8c002c8dffffffffffff002c8d002c8cffffffffffff002c8e002c8fffffffffffff002c8f002c8effffffffffff002c90002c91ffffffffffff002c91002c90ffffffffffff002c92002c93ffffffffffff002c93002c92ffffffffffff002c94002c95ffffffffffff002c95002c94ffffffffffff002c96002c97ffffffffffff002c97002c96ffffffffffff002c98002c99ffffffffffff002c99002c98ffffffffffff002c9a002c9bffffffffffff002c9b002c9affffffffffff002c9c002c9dffffffffffff002c9d002c9cffffffffffff002c9e002c9fffffffffffff002c9f002c9effffffffffff002ca0002ca1ffffffffffff002ca1002ca0ffffffffffff002ca2002ca3ffffffffffff002ca3002ca2ffffffffffff002ca4002ca5ffffffffffff002ca5002ca4ffffffffffff002ca6002ca7ffffffffffff002ca7002ca6ffffffffffff002ca8002ca9ffffffffffff002ca9002ca8ffffffffffff002caa002cabffffffffffff002cab002caaffffffffffff002cac002cadffffffffffff002cad002cacffffffffffff002cae002cafffffffffffff002caf002caeffffffffffff002cb0002cb1ffffffffffff002cb1002cb0ffffffffffff002cb2002cb3ffffffffffff002cb3002cb2ffffffffffff002cb4002cb5ffffffffffff002cb5002cb4ffffffffffff002cb6002cb7ffffffffffff002cb7002cb6ffffffffffff002cb8002cb9ffffffffffff002cb9002cb8ffffffffffff002cba002cbbffffffffffff002cbb002cbaffffffffffff002cbc002cbdffffffffffff002cbd002cbcffffffffffff002cbe002cbfffffffffffff002cbf002cbeffffffffffff002cc0002cc1ffffffffffff002cc1002cc0ffffffffffff002cc2002cc3ffffffffffff002cc3002cc2ffffffffffff002cc4002cc5ffffffffffff002cc5002cc4ffffffffffff002cc6002cc7ffffffffffff002cc7002cc6ffffffffffff002cc8002cc9ffffffffffff002cc9002cc8ffffffffffff002cca002ccbffffffffffff002ccb002ccaffffffffffff002ccc002ccdffffffffffff002ccd002cccffffffffffff002cce002ccfffffffffffff002ccf002cceffffffffffff002cd0002cd1ffffffffffff002cd1002cd0ffffffffffff002cd2002cd3ffffffffffff002cd3002cd2ffffffffffff002cd4002cd5ffffffffffff002cd5002cd4ffffffffffff002cd6002cd7ffffffffffff002cd7002cd6ffffffffffff002cd8002cd9ffffffffffff002cd9002cd8ffffffffffff002cda002cdbffffffffffff002cdb002cdaffffffffffff002cdc002cddffffffffffff002cdd002cdcffffffffffff002cde002cdfffffffffffff002cdf002cdeffffffffffff002ce0002ce1ffffffffffff002ce1002ce0ffffffffffff002ce2002ce3ffffffffffff002ce3002ce2ffffffffffff002ceb002cecffffffffffff002cec002cebffffffffffff002ced002ceeffffffffffff002cee002cedffffffffffff002cf2002cf3ffffffffffff002cf3002cf2ffffffffffff002d000010a0ffffffffffff002d010010a1ffffffffffff002d020010a2ffffffffffff002d030010a3ffffffffffff002d040010a4ffffffffffff002d050010a5ffffffffffff002d060010a6ffffffffffff002d070010a7ffffffffffff002d080010a8ffffffffffff002d090010a9ffffffffffff002d0a0010aaffffffffffff002d0b0010abffffffffffff002d0c0010acffffffffffff002d0d0010adffffffffffff002d0e0010aeffffffffffff002d0f0010afffffffffffff002d100010b0ffffffffffff002d110010b1ffffffffffff002d120010b2ffffffffffff002d130010b3ffffffffffff002d140010b4ffffffffffff002d150010b5ffffffffffff002d160010b6ffffffffffff002d170010b7ffffffffffff002d180010b8ffffffffffff002d190010b9ffffffffffff002d1a0010baffffffffffff002d1b0010bbffffffffffff002d1c0010bcffffffffffff002d1d0010bdffffffffffff002d1e0010beffffffffffff002d1f0010bfffffffffffff002d200010c0ffffffffffff002d210010c1ffffffffffff002d220010c2ffffffffffff002d230010c3ffffffffffff002d240010c4ffffffffffff002d250010c5ffffffffffff002d270010c7ffffffffffff002d2d0010cdffffffffffff00a64000a641ffffffffffff00a64100a640ffffffffffff00a64200a643ffffffffffff00a64300a642ffffffffffff00a64400a645ffffffffffff00a64500a644ffffffffffff00a64600a647ffffffffffff00a64700a646ffffffffffff00a64800a649ffffffffffff00a64900a648ffffffffffff00a64a001c8800a64bffffff00a64b001c8800a64affffff00a64c00a64dffffffffffff00a64d00a64cffffffffffff00a64e00a64fffffffffffff00a64f00a64effffffffffff00a65000a651ffffffffffff00a65100a650ffffffffffff00a65200a653ffffffffffff00a65300a652ffffffffffff00a65400a655ffffffffffff00a65500a654ffffffffffff00a65600a657ffffffffffff00a65700a656ffffffffffff00a65800a659ffffffffffff00a65900a658ffffffffffff00a65a00a65bffffffffffff00a65b00a65affffffffffff00a65c00a65dffffffffffff00a65d00a65cffffffffffff00a65e00a65fffffffffffff00a65f00a65effffffffffff00a66000a661ffffffffffff00a66100a660ffffffffffff00a66200a663ffffffffffff00a66300a662ffffffffffff00a66400a665ffffffffffff00a66500a664ffffffffffff00a66600a667ffffffffffff00a66700a666ffffffffffff00a66800a669ffffffffffff00a66900a668ffffffffffff00a66a00a66bffffffffffff00a66b00a66affffffffffff00a66c00a66dffffffffffff00a66d00a66cffffffffffff00a68000a681ffffffffffff00a68100a680ffffffffffff00a68200a683ffffffffffff00a68300a682ffffffffffff00a68400a685ffffffffffff00a68500a684ffffffffffff00a68600a687ffffffffffff00a68700a686ffffffffffff00a68800a689ffffffffffff00a68900a688ffffffffffff00a68a00a68bffffffffffff00a68b00a68affffffffffff00a68c00a68dffffffffffff00a68d00a68cffffffffffff00a68e00a68fffffffffffff00a68f00a68effffffffffff00a69000a691ffffffffffff00a69100a690ffffffffffff00a69200a693ffffffffffff00a69300a692ffffffffffff00a69400a695ffffffffffff00a69500a694ffffffffffff00a69600a697ffffffffffff00a69700a696ffffffffffff00a69800a699ffffffffffff00a69900a698ffffffffffff00a69a00a69bffffffffffff00a69b00a69affffffffffff00a72200a723ffffffffffff00a72300a722ffffffffffff00a72400a725ffffffffffff00a72500a724ffffffffffff00a72600a727ffffffffffff00a72700a726ffffffffffff00a72800a729ffffffffffff00a72900a728ffffffffffff00a72a00a72bffffffffffff00a72b00a72affffffffffff00a72c00a72dffffffffffff00a72d00a72cffffffffffff00a72e00a72fffffffffffff00a72f00a72effffffffffff00a73200a733ffffffffffff00a73300a732ffffffffffff00a73400a735ffffffffffff00a73500a734ffffffffffff00a73600a737ffffffffffff00a73700a736ffffffffffff00a73800a739ffffffffffff00a73900a738ffffffffffff00a73a00a73bffffffffffff00a73b00a73affffffffffff00a73c00a73dffffffffffff00a73d00a73cffffffffffff00a73e00a73fffffffffffff00a73f00a73effffffffffff00a74000a741ffffffffffff00a74100a740ffffffffffff00a74200a743ffffffffffff00a74300a742ffffffffffff00a74400a745ffffffffffff00a74500a744ffffffffffff00a74600a747ffffffffffff00a74700a746ffffffffffff00a74800a749ffffffffffff00a74900a748ffffffffffff00a74a00a74bffffffffffff00a74b00a74affffffffffff00a74c00a74dffffffffffff00a74d00a74cffffffffffff00a74e00a74fffffffffffff00a74f00a74effffffffffff00a75000a751ffffffffffff00a75100a750ffffffffffff00a75200a753ffffffffffff00a75300a752ffffffffffff00a75400a755ffffffffffff00a75500a754ffffffffffff00a75600a757ffffffffffff00a75700a756ffffffffffff00a75800a759ffffffffffff00a75900a758ffffffffffff00a75a00a75bffffffffffff00a75b00a75affffffffffff00a75c00a75dffffffffffff00a75d00a75cffffffffffff00a75e00a75fffffffffffff00a75f00a75effffffffffff00a76000a761ffffffffffff00a76100a760ffffffffffff00a76200a763ffffffffffff00a76300a762ffffffffffff00a76400a765ffffffffffff00a76500a764ffffffffffff00a76600a767ffffffffffff00a76700a766ffffffffffff00a76800a769ffffffffffff00a76900a768ffffffffffff00a76a00a76bffffffffffff00a76b00a76affffffffffff00a76c00a76dffffffffffff00a76d00a76cffffffffffff00a76e00a76fffffffffffff00a76f00a76effffffffffff00a77900a77affffffffffff00a77a00a779ffffffffffff00a77b00a77cffffffffffff00a77c00a77bffffffffffff00a77d001d79ffffffffffff00a77e00a77fffffffffffff00a77f00a77effffffffffff00a78000a781ffffffffffff00a78100a780ffffffffffff00a78200a783ffffffffffff00a78300a782ffffffffffff00a78400a785ffffffffffff00a78500a784ffffffffffff00a78600a787ffffffffffff00a78700a786ffffffffffff00a78b00a78cffffffffffff00a78c00a78bffffffffffff00a78d000265ffffffffffff00a79000a791ffffffffffff00a79100a790ffffffffffff00a79200a793ffffffffffff00a79300a792ffffffffffff00a79400a7c4ffffffffffff00a79600a797ffffffffffff00a79700a796ffffffffffff00a79800a799ffffffffffff00a79900a798ffffffffffff00a79a00a79bffffffffffff00a79b00a79affffffffffff00a79c00a79dffffffffffff00a79d00a79cffffffffffff00a79e00a79fffffffffffff00a79f00a79effffffffffff00a7a000a7a1ffffffffffff00a7a100a7a0ffffffffffff00a7a200a7a3ffffffffffff00a7a300a7a2ffffffffffff00a7a400a7a5ffffffffffff00a7a500a7a4ffffffffffff00a7a600a7a7ffffffffffff00a7a700a7a6ffffffffffff00a7a800a7a9ffffffffffff00a7a900a7a8ffffffffffff00a7aa000266ffffffffffff00a7ab00025cffffffffffff00a7ac000261ffffffffffff00a7ad00026cffffffffffff00a7ae00026affffffffffff00a7b000029effffffffffff00a7b1000287ffffffffffff00a7b200029dffffffffffff00a7b300ab53ffffffffffff00a7b400a7b5ffffffffffff00a7b500a7b4ffffffffffff00a7b600a7b7ffffffffffff00a7b700a7b6ffffffffffff00a7b800a7b9ffffffffffff00a7b900a7b8ffffffffffff00a7ba00a7bbffffffffffff00a7bb00a7baffffffffffff00a7bc00a7bdffffffffffff00a7bd00a7bcffffffffffff00a7be00a7bfffffffffffff00a7bf00a7beffffffffffff00a7c000a7c1ffffffffffff00a7c100a7c0ffffffffffff00a7c200a7c3ffffffffffff00a7c300a7c2ffffffffffff00a7c400a794ffffffffffff00a7c5000282ffffffffffff00a7c6001d8effffffffffff00a7c700a7c8ffffffffffff00a7c800a7c7ffffffffffff00a7c900a7caffffffffffff00a7ca00a7c9ffffffffffff00a7cb000264ffffffffffff00a7cc00a7cdffffffffffff00a7cd00a7ccffffffffffff00a7d000a7d1ffffffffffff00a7d100a7d0ffffffffffff00a7d600a7d7ffffffffffff00a7d700a7d6ffffffffffff00a7d800a7d9ffffffffffff00a7d900a7d8ffffffffffff00a7da00a7dbffffffffffff00a7db00a7daffffffffffff00a7dc00019bffffffffffff00a7f500a7f6ffffffffffff00a7f600a7f5ffffffffffff00ab5300a7b3ffffffffffff00ab700013a0ffffffffffff00ab710013a1ffffffffffff00ab720013a2ffffffffffff00ab730013a3ffffffffffff00ab740013a4ffffffffffff00ab750013a5ffffffffffff00ab760013a6ffffffffffff00ab770013a7ffffffffffff00ab780013a8ffffffffffff00ab790013a9ffffffffffff00ab7a0013aaffffffffffff00ab7b0013abffffffffffff00ab7c0013acffffffffffff00ab7d0013adffffffffffff00ab7e0013aeffffffffffff00ab7f0013afffffffffffff00ab800013b0ffffffffffff00ab810013b1ffffffffffff00ab820013b2ffffffffffff00ab830013b3ffffffffffff00ab840013b4ffffffffffff00ab850013b5ffffffffffff00ab860013b6ffffffffffff00ab870013b7ffffffffffff00ab880013b8ffffffffffff00ab890013b9ffffffffffff00ab8a0013baffffffffffff00ab8b0013bbffffffffffff00ab8c0013bcffffffffffff00ab8d0013bdffffffffffff00ab8e0013beffffffffffff00ab8f0013bfffffffffffff00ab900013c0ffffffffffff00ab910013c1ffffffffffff00ab920013c2ffffffffffff00ab930013c3ffffffffffff00ab940013c4ffffffffffff00ab950013c5ffffffffffff00ab960013c6ffffffffffff00ab970013c7ffffffffffff00ab980013c8ffffffffffff00ab990013c9ffffffffffff00ab9a0013caffffffffffff00ab9b0013cbffffffffffff00ab9c0013ccffffffffffff00ab9d0013cdffffffffffff00ab9e0013ceffffffffffff00ab9f0013cfffffffffffff00aba00013d0ffffffffffff00aba10013d1ffffffffffff00aba20013d2ffffffffffff00aba30013d3ffffffffffff00aba40013d4ffffffffffff00aba50013d5ffffffffffff00aba60013d6ffffffffffff00aba70013d7ffffffffffff00aba80013d8ffffffffffff00aba90013d9ffffffffffff00abaa0013daffffffffffff00abab0013dbffffffffffff00abac0013dcffffffffffff00abad0013ddffffffffffff00abae0013deffffffffffff00abaf0013dfffffffffffff00abb00013e0ffffffffffff00abb10013e1ffffffffffff00abb20013e2ffffffffffff00abb30013e3ffffffffffff00abb40013e4ffffffffffff00abb50013e5ffffffffffff00abb60013e6ffffffffffff00abb70013e7ffffffffffff00abb80013e8ffffffffffff00abb90013e9ffffffffffff00abba0013eaffffffffffff00abbb0013ebffffffffffff00abbc0013ecffffffffffff00abbd0013edffffffffffff00abbe0013eeffffffffffff00abbf0013efffffffffffff00fb0500fb06ffffffffffff00fb0600fb05ffffffffffff00ff2100ff41ffffffffffff00ff2200ff42ffffffffffff00ff2300ff43ffffffffffff00ff2400ff44ffffffffffff00ff2500ff45ffffffffffff00ff2600ff46ffffffffffff00ff2700ff47ffffffffffff00ff2800ff48ffffffffffff00ff2900ff49ffffffffffff00ff2a00ff4affffffffffff00ff2b00ff4bffffffffffff00ff2c00ff4cffffffffffff00ff2d00ff4dffffffffffff00ff2e00ff4effffffffffff00ff2f00ff4fffffffffffff00ff3000ff50ffffffffffff00ff3100ff51ffffffffffff00ff3200ff52ffffffffffff00ff3300ff53ffffffffffff00ff3400ff54ffffffffffff00ff3500ff55ffffffffffff00ff3600ff56ffffffffffff00ff3700ff57ffffffffffff00ff3800ff58ffffffffffff00ff3900ff59ffffffffffff00ff3a00ff5affffffffffff00ff4100ff21ffffffffffff00ff4200ff22ffffffffffff00ff4300ff23ffffffffffff00ff4400ff24ffffffffffff00ff4500ff25ffffffffffff00ff4600ff26ffffffffffff00ff4700ff27ffffffffffff00ff4800ff28ffffffffffff00ff4900ff29ffffffffffff00ff4a00ff2affffffffffff00ff4b00ff2bffffffffffff00ff4c00ff2cffffffffffff00ff4d00ff2dffffffffffff00ff4e00ff2effffffffffff00ff4f00ff2fffffffffffff00ff5000ff30ffffffffffff00ff5100ff31ffffffffffff00ff5200ff32ffffffffffff00ff5300ff33ffffffffffff00ff5400ff34ffffffffffff00ff5500ff35ffffffffffff00ff5600ff36ffffffffffff00ff5700ff37ffffffffffff00ff5800ff38ffffffffffff00ff5900ff39ffffffffffff00ff5a00ff3affffffffffff010400010428ffffffffffff010401010429ffffffffffff01040201042affffffffffff01040301042bffffffffffff01040401042cffffffffffff01040501042dffffffffffff01040601042effffffffffff01040701042fffffffffffff010408010430ffffffffffff010409010431ffffffffffff01040a010432ffffffffffff01040b010433ffffffffffff01040c010434ffffffffffff01040d010435ffffffffffff01040e010436ffffffffffff01040f010437ffffffffffff010410010438ffffffffffff010411010439ffffffffffff01041201043affffffffffff01041301043bffffffffffff01041401043cffffffffffff01041501043dffffffffffff01041601043effffffffffff01041701043fffffffffffff010418010440ffffffffffff010419010441ffffffffffff01041a010442ffffffffffff01041b010443ffffffffffff01041c010444ffffffffffff01041d010445ffffffffffff01041e010446ffffffffffff01041f010447ffffffffffff010420010448ffffffffffff010421010449ffffffffffff01042201044affffffffffff01042301044bffffffffffff01042401044cffffffffffff01042501044dffffffffffff01042601044effffffffffff01042701044fffffffffffff010428010400ffffffffffff010429010401ffffffffffff01042a010402ffffffffffff01042b010403ffffffffffff01042c010404ffffffffffff01042d010405ffffffffffff01042e010406ffffffffffff01042f010407ffffffffffff010430010408ffffffffffff010431010409ffffffffffff01043201040affffffffffff01043301040bffffffffffff01043401040cffffffffffff01043501040dffffffffffff01043601040effffffffffff01043701040fffffffffffff010438010410ffffffffffff010439010411ffffffffffff01043a010412ffffffffffff01043b010413ffffffffffff01043c010414ffffffffffff01043d010415ffffffffffff01043e010416ffffffffffff01043f010417ffffffffffff010440010418ffffffffffff010441010419ffffffffffff01044201041affffffffffff01044301041bffffffffffff01044401041cffffffffffff01044501041dffffffffffff01044601041effffffffffff01044701041fffffffffffff010448010420ffffffffffff010449010421ffffffffffff01044a010422ffffffffffff01044b010423ffffffffffff01044c010424ffffffffffff01044d010425ffffffffffff01044e010426ffffffffffff01044f010427ffffffffffff0104b00104d8ffffffffffff0104b10104d9ffffffffffff0104b20104daffffffffffff0104b30104dbffffffffffff0104b40104dcffffffffffff0104b50104ddffffffffffff0104b60104deffffffffffff0104b70104dfffffffffffff0104b80104e0ffffffffffff0104b90104e1ffffffffffff0104ba0104e2ffffffffffff0104bb0104e3ffffffffffff0104bc0104e4ffffffffffff0104bd0104e5ffffffffffff0104be0104e6ffffffffffff0104bf0104e7ffffffffffff0104c00104e8ffffffffffff0104c10104e9ffffffffffff0104c20104eaffffffffffff0104c30104ebffffffffffff0104c40104ecffffffffffff0104c50104edffffffffffff0104c60104eeffffffffffff0104c70104efffffffffffff0104c80104f0ffffffffffff0104c90104f1ffffffffffff0104ca0104f2ffffffffffff0104cb0104f3ffffffffffff0104cc0104f4ffffffffffff0104cd0104f5ffffffffffff0104ce0104f6ffffffffffff0104cf0104f7ffffffffffff0104d00104f8ffffffffffff0104d10104f9ffffffffffff0104d20104faffffffffffff0104d30104fbffffffffffff0104d80104b0ffffffffffff0104d90104b1ffffffffffff0104da0104b2ffffffffffff0104db0104b3ffffffffffff0104dc0104b4ffffffffffff0104dd0104b5ffffffffffff0104de0104b6ffffffffffff0104df0104b7ffffffffffff0104e00104b8ffffffffffff0104e10104b9ffffffffffff0104e20104baffffffffffff0104e30104bbffffffffffff0104e40104bcffffffffffff0104e50104bdffffffffffff0104e60104beffffffffffff0104e70104bfffffffffffff0104e80104c0ffffffffffff0104e90104c1ffffffffffff0104ea0104c2ffffffffffff0104eb0104c3ffffffffffff0104ec0104c4ffffffffffff0104ed0104c5ffffffffffff0104ee0104c6ffffffffffff0104ef0104c7ffffffffffff0104f00104c8ffffffffffff0104f10104c9ffffffffffff0104f20104caffffffffffff0104f30104cbffffffffffff0104f40104ccffffffffffff0104f50104cdffffffffffff0104f60104ceffffffffffff0104f70104cfffffffffffff0104f80104d0ffffffffffff0104f90104d1ffffffffffff0104fa0104d2ffffffffffff0104fb0104d3ffffffffffff010570010597ffffffffffff010571010598ffffffffffff010572010599ffffffffffff01057301059affffffffffff01057401059bffffffffffff01057501059cffffffffffff01057601059dffffffffffff01057701059effffffffffff01057801059fffffffffffff0105790105a0ffffffffffff01057a0105a1ffffffffffff01057c0105a3ffffffffffff01057d0105a4ffffffffffff01057e0105a5ffffffffffff01057f0105a6ffffffffffff0105800105a7ffffffffffff0105810105a8ffffffffffff0105820105a9ffffffffffff0105830105aaffffffffffff0105840105abffffffffffff0105850105acffffffffffff0105860105adffffffffffff0105870105aeffffffffffff0105880105afffffffffffff0105890105b0ffffffffffff01058a0105b1ffffffffffff01058c0105b3ffffffffffff01058d0105b4ffffffffffff01058e0105b5ffffffffffff01058f0105b6ffffffffffff0105900105b7ffffffffffff0105910105b8ffffffffffff0105920105b9ffffffffffff0105940105bbffffffffffff0105950105bcffffffffffff010597010570ffffffffffff010598010571ffffffffffff010599010572ffffffffffff01059a010573ffffffffffff01059b010574ffffffffffff01059c010575ffffffffffff01059d010576ffffffffffff01059e010577ffffffffffff01059f010578ffffffffffff0105a0010579ffffffffffff0105a101057affffffffffff0105a301057cffffffffffff0105a401057dffffffffffff0105a501057effffffffffff0105a601057fffffffffffff0105a7010580ffffffffffff0105a8010581ffffffffffff0105a9010582ffffffffffff0105aa010583ffffffffffff0105ab010584ffffffffffff0105ac010585ffffffffffff0105ad010586ffffffffffff0105ae010587ffffffffffff0105af010588ffffffffffff0105b0010589ffffffffffff0105b101058affffffffffff0105b301058cffffffffffff0105b401058dffffffffffff0105b501058effffffffffff0105b601058fffffffffffff0105b7010590ffffffffffff0105b8010591ffffffffffff0105b9010592ffffffffffff0105bb010594ffffffffffff0105bc010595ffffffffffff010c80010cc0ffffffffffff010c81010cc1ffffffffffff010c82010cc2ffffffffffff010c83010cc3ffffffffffff010c84010cc4ffffffffffff010c85010cc5ffffffffffff010c86010cc6ffffffffffff010c87010cc7ffffffffffff010c88010cc8ffffffffffff010c89010cc9ffffffffffff010c8a010ccaffffffffffff010c8b010ccbffffffffffff010c8c010cccffffffffffff010c8d010ccdffffffffffff010c8e010cceffffffffffff010c8f010ccfffffffffffff010c90010cd0ffffffffffff010c91010cd1ffffffffffff010c92010cd2ffffffffffff010c93010cd3ffffffffffff010c94010cd4ffffffffffff010c95010cd5ffffffffffff010c96010cd6ffffffffffff010c97010cd7ffffffffffff010c98010cd8ffffffffffff010c99010cd9ffffffffffff010c9a010cdaffffffffffff010c9b010cdbffffffffffff010c9c010cdcffffffffffff010c9d010cddffffffffffff010c9e010cdeffffffffffff010c9f010cdfffffffffffff010ca0010ce0ffffffffffff010ca1010ce1ffffffffffff010ca2010ce2ffffffffffff010ca3010ce3ffffffffffff010ca4010ce4ffffffffffff010ca5010ce5ffffffffffff010ca6010ce6ffffffffffff010ca7010ce7ffffffffffff010ca8010ce8ffffffffffff010ca9010ce9ffffffffffff010caa010ceaffffffffffff010cab010cebffffffffffff010cac010cecffffffffffff010cad010cedffffffffffff010cae010ceeffffffffffff010caf010cefffffffffffff010cb0010cf0ffffffffffff010cb1010cf1ffffffffffff010cb2010cf2ffffffffffff010cc0010c80ffffffffffff010cc1010c81ffffffffffff010cc2010c82ffffffffffff010cc3010c83ffffffffffff010cc4010c84ffffffffffff010cc5010c85ffffffffffff010cc6010c86ffffffffffff010cc7010c87ffffffffffff010cc8010c88ffffffffffff010cc9010c89ffffffffffff010cca010c8affffffffffff010ccb010c8bffffffffffff010ccc010c8cffffffffffff010ccd010c8dffffffffffff010cce010c8effffffffffff010ccf010c8fffffffffffff010cd0010c90ffffffffffff010cd1010c91ffffffffffff010cd2010c92ffffffffffff010cd3010c93ffffffffffff010cd4010c94ffffffffffff010cd5010c95ffffffffffff010cd6010c96ffffffffffff010cd7010c97ffffffffffff010cd8010c98ffffffffffff010cd9010c99ffffffffffff010cda010c9affffffffffff010cdb010c9bffffffffffff010cdc010c9cffffffffffff010cdd010c9dffffffffffff010cde010c9effffffffffff010cdf010c9fffffffffffff010ce0010ca0ffffffffffff010ce1010ca1ffffffffffff010ce2010ca2ffffffffffff010ce3010ca3ffffffffffff010ce4010ca4ffffffffffff010ce5010ca5ffffffffffff010ce6010ca6ffffffffffff010ce7010ca7ffffffffffff010ce8010ca8ffffffffffff010ce9010ca9ffffffffffff010cea010caaffffffffffff010ceb010cabffffffffffff010cec010cacffffffffffff010ced010cadffffffffffff010cee010caeffffffffffff010cef010cafffffffffffff010cf0010cb0ffffffffffff010cf1010cb1ffffffffffff010cf2010cb2ffffffffffff010d50010d70ffffffffffff010d51010d71ffffffffffff010d52010d72ffffffffffff010d53010d73ffffffffffff010d54010d74ffffffffffff010d55010d75ffffffffffff010d56010d76ffffffffffff010d57010d77ffffffffffff010d58010d78ffffffffffff010d59010d79ffffffffffff010d5a010d7affffffffffff010d5b010d7bffffffffffff010d5c010d7cffffffffffff010d5d010d7dffffffffffff010d5e010d7effffffffffff010d5f010d7fffffffffffff010d60010d80ffffffffffff010d61010d81ffffffffffff010d62010d82ffffffffffff010d63010d83ffffffffffff010d64010d84ffffffffffff010d65010d85ffffffffffff010d70010d50ffffffffffff010d71010d51ffffffffffff010d72010d52ffffffffffff010d73010d53ffffffffffff010d74010d54ffffffffffff010d75010d55ffffffffffff010d76010d56ffffffffffff010d77010d57ffffffffffff010d78010d58ffffffffffff010d79010d59ffffffffffff010d7a010d5affffffffffff010d7b010d5bffffffffffff010d7c010d5cffffffffffff010d7d010d5dffffffffffff010d7e010d5effffffffffff010d7f010d5fffffffffffff010d80010d60ffffffffffff010d81010d61ffffffffffff010d82010d62ffffffffffff010d83010d63ffffffffffff010d84010d64ffffffffffff010d85010d65ffffffffffff0118a00118c0ffffffffffff0118a10118c1ffffffffffff0118a20118c2ffffffffffff0118a30118c3ffffffffffff0118a40118c4ffffffffffff0118a50118c5ffffffffffff0118a60118c6ffffffffffff0118a70118c7ffffffffffff0118a80118c8ffffffffffff0118a90118c9ffffffffffff0118aa0118caffffffffffff0118ab0118cbffffffffffff0118ac0118ccffffffffffff0118ad0118cdffffffffffff0118ae0118ceffffffffffff0118af0118cfffffffffffff0118b00118d0ffffffffffff0118b10118d1ffffffffffff0118b20118d2ffffffffffff0118b30118d3ffffffffffff0118b40118d4ffffffffffff0118b50118d5ffffffffffff0118b60118d6ffffffffffff0118b70118d7ffffffffffff0118b80118d8ffffffffffff0118b90118d9ffffffffffff0118ba0118daffffffffffff0118bb0118dbffffffffffff0118bc0118dcffffffffffff0118bd0118ddffffffffffff0118be0118deffffffffffff0118bf0118dfffffffffffff0118c00118a0ffffffffffff0118c10118a1ffffffffffff0118c20118a2ffffffffffff0118c30118a3ffffffffffff0118c40118a4ffffffffffff0118c50118a5ffffffffffff0118c60118a6ffffffffffff0118c70118a7ffffffffffff0118c80118a8ffffffffffff0118c90118a9ffffffffffff0118ca0118aaffffffffffff0118cb0118abffffffffffff0118cc0118acffffffffffff0118cd0118adffffffffffff0118ce0118aeffffffffffff0118cf0118afffffffffffff0118d00118b0ffffffffffff0118d10118b1ffffffffffff0118d20118b2ffffffffffff0118d30118b3ffffffffffff0118d40118b4ffffffffffff0118d50118b5ffffffffffff0118d60118b6ffffffffffff0118d70118b7ffffffffffff0118d80118b8ffffffffffff0118d90118b9ffffffffffff0118da0118baffffffffffff0118db0118bbffffffffffff0118dc0118bcffffffffffff0118dd0118bdffffffffffff0118de0118beffffffffffff0118df0118bfffffffffffff016e40016e60ffffffffffff016e41016e61ffffffffffff016e42016e62ffffffffffff016e43016e63ffffffffffff016e44016e64ffffffffffff016e45016e65ffffffffffff016e46016e66ffffffffffff016e47016e67ffffffffffff016e48016e68ffffffffffff016e49016e69ffffffffffff016e4a016e6affffffffffff016e4b016e6bffffffffffff016e4c016e6cffffffffffff016e4d016e6dffffffffffff016e4e016e6effffffffffff016e4f016e6fffffffffffff016e50016e70ffffffffffff016e51016e71ffffffffffff016e52016e72ffffffffffff016e53016e73ffffffffffff016e54016e74ffffffffffff016e55016e75ffffffffffff016e56016e76ffffffffffff016e57016e77ffffffffffff016e58016e78ffffffffffff016e59016e79ffffffffffff016e5a016e7affffffffffff016e5b016e7bffffffffffff016e5c016e7cffffffffffff016e5d016e7dffffffffffff016e5e016e7effffffffffff016e5f016e7fffffffffffff016e60016e40ffffffffffff016e61016e41ffffffffffff016e62016e42ffffffffffff016e63016e43ffffffffffff016e64016e44ffffffffffff016e65016e45ffffffffffff016e66016e46ffffffffffff016e67016e47ffffffffffff016e68016e48ffffffffffff016e69016e49ffffffffffff016e6a016e4affffffffffff016e6b016e4bffffffffffff016e6c016e4cffffffffffff016e6d016e4dffffffffffff016e6e016e4effffffffffff016e6f016e4fffffffffffff016e70016e50ffffffffffff016e71016e51ffffffffffff016e72016e52ffffffffffff016e73016e53ffffffffffff016e74016e54ffffffffffff016e75016e55ffffffffffff016e76016e56ffffffffffff016e77016e57ffffffffffff016e78016e58ffffffffffff016e79016e59ffffffffffff016e7a016e5affffffffffff016e7b016e5bffffffffffff016e7c016e5cffffffffffff016e7d016e5dffffffffffff016e7e016e5effffffffffff016e7f016e5fffffffffffff01e90001e922ffffffffffff01e90101e923ffffffffffff01e90201e924ffffffffffff01e90301e925ffffffffffff01e90401e926ffffffffffff01e90501e927ffffffffffff01e90601e928ffffffffffff01e90701e929ffffffffffff01e90801e92affffffffffff01e90901e92bffffffffffff01e90a01e92cffffffffffff01e90b01e92dffffffffffff01e90c01e92effffffffffff01e90d01e92fffffffffffff01e90e01e930ffffffffffff01e90f01e931ffffffffffff01e91001e932ffffffffffff01e91101e933ffffffffffff01e91201e934ffffffffffff01e91301e935ffffffffffff01e91401e936ffffffffffff01e91501e937ffffffffffff01e91601e938ffffffffffff01e91701e939ffffffffffff01e91801e93affffffffffff01e91901e93bffffffffffff01e91a01e93cffffffffffff01e91b01e93dffffffffffff01e91c01e93effffffffffff01e91d01e93fffffffffffff01e91e01e940ffffffffffff01e91f01e941ffffffffffff01e92001e942ffffffffffff01e92101e943ffffffffffff01e92201e900ffffffffffff01e92301e901ffffffffffff01e92401e902ffffffffffff01e92501e903ffffffffffff01e92601e904ffffffffffff01e92701e905ffffffffffff01e92801e906ffffffffffff01e92901e907ffffffffffff01e92a01e908ffffffffffff01e92b01e909ffffffffffff01e92c01e90affffffffffff01e92d01e90bffffffffffff01e92e01e90cffffffffffff01e92f01e90dffffffffffff01e93001e90effffffffffff01e93101e90fffffffffffff01e93201e910ffffffffffff01e93301e911ffffffffffff01e93401e912ffffffffffff01e93501e913ffffffffffff01e93601e914ffffffffffff01e93701e915ffffffffffff01e93801e916ffffffffffff01e93901e917ffffffffffff01e93a01e918ffffffffffff01e93b01e919ffffffffffff01e93c01e91affffffffffff01e93d01e91bffffffffffff01e93e01e91cffffffffffff01e93f01e91dffffffffffff01e94001e91effffffffffff01e94101e91fffffffffffff01e94201e920ffffffffffff01e94301e921ffffffffffff",
)

table = (hex : String) -> BitArray {
	bit_array.base16_decode(hex) |>
		(Ok(bytes)) { bytes }
		| (Error(_)) { panic("regex: Unicode table encoding") }
}
