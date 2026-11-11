(
	i/stdio
	i/request
	std/cli
	std/log
	(None) = std/option
)

Options {
	Options(name: String, count: Int)
}

options = () {
	cli.map2(
		cli.option(cli.Names("name", None), "Name to greet", cli.text)
		cli.option(cli.Names("count", None), "Number of greetings", cli.integer)
		(name, count) { Options(name, count) }
	)
}

greet = (name, count) {
	{ count > 0 } |> {
		True -> {
			stdio.print("hello " <> name)
			greet(name, count - 1)
		}
		False -> Nil
	}
}

main = () {
	cli.run(options(), "cli-log", (opts) {
		! log.Sink(
			stdio.StdioError
			stdio.Stdio | request.Await | request.Owner
		) = log.stderr()
		log.write(log.timestamped(log.Entry(log.Info, "greeting", [], None)))
		greet(opts.name, opts.count)
		stdio.print("cli ok")
	})
}
