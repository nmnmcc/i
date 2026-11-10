;; executed 29 times
  (func $float_get (;109;) (type 52) (param $v eqref) (result f64)
    local.get $v
    ref.cast (ref $obj)
    struct.get $obj 3
  )

;; executed 15 times
  (func $float_new (;108;) (type 53) (param $v f64) (result eqref)
    local.get $v
    f64.abs
    f64.const 0x1.fffffffffffffp+1023 (;=179769313486231570000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000;)
    f64.le
    i32.eqz
    if ;; label = @1
      call $stop_float
      unreachable
    end
    i32.const 2
    i32.const 0
    i64.const 0
    local.get $v
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    struct.new $obj
  )

;; executed 14 times
  (func $shadow_site (;180;) (type 95) (param $site i32)
    global.get $sp
    i32.eqz
    if ;; label = @1
      local.get $site
      return_call $shadow_enter
    end
    global.get $shadow
    global.get $sp
    i32.const 1
    i32.sub
    local.get $site
    array.set $frames
  )

;; executed 7 times
  (func $float_mul (;203;) (type $call) (param $a eqref) (param $b eqref) (result eqref)
    local.get $a
    call $float_get
    local.get $b
    call $float_get
    f64.mul
    call $float_new
  )

;; executed 7 times
  (func $float_add (;201;) (type $call) (param $a eqref) (param $b eqref) (result eqref)
    local.get $a
    call $float_get
    local.get $b
    call $float_get
    f64.add
    call $float_new
  )

;; executed 1 times
  (func (;348;) (type 139) (param f64) (result f64)
    ref.null $obj
    global.get $root_env
    local.get 0
    call $abi_float_in
    call $f2
    call $abi_float_out
  )

;; executed 1 times
  (func $f2 (;317;) (type $fn1) (param $clo (ref null $obj)) (param $env (ref null $env)) (param $l0 eqref) (result eqref)
    (local $l1 eqref) (local $l2 eqref) (local $l3 eqref) (local $l4 eqref) (local $l5 eqref) (local $l6 eqref) (local $l7 eqref) (local $l8 eqref) (local $l9 eqref) (local $l10 eqref) (local $l11 eqref) (local $l12 eqref) (local $l13 eqref) (local $l14 eqref) (local $l15 eqref) (local $l16 eqref) (local $l17 eqref) (local $l18 eqref) (local $l19 eqref) (local $l20 eqref) (local $l21 eqref) (local $slot i32)
    call $shadow_enter_call
    local.get $l0
    local.set $l1
    i32.const 21
    call $shadow_site
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4593671619917905920
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    struct.new $obj
    local.get $l1
    call $float_mul
    local.set $l2
    i32.const 21
    call $shadow_site
    local.get $l2
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4598175219545276416
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    struct.new $obj
    call $float_add
    local.set $l3
    local.get $l3
    local.set $l4
    i32.const 22
    call $shadow_site
    local.get $l4
    local.get $l1
    call $float_mul
    local.set $l5
    i32.const 22
    call $shadow_site
    local.get $l5
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4600427019358961664
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    struct.new $obj
    call $float_add
    local.set $l6
    local.get $l6
    local.set $l7
    i32.const 23
    call $shadow_site
    local.get $l7
    local.get $l1
    call $float_mul
    local.set $l8
    i32.const 23
    call $shadow_site
    local.get $l8
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4602678819172646912
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    struct.new $obj
    call $float_add
    local.set $l9
    local.get $l9
    local.set $l10
    i32.const 24
    call $shadow_site
    local.get $l10
    local.get $l1
    call $float_mul
    local.set $l11
    i32.const 24
    call $shadow_site
    local.get $l11
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4603804719079489536
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    struct.new $obj
    call $float_add
    local.set $l12
    local.get $l12
    local.set $l13
    i32.const 25
    call $shadow_site
    local.get $l13
    local.get $l1
    call $float_mul
    local.set $l14
    i32.const 25
    call $shadow_site
    local.get $l14
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4604930618986332160
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    struct.new $obj
    call $float_add
    local.set $l15
    local.get $l15
    local.set $l16
    i32.const 26
    call $shadow_site
    local.get $l16
    local.get $l1
    call $float_mul
    local.set $l17
    i32.const 26
    call $shadow_site
    local.get $l17
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4606056518893174784
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    struct.new $obj
    call $float_add
    local.set $l18
    local.get $l18
    local.set $l19
    i32.const 27
    call $shadow_site
    local.get $l19
    local.get $l1
    call $float_mul
    local.set $l20
    i32.const 27
    call $shadow_site
    local.get $l20
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4607182418800017408
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    struct.new $obj
    call $float_add
    local.set $l21
    call $shadow_leave
    local.get $l21
    return
    unreachable
  )

;; executed 1 times
  (func $abi_float_out (;249;) (type 52) (param $value eqref) (result f64)
    local.get $value
    call $float_get
  )

;; executed 1 times
  (func $abi_float_in (;248;) (type 53) (param $value f64) (result eqref)
    local.get $value
    f64.abs
    f64.const 0x1.fffffffffffffp+1023 (;=179769313486231570000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000;)
    f64.le
    i32.eqz
    if ;; label = @1
      call $stop_host_value
      unreachable
    end
    local.get $value
    call $float_new
  )

;; executed 1 times
  (func $shadow_enter_call (;178;) (type $unpend)
    global.get $tail
    call $shadow_enter
    i32.const -1
    global.set $tail
  )

;; executed 1 times
  (func $shadow_leave (;177;) (type $unpend)
    global.get $sp
    i32.const 1
    i32.sub
    global.set $sp
  )

;; executed 1 times
  (func $shadow_enter (;176;) (type 95) (param $site i32)
    (local $sp i32)
    global.get $sp
    local.set $sp
    local.get $sp
    global.get $shadow
    array.len
    i32.ge_u
    if ;; label = @1
      call $shadow_grow
    end
    global.get $shadow
    local.get $sp
    local.get $site
    array.set $frames
    local.get $sp
    i32.const 1
    i32.add
    global.set $sp
  )