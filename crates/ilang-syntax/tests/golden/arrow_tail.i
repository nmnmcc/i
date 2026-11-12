T = (A) <- (Fail(E)) -> (B) <- (N) -> C
U = (A) -> (B) <- (Fail(E), N) -> C
V = (A) <- (N) -> (B) <- (Fail(E)) -> C
W = (A) <- (Fail(E)) -> (B) -> C
X = (A) <- (N) -> (B) -> C
Y = (A) <- (Fail((C) <- (N) -> D)) -> B
f = (x) -> (B) <- (N) -> C { x }
Z = (A) <- (Fail((C) -> D), N) -> B
