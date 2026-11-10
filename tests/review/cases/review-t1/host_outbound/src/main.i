Level = "info" | "warn"
@external("app", "log")
log: (Level | "error") -> Nil
@external("app", "number")
number: (Int - 0) -> Nil
