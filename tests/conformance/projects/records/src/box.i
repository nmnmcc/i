pub opaque type Box { Box(Int) }
pub fn new(value: Int) -> Box { Box(value) }
pub fn value(box: Box) -> Int { let Box(value) = box value }
