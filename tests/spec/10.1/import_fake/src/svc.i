(
	Svc(..)
	twice
)

@import("app:x/v1")
Svc = (ping = () -> Int)

twice = () <- (s : Svc) -> Int { s.ping() + s.ping() }
