(
 i/tls
 i/core
 i/stdio
)
main = () {
 opts = (..tls.options(), alpn: ["http/1.1"])
 stream = tls.dial("example.com", 443, opts)
 tls.write(stream, <<"GET / HTTP/1.1\r\nHost: example.com\r\nConnection: close\r\n\r\n":utf8>>)
 reply = tls.read(stream, 65536)
 text = core.bit_array_to_string(reply) |> { Ok(value) -> value Error(_) -> panic("invalid response") }
 assert(core.byte_slice(text, 0, 12) == Ok("HTTP/1.1 200"))
 tls.close(stream)
 stdio.print("tls ok")
}
