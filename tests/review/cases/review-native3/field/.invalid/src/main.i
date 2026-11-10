// old-syntax-check: E0101
import (r = app/resource, i/core, (Io) = i/io)
raw_number = (raw: r.Raw) { raw.open() }
@main
main = (io ! Io) { io.print(core.inspect(raw_number(r.seed()))) }
