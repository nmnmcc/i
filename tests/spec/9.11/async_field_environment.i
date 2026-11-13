// spec: 9.7
// expect: compile-error E0320
// runtime.md D3 and 5.2: the async field environment is the fixed Await | Owner expansion.
Other = ()
@host("test:request", "async")
Wrong = (start = () <- (Other) -> Nil)
