//! The parts of Binaryen's C API (`binaryen/src/binaryen-c.h`) that are used.

#![allow(non_snake_case, non_camel_case_types)]

use std::ffi::{c_char, c_int, c_void};

pub type BinaryenFeatures = u32;

#[repr(C)]
pub struct BinaryenModule {
    _private: [u8; 0],
}

pub type BinaryenModuleRef = *mut BinaryenModule;

#[repr(C)]
pub struct BinaryenModuleAllocateAndWriteResult {
    pub binary: *mut c_void,
    pub binaryBytes: usize,
    pub sourceMap: *mut c_char,
}

unsafe extern "C" {
    pub fn BinaryenFeatureGC() -> BinaryenFeatures;
    pub fn BinaryenFeatureReferenceTypes() -> BinaryenFeatures;
    pub fn BinaryenFeatureTailCall() -> BinaryenFeatures;
    pub fn BinaryenFeatureExceptionHandling() -> BinaryenFeatures;
    pub fn BinaryenFeatureMultivalue() -> BinaryenFeatures;
    pub fn BinaryenFeatureBulkMemory() -> BinaryenFeatures;
    pub fn BinaryenFeatureBulkMemoryOpt() -> BinaryenFeatures;
    pub fn BinaryenFeatureSignExt() -> BinaryenFeatures;
    pub fn BinaryenFeatureNontrappingFPToInt() -> BinaryenFeatures;
    pub fn BinaryenFeatureMutableGlobals() -> BinaryenFeatures;

    pub fn BinaryenModuleReadWithFeatures(
        input: *mut c_char,
        input_size: usize,
        features: BinaryenFeatures,
    ) -> BinaryenModuleRef;
    pub fn BinaryenModuleSetFeatures(module: BinaryenModuleRef, features: BinaryenFeatures);
    pub fn BinaryenModuleValidate(module: BinaryenModuleRef) -> bool;
    pub fn BinaryenModuleOptimize(module: BinaryenModuleRef);
    pub fn BinaryenModuleAllocateAndWrite(
        module: BinaryenModuleRef,
        source_map_url: *const c_char,
    ) -> BinaryenModuleAllocateAndWriteResult;
    pub fn BinaryenModuleDispose(module: BinaryenModuleRef);

    pub fn BinaryenSetOptimizeLevel(level: c_int);
    pub fn BinaryenSetShrinkLevel(level: c_int);
    pub fn BinaryenSetClosedWorld(on: bool);
    pub fn BinaryenSetTrapsNeverHappen(on: bool);
    pub fn BinaryenSetDebugInfo(on: bool);
    pub fn BinaryenSetAlwaysInlineMaxSize(size: u32);
}
