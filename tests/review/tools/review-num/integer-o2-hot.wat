;; executed 257 times
  (func (;27;) (type 13) (param eqref i32) (result i32)
    block (result i32) ;; label = @1
      i32.const 0
      local.get 0
      ref.is_null
      br_if 0 (;@1;)
      drop
      i32.const 8
      local.get 0
      ref.test (ref i31)
      br_if 0 (;@1;)
      drop
      local.get 0
      ref.cast (ref 1)
      struct.get 1 0
    end
    i32.const 1
    i32.eq
    if (result i32) ;; label = @1
      local.get 0
      ref.cast (ref 2)
      struct.get 2 1
      local.get 1
      i32.eq
    else
      i32.const 0
    end
  )

;; executed 256 times
  (func (;6;) (type 56) (param i64) (result (ref eq))
    (local i32)
    local.get 0
    i64.const 1073741823
    i64.le_s
    local.get 0
    i64.const -1073741824
    i64.ge_s
    i32.and
    if ;; label = @1
      local.get 0
      i32.wrap_i64
      ref.i31
      return
    end
    i64.const 0
    local.get 0
    i64.sub
    local.get 0
    i32.const -1
    i32.const 1
    local.get 0
    i64.const 0
    i64.lt_s
    select
    local.tee 1
    i32.const 0
    i32.lt_s
    select
    local.tee 0
    i64.const 4294967295
    i64.le_u
    if ;; label = @1
      local.get 1
      local.get 0
      i32.wrap_i64
      array.new_fixed 48 1
      call 5
      return
    end
    local.get 1
    local.get 0
    i32.wrap_i64
    local.get 0
    i64.const 32
    i64.shr_u
    i32.wrap_i64
    array.new_fixed 48 2
    call 5
  )

;; executed 130 times
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

;; executed 129 times
  (func (;31;) (type 62) (param (ref null 47) eqref eqref) (result eqref)
    (local eqref)
    global.get 10
    call 15
    i32.const -1
    global.set 10
    global.get 3
    global.get 2
    local.get 1
    i32.const 0
    ref.i31
    call 13
    select (result (ref 2))
    local.tee 3
    i32.const 1
    call 27
    if ;; label = @1
      global.get 6
      i32.const 1
      i32.sub
      global.set 6
      local.get 2
      return
    end
    local.get 3
    i32.const 0
    call 27
    if ;; label = @1
      local.get 1
      ref.test (ref i31)
      if (result eqref) ;; label = @2
        local.get 1
        ref.cast (ref i31)
        i31.get_s
        i64.extend_i32_s
        i64.const 1
        i64.sub
        call 6
      else
        local.get 1
        i64.const -1
        call 6
        call 10
      end
      local.set 3
      local.get 2
      local.get 1
      call 10
      local.set 1
      i32.const -18
      call 16
      local.get 0
      local.get 3
      local.get 1
      return_call 31
    end
    unreachable
  )

;; executed 129 times
  (func (;13;) (type 37) (param eqref eqref) (result i32)
    (local i32 i32 i32 i64 i64 i64 (ref null 19) (ref null 47) (ref null 47) (ref 2) (ref 2) eqref eqref (ref 0) (ref 0) (ref 26) (ref 26))
    local.get 0
    local.get 1
    ref.eq
    if ;; label = @1
      i32.const 1
      return
    end
    local.get 0
    ref.test (ref i31)
    local.get 1
    ref.test (ref i31)
    i32.and
    if ;; label = @1
      i32.const 0
      return
    end
    loop (result i32) ;; label = @1
      block (result i32) ;; label = @2
        i32.const 0
        local.set 2
        block ;; label = @3
          local.get 0
          local.get 1
          ref.eq
          br_if 0 (;@3;)
          block (result i32) ;; label = @4
            i32.const 0
            local.get 0
            ref.is_null
            br_if 0 (;@4;)
            drop
            i32.const 8
            local.get 0
            ref.test (ref i31)
            br_if 0 (;@4;)
            drop
            local.get 0
            ref.cast (ref 1)
            struct.get 1 0
          end
          local.tee 3
          block (result i32) ;; label = @4
            i32.const 0
            local.get 1
            ref.is_null
            br_if 0 (;@4;)
            drop
            i32.const 8
            local.get 1
            ref.test (ref i31)
            br_if 0 (;@4;)
            drop
            local.get 1
            ref.cast (ref 1)
            struct.get 1 0
          end
          local.tee 4
          i32.ne
          if ;; label = @4
            local.get 3
            local.get 4
            i32.gt_s
            local.get 3
            local.get 4
            i32.lt_s
            i32.sub
            br 2 (;@2;)
          end
          local.get 3
          i32.eqz
          br_if 0 (;@3;)
          local.get 3
          i32.const 8
          i32.eq
          if ;; label = @4
            local.get 0
            local.get 1
            call 8
            local.set 2
            br 1 (;@3;)
          end
          local.get 3
          i32.const 10
          i32.eq
          if ;; label = @4
            unreachable
          end
          i32.const 1
          local.get 3
          i32.const 11
          i32.eq
          local.get 3
          i32.const 12
          i32.eq
          i32.or
          local.get 3
          i32.const 7
          i32.eq
          i32.or
          br_if 1 (;@2;)
          drop
          local.get 3
          i32.const 6
          i32.eq
          if ;; label = @4
            local.get 0
            ref.test (ref 26)
            local.tee 3
            local.get 1
            ref.test (ref 26)
            local.tee 4
            i32.and
            i32.eqz
            if ;; label = @5
              local.get 3
              local.get 4
              i32.gt_s
              local.get 3
              local.get 4
              i32.lt_s
              i32.sub
              local.set 2
              br 2 (;@3;)
            end
            local.get 0
            ref.cast (ref 26)
            local.tee 17
            struct.get 26 1
            local.set 13
            local.get 1
            ref.cast (ref 26)
            local.tee 18
            struct.get 26 1
            local.set 14
            local.get 17
            struct.get 26 2
            local.set 0
            local.get 18
            struct.get 26 2
            local.set 1
            local.get 13
            local.get 14
            ref.eq
            br_if 3 (;@1;)
            local.get 13
            ref.test (ref i31)
            local.get 14
            ref.test (ref i31)
            i32.and
            if ;; label = @5
              local.get 13
              ref.cast (ref i31)
              i31.get_s
              local.tee 2
              local.get 14
              ref.cast (ref i31)
              i31.get_s
              local.tee 3
              i32.gt_s
              local.get 2
              local.get 3
              i32.lt_s
              i32.sub
              br 3 (;@2;)
            end
            local.get 8
            ref.null none
            ref.null none
            i32.const 0
            local.get 0
            local.get 1
            struct.new 19
            local.set 8
            local.get 13
            local.set 0
            local.get 14
            local.set 1
            br 3 (;@1;)
          end
          local.get 0
          ref.cast (ref 2)
          local.set 11
          local.get 1
          ref.cast (ref 2)
          local.set 12
          local.get 3
          i32.const 2
          i32.eq
          if ;; label = @4
            local.get 11
            struct.get 2 3
            local.get 12
            struct.get 2 3
            f64.gt
            local.get 11
            struct.get 2 3
            local.get 12
            struct.get 2 3
            f64.lt
            i32.sub
            local.set 2
            br 1 (;@3;)
          end
          local.get 3
          i32.const 3
          i32.eq
          if ;; label = @4
            block (result i32) ;; label = @5
              local.get 11
              struct.get 2 4
              ref.as_non_null
              local.set 15
              local.get 12
              struct.get 2 4
              ref.as_non_null
              local.set 16
              loop ;; label = @6
                local.get 2
                local.get 15
                array.len
                i32.ge_u
                local.get 2
                local.get 16
                array.len
                i32.ge_u
                i32.or
                i32.eqz
                if ;; label = @7
                  i32.const -1
                  local.get 15
                  local.get 2
                  array.get_u 0
                  local.tee 3
                  local.get 16
                  local.get 2
                  array.get_u 0
                  local.tee 4
                  i32.lt_u
                  br_if 2 (;@5;)
                  drop
                  i32.const 1
                  local.get 3
                  local.get 4
                  i32.gt_u
                  br_if 2 (;@5;)
                  drop
                  local.get 2
                  i32.const 1
                  i32.add
                  local.set 2
                  br 1 (;@6;)
                end
              end
              local.get 15
              array.len
              local.get 16
              array.len
              i32.gt_u
              local.get 15
              array.len
              local.get 16
              array.len
              i32.lt_u
              i32.sub
            end
            local.set 2
            br 1 (;@3;)
          end
          local.get 3
          i32.const 9
          i32.eq
          if ;; label = @4
            local.get 11
            struct.get 2 2
            local.set 6
            local.get 12
            struct.get 2 2
            local.set 7
            i64.const 0
            local.set 5
            loop ;; label = @5
              local.get 5
              local.get 6
              i64.ge_u
              local.get 5
              local.get 7
              i64.ge_u
              i32.or
              i32.eqz
              if ;; label = @6
                local.get 0
                local.get 5
                call 12
                local.tee 2
                local.get 1
                local.get 5
                call 12
                local.tee 3
                i32.gt_s
                local.get 2
                local.get 3
                i32.lt_s
                i32.sub
                local.tee 2
                if ;; label = @7
                  local.get 2
                  br 5 (;@2;)
                else
                  local.get 5
                  i64.const 1
                  i64.add
                  local.set 5
                  br 2 (;@5;)
                end
                unreachable
              end
            end
            local.get 6
            local.get 7
            i64.gt_s
            local.get 6
            local.get 7
            i64.lt_s
            i32.sub
            local.set 2
            br 1 (;@3;)
          end
          local.get 11
          struct.get 2 2
          local.tee 5
          local.get 12
          struct.get 2 2
          local.tee 6
          i64.gt_s
          local.get 5
          local.get 6
          i64.lt_s
          i32.sub
          local.tee 2
          if ;; label = @4
            local.get 2
            br 2 (;@2;)
          end
          local.get 11
          struct.get 2 1
          local.tee 2
          local.get 12
          struct.get 2 1
          local.tee 3
          i32.gt_s
          local.get 2
          local.get 3
          i32.lt_s
          i32.sub
          local.tee 2
          if ;; label = @4
            local.get 2
            br 2 (;@2;)
          end
          local.get 11
          struct.get 2 5
          local.tee 9
          ref.is_null
          local.get 12
          struct.get 2 5
          local.tee 10
          ref.is_null
          i32.or
          if ;; label = @4
            local.get 10
            ref.is_null
            local.get 9
            ref.is_null
            i32.sub
            local.set 2
            br 1 (;@3;)
          end
          local.get 9
          array.len
          local.set 3
          local.get 10
          array.len
          local.set 4
          i32.const 0
          local.set 2
          loop ;; label = @4
            block ;; label = @5
              local.get 2
              local.get 3
              i32.ge_u
              local.get 2
              local.get 4
              i32.ge_u
              i32.or
              br_if 0 (;@5;)
              local.get 9
              local.get 2
              array.get 47
              local.get 10
              local.get 2
              array.get 47
              ref.eq
              i32.eqz
              br_if 0 (;@5;)
              local.get 2
              i32.const 1
              i32.add
              local.set 2
              br 1 (;@4;)
            end
          end
          local.get 2
          local.get 3
          i32.ge_u
          local.get 2
          local.get 4
          i32.ge_u
          i32.or
          if ;; label = @4
            local.get 3
            local.get 4
            i32.gt_s
            local.get 3
            local.get 4
            i32.lt_s
            i32.sub
            local.set 2
            br 1 (;@3;)
          end
          local.get 9
          local.get 2
          array.get 47
          local.tee 0
          ref.test (ref i31)
          local.get 10
          local.get 2
          array.get 47
          local.tee 1
          ref.test (ref i31)
          i32.and
          if ;; label = @4
            local.get 0
            ref.cast (ref i31)
            i31.get_s
            local.tee 2
            local.get 1
            ref.cast (ref i31)
            i31.get_s
            local.tee 3
            i32.gt_s
            local.get 2
            local.get 3
            i32.lt_s
            i32.sub
            br 2 (;@2;)
          end
          local.get 3
          local.get 4
          i32.ne
          local.get 3
          local.get 2
          i32.const 1
          i32.add
          local.tee 2
          i32.gt_u
          i32.or
          if ;; label = @4
            local.get 8
            local.get 9
            local.get 10
            local.get 2
            ref.null none
            ref.null none
            struct.new 19
            local.set 8
          end
          br 2 (;@1;)
        end
        local.get 2
        local.get 2
        br_if 0 (;@2;)
        drop
        loop ;; label = @3
          i32.const 0
          local.get 8
          ref.is_null
          br_if 1 (;@2;)
          drop
          local.get 8
          struct.get 19 1
          ref.is_null
          if ;; label = @4
            local.get 8
            struct.get 19 4
            local.set 0
            local.get 8
            struct.get 19 5
            local.set 1
            local.get 8
            struct.get 19 0
            local.set 8
            br 3 (;@1;)
          end
          local.get 8
          struct.get 19 3
          local.set 2
          local.get 8
          struct.get 19 1
          local.tee 9
          array.len
          local.set 3
          local.get 8
          struct.get 19 2
          local.tee 10
          array.len
          local.set 4
          loop ;; label = @4
            block ;; label = @5
              local.get 2
              local.get 3
              i32.ge_u
              local.get 2
              local.get 4
              i32.ge_u
              i32.or
              br_if 0 (;@5;)
              local.get 9
              local.get 2
              array.get 47
              local.get 10
              local.get 2
              array.get 47
              ref.eq
              i32.eqz
              br_if 0 (;@5;)
              local.get 2
              i32.const 1
              i32.add
              local.set 2
              br 1 (;@4;)
            end
          end
          local.get 2
          local.get 3
          i32.ge_u
          local.get 2
          local.get 4
          i32.ge_u
          i32.or
          if ;; label = @4
            local.get 8
            struct.get 19 0
            local.set 8
            local.get 3
            local.get 4
            i32.gt_s
            local.get 3
            local.get 4
            i32.lt_s
            i32.sub
            local.tee 2
            if ;; label = @5
              local.get 2
              br 3 (;@2;)
            end
            br 1 (;@3;)
          end
        end
        local.get 9
        local.get 2
        array.get 47
        local.tee 0
        ref.test (ref i31)
        local.get 10
        local.get 2
        array.get 47
        local.tee 1
        ref.test (ref i31)
        i32.and
        if ;; label = @3
          local.get 0
          ref.cast (ref i31)
          i31.get_s
          local.tee 2
          local.get 1
          ref.cast (ref i31)
          i31.get_s
          local.tee 3
          i32.gt_s
          local.get 2
          local.get 3
          i32.lt_s
          i32.sub
          br 1 (;@2;)
        end
        local.get 3
        local.get 4
        i32.eq
        local.get 3
        local.get 2
        i32.const 1
        i32.add
        local.tee 2
        i32.eq
        i32.and
        if ;; label = @3
          local.get 8
          struct.get 19 0
          local.set 8
        else
          local.get 8
          local.get 2
          struct.set 19 3
        end
        br 1 (;@1;)
      end
    end
    i32.eqz
  )

;; executed 128 times
  (func (;16;) (type 16) (param i32)
    (local i32)
    global.get 6
    i32.const 1
    i32.sub
    local.set 1
    local.get 0
    i32.const -1
    i32.eq
    if (result i32) ;; label = @1
      global.get 5
      local.get 1
      array.get 48
      local.tee 0
      i32.const -2
      i32.gt_s
      if (result i32) ;; label = @2
        i32.const -1
      else
        local.get 0
      end
    else
      local.get 0
    end
    global.set 10
    local.get 1
    global.set 6
  )

;; executed 128 times
  (func (;10;) (type 35) (param eqref eqref) (result eqref)
    (local i32 i32 i32 i32 i64 (ref 48) (ref 48) (ref 48))
    local.get 0
    ref.test (ref i31)
    local.get 1
    ref.test (ref i31)
    i32.and
    if ;; label = @1
      local.get 0
      ref.cast (ref i31)
      i31.get_s
      i64.extend_i32_s
      local.get 1
      ref.cast (ref i31)
      i31.get_s
      i64.extend_i32_s
      i64.add
      call 6
      return
    end
    local.get 0
    ref.test (ref i31)
    if (result i32) ;; label = @1
      local.get 0
      ref.cast (ref i31)
      i31.get_s
      local.tee 2
      i32.const 0
      i32.gt_s
      local.get 2
      i32.const 0
      i32.lt_s
      i32.sub
    else
      local.get 0
      ref.cast (ref 2)
      struct.get 2 1
    end
    local.set 2
    local.get 1
    ref.test (ref i31)
    if (result i32) ;; label = @1
      local.get 1
      ref.cast (ref i31)
      i31.get_s
      local.tee 3
      i32.const 0
      i32.gt_s
      local.get 3
      i32.const 0
      i32.lt_s
      i32.sub
    else
      local.get 1
      ref.cast (ref 2)
      struct.get 2 1
    end
    local.set 3
    local.get 2
    i32.eqz
    if ;; label = @1
      local.get 1
      return
    end
    local.get 3
    i32.eqz
    if ;; label = @1
      local.get 0
      return
    end
    local.get 0
    call 3
    local.set 7
    local.get 1
    call 3
    local.set 8
    local.get 2
    local.get 3
    i32.eq
    if ;; label = @1
      local.get 2
      i32.const 0
      local.set 2
      local.get 7
      array.len
      local.tee 3
      local.get 8
      array.len
      local.tee 4
      local.get 3
      local.get 4
      i32.gt_u
      select
      local.tee 5
      i32.const 1
      i32.add
      array.new_default 48
      local.set 9
      loop ;; label = @2
        local.get 2
        local.get 5
        i32.lt_u
        if ;; label = @3
          local.get 2
          local.get 3
          i32.lt_u
          if ;; label = @4
            local.get 6
            local.get 7
            local.get 2
            array.get 48
            i64.extend_i32_u
            i64.add
            local.set 6
          end
          local.get 2
          local.get 4
          i32.lt_u
          if ;; label = @4
            local.get 6
            local.get 8
            local.get 2
            array.get 48
            i64.extend_i32_u
            i64.add
            local.set 6
          end
          local.get 9
          local.get 2
          local.get 6
          i32.wrap_i64
          array.set 48
          local.get 6
          i64.const 32
          i64.shr_u
          local.set 6
          local.get 2
          i32.const 1
          i32.add
          local.set 2
          br 1 (;@2;)
        end
      end
      local.get 9
      local.get 5
      local.get 6
      i32.wrap_i64
      array.set 48
      local.get 9
      call 5
      return
    end
    local.get 7
    local.get 8
    call 7
    local.tee 4
    i32.eqz
    if ;; label = @1
      i32.const 0
      ref.i31
      return
    end
    local.get 4
    i32.const 0
    i32.gt_s
    if (result (ref eq)) ;; label = @1
      local.get 2
      local.get 7
      local.get 8
      call 9
      call 5
    else
      local.get 3
      local.get 8
      local.get 7
      call 9
      call 5
    end
  )

;; executed 1 times
  (func (;42;) (type 49) (param f64) (result f64)
    (local i32 i32 i32 i32 i32 i64 f64 eqref (ref 48) (ref 48) (ref null 47) (ref 2))
    global.get 11
    local.set 11
    local.get 0
    call 25
    local.set 12
    global.get 10
    call 15
    i32.const -1
    global.set 10
    block ;; label = @1
      global.get 6
      i32.eqz
      if ;; label = @2
        i32.const 13
        call 15
        br 1 (;@1;)
      end
      global.get 5
      global.get 6
      i32.const 1
      i32.sub
      i32.const 13
      array.set 48
    end
    block (result (ref eq)) ;; label = @1
      local.get 12
      struct.get 2 3
      local.tee 0
      local.get 0
      f64.ne
      local.get 0
      f64.abs
      local.tee 7
      f64.const 0x1.fffffffffffffp+1023 (;=179769313486231570000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000;)
      f64.gt
      i32.or
      if ;; label = @2
        unreachable
      end
      i32.const 0
      ref.i31
      local.get 7
      f64.const 0x1p+0 (;=1;)
      f64.lt
      br_if 0 (;@1;)
      drop
      i32.const -1
      i32.const 1
      local.get 0
      f64.const 0x0p+0 (;=0;)
      f64.lt
      select
      local.set 3
      local.get 7
      i64.reinterpret_f64
      local.tee 6
      i64.const 52
      i64.shr_u
      i64.const 2047
      i64.and
      i32.wrap_i64
      i32.const 1023
      i32.sub
      local.set 1
      local.get 6
      i64.const 4503599627370495
      i64.and
      i64.const 4503599627370496
      i64.or
      local.set 6
      local.get 1
      i32.const 52
      i32.le_s
      if ;; label = @2
        i64.const 0
        local.get 6
        i32.const 52
        local.get 1
        i32.sub
        i64.extend_i32_u
        i64.shr_u
        local.tee 6
        i64.sub
        local.get 6
        local.get 3
        i32.const 0
        i32.lt_s
        select
        call 6
        br 1 (;@1;)
      end
      local.get 1
      i32.const 52
      i32.sub
      local.set 4
      local.get 1
      i32.const 32
      i32.add
      i32.const 5
      i32.shr_u
      array.new_default 48
      local.set 10
      loop ;; label = @2
        local.get 2
        i32.const 53
        i32.lt_u
        if ;; label = @3
          local.get 6
          local.get 2
          i64.extend_i32_u
          i64.shr_u
          i64.const 1
          i64.and
          i32.wrap_i64
          if ;; label = @4
            local.get 10
            local.get 2
            local.get 4
            i32.add
            local.tee 1
            i32.const 5
            i32.shr_u
            local.tee 5
            i32.const 1
            local.get 1
            i32.shl
            local.get 10
            local.get 5
            array.get 48
            i32.or
            array.set 48
          end
          local.get 2
          i32.const 1
          i32.add
          local.set 2
          br 1 (;@2;)
        end
      end
      local.get 3
      local.get 10
      call 5
    end
    local.set 8
    block ;; label = @1
      global.get 6
      i32.eqz
      if ;; label = @2
        i32.const 14
        call 15
        br 1 (;@1;)
      end
      global.get 5
      global.get 6
      i32.const 1
      i32.sub
      i32.const 14
      array.set 48
    end
    local.get 11
    i32.const 128
    ref.i31
    local.get 8
    call 31
    local.set 8
    block ;; label = @1
      global.get 6
      i32.eqz
      if ;; label = @2
        i32.const -17
        call 15
        br 1 (;@1;)
      end
      global.get 5
      global.get 6
      i32.const 1
      i32.sub
      i32.const -17
      array.set 48
    end
    i64.const 0
    local.set 6
    block (result f64) ;; label = @1
      local.get 8
      ref.test (ref i31)
      if ;; label = @2
        local.get 8
        ref.cast (ref i31)
        i31.get_s
        f64.convert_i32_s
        br 1 (;@1;)
      end
      f64.const 0x0p+0 (;=0;)
      local.get 8
      ref.test (ref i31)
      if (result i32) ;; label = @2
        local.get 8
        ref.cast (ref i31)
        i31.get_s
        local.tee 1
        i32.const 0
        i32.gt_s
        local.get 1
        i32.const 0
        i32.lt_s
        i32.sub
      else
        local.get 8
        ref.cast (ref 2)
        struct.get 2 1
      end
      local.tee 3
      i32.eqz
      br_if 0 (;@1;)
      drop
      local.get 8
      call 3
      local.tee 9
      array.len
      local.tee 1
      i32.const 32
      i32.gt_u
      if ;; label = @2
        call 19
        unreachable
      end
      local.get 1
      i32.const 5
      i32.shl
      local.get 9
      local.get 1
      i32.const 1
      i32.sub
      array.get 48
      i32.clz
      i32.sub
      local.tee 1
      i32.const 53
      i32.sub
      i32.const 0
      local.get 1
      i32.const 53
      i32.gt_u
      select
      local.set 2
      loop ;; label = @2
        local.get 1
        local.get 2
        i32.gt_u
        if ;; label = @3
          local.get 9
          local.get 1
          i32.const 1
          i32.sub
          local.tee 1
          i32.const 5
          i32.shr_u
          array.get 48
          local.get 1
          i32.shr_u
          i32.const 1
          i32.and
          i64.extend_i32_u
          local.get 6
          i64.const 1
          i64.shl
          i64.or
          local.set 6
          br 1 (;@2;)
        end
      end
      local.get 2
      if (result i64) ;; label = @2
        local.get 9
        local.get 2
        i32.const 1
        i32.sub
        local.tee 1
        i32.const 5
        i32.shr_u
        array.get 48
        local.get 1
        i32.shr_u
        i32.const 1
        i32.and
        if (result i64) ;; label = @3
          i32.const 0
          local.set 1
          local.get 6
          i64.const 1
          i64.add
          local.get 6
          loop (result i32) ;; label = @4
            local.get 2
            i32.const 1
            i32.sub
            local.get 1
            i32.gt_u
            if (result i32) ;; label = @5
              local.get 9
              local.get 1
              i32.const 5
              i32.shr_u
              array.get 48
              local.get 1
              i32.shr_u
              i32.const 1
              i32.and
              if (result i32) ;; label = @6
                i32.const 1
              else
                local.get 1
                i32.const 1
                i32.add
                local.set 1
                br 2 (;@4;)
              end
            else
              i32.const 0
            end
          end
          local.get 6
          i64.const 1
          i64.and
          i32.wrap_i64
          i32.or
          select
        else
          local.get 6
        end
      else
        local.get 6
      end
      f64.convert_i64_u
      local.get 2
      i32.const 1023
      i32.add
      i64.extend_i32_u
      i64.const 52
      i64.shl
      f64.reinterpret_i64
      f64.mul
      local.tee 0
      f64.const 0x1.fffffffffffffp+1023 (;=179769313486231570000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000;)
      f64.gt
      if ;; label = @2
        call 19
        unreachable
      end
      local.get 0
      f64.neg
      local.get 0
      local.get 3
      i32.const 0
      i32.lt_s
      select
    end
    local.tee 0
    f64.abs
    f64.const 0x1.fffffffffffffp+1023 (;=179769313486231570000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000;)
    f64.le
    i32.eqz
    if ;; label = @1
      call 19
      unreachable
    end
    global.get 6
    i32.const 1
    i32.sub
    global.set 6
    local.get 0
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