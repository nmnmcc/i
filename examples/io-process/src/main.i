(
	i/process
	i/stdio
)

@main
main = () {
	process.args()
	process.env("PATH")
	child = process.spawn_piped(
		"sh"
		["-c", "echo hi; exit 3"]
		process.Null
		process.Piped
		process.Inherit
	)
	assert(process.read_stdout(child, 100) == <<"hi\n":utf8>>)
	assert(process.wait(child) == 3)
	process.close(child)
	cat = process.spawn_piped(
		"cat"
		[]
		process.Piped
		process.Piped
		process.Inherit
	)
	process.write_stdin(cat, <<"hello\n":utf8>>)
	assert(process.read_stdout(cat, 100) == <<"hello\n":utf8>>)
	process.close_stdin(cat)
	assert(process.wait(cat) == 0)
	process.close(cat)
	assert(process.entropy(16) != process.entropy(16))
	stdio.print("process ok")
}
