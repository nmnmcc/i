@host("app")
Svc = (get = () <- (Fail("error")) -> "a")
