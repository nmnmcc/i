;; executed 7 times
  (func (;22;) (type 51) (param eqref eqref) (result (ref 2))
    (local f64)
    local.get 0
    ref.cast (ref 2)
    struct.get 2 3
    local.get 1
    ref.cast (ref 2)
    struct.get 2 3
    f64.mul
    local.tee 2
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
    local.get 2
    ref.null none
    ref.null none
    ref.null none
    struct.new 2
  )

;; executed 7 times
  (func (;20;) (type 51) (param eqref eqref) (result (ref 2))
    (local f64)
    local.get 0
    ref.cast (ref 2)
    struct.get 2 3
    local.get 1
    ref.cast (ref 2)
    struct.get 2 3
    f64.add
    local.tee 2
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
    local.get 2
    ref.null none
    ref.null none
    ref.null none
    struct.new 2
  )

;; executed 1 times
  (func (;40;) (type 49) (param f64) (result f64)
    (local (ref 2) (ref 2))
    local.get 0
    call 25
    local.set 2
    global.get 10
    call 15
    i32.const -1
    global.set 10
    block ;; label = @1
      global.get 6
      i32.eqz
      if ;; label = @2
        i32.const 21
        call 15
        br 1 (;@1;)
      end
      global.get 5
      global.get 6
      i32.const 1
      i32.sub
      i32.const 21
      array.set 48
    end
    i32.const 2
    i32.const 0
    i64.const 0
    f64.const 0x1p-3 (;=0.125;)
    ref.null none
    ref.null none
    ref.null none
    struct.new 2
    local.get 2
    call 22
    local.set 1
    block ;; label = @1
      global.get 6
      i32.eqz
      if ;; label = @2
        i32.const 21
        call 15
        br 1 (;@1;)
      end
      global.get 5
      global.get 6
      i32.const 1
      i32.sub
      i32.const 21
      array.set 48
    end
    local.get 1
    i32.const 2
    i32.const 0
    i64.const 0
    f64.const 0x1p-2 (;=0.25;)
    ref.null none
    ref.null none
    ref.null none
    struct.new 2
    call 20
    local.set 1
    block ;; label = @1
      global.get 6
      i32.eqz
      if ;; label = @2
        i32.const 22
        call 15
        br 1 (;@1;)
      end
      global.get 5
      global.get 6
      i32.const 1
      i32.sub
      i32.const 22
      array.set 48
    end
    local.get 1
    local.get 2
    call 22
    local.set 1
    block ;; label = @1
      global.get 6
      i32.eqz
      if ;; label = @2
        i32.const 22
        call 15
        br 1 (;@1;)
      end
      global.get 5
      global.get 6
      i32.const 1
      i32.sub
      i32.const 22
      array.set 48
    end
    local.get 1
    i32.const 2
    i32.const 0
    i64.const 0
    f64.const 0x1.8p-2 (;=0.375;)
    ref.null none
    ref.null none
    ref.null none
    struct.new 2
    call 20
    local.set 1
    block ;; label = @1
      global.get 6
      i32.eqz
      if ;; label = @2
        i32.const 23
        call 15
        br 1 (;@1;)
      end
      global.get 5
      global.get 6
      i32.const 1
      i32.sub
      i32.const 23
      array.set 48
    end
    local.get 1
    local.get 2
    call 22
    local.set 1
    block ;; label = @1
      global.get 6
      i32.eqz
      if ;; label = @2
        i32.const 23
        call 15
        br 1 (;@1;)
      end
      global.get 5
      global.get 6
      i32.const 1
      i32.sub
      i32.const 23
      array.set 48
    end
    local.get 1
    i32.const 2
    i32.const 0
    i64.const 0
    f64.const 0x1p-1 (;=0.5;)
    ref.null none
    ref.null none
    ref.null none
    struct.new 2
    call 20
    local.set 1
    block ;; label = @1
      global.get 6
      i32.eqz
      if ;; label = @2
        i32.const 24
        call 15
        br 1 (;@1;)
      end
      global.get 5
      global.get 6
      i32.const 1
      i32.sub
      i32.const 24
      array.set 48
    end
    local.get 1
    local.get 2
    call 22
    local.set 1
    block ;; label = @1
      global.get 6
      i32.eqz
      if ;; label = @2
        i32.const 24
        call 15
        br 1 (;@1;)
      end
      global.get 5
      global.get 6
      i32.const 1
      i32.sub
      i32.const 24
      array.set 48
    end
    local.get 1
    i32.const 2
    i32.const 0
    i64.const 0
    f64.const 0x1.4p-1 (;=0.625;)
    ref.null none
    ref.null none
    ref.null none
    struct.new 2
    call 20
    local.set 1
    block ;; label = @1
      global.get 6
      i32.eqz
      if ;; label = @2
        i32.const 25
        call 15
        br 1 (;@1;)
      end
      global.get 5
      global.get 6
      i32.const 1
      i32.sub
      i32.const 25
      array.set 48
    end
    local.get 1
    local.get 2
    call 22
    local.set 1
    block ;; label = @1
      global.get 6
      i32.eqz
      if ;; label = @2
        i32.const 25
        call 15
        br 1 (;@1;)
      end
      global.get 5
      global.get 6
      i32.const 1
      i32.sub
      i32.const 25
      array.set 48
    end
    local.get 1
    i32.const 2
    i32.const 0
    i64.const 0
    f64.const 0x1.8p-1 (;=0.75;)
    ref.null none
    ref.null none
    ref.null none
    struct.new 2
    call 20
    local.set 1
    block ;; label = @1
      global.get 6
      i32.eqz
      if ;; label = @2
        i32.const 26
        call 15
        br 1 (;@1;)
      end
      global.get 5
      global.get 6
      i32.const 1
      i32.sub
      i32.const 26
      array.set 48
    end
    local.get 1
    local.get 2
    call 22
    local.set 1
    block ;; label = @1
      global.get 6
      i32.eqz
      if ;; label = @2
        i32.const 26
        call 15
        br 1 (;@1;)
      end
      global.get 5
      global.get 6
      i32.const 1
      i32.sub
      i32.const 26
      array.set 48
    end
    local.get 1
    i32.const 2
    i32.const 0
    i64.const 0
    f64.const 0x1.cp-1 (;=0.875;)
    ref.null none
    ref.null none
    ref.null none
    struct.new 2
    call 20
    local.set 1
    block ;; label = @1
      global.get 6
      i32.eqz
      if ;; label = @2
        i32.const 27
        call 15
        br 1 (;@1;)
      end
      global.get 5
      global.get 6
      i32.const 1
      i32.sub
      i32.const 27
      array.set 48
    end
    local.get 1
    local.get 2
    call 22
    local.set 2
    block ;; label = @1
      global.get 6
      i32.eqz
      if ;; label = @2
        i32.const 27
        call 15
        br 1 (;@1;)
      end
      global.get 5
      global.get 6
      i32.const 1
      i32.sub
      i32.const 27
      array.set 48
    end
    local.get 2
    i32.const 2
    i32.const 0
    i64.const 0
    f64.const 0x1p+0 (;=1;)
    ref.null none
    ref.null none
    ref.null none
    struct.new 2
    call 20
    local.set 2
    global.get 6
    i32.const 1
    i32.sub
    global.set 6
    local.get 2
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