# HTTP

`std/http` provides HTTP/1.1 request and response values, case-insensitive
header lookup, incremental framing, and message serialization. Bodies and
trailers are represented as bytes and headers. The parser reports incomplete
input separately from invalid input and preserves unconsumed bytes for the
next message. Framing supports content lengths and chunked transfer encoding.

`http.get(url)`, `http.post(url, bytes)` and `http.send(request)` use the
target-selected `Client` service. Native and Node select the network provider;
JS selects host fetch through ordinary provider-plan cells in std/i.toml.
The compiler has no HTTP-specific selection. Request targets supplied to the client are absolute URLs;
the transport sends the path and query as the HTTP request target. HTTPS
uses TLS with the host name for certificate verification and SNI, and requests
ALPN `http/1.1`. Redirects are resolved against the current URL with a limit
of ten; DNS resolution failures use `Dns` and connection failures use `Connect`. `body_text`, `body_json` and `body_decode` provide UTF-8, JSON and
schema decoding. `send_timeout(provider, request, nanoseconds)` races the
request against a scheduler-friendly clock sleep.

A provider is an ordinary record with a `send` function. The target default is
the ordinary nominal `Client` service; `client()` returns its send/close record.
Root plans can replace this service like any application service. The native default returns
the network provider, and `send_with(provider, request)` calls a supplied
provider. Each client owns an idle connection pool keyed by scheme, host and
port. Fully framed keep-alive responses return their connection to that pool;
close-delimited responses, upgrades and explicit closes do not. A request that
finds its reused connection closed retries once on a fresh connection. Call
`client.close()` to close its idle connections; the convenience `send`, `get`
and `post` calls close their temporary client after the request. A fake provider
lets tests and applications select a different transport without compiler support. A browser `fetch` provider must observe
browser restrictions: scripts cannot set forbidden request headers, CORS limits
cross-origin access, and raw trailers are not exposed by fetch.

`serve(listener, handler)` runs a scoped task per connection, parses requests
incrementally, and answers requests on a connection in order. It honors
`Connection: close`, sends 100 Continue when requested, limits the head to
32768 bytes, returns 400 for malformed requests, and converts handler errors to
500. `route(method, pattern, request)` matches slash-separated paths; a
segment beginning with `:` captures that segment by name.

`serve_stream(listener, handler)` calls `handler(head, body)` as soon as the
request head is complete. The head is a `Request` with an empty byte body;
`body.read(max_bytes)` pulls at most that many decoded bytes, returning empty
bytes at the end. Both Content-Length and chunked uploads are incremental,
including chunks larger than one read. `body.trailers()` returns decoded
trailers after the body ends. The reader is valid during the handler call.
The server drains any unread body before parsing another request, preserving
pipelined bytes. `serve` uses this reader to provide its buffered Request API.

Pool and body-reader state use ordinary scope-owned `task.ref` values and
pure atomic `task.update` callbacks. No HTTP state is held in the compiler or
host. The general service-row checker preserves arbitrary services when a
network client is passed through `send_timeout`.

`send_stream(request)` returns `(head, read, close)`: read(max_bytes) pulls at
most that many bytes, with empty bytes at EOF. The fetch provider delivers
chunks incrementally as ReadableStream data arrives; the native provider
exposes its buffered response through the same interface. Always close the
reader, preferably through resource.ensure; the run context also owns fetch
responses and closes them on teardown. Native Client instances share their
connection pool across calls during an entry invocation.

Fetch uses the host's redirect and decompression policy, visible headers,
CORS and forbidden-header restrictions; trailers are empty. Non-2xx status
is returned unchanged. Timeout and task cancellation abort fetch through an
AbortController. Observable DNS causes map to Dns, other network failures to
Connect; browser fetch deliberately hides DNS details, so it reports Connect.
