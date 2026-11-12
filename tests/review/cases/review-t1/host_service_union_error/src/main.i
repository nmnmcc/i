@host("app")
Svc = (get = () <- (Fail(String | "error")) -> String)
