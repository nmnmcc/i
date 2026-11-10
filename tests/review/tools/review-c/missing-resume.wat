(module
  (import "test:work/v1" "work" (func $work (param i32)))
  (memory (export "memory") 1)
  (func (export "ilang_start"))
  (func (export "ilang_alloc") (param i32) (result i32) (i32.const 32))
  (func (export "ilang_free") (param i32 i32))
  (func (export "app/sync"))
  (func (export "app/run") (result i32) (call $work (i32.const 0)) (i32.const 0))
  
  (func (export "ilang_result:app/run"))
  (@custom "ilang.abi" "{\"version\":3,\"entry\":null,\"tests\":[],\"exports\":{\"app/run\":{\"params\":[],\"result\":\"nil\",\"errors\":[],\"async\":true},\"app/sync\":{\"params\":[],\"result\":\"nil\",\"errors\":[],\"async\":false}},\"services\":{\"test:work/v1\":{\"work\":{\"params\":[],\"result\":\"nil\",\"errors\":[],\"async\":true}}},\"externals\":{}}"))