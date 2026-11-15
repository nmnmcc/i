(spec/helper)

// A module boundary must retain the generalized helper's empty-meet check.
need_int = () <- (r: helper.Read(Int)) { r.value }
need_string = () <- (r: helper.Read(String)) { r.value }
use = () { helper.both(need_int, need_string) }
