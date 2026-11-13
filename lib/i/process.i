//// Process capabilities and secure entropy.
(
	i/request
	i/resource
	i/fail
	i/core
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
	Directory(..)
	read_stdout
	read_stderr
	write_stdin
	close_stdin
	ProcessError(..)
	args
	env
	cwd
	entropy
	spawn
	wait
	terminate
	close
)

ProcessError = (operation = String, code = String)

Inherit = ()
Null = ()
Piped = ()
Stdio = Inherit | Null | Piped

InheritEnvironment = (List(String))
ReplaceEnvironment = (List(String))
EnvironmentMode = InheritEnvironment | ReplaceEnvironment

InheritDirectory = ()
Directory = (String)
Directory = InheritDirectory | Directory

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
	own = (Int) -> Request(Nil),
	read_stdout = (Int, Int) -> Request(Result(BitArray, String)),
	read_stderr = (Int, Int) -> Request(Result(BitArray, String)),
	write_stdin = (Int, BitArray) -> Request(Result(Nil, String)),
	close_stdin = (Int) -> Request(Result(Nil, String)),
	spawn = (String, BitArray) -> Request(Result(Int, String)),
	wait = (Int) -> Request(Result(Int, String)),
	terminate = (Int) -> Request(Result(Nil, String)),
	close = (Int) -> Request(Nil),
)

length = ([]) { 0 } | ([_, ..rest]) { 1 + length(rest) }

u32 = (n) { <<n % 256, n / 256 % 256, n / 65536 % 256, n / 16777216 % 256>> }

encode_items =
	([]) { <<>> }
	| ([s, ..rest]) {
		bytes = <<s:utf8>>
		n = core.bit_size(bytes) / 8
		{ n <= 4294967295 } |>
			(True) { Nil }
			| (False) {
				fail.raise(ProcessError(operation = "spawn", code = "invalid"))
			}
		core.bit_array_concat([u32(n), bytes, encode_items(rest)])
	}

encode_args = (values) {
	n = length(values)
	{ n <= 4294967295 } |>
		(True) { Nil }
		| (False) {
			fail.raise(ProcessError(operation = "spawn", code = "invalid"))
		}
	core.bit_array_concat([u32(n), encode_items(values)])
}

decode_items = (bytes, count) {
	(bytes, count) |>
		((<<>>, 0)) { [] }
		| ((<<a, b, c, d, rest:bytes>>, n) ? n > 0) {
			size = a + b * 256 + c * 65536 + d * 16777216
			rest |>
				(<<item:bytes(size), tail:bytes>>) {
					text = core.bit_array_to_string(item) |>
						(Ok(s)) { s }
						| (Error(_)) { panic("process: invalid arguments") }
					[text, ..decode_items(tail, n - 1)]
				}
				| (_) { panic("process: invalid arguments") }
		}
		| (_) { panic("process: invalid arguments") }
}

decode_args =
	(<<a, b, c, d, rest:bytes>>) {
		decode_items(rest, a + b * 256 + c * 65536 + d * 16777216)
	}
	| (_) { panic("process: invalid arguments") }

args = () <- (raw : Arguments) {
	request.await_owned(raw.args()) |>
		(Ok(value)) { decode_args(value) }
		| (Error(code)) {
			fail.raise(ProcessError(operation = "args", code = code))
		}
}

env = (name) <- (raw : Environment) {
	request.await_owned(raw.env(name)) |>
		(Ok(value)) { value }
		| (Error(code)) {
			fail.raise(ProcessError(operation = "env", code = code))
		}
}

cwd = () <- (raw : WorkingDirectory) {
	request.await_owned(raw.cwd()) |>
		(Ok(value)) { value }
		| (Error(code)) {
			fail.raise(ProcessError(operation = "cwd", code = code))
		}
}

entropy = (bytes) <- (raw : Entropy) {
	request.await_owned(raw.entropy(bytes)) |>
		(Ok(value)) { value }
		| (Error(code)) {
			fail.raise(ProcessError(operation = "entropy", code = code))
		}
}

spawn = (program, values) <- (raw : Process) {
	r = raw.spawn(program, encode_args(values))
	number = resource.ensure(
		() {
			request.await(r) |>
				(Ok(value)) { value }
				| (Error(code)) {
					fail.raise(ProcessError(operation = "spawn", code = code))
				}
		},
		() {
			request.state(r) |>
				(request.Ready(Ok(value))) {
					request.await_owned(raw.close(value))
				}
				| (_) { Nil }
			request.cancel(request.ticket(r))
		},
	)
	raw.own(number)
	Child(number)
}

wait = (Child(number)) <- (raw : Process) {
	request.await_owned(raw.wait(number)) |>
		(Ok(value)) { value }
		| (Error(code)) {
			fail.raise(ProcessError(operation = "wait", code = code))
		}
}

terminate = (Child(number)) <- (raw : Process) {
	request.await_owned(raw.terminate(number)) |>
		(Ok(value)) { value }
		| (Error(code)) {
			fail.raise(ProcessError(operation = "terminate", code = code))
		}
}

close = (Child(number)) <- (raw : Process) {
	request.await_owned(raw.close(number))
}

stdio_mode = (Inherit) { "inherit" } | (Null) { "null" } | (Piped) { "piped" }

spawn_piped = (program, values, input, output, errors) <- (raw : Process) {
	r = raw.spawn_piped(
		program,
		encode_args(values),
		stdio_mode(input),
		stdio_mode(output),
		stdio_mode(errors),
	)
	number = resource.ensure(
		() {
			request.await(r) |>
				(Ok(value)) { value }
				| (Error(code)) {
					fail.raise(ProcessError(operation = "spawn", code = code))
				}
		},
		() {
			request.state(r) |>
				(request.Ready(Ok(value))) {
					request.await_owned(raw.close(value))
				}
				| (_) { Nil }
			request.cancel(request.ticket(r))
		},
	)
	raw.own(number)
	Child(number)
}

/// Spawn with inherited or replacement environment entries and a working directory.
/// Entries NAME=value set variables; NAME removes a variable.
spawn_config = (
	program,
	values,
	input,
	output,
	errors,
	environment,
	directory
) <- (raw : Process) {
	(env_mode, entries) = environment |>
		(InheritEnvironment(entries)) { ("inherit", entries) }
		| (ReplaceEnvironment(entries)) { ("replace", entries) }
	(cwd_mode, path) = directory |>
		(InheritDirectory) { ("inherit", "") }
		| (Directory(path)) { ("set", path) }
	r = raw.spawn_config(
		program,
		encode_args(values),
		stdio_mode(input),
		stdio_mode(output),
		stdio_mode(errors),
		env_mode,
		encode_args(entries),
		path,
		cwd_mode,
	)
	number = resource.ensure(
		() {
			request.await(r) |>
				(Ok(value)) { value }
				| (Error(code)) {
					fail.raise(ProcessError(operation = "spawn", code = code))
				}
		},
		() {
			request.state(r) |>
				(request.Ready(Ok(value))) {
					request.await_owned(raw.close(value))
				}
				| (_) { Nil }
			request.cancel(request.ticket(r))
		},
	)
	raw.own(number)
	Child(number)
}

read_stdout = (Child(number), max) <- (raw : Process) {
	request.await_owned(raw.read_stdout(number, max)) |>
		(Ok(value)) { value }
		| (Error(code)) {
			fail.raise(ProcessError(operation = "read_stdout", code = code))
		}
}

read_stderr = (Child(number), max) <- (raw : Process) {
	request.await_owned(raw.read_stderr(number, max)) |>
		(Ok(value)) { value }
		| (Error(code)) {
			fail.raise(ProcessError(operation = "read_stderr", code = code))
		}
}

write_stdin = (Child(number), bytes) <- (raw : Process) {
	request.await_owned(raw.write_stdin(number, bytes)) |>
		(Ok(value)) { value }
		| (Error(code)) {
			fail.raise(ProcessError(operation = "write_stdin", code = code))
		}
}

close_stdin = (Child(number)) <- (raw : Process) {
	request.await_owned(raw.close_stdin(number)) |>
		(Ok(value)) { value }
		| (Error(code)) {
			fail.raise(ProcessError(operation = "close_stdin", code = code))
		}
}
