(
	testlib = (test) = std/test
	io_udp/checks
)

test("UDP IPv4 boundaries, truncation and empty payload", checks.ipv4)

test("UDP IPv6 datagrams", checks.ipv6)

test("UDP hostname binding and errors as values", checks.errors)

test("UDP close wakes blocked receive", checks.close_pending)

test("UDP scope cancellation releases receive", checks.cancel_pending)
