/* Per-call native timing through Wasmtime's C API. The caller supplies the
 * function pointer and opaque handles from wasmtime 50.0.1's Python bindings.
 * Layout assertions in the Python driver check the documented raw value ABI.
 * Each target invocation is individually timed and paired with the identical
 * empty-loop/identity invocation. Compilation, instantiation and warmup are
 * outside both intervals. No Python executes inside these measurement loops. */
#define _POSIX_C_SOURCE 200809L
#include <stdint.h>
#include <stddef.h>
#include <time.h>

typedef union { int32_t i32; int64_t i64; double f64; uint8_t v128[16]; } Raw;
typedef void *(*Call)(void *, const void *, Raw *, size_t, void **);
static int64_t ns(struct timespec a, struct timespec b) {
 return (b.tv_sec - a.tv_sec) * INT64_C(1000000000) + b.tv_nsec - a.tv_nsec;
}
static int64_t invoke(Call call, void *context, const void *function, double x, double *sink) {
 _Alignas(16) Raw values[2] = {{0}, {0}};
 values[0].f64 = x;
 void *trap = NULL;
 struct timespec start, end;
 clock_gettime(CLOCK_MONOTONIC, &start);
 void *error = call(context, function, values, 1, &trap);
 clock_gettime(CLOCK_MONOTONIC, &end);
 if (error || trap) return -INT64_C(1000000000000000);
 *sink = values[0].f64;
 return ns(start, end);
}
int measure(void *api, void *context, const void *function, const void *empty,
            double x, size_t count, int64_t *target_ns, int64_t *empty_ns) {
 Call call = (Call)api;
 double sink = 0;
 for (size_t i = 0; i < (count < 20 ? 2 : 200); ++i) {
  if (invoke(call, context, function, x, &sink) < 0) return 1;
 }
 for (size_t i = 0; i < count; ++i) {
  /* Alternate the order to expose drift rather than always favoring one. */
  if (i & 1) {
   target_ns[i] = invoke(call, context, function, x, &sink);
   empty_ns[i] = invoke(call, context, empty, x, &sink);
  } else {
   empty_ns[i] = invoke(call, context, empty, x, &sink);
   target_ns[i] = invoke(call, context, function, x, &sink);
  }
  if (target_ns[i] < 0 || empty_ns[i] < 0) return 1;
 }
 return sink == 123456789.0;
}

typedef double (*Native)(double);
static int64_t native_invoke(Native function, double x, double *sink) {
 struct timespec start,end;
 clock_gettime(CLOCK_MONOTONIC,&start);
 *sink = function(x);
 clock_gettime(CLOCK_MONOTONIC,&end);
 return ns(start,end);
}
int measure_native(void *target, void *baseline, double x, size_t count,
                   int64_t *target_ns, int64_t *empty_ns) {
 Native function=(Native)target, empty=(Native)baseline;
 double sink=0;
 for(size_t i=0;i<200;++i) native_invoke(function,x,&sink);
 for(size_t i=0;i<count;++i) {
  if(i&1) { target_ns[i]=native_invoke(function,x,&sink); empty_ns[i]=native_invoke(empty,x,&sink); }
  else { empty_ns[i]=native_invoke(empty,x,&sink); target_ns[i]=native_invoke(function,x,&sink); }
 }
 return sink==123456789.0;
}
