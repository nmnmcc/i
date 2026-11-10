from probes import *
run('host_one_control','''@external("app", "write")
write: (1) -> Nil
''')
run('host_whole_control','''@external("app", "read")
read: () -> Int
''')
run('host_service_union_error','''@host("app")
Svc { Svc(get: () -> String ? String | "error") }
''')
run('host_union_internal','''Whole = {Int - 0} | 0
One = {1 | 2} - 2
In = Int & 1
identity = (n: Whole) -> Int { n }
reverse = (n: Int) -> Whole { n }
a: One = 1
b: In = 1
''')
