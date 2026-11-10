/* Native libm comparison for float_gate.py. No part of the i runtime. */
#define _POSIX_C_SOURCE 200809L
#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <time.h>

static double to_radians(double x, double y) { (void)y; return x * (acos(-1.0) / 180.0); }
static double to_degrees(double x, double y) { (void)y; return x * (180.0 / acos(-1.0)); }
#define UNARY(name) static double unary_##name(double x, double y) { (void)y; return name(x); }
UNARY(sin)
UNARY(cos)
UNARY(tan)
UNARY(asin)
UNARY(acos)
UNARY(atan)
static double elapsed(struct timespec a, struct timespec b) {
 return (double)(b.tv_sec - a.tv_sec) + (double)(b.tv_nsec - a.tv_nsec) * 1e-9;
}
int main(int argc, char **argv) {
 if (argc != 4) return 2;
 double (*functions[])(double, double) = {unary_sin, unary_cos, unary_tan,
  unary_asin, unary_acos, unary_atan, atan2, hypot, to_radians, to_degrees};
 int code = atoi(argv[1]);
 if (code < 0 || code >= 10) return 2;
 double (*volatile operation)(double, double) = functions[code];
 double x = strtod(argv[2], NULL), y = strtod(argv[3], NULL);
 volatile double sink = 0.0;
 struct timespec start, end;
 const int count = 100000;
 clock_gettime(CLOCK_MONOTONIC, &start);
 for (int j = 0; j < count; ++j) sink = operation(x, y);
 clock_gettime(CLOCK_MONOTONIC, &end);
 printf("%.9g\n", elapsed(start, end) / count);
 return sink == 123456789.0;
}
