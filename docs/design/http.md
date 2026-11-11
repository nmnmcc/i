# HTTP

`std/http` provides HTTP/1.1 request and response values, case-insensitive
header lookup, incremental framing, and message serialization. Bodies and
trailers are represented as bytes and headers. The parser reports incomplete
input separately from invalid input and preserves unconsumed bytes for the
next message. Framing supports content lengths and chunked transfer encoding.

`http.get(url)`, `http.post(url, bytes)` and `http.send(request)` use the
network provider. Request targets supplied to the client are absolute URLs;
the transport sends the path and query as the HTTP request target. HTTPS
uses TLS with the host name for certificate verification and SNI, and requests
ALPN `http/1.1`. Redirects are resolved against the current URL with a limit
of ten; DNS resolution failures use `Dns` and connection failures use `Connect`. `body_text`, `body_json` and `body_decode` provide UTF-8, JSON and
schema decoding. `send_timeout(provider, request, nanoseconds)` races the
request against a scheduler-friendly clock sleep.

A provider is an ordinary record with a `send` function. `client()` returns
the network provider, and `send_with(provider, request)` calls a supplied
provider. This lets a test use a fake and lets applications select a different
transport without compiler support. A browser `fetch` provider must observe
browser restrictions: scripts cannot set forbidden request headers, CORS limits
cross-origin access, and raw trailers are not exposed by fetch.

`serve(listener, handler)` runs a scoped task per connection, parses requests
incrementally, and answers requests on a connection in order. It honors
`Connection: close`, sends 100 Continue when requested, limits the head to
32768 bytes, returns 400 for malformed requests, and converts handler errors to
500. `route(method, pattern, request)` matches slash-separated paths; a
segment beginning with `:` captures that segment by name.

The present transport closes connections after each request. Server handlers
receive a buffered body. `send_timeout` works with a fake provider, but the
network provider currently fails the effect check when passed to it.
