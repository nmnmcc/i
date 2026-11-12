Cell(A) = (run = (A) -> A)
make = () { Cell(run = (x) { x }) }
Same = make
