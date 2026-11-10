// old-syntax-check: E0101
import (r = app/resource)
@external("attack", "forge")
forge: (Int) -> r.Stream
@main
main = () { forge(777) }
