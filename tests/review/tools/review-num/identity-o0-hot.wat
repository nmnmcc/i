;; executed 1 times
  (func (;346;) (type 139) (param f64) (result f64)
    ref.null $obj
    global.get $root_env
    local.get 0
    call $abi_float_in
    call $f0
    call $abi_float_out
  )

;; executed 1 times
  (func $f0 (;315;) (type $fn1) (param $clo (ref null $obj)) (param $env (ref null $env)) (param $l0 eqref) (result eqref)
    (local $l1 eqref) (local $slot i32)
    call $shadow_enter_call
    local.get $l0
    local.set $l1
    call $shadow_leave
    local.get $l1
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

;; executed 1 times
  (func $float_get (;109;) (type 52) (param $v eqref) (result f64)
    local.get $v
    ref.cast (ref $obj)
    struct.get $obj 3
  )

;; executed 1 times
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