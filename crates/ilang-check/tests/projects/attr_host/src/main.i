@host("app:a/v1")
Two = One | Other

One {
	One(f: () -> Nil)
}

Other {
	Other(g: () -> Nil)
}

@host("app:a/v1")
Unlabelled {
	Unlabelled(() -> Nil)
}

@host("app:a/v1")
NotFn {
	NotFn(n: Int)
}

@host("app:a/v1")
Alias = Int

@host("app:b/v1", "async")
First {
	First(read: () -> String, write: (String) -> Nil)
}
