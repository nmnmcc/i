Level = "info" | "warn"
@external("app", "log")
log : (Level | "error") -> ()
@external("app", "number")
number : (Int - 0) -> ()
