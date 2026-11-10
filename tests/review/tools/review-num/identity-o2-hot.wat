;; executed 1 times
  (func (;38;) (type 49) (param f64) (result f64)
    (local (ref 2))
    local.get 0
    call 25
    local.set 1
    global.get 10
    call 15
    i32.const -1
    global.set 10
    global.get 6
    i32.const 1
    i32.sub
    global.set 6
    local.get 1
    struct.get 2 3
  )

;; executed 1 times
  (func (;25;) (type 60) (param f64) (result (ref 2))
    local.get 0
    f64.abs
    f64.const 0x1.fffffffffffffp+1023 (;=179769313486231570000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000;)
    f64.le
    i32.eqz
    if ;; label = @1
      i32.const 3
      i32.const 0
      i64.const 0
      f64.const 0x0p+0 (;=0;)
      i32.const 104
      i32.const 111
      i32.const 115
      i32.const 116
      i32.const 58
      i32.const 32
      i32.const 118
      i32.const 97
      i32.const 108
      i32.const 117
      i32.const 101
      i32.const 32
      i32.const 105
      i32.const 115
      i32.const 32
      i32.const 110
      i32.const 111
      i32.const 116
      i32.const 32
      i32.const 99
      i32.const 111
      i32.const 114
      i32.const 114
      i32.const 101
      i32.const 99
      i32.const 116
      array.new_fixed 0 26
      ref.null none
      ref.null none
      struct.new 2
      call 18
      unreachable
    end
    local.get 0
    f64.abs
    f64.const 0x1.fffffffffffffp+1023 (;=179769313486231570000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000;)
    f64.le
    i32.eqz
    if ;; label = @1
      call 19
      unreachable
    end
    i32.const 2
    i32.const 0
    i64.const 0
    local.get 0
    ref.null none
    ref.null none
    ref.null none
    struct.new 2
  )

;; executed 1 times
  (func (;15;) (type 16) (param i32)
    (local i32 i32 (ref 48) (ref 48))
    global.get 6
    local.tee 1
    global.get 5
    array.len
    i32.ge_u
    if ;; label = @1
      global.get 5
      local.tee 3
      array.len
      local.tee 2
      i32.const 268435456
      i32.ge_u
      if ;; label = @2
        unreachable
      end
      local.get 2
      i32.const 1
      i32.shl
      array.new_default 48
      local.tee 4
      i32.const 0
      local.get 3
      i32.const 0
      local.get 2
      array.copy 48 48
      local.get 4
      global.set 5
    end
    global.get 5
    local.get 1
    local.get 0
    array.set 48
    local.get 1
    i32.const 1
    i32.add
    global.set 6
  )