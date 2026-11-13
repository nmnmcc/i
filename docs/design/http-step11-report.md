# HTTP step 11

Implemented on `lane/io`.

## Results

- `cargo test --locked -p ilang-cli --test http --test tls --test echo`:
  11 HTTP tests, one TLS test and one echo test pass.
- `http_timeout` exercises `send_timeout(http.client(), ...)` against a real
  listener that withholds its response; the result is `http.Timeout`.
- `http_reuse` uses one accepted connection for two requests.
  `http_reuse_replaces_closed_connection` replaces a silently closed idle
  connection. `http_reuse_keeps_origins_separate` interleaves two ports and
  checks chunked response bodies and trailers.
- `http_stream_request_body` echoes a 1 MiB upload in reads of at most 4096
  bytes, for both chunked and Content-Length framing. It waits for the handler
  to read the first piece before sending the remainder, checks trailers, and
  verifies draining an unread body preserves the next pipelined request.
- `i run --project examples/http-get --opt 0`: prints `http ok` over HTTPS.
- `i fmt --check lib std examples`: passes.
- `i test --project std --opt 0`: 1839 passed, 0 failed.

## Timeout diagnosis

The cause was the checker, not a closed library signature. Before changing
it, the HTTP program was reduced first to a record callback passed through
`task.race`, then to `tests/spec/9/callback_open_service_row.i`, which needs
no task or HTTP library. Checking a body against `H | (N - H)` stored the
identity `N <: H | (N - H)` as an upper bound. Generalization split its repeated
N into unrelated variables and imposed that false restriction on the wrapper's
callback. The reduced failure was:

```
error[E0501]: `Second` is not a subtype of `First | _ - First`
```

The solver now recognizes that inclusion before recording variable bounds;
the difference's existing same-form obligations remain. `send_timeout` and
`task.race` retain their library signatures. The reduced spec case passes at
both optimization levels 0 and 2.

## Sources and interpretations

- Read Go `src/net/http/transport.go` (`shouldRetryRequest`, `readLoop`),
  `transport_pool.go` (`poolGetConn`, `poolPutIdleConnLocked`), `server.go`
  (`connReader`, `expectContinueReader.Read`) and `transfer.go` (`body.Read`,
  `body.readLocked`). Followed origin-keyed idle ownership, response completion
  before reuse, bounded stale-connection retry, head-first server dispatch and
  a separate framed body reader. Requests already contain replayable bytes;
  the task's explicit once-only retry applies to a stale connection.
- Read `lib/i/task.i`'s `race`, `both`, scope cleanup and persistent share
  table, and Simple-sub `Typer.scala`'s `extrude` and `freshenAbove`.
- Spec 11.1 makes values immutable and spec 9.9 makes channels rendezvous
  channels. To retain pool and body positions without HTTP-specific host or
  compiler state, added general `task.ref` and pure atomic `task.update`,
  owned by the current scope/run and using the ordinary scheduler share table.
  Their public contract is recorded in spec 9.9 and runtime design 6.1.
- The existing Request byte-body API remains the buffered `serve` interface;
  `serve_stream` provides the head plus a scoped pull reader. This follows the
  separate head/body approach in Go without changing message-value framing.
- `redesign` advanced during implementation to `7e240a4e`. Merged it as
  `0c382ea0`, retaining the ref API and its new `^` notation in the one doc
  conflict. Its formatter required canonical syntax in the acceptance source
  trees; that mechanical pass is isolated in `c70bca62`.

## Validation history

All cargo commands used `/home/agent/Programming/i/.rd/cache/heavy direnv exec
/home/agent/Programming/i`; no command was interrupted.

- `cargo check --locked -p ilang-check --all-targets`: passed.
- `cargo check --locked -p ilang-package -p ilang-check --all-targets`: passed.
- `cargo check --locked -p ilang-cli --all-targets`: passed, including after
  merging redesign.
- `cargo build --locked -p ilang-cli`: passed before and after the merge and
  the canonical formatting pass.
- `i check --project target/io-repro --types`: failed as expected with the
  E0501 above before the solver fix; passed afterward.
- During implementation `i check --project std` reported omitted effects on
  `dial` (`HttpError is not a subtype of Never`) and the inferred head's
  nominal-field requirement (`Request is not a subtype of (headers : _)`).
  Removed the closed dial result annotation and annotated `Progress(Request)`;
  the final standard-library check passes.
- Targeted `http_timeout` and `http_reuse`: passed.
- HTTP/TLS/echo suite: passed before and after merging redesign.
- `cargo test --locked -p ilang-cli --test spec -- callback_open_service_row`
  and the `task_ref` filter: passed at O0/O2.
- The first checker/package/host run found the old exported-name expectation:
  `assertion left == right failed: i/task` in `library_names`. Updated it for
  `ref`/`update`.
- A checker run started before the syntax merge subsequently read the formatted
  hello example with its old executable:
  `warning[W0504]: top-level binding main is not used` in `hello_example_checks`.
  The final acceptance recompiles against the merged checker and sources.
- The first merged acceptance run reached the golden library-type check and
  reported `library types differ (run with UPDATE=1 to bless)`. Reviewed the
  complete diff: only task callback rows lose the redundant `N & (H | N - H)`
  restrictions, and `ref`/`update` are added. Updated only `library.out`; the
  targeted `cargo test --locked -p ilang-check --test types library -- --exact`
  passed, followed by the full acceptance rerun.
- The merged formatter initially listed files such as
  `examples/echo/src/main.i` and `lib/i/task.i`; its canonical pass and subsequent
  `i fmt --check lib std examples` passed.

## Final acceptance

The supplied acceptance check was run in full, with `tee` added only to retain
its logs under `target/io-acceptance-*`. It exited 0 and printed:

```
checker/package/host passed
11 passed; 0 failed             # HTTP integration tests
1 passed; 0 failed              # TLS (echo also passed)
http ok
1839 passed, 0 failed
IO step 11 acceptance passed
```

The test-name checks, real-client timeout source check, redesign ancestry check
and format check all passed. Both new spec cases also passed at O0/O2 after
the merge and syntax formatting. No implementation or validation items remain
unfinished. The final implementation commit is `82756c56`; this report is
committed separately on `lane/io`.
