f = (r : Result(Int, Int)) { r |> (Ok(v)) { v } | (Error(v)) { v } }
