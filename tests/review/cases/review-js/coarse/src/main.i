@host("probe:process/v1")
Process { Process(args: () -> String, spawn: (String) -> Int) }
@main
main = (p ! Process) { p.args() }
