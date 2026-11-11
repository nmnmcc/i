// old-syntax-check: E0101
import (r = app/resource, i/core, (Io) = i/io)
forge = (n: Int) { ! r.Raw = r.Raw(() { n })
 r.open() }

main = (io ! Io) { io.print(core.inspect(forge(777))) }
