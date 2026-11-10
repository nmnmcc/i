#include "binaryen-c.h"
#include <fstream>
#include <iterator>
#include <vector>
#include <cstdlib>
int main(int argc, char **argv) {
 std::ifstream in(argv[1], std::ios::binary);
 std::vector<char> b((std::istreambuf_iterator<char>(in)), {});
 auto features=BinaryenFeatureGC()|BinaryenFeatureReferenceTypes()|BinaryenFeatureTailCall()|BinaryenFeatureExceptionHandling()|BinaryenFeatureMultivalue()|BinaryenFeatureBulkMemory()|BinaryenFeatureBulkMemoryOpt()|BinaryenFeatureSignExt()|BinaryenFeatureNontrappingFPToInt()|BinaryenFeatureMutableGlobals();
 auto m=BinaryenModuleReadWithFeatures(b.data(), b.size(), features);
 BinaryenModuleSetFeatures(m,features);
 if(!BinaryenModuleValidate(m)) return 1;
 BinaryenSetOptimizeLevel(2); BinaryenSetShrinkLevel(0); BinaryenSetClosedWorld(true); BinaryenSetTrapsNeverHappen(false); BinaryenSetDebugInfo(false); BinaryenSetAlwaysInlineMaxSize(20);
 BinaryenModuleOptimize(m);
 if(!BinaryenModuleValidate(m)) return 2;
 auto out=BinaryenModuleAllocateAndWrite(m,nullptr);
 std::ofstream f(argv[2],std::ios::binary); f.write((char*)out.binary,out.binaryBytes);
 free(out.binary); free(out.sourceMap); BinaryenModuleDispose(m);
}
