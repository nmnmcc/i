(Bundle, probe)
Bundle { Bundle }
@host("probe", "request")
Work { Work(start: () -> Request(Int)) }
make = (work ! Work) { Bundle }
probe = (bundle ! Bundle) { Nil }
