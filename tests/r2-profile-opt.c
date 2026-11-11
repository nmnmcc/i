/* Reproduce the compiler's O2 pass with names retained for profile attribution.
 * Strip custom sections and compare with the real artifact before using names.
 * This helper never changes compiler code or times an instrumented module. */
#include "binaryen-c.h"
#include <assert.h>
#include <stdio.h>
#include <stdlib.h>

int main(int argc, char **argv) {
  assert(argc == 3);
  FILE *input = fopen(argv[1], "rb");
  assert(input);
  fseek(input, 0, SEEK_END);
  long length = ftell(input);
  rewind(input);
  char *bytes = malloc((size_t)length);
  assert(bytes && fread(bytes, 1, (size_t)length, input) == (size_t)length);
  fclose(input);
  BinaryenFeatures features = BinaryenFeatureGC() |
      BinaryenFeatureReferenceTypes() | BinaryenFeatureTailCall() |
      BinaryenFeatureExceptionHandling() | BinaryenFeatureMultivalue() |
      BinaryenFeatureBulkMemory() | BinaryenFeatureBulkMemoryOpt() |
      BinaryenFeatureSignExt() | BinaryenFeatureNontrappingFPToInt() |
      BinaryenFeatureMutableGlobals();
  BinaryenModuleRef module =
      BinaryenModuleReadWithFeatures(bytes, (size_t)length, features);
  assert(module);
  BinaryenModuleSetFeatures(module, features);
  BinaryenSetOptimizeLevel(2);
  BinaryenSetShrinkLevel(0);
  BinaryenSetClosedWorld(true);
  BinaryenSetTrapsNeverHappen(false);
  BinaryenSetDebugInfo(true);
  BinaryenSetAlwaysInlineMaxSize(20);
  BinaryenModuleOptimize(module);
  assert(BinaryenModuleValidate(module));
  BinaryenModuleAllocateAndWriteResult output =
      BinaryenModuleAllocateAndWrite(module, NULL);
  assert(output.binary);
  FILE *file = fopen(argv[2], "wb");
  assert(file && fwrite(output.binary, 1, output.binaryBytes, file) == output.binaryBytes);
  fclose(file);
  free(output.binary);
  free(output.sourceMap);
  BinaryenModuleDispose(module);
  free(bytes);
}
