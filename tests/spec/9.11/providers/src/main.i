(Bundle, probe)
Bundle { Bundle(value: Int) }
@host("test:io/v1")
Work { Work(print: (text: String) -> Nil) }
make = (work ! Work) { work.print("provider built") Bundle(42) }
probe = (bundle ! Bundle) { assert(bundle.value == 42) }
main = () { probe() }
