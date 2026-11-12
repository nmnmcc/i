# JS step 02 report

No new language state mechanism. HTTP uses the existing task.ref/task.update;
host response resources use the existing ResourceContext table. Fetch response
identifiers remain private behind i/fetch.Response, like i/fs.File.

Implemented:

- `i deps --project examples/http-get --target js` selects
  std/http.fetch_client and i/fetch.Fetch, with no Net/Tls service. Native and
  Node defaults capture Net/Tls and retain the native connection pool.
- `http.get`, post, send and send_timeout use that ordinary Client service.
  Fetch sends method/URL/headers/body and provides status, visible headers,
  whole bodies and incremental send_stream readers. AbortController follows
  request cancellation; acquisition guards close a response delivered during
  cancellation, and the run context owns abandoned readers until teardown.
- WASI preview-1 checked adapters supply Io/Stdio, clocks, random_get,
  arguments, environment and Fs over preopens. Raw iovecs, subscriptions,
  pointer vectors, paths, stats and dirents are kernel-owned layouts. File
  tokens are never reused; command completion and stops close remaining files.
  Unavailable Net/Tls/Process/WorkingDirectory are compile errors.
- Node fetch tests use a local node:http server. The stream server withholds
  its second chunk until i reads the first and sends an acknowledgement.
  WASI tests run hello/io-clock/io-fs plus args/env/entropy at O0 and O2 through
  the included Node preview-1 runner. Browser fixture exercises real http.get.

References read: Go src/net/http/roundtrip_js.go (RoundTrip, streamReader,
AbortController and context cancellation); Deno ext/fetch/26_fetch.js
(createResponseBodyStream, mainFetch, abortFetch); jco preview2-shim filesystem
helper documentation and browser http/clocks shims; Wasmtime
crates/wasi-preview1-component-adapter/src/lib.rs (clock_time_get,
fd_prestat_get/dir_name, path_open, fd_read/readdir, random_get).
Undici was absent; Deno and Go supplied the fetch pattern.

Interpretations:

- Spec 2.6.1 forbids dependency packages overriding root selections. Package
  defaults for their *own* nominal service types are permitted; root cells
  override them, and a dependency row for someone else's service is an error.
  This implements the task's ordinary std/http provider without an HTTP
  compiler special case. Compiler macro programs clear all runtime plans.
- Spec 9.4 resumes tail calls in the resumer environment. Provider closures
  retain an ordinary non-tail frame around transport calls to preserve their
  explicit raw-service bindings while leaving task services dynamic.
- Fetch hides DNS causes in browsers, so opaque browser failures use Connect;
  exposed ENOTFOUND/EAI_AGAIN causes use Dns. CORS, forbidden headers,
  decompression and redirects follow host fetch; trailers are empty (ledger
  USER HTTP ruling and http.md). Non-2xx responses remain responses.
- B.7 keeps blocking WASI requests and custom imports. This step implements
  checked capability adapters with internal copied buffers, not the separately
  designed public Wasi root/token interface. temp_dir selects /tmp or the first
  preopen, because preview 1 has no ambient cwd or temporary-directory query.

Verification before the full acceptance run:

- Heavy-wrapper cargo check for ilang-check/package/wasm/cli and macro with
  --all-targets: passed.
- Heavy-wrapper cargo test for ilang-cli providers and ilang-package load /
  providers: passed after updating the library module inventory for i/fetch.
- Node fetch suite: passed (GET, POST, headers, non-2xx, timeout, cancellation,
  refused connection, DNS failure, incremental body). Extra body-cancel case
  is included in the final npm check.
- Node WASI suite: 5 passed at O0/O2; browser fixture: 3 passed, zero skipped.
- Native `i run --project examples/http-get`: http ok.
- old-syntax-check: 4624 extracted literals/blocks passed. Added manifest-only
  classifications for TOML strings; i snippets remain parsed.

Failures found and fixed during implementation:

- `assertion failed: assert io_clock/main:12:2` at WASI O2: Node rounds poll
  timeouts; the adapter now rechecks the monotonic deadline.
- `provider target host ... app/main.S: UnknownModule`: macro compilation
  must discard all packages' runtime defaults, not only root/library plans.
- `trap: illegal cast` after a tail-call provider resumed: retain the raw
  service binding's frame as required by spec 9.4; native HTTPS and streaming
  now pass without changing continuation semantics.
- `there is no package directory ../../../../../std`: corrected browser
  fixture's relative dependency path.

Full acceptance: pending execution on the committed tree. No requested
implementation is left unfinished. Native send_stream is buffered; JS fetch
streams incrementally, as required by this step.

Branch: lane/js. Implementation commit: 3935d22a; see the final verification
commit for the report's completed acceptance results.
