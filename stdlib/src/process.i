//// Process capabilities and secure entropy.
////
//// Each capability is a separate host service: `Arguments`, `Environment`,
//// `WorkingDirectory`, `Entropy` and `Process`. An entry whose need holds a
//// service its target does not give is the compile error "`T` is not
//// provided by the target `t`" (E0523). The checker's target table gives
//// `Process` and `WorkingDirectory` on `native` and `nodejs`, `Arguments`
//// and `Environment` on `native`, `nodejs` and `wasi`, and `Entropy` on
//// every target. The native host and the Node binding implement `Process`;
//// on a target that does not give it, an entry that needs `Process` does
//// not compile.
////
//// A child belongs to the current `resource.Scope`. `spawn`, `spawn_piped`
//// and `spawn_config` attach a release to it before they give the `Child`;
//// when the scope ends, in any way, the release terminates the child,
//// waits for it so that no zombie outlives the scope, and closes its
//// handle and pipes. A child that already exited is fine: the release
//// ignores the errors of each step.
////
//// Errors are `ProcessError` values. `reason` is the host code mapped once
//// to a `ProcessProblem`; `code` keeps the host's code for diagnostics.
////
//// The scope belongs to the task that made it: a child task opens its own
//// `resource.scoped` before it spawns (the specification's resource
//// contract); attaching to a scope
//// from another task stops.
(
	i/request
	i/resource
	i/fail
	i/core
	i/task
)

(
	Arguments(..)
	Environment(..)
	WorkingDirectory(..)
	Entropy(..)
	Process
	Child
	Stdio(..)
	spawn_piped
	spawn_config
	EnvironmentMode(..)
	DirectoryMode(..)
	read_stdout
	read_stderr
	write_stdin
	close_stdin
	collect
	ProcessError(..)
	ProcessProblem(..)
	args
	env
	cwd
	entropy
	spawn
	wait
	terminate
	close
)

NotFound = ()
Denied = ()
Closed = ()
Busy = ()
Invalid = ()
Other = ()

/// Why a process operation failed, mapped from the host's error code:
/// `"not_found"` is `NotFound`, `"denied"` is `Denied`, `"closed"` is
/// `Closed`, `"busy"` is `Busy`, `"invalid"` is `Invalid`, and every other
/// code (`"io"`, `"not_utf8"`, ...) is `Other`.
ProcessProblem = NotFound | Denied | Closed | Busy | Invalid | Other

/// A failed process operation. `operation` names the function (`"spawn"`,
/// `"wait"`, `"read_stdout"`, ...), `reason` is the mapped problem, and
/// `code` is the host's original code, kept for diagnostics only.
///
/// ```i
/// example = () {
/// 	r = fail.catch(() {
/// 		resource.scoped(() { process.spawn("/no/such/program", []) })
/// 	})
/// 	r |>
/// 		(Error(e)) { assert(e.reason == process.NotFound) }
/// 		& (Ok(_)) { panic("spawned") }
/// }
/// ```
ProcessError = (operation = String, reason = ProcessProblem, code = String)

Inherit = ()
Null = ()
Piped = ()
Stdio = Inherit | Null | Piped

InheritEnvironment = (List(String))
ReplaceEnvironment = (List(String))
EnvironmentMode = InheritEnvironment | ReplaceEnvironment

InheritDirectory = ()
AtDirectory = (String)
/// The working directory of a child: `InheritDirectory` keeps the parent's,
/// `AtDirectory(path)` sets an explicit path.
DirectoryMode = InheritDirectory | AtDirectory

Child = (Int)

@host("i:arguments/v1", "request")
Arguments = (args = () -> Request(Result(BitArray, String)))

@host("i:environment/v1", "request")
Environment = (env = (String) -> Request(Result(String, String)))

@host("i:working-directory/v1", "request")
WorkingDirectory = (cwd = () -> Request(Result(String, String)))

@host("i:entropy/v1", "request")
Entropy = (entropy = (Int) -> Request(Result(BitArray, String)))

@host("i:process/v1", "request")
Process = (
	spawn_piped = (String, BitArray, String, String, String) -> Request(
		Result(Int, String),
	),
	spawn_config = (
		String,
		BitArray,
		String,
		String,
		String,
		String,
		BitArray,
		String,
		String,
	) -> Request(Result(Int, String)),
	own = (Int) -> Request(()),
	read_stdout = (Int, Int) -> Request(Result(BitArray, String)),
	read_stderr = (Int, Int) -> Request(Result(BitArray, String)),
	write_stdin = (Int, BitArray) -> Request(Result((), String)),
	close_stdin = (Int) -> Request(Result((), String)),
	spawn = (String, BitArray) -> Request(Result(Int, String)),
	wait = (Int) -> Request(Result(Int, String)),
	terminate = (Int) -> Request(Result((), String)),
	close = (Int) -> Request(()),
)

// The one place that reads host error codes.
problem = (code : String) -> ProcessProblem {
	code |>
		("not_found") { NotFound() }
		& ("denied") { Denied() }
		& ("closed") { Closed() }
		& ("busy") { Busy() }
		& ("invalid") { Invalid() }
		& (_) { Other() }
}

raise_error = (operation : String, code : String) <- (
	error : Fail(ProcessError),
) -> Never {
	fail.raise(
		ProcessError(
			operation = operation,
			reason = problem(code),
			code = code,
		),
	)
}

length = ([]) { 0 } & ([_, ..rest]) { 1 + length(rest) }

u32 = (n) { <<n % 256, n / 256 % 256, n / 65536 % 256, n / 16777216 % 256>> }

encode_items =
	([]) { <<>> }
	& ([s, ..rest]) {
		bytes = <<s:utf8>>
		n = core.bit_size(bytes) / 8
		{ n <= 4294967295 } |>
			(True()) { () }
			& (False()) { raise_error("spawn", "invalid") }
		core.bit_array_concat([u32(n), bytes, encode_items(rest)])
	}

encode_args = (values) {
	n = length(values)
	{ n <= 4294967295 } |>
		(True()) { () }
		& (False()) { raise_error("spawn", "invalid") }
	core.bit_array_concat([u32(n), encode_items(values)])
}

decode_items = (bytes, count) {
	(bytes, count) |>
		((<<>>, 0)) { [] }
		& ((<<a, b, c, d, rest:bytes>>, n) ? n > 0) {
			size = a + b * 256 + c * 65536 + d * 16777216
			rest |>
				(<<item:bytes(size), tail:bytes>>) {
					text = core.bit_array_to_string(item) |>
						(Ok(s)) { s }
						& (Error(_)) { panic("process: invalid arguments") }
					[text, ..decode_items(tail, n - 1)]
				}
				& (_) { panic("process: invalid arguments") }
		}
		& (_) { panic("process: invalid arguments") }
}

decode_args =
	(<<a, b, c, d, rest:bytes>>) {
		decode_items(rest, a + b * 256 + c * 65536 + d * 16777216)
	}
	& (_) { panic("process: invalid arguments") }

/// The program's arguments, without the program name.
/// Needs: `Arguments`.
/// Errors: `ProcessError(operation = "args", ...)` when the host cannot give them.
args = () <- (raw : Arguments) {
	request.await_owned(raw.args()) |>
		(Ok(value)) { decode_args(value) }
		& (Error(code)) { raise_error("args", code) }
}

/// The value of the environment variable `name`.
/// Needs: `Environment`.
/// Errors: `ProcessError(operation = "env", reason = NotFound, ...)` when it
/// is not set; `Invalid` for an empty name or one with `=` or NUL; `Other`
/// (code `"not_utf8"`) when the value is not UTF-8.
env = (name) <- (raw : Environment) {
	request.await_owned(raw.env(name)) |>
		(Ok(value)) { value }
		& (Error(code)) { raise_error("env", code) }
}

/// The working directory of the program.
/// Needs: `WorkingDirectory`.
/// Errors: `ProcessError(operation = "cwd", ...)`.
cwd = () <- (raw : WorkingDirectory) {
	request.await_owned(raw.cwd()) |>
		(Ok(value)) { value }
		& (Error(code)) { raise_error("cwd", code) }
}

/// `bytes` secure random bytes.
/// Needs: `Entropy`.
/// Errors: `ProcessError(operation = "entropy", reason = Invalid, ...)` for a
/// negative or too large count.
entropy = (bytes) <- (raw : Entropy) {
	request.await_owned(raw.entropy(bytes)) |>
		(Ok(value)) { value }
		& (Error(code)) { raise_error("entropy", code) }
}

// Waits for a spawn request. A spawn that is cancelled after the host made
// the child still closes it.
await_spawn = (r : Request(Result(Int, String))) <- (raw : Process) -> Child {
	number = resource.ensure(
		() {
			request.await(r) |>
				(Ok(value)) { value }
				& (Error(code)) { raise_error("spawn", code) }
		},
		() {
			request.state(r) |>
				(request.Ready(Ok(value))) {
					request.await_owned(raw.close(value))
				}
				& (_) { () }
			request.cancel(request.ticket(r))
		},
	)
	raw.own(number)
	Child(number)
}

// The scope's release: terminate, then wait (reaps the child), then close.
// A child that already exited, or a handle that is already closed, makes
// the first two steps fail; their errors are dropped so the release itself
// cannot fail.
release = (child : Child) <- (raw : Process) {
	_ = fail.catch(() { terminate(child) })
	_ = fail.catch(() { wait(child) })
	close(child)
}

/// Starts `program` with `values` as its arguments; its standard streams
/// are inherited. The child belongs to the current `resource.Scope`: when
/// the scope ends it is terminated, reaped and closed.
/// Needs: `resource.Scope`, `Process`.
/// Errors: `ProcessError(operation = "spawn", ...)`; `NotFound` when the
/// program does not exist, `Denied` when it cannot be run, `Invalid` for
/// arguments the host cannot pass.
///
/// ```i
/// example = () {
/// 	resource.scoped(() {
/// 		child = process.spawn("sh", ["-c", "exit 3"])
/// 		assert(process.wait(child) == 3)
/// 	})
/// }
/// ```
spawn = (program, values) <- (_scope : resource.Scope, raw : Process) {
	resource.acquire(
		() {
			await_spawn(
				raw.spawn(program, encode_args(values)),
			)
		},
		release,
	)
}

/// Waits for `child` to exit and gives its exit code (`128 + signal` for a
/// child ended by a signal). Waiting again gives the same code.
/// Needs: `Process`.
/// Errors: `ProcessError(operation = "wait", reason = Closed, ...)` after
/// `close`.
///
/// ```i
/// example = () {
/// 	resource.scoped(() {
/// 		child = process.spawn("sh", ["-c", "exit 3"])
/// 		assert(process.wait(child) == 3)
/// 	})
/// }
/// ```
wait = (Child(number)) <- (raw : Process) {
	request.await_owned(raw.wait(number)) |>
		(Ok(value)) { value }
		& (Error(code)) { raise_error("wait", code) }
}

/// Asks `child` to stop (SIGKILL on Unix) and returns at once; `wait` then
/// gives its code. A child that already exited is not an error.
/// Needs: `Process`.
/// Errors: `ProcessError(operation = "terminate", reason = Closed, ...)`
/// after `close`.
///
/// ```i
/// example = () {
/// 	resource.scoped(() {
/// 		child = process.spawn("sleep", ["30"])
/// 		process.terminate(child)
/// 		assert(process.wait(child) != 0)
/// 	})
/// }
/// ```
terminate = (Child(number)) <- (raw : Process) {
	request.await_owned(raw.terminate(number)) |>
		(Ok(value)) { value }
		& (Error(code)) { raise_error("terminate", code) }
}

/// Drops the handle of `child` and its pipes, stopping and reaping a child
/// that still runs. Closing twice is fine; the scope's release closes the
/// child as well.
/// Needs: `Process`. Raises nothing.
close = (Child(number)) <- (raw : Process) {
	request.await_owned(raw.close(number))
}

stdio_mode =
	(Inherit()) { "inherit" }
	& (Null()) { "null" }
	& (Piped()) { "piped" }

/// Like `spawn`, with a choice of `Inherit`, `Null` or `Piped` for stdin,
/// stdout and stderr. Piped streams are used with `write_stdin`,
/// `close_stdin`, `read_stdout`, `read_stderr` and `collect`. The child
/// belongs to the current `resource.Scope`.
/// Needs: `resource.Scope`, `Process`.
/// Errors: as `spawn`.
///
/// ```i
/// example = () {
/// 	resource.scoped(() {
/// 		child = process.spawn_piped(
/// 			"sh",
/// 			["-c", "printf hi"],
/// 			process.Null(),
/// 			process.Piped(),
/// 			process.Inherit(),
/// 		)
/// 		assert(process.read_stdout(child, 100) == <<"hi":utf8>>)
/// 		assert(process.wait(child) == 0)
/// 	})
/// }
/// ```
spawn_piped = (program, values, input, output, errors) <- (
	_scope : resource.Scope,
	raw : Process,
) {
	resource.acquire(
		() {
			await_spawn(
				raw.spawn_piped(
					program,
					encode_args(values),
					stdio_mode(input),
					stdio_mode(output),
					stdio_mode(errors),
				),
			)
		},
		release,
	)
}

/// Like `spawn_piped`, with an environment and a working directory.
/// `InheritEnvironment(entries)` starts from the parent's variables,
/// `ReplaceEnvironment(entries)` from none; an entry `NAME=value` sets a
/// variable and `NAME` removes one. `AtDirectory(path)` runs the child in
/// `path`. The child belongs to the current `resource.Scope`.
/// Needs: `resource.Scope`, `Process`.
/// Errors: as `spawn`; `Invalid` for a malformed entry.
///
/// ```i
/// example = () {
/// 	resource.scoped(() {
/// 		child = process.spawn_config(
/// 			"sh",
/// 			["-c", "test \"$X\" = y && test \"$PWD\" = /"],
/// 			process.Null(),
/// 			process.Null(),
/// 			process.Inherit(),
/// 			process.ReplaceEnvironment(["X=y"]),
/// 			process.AtDirectory("/"),
/// 		)
/// 		assert(process.wait(child) == 0)
/// 	})
/// }
/// ```
spawn_config = (
	program,
	values,
	input,
	output,
	errors,
	environment,
	directory
) <- (_scope : resource.Scope, raw : Process) {
	(env_mode, entries) = environment |>
		(InheritEnvironment(entries)) { ("inherit", entries) }
		& (ReplaceEnvironment(entries)) { ("replace", entries) }
	(cwd_mode, path) = directory |>
		(InheritDirectory()) { ("inherit", "") }
		& (AtDirectory(path)) { ("set", path) }
	resource.acquire(
		() {
			await_spawn(
				raw.spawn_config(
					program,
					encode_args(values),
					stdio_mode(input),
					stdio_mode(output),
					stdio_mode(errors),
					env_mode,
					encode_args(entries),
					path,
					cwd_mode,
				),
			)
		},
		release,
	)
}

/// Reads at most `max` bytes (at most 65536) from the child's piped stdout.
/// `<<>>` means end of file.
/// Needs: `Process`.
/// Errors: `ProcessError(operation = "read_stdout", reason = Closed, ...)`
/// when stdout is not piped or the child is closed; `Invalid` for `max < 1`.
///
/// ```i
/// example = () {
/// 	resource.scoped(() {
/// 		child = process.spawn_piped(
/// 			"sh",
/// 			["-c", "printf hi"],
/// 			process.Null(),
/// 			process.Piped(),
/// 			process.Inherit(),
/// 		)
/// 		assert(process.read_stdout(child, 100) == <<"hi":utf8>>)
/// 		assert(process.read_stdout(child, 100) == <<>>)
/// 	})
/// }
/// ```
read_stdout = (Child(number), max) <- (raw : Process) {
	request.await_owned(raw.read_stdout(number, max)) |>
		(Ok(value)) { value }
		& (Error(code)) { raise_error("read_stdout", code) }
}

/// Reads at most `max` bytes (at most 65536) from the child's piped stderr.
/// `<<>>` means end of file.
/// Needs: `Process`.
/// Errors: as `read_stdout`, with `operation = "read_stderr"`.
///
/// ```i
/// example = () {
/// 	resource.scoped(() {
/// 		child = process.spawn_piped(
/// 			"sh",
/// 			["-c", "printf oops >&2"],
/// 			process.Null(),
/// 			process.Null(),
/// 			process.Piped(),
/// 		)
/// 		assert(process.read_stderr(child, 100) == <<"oops":utf8>>)
/// 	})
/// }
/// ```
read_stderr = (Child(number), max) <- (raw : Process) {
	request.await_owned(raw.read_stderr(number, max)) |>
		(Ok(value)) { value }
		& (Error(code)) { raise_error("read_stderr", code) }
}

/// Writes all of `bytes` to the child's piped stdin.
/// Needs: `Process`.
/// Errors: `ProcessError(operation = "write_stdin", reason = Closed, ...)`
/// when stdin is not piped or was closed; `Other` (code `"io"`) when the
/// child closed its end.
///
/// ```i
/// example = () {
/// 	resource.scoped(() {
/// 		child = process.spawn_piped(
/// 			"cat",
/// 			[],
/// 			process.Piped(),
/// 			process.Piped(),
/// 			process.Inherit(),
/// 		)
/// 		process.write_stdin(child, <<"hi":utf8>>)
/// 		process.close_stdin(child)
/// 		assert(process.read_stdout(child, 100) == <<"hi":utf8>>)
/// 	})
/// }
/// ```
write_stdin = (Child(number), bytes) <- (raw : Process) {
	request.await_owned(raw.write_stdin(number, bytes)) |>
		(Ok(value)) { value }
		& (Error(code)) { raise_error("write_stdin", code) }
}

/// Closes the child's stdin, so a child reading it sees end of file.
/// Closing it again is fine.
/// Needs: `Process`.
/// Errors: `ProcessError(operation = "close_stdin", reason = Closed, ...)`
/// after `close`.
///
/// ```i
/// example = () {
/// 	resource.scoped(() {
/// 		child = process.spawn_piped(
/// 			"cat",
/// 			[],
/// 			process.Piped(),
/// 			process.Null(),
/// 			process.Inherit(),
/// 		)
/// 		process.close_stdin(child)
/// 		assert(process.wait(child) == 0)
/// 	})
/// }
/// ```
close_stdin = (Child(number)) <- (raw : Process) {
	request.await_owned(raw.close_stdin(number)) |>
		(Ok(value)) { value }
		& (Error(code)) { raise_error("close_stdin", code) }
}

chunk_size = 65536

reverse_onto = <A>(list : List(A), acc : List(A)) -> List(A) {
	list |> ([]) { acc } & ([x, ..rest]) { reverse_onto(rest, [x, ..acc]) }
}

// Reads stdout to end of file; `chunks` holds the reads so far, newest first.
drain_stdout = (child : Child, chunks : List(BitArray)) <- (
	raw : Process,
) -> BitArray {
	chunk = read_stdout(child, chunk_size)
	{ core.bit_size(chunk) == 0 } |>
		(True()) { core.bit_array_concat(reverse_onto(chunks, [])) }
		& (False()) { drain_stdout(child, [chunk, ..chunks]) }
}

drain_stderr = (child : Child, chunks : List(BitArray)) <- (
	raw : Process,
) -> BitArray {
	chunk = read_stderr(child, chunk_size)
	{ core.bit_size(chunk) == 0 } |>
		(True()) { core.bit_array_concat(reverse_onto(chunks, [])) }
		& (False()) { drain_stderr(child, [chunk, ..chunks]) }
}

read_all_stdout = (child : Child) <- (raw : Process) -> BitArray {
	drain_stdout(child, [])
}

read_all_stderr = (child : Child) <- (raw : Process) -> BitArray {
	drain_stderr(child, [])
}

/// Reads the child's stdout and stderr to end of file, both at once in two
/// tasks, so a child that fills both pipes does not deadlock.
///
/// Both streams must be `Piped` (`spawn_piped` or `spawn_config`). When the
/// child reads stdin until end of file, call `close_stdin` first. The result
/// is not bounded: `collect` keeps everything the child writes, so use it
/// only for a child whose output size the caller accepts, and
/// `read_stdout`/`read_stderr` otherwise. It does not wait for the child;
/// call `wait` for its exit code.
/// Needs: `resource.Scope`, `task.Scheduler`, `task.Runner`, `Process`.
/// Errors: `ProcessError(operation = "read_stdout", ...)` or
/// `operation = "read_stderr"` for the first read that fails, e.g.
/// `Closed` when a stream is not piped; the other read is then cancelled.
///
/// ```i
/// example = () {
/// 	task.run(() {
/// 		resource.scoped(() {
/// 			child = process.spawn_piped(
/// 				"sh",
/// 				["-c", "printf out; printf err >&2"],
/// 				process.Null(),
/// 				process.Piped(),
/// 				process.Piped(),
/// 			)
/// 			output = process.collect(child)
/// 			assert(output.stdout == <<"out":utf8>>)
/// 			assert(output.stderr == <<"err":utf8>>)
/// 			assert(process.wait(child) == 0)
/// 		})
/// 	})
/// }
/// ```
collect = (child : Child) <- (
	_scope : resource.Scope,
	own : task.Scheduler,
	_runner : task.Runner,
	raw : Process,
	error : Fail(ProcessError),
) -> (stdout = BitArray, stderr = BitArray) {
	(out, err) = task.both(
		() { read_all_stdout(child) },
		() { read_all_stderr(child) },
	)
	(stdout = out, stderr = err)
}
