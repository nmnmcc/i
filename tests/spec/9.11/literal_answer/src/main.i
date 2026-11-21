// spec: 9.11, 10.3
// expect: compile-error E0320
// Spec 10.3 after T1: host-given Int/String members must contain all values.
// R2 applies that rule to the answer inside Request, not to its outer wrapper.
(
	i/request
)

@host("test:literal/v1", "request")
Work = (next = () -> Request("ok"))
