(Schema,make)
Schema(D,E) {Schema(decode:(String)->D,encode:(E)->String)}
make(D,E) = (d:(String)->D,e:(E)->String) {Schema(d,e)}
