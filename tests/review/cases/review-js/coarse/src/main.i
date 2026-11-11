@host("probe:process/v1")
Process { Process(args: () -> String, spawn: (String) -> Int) }

main = (p ! Process) { p.args() }
