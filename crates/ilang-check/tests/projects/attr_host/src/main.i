One = (f = () -> Nil)

Other = (g = () -> Nil)
@host("app:a/v1")
Two = One | Other
@host("app:a/v1")
Unlabelled = (() -> Nil)

@host("app:a/v1")
NotFn = (n = Int)

@host("app:a/v1")
Alias = Int

@host("app:b/v1", "async")
First = (read = () -> String, write = (String) -> Nil)
