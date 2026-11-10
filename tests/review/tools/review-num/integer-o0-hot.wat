;; executed 257 times
  (func $is_bool (;265;) (type 112) (param $v eqref) (param $b i32) (result i32)
    local.get $v
    call $value_tag
    i32.const 1
    i32.eq
    if (result i32) ;; label = @1
      local.get $v
      ref.cast (ref $obj)
      struct.get $obj 1
      local.get $b
      i32.eq
    else
      i32.const 0
    end
  )

;; executed 257 times
  (func $value_tag (;100;) (type 36) (param $v eqref) (result i32)
    local.get $v
    ref.is_null
    if ;; label = @1
      i32.const 0
      return
    end
    local.get $v
    ref.test (ref i31)
    if ;; label = @1
      i32.const 8
      return
    end
    local.get $v
    ref.test (ref $hostref)
    if ;; label = @1
      i32.const 10
      return
    end
    local.get $v
    ref.cast (ref $val)
    struct.get $val 0
  )

;; executed 256 times
  (func $int_from_i64 (;8;) (type 40) (param $v i64) (result eqref)
    (local $sign i32) (local $mag i64)
    local.get $v
    i64.const -1073741824
    i64.ge_s
    local.get $v
    i64.const 1073741823
    i64.le_s
    i32.and
    if ;; label = @1
      local.get $v
      i32.wrap_i64
      ref.i31
      return
    end
    local.get $v
    i64.const 0
    i64.lt_s
    if (result i32) ;; label = @1
      i32.const -1
    else
      i32.const 1
    end
    local.set $sign
    local.get $sign
    i32.const 0
    i32.lt_s
    if (result i64) ;; label = @1
      i64.const 0
      local.get $v
      i64.sub
    else
      local.get $v
    end
    local.set $mag
    local.get $mag
    i64.const 4294967295
    i64.le_u
    if ;; label = @1
      local.get $sign
      local.get $mag
      i32.wrap_i64
      array.new_fixed $limbs 1
      call $int_pack
      return
    end
    local.get $sign
    local.get $mag
    i32.wrap_i64
    local.get $mag
    i64.const 32
    i64.shr_u
    i32.wrap_i64
    array.new_fixed $limbs 2
    call $int_pack
  )

;; executed 130 times
  (func $shadow_enter_call (;178;) (type $unpend)
    global.get $tail
    call $shadow_enter
    i32.const -1
    global.set $tail
  )

;; executed 130 times
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

;; executed 129 times
  (func $f10 (;325;) (type $fn2) (param $clo (ref null $obj)) (param $env (ref null $env)) (param $l0 eqref) (param $l1 eqref) (result eqref)
    (local $l2 eqref) (local $l3 eqref) (local $l4 eqref) (local $l5 eqref) (local $l6 eqref) (local $slot i32)
    call $shadow_enter_call
    local.get $l0
    local.set $l2
    local.get $l1
    local.set $l3
    local.get $l2
    i32.const 0
    ref.i31
    call $equal
    call $bool_new
    local.set $l4
    block $b0
      block $b1
        local.get $l4
        i32.const 1
        call $is_bool
        i32.eqz
        if ;; label = @3
          br $b1
        else
        end
        call $shadow_leave
        local.get $l3
        return
      end
      block $b2
        local.get $l4
        i32.const 0
        call $is_bool
        i32.eqz
        if ;; label = @3
          br $b2
        else
        end
        local.get $l2
        i32.const 1
        ref.i31
        call $int_sub
        local.set $l5
        local.get $l3
        local.get $l2
        call $int_add
        local.set $l6
        i32.const -18
        call $shadow_leave_tail
        ref.null $obj
        local.get $env
        local.get $l5
        local.get $l6
        return_call $f10
      end
      unreachable
    end
    unreachable
  )

;; executed 129 times
  (func $equal (;122;) (type 43) (param $a eqref) (param $b eqref) (result i32)
    local.get $a
    local.get $b
    call $value_eq
  )

;; executed 129 times
  (func $value_eq (;121;) (type 43) (param $a eqref) (param $b eqref) (result i32)
    local.get $a
    local.get $b
    ref.eq
    if ;; label = @1
      i32.const 1
      return
    end
    local.get $a
    local.get $b
    call $both_small
    if ;; label = @1
      i32.const 0
      return
    end
    local.get $a
    local.get $b
    call $value_compare_i32
    i32.eqz
  )

;; executed 129 times
  (func $bool_new (;106;) (type 77) (param $v i32) (result eqref)
    local.get $v
    if (result eqref) ;; label = @1
      global.get $value_true
    else
      global.get $value_false
    end
  )

;; executed 128 times
  (func $shadow_leave_tail (;179;) (type 95) (param $site i32)
    (local $sp i32)
    global.get $sp
    i32.const 1
    i32.sub
    local.set $sp
    local.get $site
    i32.const -1
    i32.eq
    if ;; label = @1
      global.get $shadow
      local.get $sp
      array.get $frames
      local.set $site
      local.get $site
      i32.const -2
      i32.gt_s
      if ;; label = @2
        i32.const -1
        local.set $site
      end
    end
    local.get $site
    global.set $tail
    local.get $sp
    global.set $sp
  )

;; executed 128 times
  (func $int_sub (;16;) (type $call) (param $a eqref) (param $b eqref) (result eqref)
    local.get $a
    ref.test (ref i31)
    local.get $b
    ref.test (ref i31)
    i32.and
    if (result eqref) ;; label = @1
      local.get $a
      ref.cast (ref i31)
      i31.get_s
      i64.extend_i32_s
      local.get $b
      ref.cast (ref i31)
      i31.get_s
      i64.extend_i32_s
      i64.sub
      call $int_from_i64
    else
      local.get $a
      local.get $b
      call $int_neg
      call $int_add
    end
  )

;; executed 128 times
  (func $int_add (;14;) (type $call) (param $a eqref) (param $b eqref) (result eqref)
    (local $as i32) (local $bs i32) (local $cmp i32) (local $al (ref null $limbs)) (local $bl (ref null $limbs))
    local.get $a
    ref.test (ref i31)
    local.get $b
    ref.test (ref i31)
    i32.and
    if ;; label = @1
      local.get $a
      ref.cast (ref i31)
      i31.get_s
      i64.extend_i32_s
      local.get $b
      ref.cast (ref i31)
      i31.get_s
      i64.extend_i32_s
      i64.add
      call $int_from_i64
      return
    end
    local.get $a
    call $int_sign
    local.set $as
    local.get $b
    call $int_sign
    local.set $bs
    local.get $as
    i32.eqz
    if ;; label = @1
      local.get $b
      return
    end
    local.get $bs
    i32.eqz
    if ;; label = @1
      local.get $a
      return
    end
    local.get $a
    call $int_limbs
    local.set $al
    local.get $b
    call $int_limbs
    local.set $bl
    local.get $as
    local.get $bs
    i32.eq
    if ;; label = @1
      local.get $as
      local.get $al
      ref.as_non_null
      local.get $bl
      ref.as_non_null
      call $mag_add
      call $int_pack
      return
    end
    local.get $al
    ref.as_non_null
    local.get $bl
    ref.as_non_null
    call $mag_compare
    local.set $cmp
    local.get $cmp
    i32.eqz
    if ;; label = @1
      i32.const 0
      ref.i31
      return
    end
    local.get $cmp
    i32.const 0
    i32.gt_s
    if (result eqref) ;; label = @1
      local.get $as
      local.get $al
      ref.as_non_null
      local.get $bl
      ref.as_non_null
      call $mag_sub
      call $int_pack
    else
      local.get $bs
      local.get $bl
      ref.as_non_null
      local.get $al
      ref.as_non_null
      call $mag_sub
      call $int_pack
    end
  )