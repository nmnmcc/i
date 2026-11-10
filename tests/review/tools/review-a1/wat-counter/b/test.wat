(module
(type $sites (array (mut i32)))
(type $values (array (mut i32)))
(data $site_data "")
(func $sites_install (param (ref $sites)) (param (ref $values)))
;; app/main.b
(func $f1 (result i32) (i32.const 22))
;; app/main.a
(func $f0 (result i32) (i32.const 11))
(func $ilang_start
(call $sites_install (array.new_data $sites $site_data (i32.const 0) (i32.const 0))
(array.new_fixed $values 0)))
(export "answer" (func 1))
)
