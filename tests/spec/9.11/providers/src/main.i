(Bundle, probe)
Bundle { Bundle(value: Int) }
@host("test:io/v1")
Work { Work(print: (String) -> Nil) }
make = (work ! Work) { work.print("provider built") Bundle(value: 42) }
probe = (bundle ! Bundle) { assert(bundle.value == 42) }
main = () { probe() }
