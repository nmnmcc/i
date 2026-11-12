# UDP delivery

No new language or library surface for state. Datagram is an IO resource, using
Net's existing private service and native N9 resource context, not a new state
mechanism. Branch: `lane/io`.

## Implemented

- `net.bind(address, port)` returns private `Datagram(Int)`; port 0 selects an OS
  port. `net.local_address` returns its numeric address and `net.local_port` works
  on both Datagram and Listener.
- `net.send_to(socket, address, port, bytes)` sends one whole-byte datagram;
  `net.receive_from(socket, max_bytes)` returns `(data, address, port)` with labels.
  IPv4, IPv6, host names, zero-length datagrams, message boundaries and truncation
  work. Receive bounds must be positive and are capped at 65536.
- `net.close_datagram` is idempotent/error-free. The existing context/lease rules
  give busy for overlapping operations in one direction, allow sending during a
  receive, wake receivers on close, release a cancelled receiver's lease and close
  forgotten sockets on entry teardown. Expected failures are NetError values.
- Rust uses tokio UdpSocket. Node uses node:dgram with a whole-datagram queue;
  request cancellation removes the waiter and releases the lease. Browser and WASI
  lack the Net provider and fail compilation naming `i/net.Net`.
- `examples/io-udp`, five i test cases, CLI integration tests on both providers at
  O0/O2, Node tests at O0/O2 and spec/runtime/native text are included.

## References and interpretations

Read local Go `src/net/udpsock.go` (UDPConn/ReadFromUDP/WriteToUDP), tokio
`tokio/src/net/udp.rs` (bind/send_to/recv_from and cancellation safety), and Eio
`lib_eio/net.mli` (switch-owned datagram_socket, send/recv, sender and truncation).
Follow their socket/resource approach rather than adding a task or channel API.

- The 2026-11-12 USER ledger decision supersedes the old UDP exclusion. No other
  language or state decision changes.
- The task's hostname requirement applies to bind as well as send_to, following
  TCP connect's system resolver/order, although TCP listen only accepts numeric
  addresses. Bind accepts 0..65535; send accepts 1..65535.
- The task leaves public spelling unspecified beyond bind/send_to/receive_from:
  Datagram, local_address, existing local_port, close_datagram follow i/net's
  Listener/Stream and close_stream/close_listener convention.
- Receive returns a labelled record (data/address/port); Eio/Go return payload and
  sender. Per spec §10.3 records cannot cross the ABI, so the private raw receive
  reply is a BitArray: little-endian u16 port, u8 address byte length, numeric IP
  UTF-8, payload. Ordinary i decodes it; malformed internal replies panic.
- Truncation discards the remaining message (Go/tokio/Eio buffer semantics).
  An empty datagram is data rather than TCP EOF; UDP promises no delivery/order.
- The worktree's existing branch is lane/io. The task's mention of redesign is
  interpreted as its base, since the shared rules forbid editing that branch.

## Verification

- `cargo check -p ilang-host -p ilang-cli --all-targets`: passed, twice.
- `cargo build --locked -p ilang-cli`: passed.
- `i run --project examples/io-udp`: passed, prints `udp ok`.
- `i test --project examples/io-udp --opt 0`: 5 passed.
- `node --test bindings/javascript/test/udp.test.mjs`: passed (O0/O2).
- `cargo test --locked -p ilang-cli --test udp`: 2 passed (both providers O0/O2,
  plus browser/WASI compile rejection).
- `UPDATE=1 cargo test --locked -p ilang-check --test types library`: passed;
  inspected snapshot changes are the UDP functions and local_port's union.
- Initial test authoring failures were fixed: `expected a name, found 1` for
  `<<1:1>>` (corrected to `<<1:u1>>`), then entry manifest diagnostics
  `an entry is name = "module.path.function"` and
  `entry checks must name a top-level function of this package` (corrected to
  the package-relative `checks.all`). The first CLI UDP test run failed on the
  former manifest diagnostic; its rerun passed.

The exact acceptance gate will be recorded after it completes. No feature remains
unimplemented.
