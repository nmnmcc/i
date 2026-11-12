//// Structured entries and call-time sink services. See std-2.md §6/§7.
//// Level(..), Entry(..) export constructors; Sink(E,N) hides its constructor.
//// Level { Debug Info Warning Failure }
//// Entry { Entry(level: Level, message: String, fields: List((String,String)),
////               at: Option(time.Instant)) }
//// Sink(E,N) { Sink((Level)->Bool, (Entry)->Nil ? E ! N, (N)->Nil) }
//// Interface:
//// make(E,N): (accept: (Level)->Bool, send: (Entry)->Nil ? E ! N)->Sink(E,N)
//// silent(N): ()->Sink(Never,N)
//// filter(E,N): (sink: Sink(E,N), accept: (Level)->Bool)->Sink(E,N)
//// minimum(E,N): (sink: Sink(E,N), level: Level)->Sink(E,N)
//// enabled(E,N): (level: Level)->Bool ! Sink(E,N)
//// write(E,N): (entry: Entry)->Nil ? E ! N | Sink(E,N)
//// lazy(E,F,N): (level: Level, message: ()->String ? F ! N)->Nil ? E | F ! N | Sink(E,N)
//// format: (entry: Entry)->String
//// stderr/stdout: () -> Sink for the corresponding stream
//// file: (path: String) -> Sink appending formatted lines
//// timestamped: (entry: Entry) -> Entry, reading the Unix clock
////
//// Level order is Debug < Info < Warning < Failure. Entry strings are arbitrary;
//// fields preserve order and duplicate keys. Timestamps are explicit Unix
//// instants and are never added or replaced by delivery. No clock is read.
////
//// make stores callbacks without calling them. enabled checks accept once.
//// write checks the current sink once, then sends the original entry once only
//// if accepted. False returns Nil. Delivery preserves the original error, with
//// no retry, buffering, deduplication, task spawn, or failure suppression.
//// lazy checks once, then calls message once and sends an entry with no fields
//// or timestamp. Rejection calls neither callback; a message error skips send.
//// Eager write arguments still evaluate before filtering.
////
//// filter checks the new predicate before the old predicate; send is unchanged.
//// minimum accepts levels at or above its threshold. silent always rejects.
//// These factories are pure. enabled followed by write performs two checks.
//// Sink's E is covariant and its stored N invariant. The unused (N)->Nil marker
//// is never called. Replacements need equal service arguments; N cannot contain
//// the same Sink(E,N) recursively. Capture a concrete downstream sink instead.
////
//// format returns lowercase level, space, quoted message, optional " @" and
//// signed decimal Unix nanoseconds, then ordered quoted key=value pairs. It
//// adds no final LF. Quotes/backslashes and LF/CR/tab are escaped; other C0/C1
//// controls and U+2028/U+2029 use minimal lowercase \\u{h}. Other scalars remain.
//// Formatting is deterministic and linear in text length, plus integer printing.
////
//// Service laws compare delivery observations, not closure equality: accepted
//// entries arrive in sequential call order; filter(s,always_true) preserves
//// delivery; minimum(minimum(s,a),b) uses threshold max(a,b). No callback
//// termination guarantee is added. Generic client use awaits the §7 compiler
//// IO adapters are explicit and use the same sink service.
(
	i/core
	i/stdio
	i/fs
	i/resource
	std/list
	std/int
	std/time
	(Option, Some, None) = std/option
)

(
	Level(..)
	Entry(..)
	Sink
	make
	silent
	filter
	minimum
	enabled
	write
	lazy
	format
	stderr
	stdout
	file
	timestamped
)

Debug = ()
Info = ()
Warning = ()
Failure = ()
Level = Debug | Info | Warning | Failure

Entry = (
	level = Level,
	message = String,
	fields = List((String, String)),
	at = Option(time.Instant),
)

Sink(E, N) = ((Level) -> Bool, (Entry) <- (Fail(E), ..N) -> Nil, (N) -> Nil)

make = <E, N>(
	accept: (Level) -> Bool,
	send: (Entry) <- (Fail(E), ..N) -> Nil
) -> Sink(E, N) { Sink(accept, send, (_: N) { Nil }) }

silent = <N>() -> Sink(Never, N) {
	make((_: Level) { False }, (_: Entry) { Nil })
}

filter = <E, N>(sink: Sink(E, N), accept: (Level) -> Bool) -> Sink(E, N) {
	Sink(old_accept, send, marker) = sink
	Sink((level) { accept(level) && old_accept(level) }, send, marker)
}

minimum = <E, N>(sink: Sink(E, N), level: Level) -> Sink(E, N) {
	filter(sink, (candidate) { rank(candidate) >= rank(level) })
}

enabled = <E, N>(level: Level) <- (sink: Sink(E, N)) -> Bool {
	Sink(accept, _, _) = sink
	accept(level)
}

// Inferred returns retain the exact public error and environment rows (§7).
write = <E, N>(entry: Entry) <- (sink: Sink(E, N)) {
	Sink(accept, send, _) = sink
	accept(entry.level) |> (True) { send(entry) } | (False) { Nil }
}

lazy = <E, N>(level: Level, message: () <- N -> String) <- (sink: Sink(E, N)) {
	Sink(accept, send, _) = sink
	accept(level) |>
		(True) {
			send(
				Entry(
					level = level,
					message = message(),
					fields = [],
					at = None,
				),
			)
		}
		| (False) { Nil }
}

format = (entry: Entry) -> String {
	stamp = entry.at |>
		(None) { [] }
		| (Some(t)) { [" @", int.to_string(time.unix_nanoseconds(t))] }
	fields = list.flat_map(entry.fields, (field) {
		(key, value) = field
		[" ", quote(key), "=", quote(value)]
	})
	core.concat(
		list.append(
			[level_text(entry.level), " ", quote(entry.message), ..stamp],
			fields,
		),
	)
}

rank = (level: Level) -> Int {
	level |> (Debug) { 0 } | (Info) { 1 } | (Warning) { 2 } | (Failure) { 3 }
}

level_text = (level: Level) -> String {
	level |>
		(Debug) { "debug" }
		| (Info) { "info" }
		| (Warning) { "warning" }
		| (Failure) { "failure" }
}

quote = (text: String) -> String {
	core.concat(
		[
			"\"",
			..list.append(
				list.map(core.codepoints(text), escape),
				["\""],
			),
		],
	)
}

escape = (codepoint: UtfCodepoint) -> String {
	core.codepoint_to_int(codepoint) |>
		(34) { "\\\"" }
		| (92) { "\\\\" }
		| (10) { "\\n" }
		| (13) { "\\r" }
		| (9) { "\\t" }
		| (n ? n < 32 || { n >= 127 && n <= 159 } || n == 8232 || n == 8233) {
			core.concat(
				["\\u{", core.to_lower(int.to_base16(n)), "}"],
			)
		}
		| (_) { core.from_codepoints([codepoint]) }
}

/// A sink that writes each formatted entry to standard error.
stderr = () {
	make((_: Level) { True }, (entry) { stdio.print_error(format(entry)) })
}

/// A sink that writes each formatted entry to standard output.
stdout = () {
	make((_: Level) { True }, (entry) { stdio.print(format(entry)) })
}

/// A sink that appends each formatted entry to a file.
file = (path: String) {
	make(
		(_: Level) { True },
		(entry) {
			resource.acquire(
				() { fs.open(path, fs.Append) },
				(i) { fs.close(i) },
				(i) {
					text = format(entry) <> "\n"
					fs.write(i, <<text:utf8>>)
					fs.flush(i)
				},
			)
		},
	)
}

/// Adds the current Unix time to an entry.
timestamped = (entry: Entry) { Entry(..entry, at = Some(time.now())) }
