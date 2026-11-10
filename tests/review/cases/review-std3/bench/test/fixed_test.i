(review/fixed, i/core)
@test
cosine_reduced_one = () {
 v=fixed.cosine(fixed.I(-194231925111544910466465579149167180195,-194231925111544910466465579149167180194))
 lo=core.to_float(v.lo) /. 340282366920938463463374607431768211456.0
 hi=core.to_float(v.hi) /. 340282366920938463463374607431768211456.0
 assert(core.float_to_bits(lo)==0x3feaed548f090cee)
 assert(core.float_to_bits(hi)==0x3feaed548f090cee)
}
