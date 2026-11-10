(module

(type $bytes (array (mut i8)))
(type $values (array (mut eqref)))
(type $limbs (array (mut i32)))
(type $call (func (param eqref eqref) (result eqref)))
(type $val (sub (struct (field i32))))
(type $obj (sub $val (struct
 (field i32) (field i32) (field i64) (field f64)
 (field (ref null $bytes)) (field (ref null $values))
 (field (ref null $limbs)) (field (ref null $call)))))
(type $cons (sub final $val (struct (field i32) (field eqref) (field eqref))))
(type $hostref (struct (field externref) (field i64)))
(type $sites (array i32))
(type $frames (array (mut i32)))
(type $env (array (mut eqref)))

(type $boundary (sub final $obj (struct (field i32) (field i32) (field i64) (field f64)
  (field (ref null $bytes)) (field (ref null $values)) (field (ref null $limbs))
  (field (ref null $call)) (field (mut i32)))))
(rec
  (type $frame (struct (field (ref $refn)) (field (ref null $values))))
  (type $refn (func (param (ref $frame) (ref null $flist) (ref null $env) eqref eqref) (result eqref)))
  (type $flist (struct (field (ref $frame)) (field (ref null $flist)))))
(type $cont (sub final $obj (struct (field i32) (field i32) (field i64) (field f64)
  (field (ref null $bytes)) (field (ref null $values)) (field (ref null $limbs))
  (field (ref null $call)) (field (mut i32)) (field (ref $boundary)) (field (ref null $flist)))))
(type $cap (struct (field (ref $boundary)) (field eqref) (field (mut (ref null $flist)))))
(type $box (struct (field i64)))
(type $waiting (struct (field i32)))
(type $fn0 (func (param (ref null $obj) (ref null $env)) (result eqref)))
(type $clo0 (sub final $obj (struct (field i32) (field i32) (field i64) (field f64) (field (ref null $bytes)) (field (ref null $values)) (field (ref null $limbs)) (field (ref null $call)) (field (ref $fn0)))))
(type $fn1 (func (param (ref null $obj) (ref null $env) eqref) (result eqref)))
(type $clo1 (sub final $obj (struct (field i32) (field i32) (field i64) (field f64) (field (ref null $bytes)) (field (ref null $values)) (field (ref null $limbs)) (field (ref null $call)) (field (ref $fn1)))))

(import "ilang:host/v1" "panic" (func $host_panic (param i32 i32)))
(import "ilang:host/v1" "reference_id" (func $host_reference_id (param externref) (result i64)))

(memory (export "memory") 1)
(global $heap (mut i32) (i32.const 1024))
(global $free_head (mut i32) (i32.const 0))
(func $alloc (export "ilang_alloc") (param $len i32) (result i32)
 (local $size i32) (local $p i32) (local $prev i32) (local $end i64) (local $available i64)
 (if (i32.gt_u (local.get $len) (i32.const 2147483632)) (then unreachable))
 (local.set $size (i32.and (i32.add (local.get $len) (i32.const 7)) (i32.const -8)))
 (if (i32.eqz (local.get $size)) (then (local.set $size (i32.const 8))))
 (local.set $p (global.get $free_head))
 (block $not_found (loop $search
   (br_if $not_found (i32.eqz (local.get $p)))
   (if (i32.ge_u (i32.load (local.get $p)) (local.get $size))
     (then
       (if (local.get $prev)
         (then (i32.store offset=4 (local.get $prev) (i32.load offset=4 (local.get $p))))
         (else (global.set $free_head (i32.load offset=4 (local.get $p)))))
       (return (i32.add (local.get $p) (i32.const 8)))))
   (local.set $prev (local.get $p))
   (local.set $p (i32.load offset=4 (local.get $p))) (br $search)))
 (local.set $p (global.get $heap))
 ;; Calculate addresses in i64: a full memory32 has 2^32 bytes, which cannot
 ;; be represented by i32. Keep the next heap pointer representable as well.
 (local.set $end (i64.add (i64.extend_i32_u (local.get $p))
   (i64.add (i64.extend_i32_u (local.get $size)) (i64.const 8))))
 (if (i64.gt_u (local.get $end) (i64.const 4294967295)) (then unreachable))
 (local.set $available (i64.shl (i64.extend_i32_u (memory.size)) (i64.const 16)))
 (if (i64.gt_u (local.get $end) (local.get $available))
   (then
     (if (i32.eq (memory.grow (i32.wrap_i64
       (i64.shr_u (i64.add (i64.sub (local.get $end) (local.get $available)) (i64.const 65535)) (i64.const 16)))) (i32.const -1))
       (then unreachable))))
 (i32.store (local.get $p) (local.get $size))
 (i32.store offset=4 (local.get $p) (i32.const 0))
 (global.set $heap (i32.wrap_i64 (local.get $end)))
 (i32.add (local.get $p) (i32.const 8)))
(func $free (export "ilang_free") (param $ptr i32) (param $len i32)
 (local $p i32)
 (if (i32.eqz (local.get $ptr)) (then return))
 (local.set $p (i32.sub (local.get $ptr) (i32.const 8)))
 (i32.store offset=4 (local.get $p) (global.get $free_head))
 (global.set $free_head (local.get $p)))

(func $int_sign (param $v eqref) (result i32)
  (local $x i32)
  (if (result i32) (ref.test (ref i31) (local.get $v))
    (then
      (local.set $x (i31.get_s (ref.cast (ref i31) (local.get $v))))
      (i32.sub (i32.gt_s (local.get $x) (i32.const 0))
               (i32.lt_s (local.get $x) (i32.const 0))))
    (else (struct.get $obj 1 (ref.cast (ref $obj) (local.get $v))))))

(func $int_limbs (param $v eqref) (result (ref $limbs))
  (local $x i32)
  (if (result (ref $limbs)) (ref.test (ref i31) (local.get $v))
    (then
      (local.set $x (i31.get_s (ref.cast (ref i31) (local.get $v))))
      (if (result (ref $limbs)) (i32.eqz (local.get $x))
        (then (array.new_default $limbs (i32.const 0)))
        (else
          (array.new_fixed $limbs 1
            (if (result i32) (i32.lt_s (local.get $x) (i32.const 0))
              (then (i32.sub (i32.const 0) (local.get $x)))
              (else (local.get $x)))))))
    (else
      (ref.as_non_null
        (struct.get $obj 6 (ref.cast (ref $obj) (local.get $v)))))))

(func $mag_len (param $a (ref $limbs)) (result i32)
  (local $n i32)
  (local.set $n (array.len (local.get $a)))
  (block $done
    (loop $trim
      (br_if $done (i32.eqz (local.get $n)))
      (br_if $done
        (array.get $limbs (local.get $a) (i32.sub (local.get $n) (i32.const 1))))
      (local.set $n (i32.sub (local.get $n) (i32.const 1)))
      (br $trim)))
  (local.get $n))

(func $int_pack (param $sign i32) (param $a (ref $limbs)) (result eqref)
  (local $n i32) (local $x i32) (local $trimmed (ref null $limbs))
  (local.set $n (call $mag_len (local.get $a)))
  (if (i32.eqz (local.get $n))
    (then (return (ref.i31 (i32.const 0)))))
  (local.set $sign
    (if (result i32) (i32.lt_s (local.get $sign) (i32.const 0))
      (then (i32.const -1)) (else (i32.const 1))))
  (if (i32.eq (local.get $n) (i32.const 1))
    (then
      (local.set $x (array.get $limbs (local.get $a) (i32.const 0)))
      (if
        (i32.or
          (i32.and (i32.gt_s (local.get $sign) (i32.const 0))
                   (i32.le_u (local.get $x) (i32.const 1073741823)))
          (i32.and (i32.lt_s (local.get $sign) (i32.const 0))
                   (i32.le_u (local.get $x) (i32.const 1073741824))))
        (then
          (return (ref.i31 (i32.mul (local.get $sign) (local.get $x))))))))
  (local.set $trimmed (local.get $a))
  (if (i32.ne (local.get $n) (array.len (local.get $a)))
    (then
      (local.set $trimmed (array.new_default $limbs (local.get $n)))
      (array.copy $limbs $limbs
        (local.get $trimmed) (i32.const 0)
        (local.get $a) (i32.const 0) (local.get $n))))
  (struct.new $obj
    (i32.const 8) (local.get $sign) (i64.const 0) (f64.const 0)
    (ref.null $bytes) (ref.null $values) (local.get $trimmed) (ref.null $call)))

(func $int_from_i64 (param $v i64) (result eqref)
  (local $sign i32) (local $mag i64)
  (if
    (i32.and (i64.ge_s (local.get $v) (i64.const -1073741824))
             (i64.le_s (local.get $v) (i64.const 1073741823)))
    (then (return (ref.i31 (i32.wrap_i64 (local.get $v))))))
  (local.set $sign
    (if (result i32) (i64.lt_s (local.get $v) (i64.const 0))
      (then (i32.const -1)) (else (i32.const 1))))
  (local.set $mag
    (if (result i64) (i32.lt_s (local.get $sign) (i32.const 0))
      (then (i64.sub (i64.const 0) (local.get $v)))
      (else (local.get $v))))
  (if (i64.le_u (local.get $mag) (i64.const 4294967295))
    (then
      (return
        (call $int_pack (local.get $sign)
          (array.new_fixed $limbs 1 (i32.wrap_i64 (local.get $mag)))))))
  (call $int_pack (local.get $sign)
    (array.new_fixed $limbs 2
      (i32.wrap_i64 (local.get $mag))
      (i32.wrap_i64 (i64.shr_u (local.get $mag) (i64.const 32))))))

(func $int_to_i64 (param $v eqref) (result i64)
  (local $a (ref null $limbs)) (local $n i32) (local $mag i64)
  (if (ref.test (ref i31) (local.get $v))
    (then
      (return (i64.extend_i32_s (i31.get_s (ref.cast (ref i31) (local.get $v)))))))
  (local.set $a (call $int_limbs (local.get $v)))
  (local.set $n (array.len (local.get $a)))
  (if (i32.gt_u (local.get $n) (i32.const 2)) (then unreachable))
  (local.set $mag (i64.extend_i32_u (array.get $limbs (local.get $a) (i32.const 0))))
  (if (i32.eq (local.get $n) (i32.const 2))
    (then
      (local.set $mag
        (i64.or (local.get $mag)
          (i64.shl
            (i64.extend_i32_u (array.get $limbs (local.get $a) (i32.const 1)))
            (i64.const 32))))))
  (if (result i64) (i32.lt_s (call $int_sign (local.get $v)) (i32.const 0))
    (then
      (if (i64.gt_u (local.get $mag) (i64.const -9223372036854775808)) (then unreachable))
      (i64.sub (i64.const 0) (local.get $mag)))
    (else
      (if (i64.gt_u (local.get $mag) (i64.const 9223372036854775807)) (then unreachable))
      (local.get $mag))))

(func $mag_compare (param $a (ref $limbs)) (param $b (ref $limbs)) (result i32)
  (local $an i32) (local $bn i32) (local $x i32) (local $y i32)
  (local.set $an (call $mag_len (local.get $a)))
  (local.set $bn (call $mag_len (local.get $b)))
  (if (i32.lt_u (local.get $an) (local.get $bn)) (then (return (i32.const -1))))
  (if (i32.gt_u (local.get $an) (local.get $bn)) (then (return (i32.const 1))))
  (block $done
    (loop $scan
      (br_if $done (i32.eqz (local.get $an)))
      (local.set $an (i32.sub (local.get $an) (i32.const 1)))
      (local.set $x (array.get $limbs (local.get $a) (local.get $an)))
      (local.set $y (array.get $limbs (local.get $b) (local.get $an)))
      (if (i32.lt_u (local.get $x) (local.get $y)) (then (return (i32.const -1))))
      (if (i32.gt_u (local.get $x) (local.get $y)) (then (return (i32.const 1))))
      (br $scan)))
  (i32.const 0))

(func $int_compare (param $a eqref) (param $b eqref) (result i32)
  (local $as i32) (local $bs i32)
  (if (i32.and (ref.test (ref i31) (local.get $a)) (ref.test (ref i31) (local.get $b)))
    (then
      (local.set $as (i31.get_s (ref.cast (ref i31) (local.get $a))))
      (local.set $bs (i31.get_s (ref.cast (ref i31) (local.get $b))))
      (return
        (i32.sub (i32.gt_s (local.get $as) (local.get $bs))
                 (i32.lt_s (local.get $as) (local.get $bs))))))
  (local.set $as (call $int_sign (local.get $a)))
  (local.set $bs (call $int_sign (local.get $b)))
  (if (i32.lt_s (local.get $as) (local.get $bs)) (then (return (i32.const -1))))
  (if (i32.gt_s (local.get $as) (local.get $bs)) (then (return (i32.const 1))))
  (if (i32.eqz (local.get $as)) (then (return (i32.const 0))))
  (i32.mul (local.get $as)
    (call $mag_compare (call $int_limbs (local.get $a)) (call $int_limbs (local.get $b)))))

(func $mag_add (param $a (ref $limbs)) (param $b (ref $limbs)) (result (ref $limbs))
  (local $out (ref null $limbs))
  (local $an i32) (local $bn i32) (local $n i32) (local $i i32)
  (local $carry i64) (local $sum i64)
  (local.set $an (array.len (local.get $a)))
  (local.set $bn (array.len (local.get $b)))
  (local.set $n
    (if (result i32) (i32.gt_u (local.get $an) (local.get $bn))
      (then (local.get $an)) (else (local.get $bn))))
  (local.set $out (array.new_default $limbs (i32.add (local.get $n) (i32.const 1))))
  (block $done
    (loop $add
      (br_if $done (i32.ge_u (local.get $i) (local.get $n)))
      (local.set $sum (local.get $carry))
      (if (i32.lt_u (local.get $i) (local.get $an))
        (then
          (local.set $sum (i64.add (local.get $sum)
            (i64.extend_i32_u (array.get $limbs (local.get $a) (local.get $i)))))))
      (if (i32.lt_u (local.get $i) (local.get $bn))
        (then
          (local.set $sum (i64.add (local.get $sum)
            (i64.extend_i32_u (array.get $limbs (local.get $b) (local.get $i)))))))
      (array.set $limbs (local.get $out) (local.get $i) (i32.wrap_i64 (local.get $sum)))
      (local.set $carry (i64.shr_u (local.get $sum) (i64.const 32)))
      (local.set $i (i32.add (local.get $i) (i32.const 1)))
      (br $add)))
  (array.set $limbs (local.get $out) (local.get $n) (i32.wrap_i64 (local.get $carry)))
  (ref.as_non_null (local.get $out)))

;; Magnitude subtraction; the caller proves a >= b.
(func $mag_sub (param $a (ref $limbs)) (param $b (ref $limbs)) (result (ref $limbs))
  (local $out (ref null $limbs)) (local $an i32) (local $bn i32) (local $i i32)
  (local $borrow i64) (local $x i64) (local $y i64)
  (local.set $an (array.len (local.get $a)))
  (local.set $bn (array.len (local.get $b)))
  (local.set $out (array.new_default $limbs (local.get $an)))
  (block $done
    (loop $sub
      (br_if $done (i32.ge_u (local.get $i) (local.get $an)))
      (local.set $x (i64.extend_i32_u (array.get $limbs (local.get $a) (local.get $i))))
      (local.set $y (local.get $borrow))
      (if (i32.lt_u (local.get $i) (local.get $bn))
        (then
          (local.set $y (i64.add (local.get $y)
            (i64.extend_i32_u (array.get $limbs (local.get $b) (local.get $i)))))))
      (array.set $limbs (local.get $out) (local.get $i)
        (i32.wrap_i64 (i64.sub (local.get $x) (local.get $y))))
      (local.set $borrow (i64.extend_i32_u (i64.lt_u (local.get $x) (local.get $y))))
      (local.set $i (i32.add (local.get $i) (i32.const 1)))
      (br $sub)))
  (ref.as_non_null (local.get $out)))

(func $int_add (param $a eqref) (param $b eqref) (result eqref)
  (local $as i32) (local $bs i32) (local $cmp i32)
  (local $al (ref null $limbs)) (local $bl (ref null $limbs))
  (if (i32.and (ref.test (ref i31) (local.get $a)) (ref.test (ref i31) (local.get $b)))
    (then
      (return (call $int_from_i64
        (i64.add
          (i64.extend_i32_s (i31.get_s (ref.cast (ref i31) (local.get $a))))
          (i64.extend_i32_s (i31.get_s (ref.cast (ref i31) (local.get $b)))))))))
  (local.set $as (call $int_sign (local.get $a)))
  (local.set $bs (call $int_sign (local.get $b)))
  (if (i32.eqz (local.get $as)) (then (return (local.get $b))))
  (if (i32.eqz (local.get $bs)) (then (return (local.get $a))))
  (local.set $al (call $int_limbs (local.get $a)))
  (local.set $bl (call $int_limbs (local.get $b)))
  (if (i32.eq (local.get $as) (local.get $bs))
    (then
      (return (call $int_pack (local.get $as)
        (call $mag_add (ref.as_non_null (local.get $al)) (ref.as_non_null (local.get $bl)))))))
  (local.set $cmp
    (call $mag_compare (ref.as_non_null (local.get $al)) (ref.as_non_null (local.get $bl))))
  (if (i32.eqz (local.get $cmp)) (then (return (ref.i31 (i32.const 0)))))
  (if (result eqref) (i32.gt_s (local.get $cmp) (i32.const 0))
    (then
      (call $int_pack (local.get $as)
        (call $mag_sub (ref.as_non_null (local.get $al)) (ref.as_non_null (local.get $bl)))))
    (else
      (call $int_pack (local.get $bs)
        (call $mag_sub (ref.as_non_null (local.get $bl)) (ref.as_non_null (local.get $al)))))))

(func $int_neg (param $v eqref) (result eqref)
  (if (result eqref) (ref.test (ref i31) (local.get $v))
    (then
      (call $int_from_i64
        (i64.sub (i64.const 0)
          (i64.extend_i32_s (i31.get_s (ref.cast (ref i31) (local.get $v)))))))
    (else
      (call $int_pack (i32.sub (i32.const 0) (call $int_sign (local.get $v)))
        (call $int_limbs (local.get $v))))))

(func $int_sub (param $a eqref) (param $b eqref) (result eqref)
  (if (result eqref)
    (i32.and (ref.test (ref i31) (local.get $a)) (ref.test (ref i31) (local.get $b)))
    (then
      (call $int_from_i64
        (i64.sub
          (i64.extend_i32_s (i31.get_s (ref.cast (ref i31) (local.get $a))))
          (i64.extend_i32_s (i31.get_s (ref.cast (ref i31) (local.get $b)))))))
    (else (call $int_add (local.get $a) (call $int_neg (local.get $b))))))

(func $mag_mul (param $a (ref $limbs)) (param $b (ref $limbs)) (result (ref $limbs))
  (local $out (ref null $limbs)) (local $an i32) (local $bn i32)
  (local $i i32) (local $j i32) (local $k i32)
  (local $carry i64) (local $product i64) (local $x i64)
  (local.set $an (array.len (local.get $a)))
  (local.set $bn (array.len (local.get $b)))
  (local.set $out (array.new_default $limbs (i32.add (local.get $an) (local.get $bn))))
  (block $done
    (loop $rows
      (br_if $done (i32.ge_u (local.get $i) (local.get $an)))
      (local.set $x (i64.extend_i32_u (array.get $limbs (local.get $a) (local.get $i))))
      (local.set $carry (i64.const 0))
      (local.set $j (i32.const 0))
      (block $row_done
        (loop $columns
          (br_if $row_done (i32.ge_u (local.get $j) (local.get $bn)))
          (local.set $k (i32.add (local.get $i) (local.get $j)))
          (local.set $product
            (i64.add
              (i64.add
                (i64.mul (local.get $x)
                  (i64.extend_i32_u (array.get $limbs (local.get $b) (local.get $j))))
                (i64.extend_i32_u (array.get $limbs (local.get $out) (local.get $k))))
              (local.get $carry)))
          (array.set $limbs (local.get $out) (local.get $k) (i32.wrap_i64 (local.get $product)))
          (local.set $carry (i64.shr_u (local.get $product) (i64.const 32)))
          (local.set $j (i32.add (local.get $j) (i32.const 1)))
          (br $columns)))
      (array.set $limbs (local.get $out) (i32.add (local.get $i) (local.get $bn))
        (i32.wrap_i64 (local.get $carry)))
      (local.set $i (i32.add (local.get $i) (i32.const 1)))
      (br $rows)))
  (ref.as_non_null (local.get $out)))

(func $int_mul (param $a eqref) (param $b eqref) (result eqref)
  (local $sign i32)
  (if (i32.and (ref.test (ref i31) (local.get $a)) (ref.test (ref i31) (local.get $b)))
    (then
      (return (call $int_from_i64
        (i64.mul
          (i64.extend_i32_s (i31.get_s (ref.cast (ref i31) (local.get $a))))
          (i64.extend_i32_s (i31.get_s (ref.cast (ref i31) (local.get $b)))))))))
  (local.set $sign (i32.mul (call $int_sign (local.get $a)) (call $int_sign (local.get $b))))
  (if (i32.eqz (local.get $sign)) (then (return (ref.i31 (i32.const 0)))))
  (call $int_pack (local.get $sign)
    (call $mag_mul (call $int_limbs (local.get $a)) (call $int_limbs (local.get $b)))))

;; Binary long division. All writes target newly allocated q and r arrays.
;; The remainder carries one extra limb while it is shifted, so no leading
;; carry is lost even when the divisor's highest bit is set.
(func $mag_divrem (param $a (ref $limbs)) (param $b (ref $limbs))
  (result (ref $limbs) (ref $limbs))
  (local $q (ref null $limbs)) (local $r (ref null $limbs))
  (local $an i32) (local $bn i32) (local $rn i32)
  (local $bit i64) (local $i i32) (local $word i32) (local $shift i32)
  (local $carry i64) (local $sum i64) (local $x i64) (local $y i64) (local $borrow i64)
  (local.set $an (call $mag_len (local.get $a)))
  (local.set $bn (call $mag_len (local.get $b)))
  (if (i32.eqz (local.get $bn)) (then unreachable))
  (if (i32.lt_s (call $mag_compare (local.get $a) (local.get $b)) (i32.const 0))
    (then (return (array.new_default $limbs (i32.const 0)) (local.get $a))))
  ;; Common divisors such as 10 and 36 need a single pass over the limbs.
  ;; carry < divisor <= u32::MAX keeps the combined dividend within u64.
  (if (i32.eq (local.get $bn) (i32.const 1))
    (then
      (local.set $q (array.new_default $limbs (local.get $an)))
      (local.set $x (i64.extend_i32_u (array.get $limbs (local.get $b) (i32.const 0))))
      (local.set $i (local.get $an))
      (local.set $carry (i64.const 0))
      (block $small_done
        (loop $small_divide
          (br_if $small_done (i32.eqz (local.get $i)))
          (local.set $i (i32.sub (local.get $i) (i32.const 1)))
          (local.set $sum
            (i64.or (i64.shl (local.get $carry) (i64.const 32))
              (i64.extend_i32_u (array.get $limbs (local.get $a) (local.get $i)))))
          (array.set $limbs (local.get $q) (local.get $i)
            (i32.wrap_i64 (i64.div_u (local.get $sum) (local.get $x))))
          (local.set $carry (i64.rem_u (local.get $sum) (local.get $x)))
          (br $small_divide)))
      (return (ref.as_non_null (local.get $q))
        (array.new_fixed $limbs 1 (i32.wrap_i64 (local.get $carry))))))
  (local.set $rn (i32.add (local.get $bn) (i32.const 1)))
  (local.set $q (array.new_default $limbs (local.get $an)))
  (local.set $r (array.new_default $limbs (local.get $rn)))
  (local.set $bit (i64.mul (i64.extend_i32_u (local.get $an)) (i64.const 32)))
  (block $done
    (loop $bits
      (br_if $done (i64.eqz (local.get $bit)))
      (local.set $bit (i64.sub (local.get $bit) (i64.const 1)))
      (local.set $word (i32.wrap_i64 (i64.shr_u (local.get $bit) (i64.const 5))))
      (local.set $shift (i32.and (i32.wrap_i64 (local.get $bit)) (i32.const 31)))
      (local.set $carry (i64.extend_i32_u
        (i32.and
          (i32.shr_u (array.get $limbs (local.get $a) (local.get $word)) (local.get $shift))
          (i32.const 1))))
      (local.set $i (i32.const 0))
      (block $shift_done
        (loop $shift_r
          (br_if $shift_done (i32.ge_u (local.get $i) (local.get $rn)))
          (local.set $sum
            (i64.or
              (i64.shl (i64.extend_i32_u (array.get $limbs (local.get $r) (local.get $i)))
                (i64.const 1))
              (local.get $carry)))
          (array.set $limbs (local.get $r) (local.get $i) (i32.wrap_i64 (local.get $sum)))
          (local.set $carry (i64.shr_u (local.get $sum) (i64.const 32)))
          (local.set $i (i32.add (local.get $i) (i32.const 1)))
          (br $shift_r)))
      (if
        (i32.ge_s
          (call $mag_compare (ref.as_non_null (local.get $r)) (local.get $b))
          (i32.const 0))
        (then
          (local.set $i (i32.const 0))
          (local.set $borrow (i64.const 0))
          (block $sub_done
            (loop $sub_r
              (br_if $sub_done (i32.ge_u (local.get $i) (local.get $rn)))
              (local.set $x (i64.extend_i32_u (array.get $limbs (local.get $r) (local.get $i))))
              (local.set $y (local.get $borrow))
              (if (i32.lt_u (local.get $i) (local.get $bn))
                (then
                  (local.set $y
                    (i64.add (local.get $y)
                      (i64.extend_i32_u (array.get $limbs (local.get $b) (local.get $i)))))))
              (array.set $limbs (local.get $r) (local.get $i)
                (i32.wrap_i64 (i64.sub (local.get $x) (local.get $y))))
              (local.set $borrow (i64.extend_i32_u (i64.lt_u (local.get $x) (local.get $y))))
              (local.set $i (i32.add (local.get $i) (i32.const 1)))
              (br $sub_r)))
          (array.set $limbs (local.get $q) (local.get $word)
            (i32.or (array.get $limbs (local.get $q) (local.get $word))
              (i32.shl (i32.const 1) (local.get $shift))))))
      (br $bits)))
  (ref.as_non_null (local.get $q))
  (ref.as_non_null (local.get $r)))

(func $int_div (param $a eqref) (param $b eqref) (result eqref)
  (local $q (ref null $limbs)) (local $r (ref null $limbs))
  (if (i32.eqz (call $int_sign (local.get $b)))
    (then (return (ref.i31 (i32.const 0)))))
  (if (i32.and (ref.test (ref i31) (local.get $a)) (ref.test (ref i31) (local.get $b)))
    (then
      (return (call $int_from_i64
        (i64.div_s
          (i64.extend_i32_s (i31.get_s (ref.cast (ref i31) (local.get $a))))
          (i64.extend_i32_s (i31.get_s (ref.cast (ref i31) (local.get $b)))))))))
  (call $mag_divrem (call $int_limbs (local.get $a)) (call $int_limbs (local.get $b)))
  (local.set $r) (local.set $q)
  (call $int_pack (i32.mul (call $int_sign (local.get $a)) (call $int_sign (local.get $b)))
    (ref.as_non_null (local.get $q))))

(func $int_rem (param $a eqref) (param $b eqref) (result eqref)
  (local $q (ref null $limbs)) (local $r (ref null $limbs))
  (if (i32.eqz (call $int_sign (local.get $b)))
    (then (return (ref.i31 (i32.const 0)))))
  (if (i32.and (ref.test (ref i31) (local.get $a)) (ref.test (ref i31) (local.get $b)))
    (then
      (return (call $int_from_i64
        (i64.rem_s
          (i64.extend_i32_s (i31.get_s (ref.cast (ref i31) (local.get $a))))
          (i64.extend_i32_s (i31.get_s (ref.cast (ref i31) (local.get $b)))))))))
  (call $mag_divrem (call $int_limbs (local.get $a)) (call $int_limbs (local.get $b)))
  (local.set $r) (local.set $q)
  (call $int_pack (call $int_sign (local.get $a)) (ref.as_non_null (local.get $r))))

  (func $int_twos (param $v eqref) (param $n i32) (result (ref $limbs))
    (local $m (ref $limbs)) (local $out (ref $limbs))
    (local $i i32) (local $size i32) (local $negative i32)
    (local $carry i64) (local $word i64)
    (local.set $m (call $int_limbs (local.get $v)))
    (local.set $size (array.len (local.get $m)))
    (local.set $out (array.new_default $limbs (local.get $n)))
    (local.set $negative (i32.lt_s (call $int_sign (local.get $v)) (i32.const 0)))
    (local.set $carry (i64.extend_i32_u (local.get $negative)))
    (block $done
      (loop $next
        (br_if $done (i32.ge_u (local.get $i) (local.get $n)))
        (local.set $word
          (if (result i64) (i32.lt_u (local.get $i) (local.get $size))
            (then (i64.extend_i32_u (array.get $limbs (local.get $m) (local.get $i))))
            (else (i64.const 0))))
        (if (local.get $negative)
          (then
            (local.set $word
              (i64.add (i64.xor (local.get $word) (i64.const 4294967295)) (local.get $carry)))
            (local.set $carry (i64.shr_u (local.get $word) (i64.const 32)))))
        (array.set $limbs (local.get $out) (local.get $i) (i32.wrap_i64 (local.get $word)))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $next)))
    (local.get $out))

  (func $int_from_twos (param $words (ref $limbs)) (result eqref)
    (local $n i32) (local $i i32) (local $carry i64) (local $word i64)
    (local.set $n (array.len (local.get $words)))
    (if (i32.eqz (local.get $n))
      (then (return (ref.i31 (i32.const 0)))))
    (if (i32.ge_s
          (array.get $limbs (local.get $words) (i32.sub (local.get $n) (i32.const 1)))
          (i32.const 0))
      (then (return (call $int_pack (i32.const 1) (local.get $words)))))
    (local.set $carry (i64.const 1))
    (block $done
      (loop $next
        (br_if $done (i32.ge_u (local.get $i) (local.get $n)))
        (local.set $word
          (i64.add
            (i64.extend_i32_u (i32.xor (array.get $limbs (local.get $words) (local.get $i)) (i32.const -1)))
            (local.get $carry)))
        (array.set $limbs (local.get $words) (local.get $i) (i32.wrap_i64 (local.get $word)))
        (local.set $carry (i64.shr_u (local.get $word) (i64.const 32)))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $next)))
    (call $int_pack (i32.const -1) (local.get $words)))

  (func $int_bitop (param $a eqref) (param $b eqref) (param $op i32) (result eqref)
    (local $an i32) (local $bn i32) (local $n i32) (local $i i32)
    (local $aa (ref $limbs)) (local $bb (ref $limbs))
    (local $x i32) (local $y i32)
    (if (i32.and (ref.test (ref i31) (local.get $a)) (ref.test (ref i31) (local.get $b)))
      (then
        (local.set $x (i31.get_s (ref.cast (ref i31) (local.get $a))))
        (local.set $y (i31.get_s (ref.cast (ref i31) (local.get $b))))
        (return
          (ref.i31
            (if (result i32) (i32.eqz (local.get $op))
              (then (i32.and (local.get $x) (local.get $y)))
              (else
                (if (result i32) (i32.eq (local.get $op) (i32.const 1))
                  (then (i32.or (local.get $x) (local.get $y)))
                  (else (i32.xor (local.get $x) (local.get $y))))))))))
    (local.set $an (array.len (call $int_limbs (local.get $a))))
    (local.set $bn (array.len (call $int_limbs (local.get $b))))
    (local.set $n
      (select (local.get $an) (local.get $bn) (i32.gt_u (local.get $an) (local.get $bn))))
    (if (i32.eq (local.get $n) (i32.const -1)) (then unreachable))
    (local.set $n (i32.add (local.get $n) (i32.const 1)))
    (local.set $aa (call $int_twos (local.get $a) (local.get $n)))
    (local.set $bb (call $int_twos (local.get $b) (local.get $n)))
    (block $done
      (loop $next
        (br_if $done (i32.ge_u (local.get $i) (local.get $n)))
        (local.set $x (array.get $limbs (local.get $aa) (local.get $i)))
        (local.set $y (array.get $limbs (local.get $bb) (local.get $i)))
        (array.set $limbs (local.get $aa) (local.get $i)
          (if (result i32) (i32.eqz (local.get $op))
            (then (i32.and (local.get $x) (local.get $y)))
            (else
              (if (result i32) (i32.eq (local.get $op) (i32.const 1))
                (then (i32.or (local.get $x) (local.get $y)))
                (else (i32.xor (local.get $x) (local.get $y)))))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $next)))
    (call $int_from_twos (local.get $aa)))

  (func $int_and (param $a eqref) (param $b eqref) (result eqref)
    ;; HAMT bitmap/index masks only inspect the low word. In particular an
    ;; arbitrary-size integer AND 31 never needs a two's-complement buffer.
    (if (ref.test (ref i31) (local.get $a))
      (then
        (if (i32.ge_s (i31.get_s (ref.cast (ref i31) (local.get $a))) (i32.const 0))
          (then
            (return (ref.i31
              (i32.and (i31.get_s (ref.cast (ref i31) (local.get $a)))
                (call $int_low_i32 (local.get $b)))))))))
    (if (ref.test (ref i31) (local.get $b))
      (then
        (if (i32.ge_s (i31.get_s (ref.cast (ref i31) (local.get $b))) (i32.const 0))
          (then
            (return (ref.i31
              (i32.and (i31.get_s (ref.cast (ref i31) (local.get $b)))
                (call $int_low_i32 (local.get $a)))))))))
    (call $int_bitop (local.get $a) (local.get $b) (i32.const 0)))
  (func $int_low_i32 (param $v eqref) (result i32)
    (local $word i32)
    (if (ref.test (ref i31) (local.get $v))
      (then (return (i31.get_s (ref.cast (ref i31) (local.get $v))))))
    (local.set $word
      (array.get $limbs
        (struct.get $obj 6 (ref.cast (ref $obj) (local.get $v))) (i32.const 0)))
    (if (result i32) (i32.lt_s (call $int_sign (local.get $v)) (i32.const 0))
      (then (i32.sub (i32.const 0) (local.get $word)))
      (else (local.get $word))))
  (func $int_or (param $a eqref) (param $b eqref) (result eqref)
    (call $int_bitop (local.get $a) (local.get $b) (i32.const 1)))
  (func $int_xor (param $a eqref) (param $b eqref) (result eqref)
    (call $int_bitop (local.get $a) (local.get $b) (i32.const 2)))
  (func $int_not (param $a eqref) (result eqref)
    (if (ref.test (ref i31) (local.get $a))
      (then
        (return (ref.i31 (i32.xor (i31.get_s (ref.cast (ref i31) (local.get $a))) (i32.const -1))))))
    (call $int_sub (call $int_neg (local.get $a)) (ref.i31 (i32.const 1))))

  (func $int_shift_left_positive (param $v eqref) (param $count eqref) (result eqref)
    (local $m (ref $limbs)) (local $out (ref $limbs))
    (local $size i32) (local $word_shift i32) (local $bit_shift i64)
    (local $n i64) (local $amount i64) (local $i i32)
    (local $carry i64) (local $word i64)
    (if (i32.eqz (call $int_sign (local.get $v)))
      (then (return (ref.i31 (i32.const 0)))))
    (if (i32.eqz (call $int_sign (local.get $count)))
      (then (return (local.get $v))))
    ;; Every signed i31 value shifted by at most 33 fits a signed i64.
    ;; Restrict the count before using Wasm's masked shift instruction.
    (if (i32.and (ref.test (ref i31) (local.get $v)) (ref.test (ref i31) (local.get $count)))
      (then
        (if (i32.le_u (i31.get_u (ref.cast (ref i31) (local.get $count))) (i32.const 33))
          (then
            (return (call $int_from_i64
              (i64.shl
                (i64.extend_i32_s (i31.get_s (ref.cast (ref i31) (local.get $v))))
                (i64.extend_i32_u (i31.get_u (ref.cast (ref i31) (local.get $count)))))))))))
    ;; A GC array has a u32 length. Counts above its maximum bit capacity
    ;; cannot be represented regardless of the available physical memory.
    (if (i32.gt_s
          (call $int_compare (local.get $count) (call $int_from_i64 (i64.const 137438953440)))
          (i32.const 0))
      (then unreachable))
    (local.set $amount (call $int_to_i64 (local.get $count)))
    (local.set $m (call $int_limbs (local.get $v)))
    (local.set $size (array.len (local.get $m)))
    (local.set $word_shift (i32.wrap_i64 (i64.shr_u (local.get $amount) (i64.const 5))))
    (local.set $bit_shift (i64.and (local.get $amount) (i64.const 31)))
    (local.set $n
      (i64.add
        (i64.add (i64.extend_i32_u (local.get $size)) (i64.extend_i32_u (local.get $word_shift)))
        (i64.const 1)))
    (if (i64.gt_u (local.get $n) (i64.const 4294967295)) (then unreachable))
    (local.set $out (array.new_default $limbs (i32.wrap_i64 (local.get $n))))
    (block $done
      (loop $next
        (br_if $done (i32.ge_u (local.get $i) (local.get $size)))
        (local.set $word
          (i64.or
            (i64.shl (i64.extend_i32_u (array.get $limbs (local.get $m) (local.get $i))) (local.get $bit_shift))
            (local.get $carry)))
        (array.set $limbs (local.get $out)
          (i32.add (local.get $word_shift) (local.get $i))
          (i32.wrap_i64 (local.get $word)))
        (local.set $carry (i64.shr_u (local.get $word) (i64.const 32)))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $next)))
    (array.set $limbs (local.get $out)
      (i32.add (local.get $word_shift) (local.get $size))
      (i32.wrap_i64 (local.get $carry)))
    (call $int_pack (call $int_sign (local.get $v)) (local.get $out)))

  (func $int_shift_right_positive (param $v eqref) (param $count eqref) (result eqref)
    (local $m (ref $limbs)) (local $out (ref $limbs))
    (local $sign i32) (local $size i32) (local $word_shift i32) (local $bit_shift i32)
    (local $n i32) (local $amount i64) (local $i i32) (local $source i32)
    (local $discarded i32) (local $word i64) (local $result eqref)
    (local.set $sign (call $int_sign (local.get $v)))
    (if (i32.eqz (local.get $sign))
      (then (return (ref.i31 (i32.const 0)))))
    (if (i32.eqz (call $int_sign (local.get $count)))
      (then (return (local.get $v))))
    (if (ref.test (ref i31) (local.get $count))
      (then
        (local.set $word_shift (i31.get_u (ref.cast (ref i31) (local.get $count))))
        (if (ref.test (ref i31) (local.get $v))
          (then
            (if (i32.ge_u (local.get $word_shift) (i32.const 31))
              (then
                (return (ref.i31
                  (select (i32.const -1) (i32.const 0) (i32.lt_s (local.get $sign) (i32.const 0)))))))
            (return (ref.i31
              (i32.shr_s (i31.get_s (ref.cast (ref i31) (local.get $v))) (local.get $word_shift))))))
        ;; Positive 32-bit hashes and signed one-limb integers fit in i64.
        (if
          (i32.eq
            (array.len (struct.get $obj 6 (ref.cast (ref $obj) (local.get $v))))
            (i32.const 1))
          (then
            (if (i32.ge_u (local.get $word_shift) (i32.const 32))
              (then
                (return (ref.i31
                  (select (i32.const -1) (i32.const 0) (i32.lt_s (local.get $sign) (i32.const 0)))))))
            (return (call $int_from_i64
              (i64.shr_s (call $int_to_i64 (local.get $v))
                (i64.extend_i32_u (local.get $word_shift)))))))))
    (local.set $m (call $int_limbs (local.get $v)))
    (local.set $size (array.len (local.get $m)))
    (if (i32.ge_s
          (call $int_compare (local.get $count)
            (call $int_from_i64 (i64.shl (i64.extend_i32_u (local.get $size)) (i64.const 5))))
          (i32.const 0))
      (then
        (return (ref.i31 (select (i32.const -1) (i32.const 0) (i32.lt_s (local.get $sign) (i32.const 0)))))))
    (local.set $amount (call $int_to_i64 (local.get $count)))
    (local.set $word_shift (i32.wrap_i64 (i64.shr_u (local.get $amount) (i64.const 5))))
    (local.set $bit_shift (i32.wrap_i64 (i64.and (local.get $amount) (i64.const 31))))
    (local.set $n (i32.sub (local.get $size) (local.get $word_shift)))
    (local.set $out (array.new_default $limbs (local.get $n)))
    (block $discard_done
      (loop $discard_next
        (br_if $discard_done (i32.ge_u (local.get $i) (local.get $word_shift)))
        (local.set $discarded
          (i32.or (local.get $discarded) (array.get $limbs (local.get $m) (local.get $i))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $discard_next)))
    (if (local.get $bit_shift)
      (then
        (local.set $discarded
          (i32.or (local.get $discarded)
            (i32.and
              (array.get $limbs (local.get $m) (local.get $word_shift))
              (i32.shr_u (i32.const -1) (i32.sub (i32.const 32) (local.get $bit_shift))))))))
    (local.set $i (i32.const 0))
    (block $done
      (loop $next
        (br_if $done (i32.ge_u (local.get $i) (local.get $n)))
        (local.set $source (i32.add (local.get $word_shift) (local.get $i)))
        (local.set $word
          (i64.shr_u
            (i64.extend_i32_u (array.get $limbs (local.get $m) (local.get $source)))
            (i64.extend_i32_u (local.get $bit_shift))))
        (if (i32.and
              (i32.ne (local.get $bit_shift) (i32.const 0))
              (i32.lt_u (i32.add (local.get $source) (i32.const 1)) (local.get $size)))
          (then
            (local.set $word
              (i64.or (local.get $word)
                (i64.shl
                  (i64.extend_i32_u (array.get $limbs (local.get $m) (i32.add (local.get $source) (i32.const 1))))
                  (i64.extend_i32_u (i32.sub (i32.const 32) (local.get $bit_shift))))))))
        (array.set $limbs (local.get $out) (local.get $i) (i32.wrap_i64 (local.get $word)))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $next)))
    (local.set $result (call $int_pack (local.get $sign) (local.get $out)))
    (if (result eqref)
      (i32.and (i32.lt_s (local.get $sign) (i32.const 0)) (i32.ne (local.get $discarded) (i32.const 0)))
      (then (call $int_sub (local.get $result) (ref.i31 (i32.const 1))))
      (else (local.get $result))))

  (func $int_shl (param $v eqref) (param $count eqref) (result eqref)
    (if (result eqref) (i32.lt_s (call $int_sign (local.get $count)) (i32.const 0))
      (then (call $int_shift_right_positive (local.get $v) (call $int_neg (local.get $count))))
      (else (call $int_shift_left_positive (local.get $v) (local.get $count)))))
  (func $int_shr (param $v eqref) (param $count eqref) (result eqref)
    (if (result eqref) (i32.lt_s (call $int_sign (local.get $count)) (i32.const 0))
      (then (call $int_shift_left_positive (local.get $v) (call $int_neg (local.get $count))))
      (else (call $int_shift_right_positive (local.get $v) (local.get $count)))))

  ;; Public standard-library intrinsics preserve the boxed internal ABI.
  (func $int_bitwise_and (param $a eqref) (param $b eqref) (result eqref)
    (call $int_and (local.get $a) (local.get $b)))
  (func $int_bitwise_or (param $a eqref) (param $b eqref) (result eqref)
    (call $int_or (local.get $a) (local.get $b)))
  (func $int_bitwise_xor (param $a eqref) (param $b eqref) (result eqref)
    (call $int_xor (local.get $a) (local.get $b)))
  (func $int_bitwise_not (param $a eqref) (result eqref)
    (call $int_not (local.get $a)))
  (func $int_bitwise_shift_left (param $v eqref) (param $count eqref) (result eqref)
    (call $int_shl (local.get $v) (local.get $count)))
  (func $int_bitwise_shift_right (param $v eqref) (param $count eqref) (result eqref)
    (call $int_shr (local.get $v) (local.get $count)))

;; Read a magnitude bit. Callers only use valid, nonnegative bit indices.
(func $conv_mag_bit (param $a (ref $limbs)) (param $bit i32) (result i32)
  (i32.and
    (i32.shr_u
      (array.get $limbs (local.get $a) (i32.shr_u (local.get $bit) (i32.const 5)))
      (i32.and (local.get $bit) (i32.const 31)))
    (i32.const 1)))

(func $conv_string (param $bytes (ref $bytes)) (result eqref)
  (struct.new $obj
    (i32.const 3) (i32.const 0) (i64.const 0) (f64.const 0)
    (local.get $bytes) (ref.null $values) (ref.null $limbs) (ref.null $call)))

;; Round the integer once, directly to the nearest binary64 significand. Building
;; a float by repeatedly multiplying rounded limb prefixes can double-round.
(func $int_to_f64 (param $value eqref) (result f64)
  (local $sign i32) (local $a (ref $limbs)) (local $n i32)
  (local $bits i32) (local $shift i32) (local $i i32)
  (local $q i64) (local $sticky i32) (local $number f64)
  (if (ref.test (ref i31) (local.get $value))
    (then
      (return
        (f64.convert_i32_s (i31.get_s (ref.cast (ref i31) (local.get $value)))))))
  (local.set $sign (call $int_sign (local.get $value)))
  (if (i32.eqz (local.get $sign)) (then (return (f64.const 0))))
  (local.set $a (call $int_limbs (local.get $value)))
  (local.set $n (array.len (local.get $a)))
  ;; 32 limbs hold every finite binary64 integer; more is not finite (§13 to_float).
  (if (i32.gt_u (local.get $n) (i32.const 32)) (then (call $stop_float) unreachable))
  (local.set $bits
    (i32.sub
      (i32.mul (local.get $n) (i32.const 32))
      (i32.clz
        (array.get $limbs (local.get $a) (i32.sub (local.get $n) (i32.const 1))))))
  (local.set $shift
    (if (result i32) (i32.gt_u (local.get $bits) (i32.const 53))
      (then (i32.sub (local.get $bits) (i32.const 53)))
      (else (i32.const 0))))
  (local.set $i (local.get $bits))
  (block $significand_done
    (loop $significand
      (br_if $significand_done (i32.le_u (local.get $i) (local.get $shift)))
      (local.set $i (i32.sub (local.get $i) (i32.const 1)))
      (local.set $q
        (i64.or
          (i64.shl (local.get $q) (i64.const 1))
          (i64.extend_i32_u (call $conv_mag_bit (local.get $a) (local.get $i)))))
      (br $significand)))
  (if (i32.gt_u (local.get $shift) (i32.const 0))
    (then
      (if (call $conv_mag_bit (local.get $a) (i32.sub (local.get $shift) (i32.const 1)))
        (then
          (local.set $i (i32.const 0))
          (block $sticky_done
            (loop $scan_sticky
              (br_if $sticky_done
                (i32.ge_u (local.get $i) (i32.sub (local.get $shift) (i32.const 1))))
              (if (call $conv_mag_bit (local.get $a) (local.get $i))
                (then (local.set $sticky (i32.const 1)) (br $sticky_done)))
              (local.set $i (i32.add (local.get $i) (i32.const 1)))
              (br $scan_sticky)))
          (if (i32.or (local.get $sticky) (i32.wrap_i64 (i64.and (local.get $q) (i64.const 1))))
            (then (local.set $q (i64.add (local.get $q) (i64.const 1)))))))))
  (local.set $number
    (f64.mul
      (f64.convert_i64_u (local.get $q))
      (f64.reinterpret_i64
        (i64.shl
          (i64.extend_i32_u (i32.add (local.get $shift) (i32.const 1023)))
          (i64.const 52)))))
  (if (f64.gt (local.get $number) (f64.const 0x1.fffffffffffffp+1023))
    (then (call $stop_float) unreachable))
  (if (result f64) (i32.lt_s (local.get $sign) (i32.const 0))
    (then (f64.neg (local.get $number)))
    (else (local.get $number))))

;; Decode binary64's exact integer significand and shift it. This avoids any
;; fixed-width intermediate restriction when truncating a finite large float.
(func $int_from_f64 (param $number f64) (result eqref)
  (local $absolute f64) (local $bits i64) (local $mantissa i64)
  (local $exponent i32) (local $shift i32) (local $sign i32)
  (local $a (ref $limbs)) (local $i i32) (local $position i32)
  (local $word i32) (local $small i64)
  (local.set $absolute (f64.abs (local.get $number)))
  (if
    (i32.or
      (f64.ne (local.get $number) (local.get $number))
      (f64.gt (local.get $absolute) (f64.const 0x1.fffffffffffffp+1023)))
    (then unreachable))
  (if (f64.lt (local.get $absolute) (f64.const 1))
    (then (return (ref.i31 (i32.const 0)))))
  (local.set $sign
    (if (result i32) (f64.lt (local.get $number) (f64.const 0))
      (then (i32.const -1)) (else (i32.const 1))))
  (local.set $bits (i64.reinterpret_f64 (local.get $absolute)))
  (local.set $exponent
    (i32.sub
      (i32.wrap_i64 (i64.and (i64.shr_u (local.get $bits) (i64.const 52)) (i64.const 2047)))
      (i32.const 1023)))
  (local.set $mantissa
    (i64.or (i64.and (local.get $bits) (i64.const 0x000fffffffffffff))
      (i64.const 0x0010000000000000)))
  (if (i32.le_s (local.get $exponent) (i32.const 52))
    (then
      (local.set $small
        (i64.shr_u (local.get $mantissa)
          (i64.extend_i32_u (i32.sub (i32.const 52) (local.get $exponent)))))
      (if (i32.lt_s (local.get $sign) (i32.const 0))
        (then (local.set $small (i64.sub (i64.const 0) (local.get $small)))))
      (return (call $int_from_i64 (local.get $small)))))
  (local.set $shift (i32.sub (local.get $exponent) (i32.const 52)))
  (local.set $a
    (array.new_default $limbs
      (i32.shr_u (i32.add (local.get $exponent) (i32.const 32)) (i32.const 5))))
  (block $decoded
    (loop $decode
      (br_if $decoded (i32.ge_u (local.get $i) (i32.const 53)))
      (if (i64.ne
        (i64.and (i64.shr_u (local.get $mantissa) (i64.extend_i32_u (local.get $i))) (i64.const 1))
        (i64.const 0))
        (then
          (local.set $position (i32.add (local.get $i) (local.get $shift)))
          (local.set $word (i32.shr_u (local.get $position) (i32.const 5)))
          (array.set $limbs (local.get $a) (local.get $word)
            (i32.or (array.get $limbs (local.get $a) (local.get $word))
              (i32.shl (i32.const 1) (i32.and (local.get $position) (i32.const 31)))))))
      (local.set $i (i32.add (local.get $i) (i32.const 1)))
      (br $decode)))
  (call $int_pack (local.get $sign) (local.get $a)))

(func $float_truncate (param $value eqref) (result eqref)
  (call $int_from_f64 (call $float_get (local.get $value))))

(func $int_to_float (param $value eqref) (result eqref)
  (call $float_new (call $int_to_f64 (local.get $value))))

(func $float_to_bits (param $value eqref) (result eqref)
  (local $bits i64)
  (local.set $bits (i64.reinterpret_f64 (call $float_get (local.get $value))))
  (call $int_pack (i32.const 1)
    (array.new_fixed $limbs 2
      (i32.wrap_i64 (local.get $bits))
      (i32.wrap_i64 (i64.shr_u (local.get $bits) (i64.const 32))))))

;; The float with the lowest 64 bits of the number in two's complement
;; (spec §13). $float_new stops when they are not a finite float.
(func $float_from_bits (param $value eqref) (result eqref)
  (local $a (ref $limbs))
  (local.set $a (call $int_twos (local.get $value) (i32.const 2)))
  (call $float_new (f64.reinterpret_i64
    (i64.or (i64.extend_i32_u (array.get $limbs (local.get $a) (i32.const 0)))
      (i64.shl (i64.extend_i32_u (array.get $limbs (local.get $a) (i32.const 1))) (i64.const 32))))))

(func $int_to_string (param $value eqref) (result eqref)
  (call $int_to_base_string (local.get $value) (ref.i31 (i32.const 10))))

;; Divide a private working magnitude in place by the small output base. Each
;; quotient step uses an exact unsigned 64-bit dividend (remainder < base <= 36).
(func $int_to_base_string (param $value eqref) (param $base_value eqref) (result eqref)
  (local $sign i32) (local $base i64) (local $a (ref $limbs))
  (local $work (ref $limbs)) (local $n i32) (local $i i32)
  (local $count i32) (local $remainder i64) (local $dividend i64)
  (local $digit i32) (local $reverse (ref $bytes)) (local $bytes (ref $bytes))
  (if
    (i32.or
      (i32.lt_s (call $int_compare (local.get $base_value) (ref.i31 (i32.const 2))) (i32.const 0))
      (i32.gt_s (call $int_compare (local.get $base_value) (ref.i31 (i32.const 36))) (i32.const 0)))
    (then unreachable))
  (local.set $base (call $int_to_i64 (local.get $base_value)))
  (local.set $sign (call $int_sign (local.get $value)))
  (if (i32.eqz (local.get $sign))
    (then (return (call $conv_string (array.new_fixed $bytes 1 (i32.const 48))))))
  (local.set $a (call $int_limbs (local.get $value)))
  (local.set $n (array.len (local.get $a)))
  (if (i32.gt_u (local.get $n) (i32.const 0x07ffffff)) (then unreachable))
  (local.set $work (array.new_default $limbs (local.get $n)))
  (array.copy $limbs $limbs (local.get $work) (i32.const 0)
    (local.get $a) (i32.const 0) (local.get $n))
  (local.set $reverse
    (array.new_default $bytes (i32.add (i32.mul (local.get $n) (i32.const 32)) (i32.const 1))))
  (block $digits_done
    (loop $digits
      (br_if $digits_done (i32.eqz (local.get $n)))
      (local.set $i (local.get $n))
      (local.set $remainder (i64.const 0))
      (block $division_done
        (loop $division
          (br_if $division_done (i32.eqz (local.get $i)))
          (local.set $i (i32.sub (local.get $i) (i32.const 1)))
          (local.set $dividend
            (i64.or (i64.shl (local.get $remainder) (i64.const 32))
              (i64.extend_i32_u (array.get $limbs (local.get $work) (local.get $i)))))
          (array.set $limbs (local.get $work) (local.get $i)
            (i32.wrap_i64 (i64.div_u (local.get $dividend) (local.get $base))))
          (local.set $remainder (i64.rem_u (local.get $dividend) (local.get $base)))
          (br $division)))
      (local.set $digit (i32.wrap_i64 (local.get $remainder)))
      (array.set $bytes (local.get $reverse) (local.get $count)
        (i32.add (local.get $digit)
          (if (result i32) (i32.lt_u (local.get $digit) (i32.const 10))
            (then (i32.const 48)) (else (i32.const 55)))))
      (local.set $count (i32.add (local.get $count) (i32.const 1)))
      (block $trimmed
        (loop $trim
          (br_if $trimmed (i32.eqz (local.get $n)))
          (br_if $trimmed
            (array.get $limbs (local.get $work) (i32.sub (local.get $n) (i32.const 1))))
          (local.set $n (i32.sub (local.get $n) (i32.const 1)))
          (br $trim)))
      (br $digits)))
  (if (i32.lt_s (local.get $sign) (i32.const 0))
    (then
      (array.set $bytes (local.get $reverse) (local.get $count) (i32.const 45))
      (local.set $count (i32.add (local.get $count) (i32.const 1)))))
  (local.set $bytes (array.new_default $bytes (local.get $count)))
  (local.set $i (i32.const 0))
  (block $reversed
    (loop $reverse_digits
      (br_if $reversed (i32.ge_u (local.get $i) (local.get $count)))
      (array.set $bytes (local.get $bytes) (local.get $i)
        (array.get_u $bytes (local.get $reverse)
          (i32.sub (i32.sub (local.get $count) (local.get $i)) (i32.const 1))))
      (local.set $i (i32.add (local.get $i) (i32.const 1)))
      (br $reverse_digits)))
  (call $conv_string (local.get $bytes)))

;; Return the parsed integer plus a success flag. No partial prefix, whitespace,
;; underscore, non-ASCII digit, or sign without digits is accepted.
(func $int_parse (param $string eqref) (param $base eqref) (result eqref i32)
  (local $bytes (ref $bytes)) (local $n i32) (local $i i32)
  (local $negative i32) (local $digit i32) (local $char i32)
  (local $radix i32) (local $value eqref)
  (block $invalid
    (br_if $invalid
      (i32.or
        (i32.lt_s (call $int_compare (local.get $base) (ref.i31 (i32.const 2))) (i32.const 0))
        (i32.gt_s (call $int_compare (local.get $base) (ref.i31 (i32.const 36))) (i32.const 0))))
    (local.set $radix (i32.wrap_i64 (call $int_to_i64 (local.get $base))))
    (local.set $bytes
      (ref.as_non_null (struct.get $obj 4 (ref.cast (ref $obj) (local.get $string)))))
    (local.set $n (array.len (local.get $bytes)))
    (br_if $invalid (i32.eqz (local.get $n)))
    (local.set $char (array.get_u $bytes (local.get $bytes) (i32.const 0)))
    (if (i32.or (i32.eq (local.get $char) (i32.const 43)) (i32.eq (local.get $char) (i32.const 45)))
      (then
        (local.set $negative (i32.eq (local.get $char) (i32.const 45)))
        (local.set $i (i32.const 1))))
    (br_if $invalid (i32.eq (local.get $i) (local.get $n)))
    (local.set $value (ref.i31 (i32.const 0)))
    (block $parsed
      (loop $parse_digit
        (br_if $parsed (i32.ge_u (local.get $i) (local.get $n)))
        (local.set $char (array.get_u $bytes (local.get $bytes) (local.get $i)))
        (local.set $digit
          (if (result i32)
            (i32.and (i32.ge_u (local.get $char) (i32.const 48)) (i32.le_u (local.get $char) (i32.const 57)))
            (then (i32.sub (local.get $char) (i32.const 48)))
            (else
              (if (result i32)
                (i32.and (i32.ge_u (local.get $char) (i32.const 65)) (i32.le_u (local.get $char) (i32.const 90)))
                (then (i32.sub (local.get $char) (i32.const 55)))
                (else
                  (if (result i32)
                    (i32.and (i32.ge_u (local.get $char) (i32.const 97)) (i32.le_u (local.get $char) (i32.const 122)))
                    (then (i32.sub (local.get $char) (i32.const 87)))
                    (else (i32.const -1))))))))
        (br_if $invalid (i32.ge_u (local.get $digit) (local.get $radix)))
        (local.set $value
          (call $int_add
            (call $int_mul (local.get $value) (local.get $base))
            (ref.i31 (local.get $digit))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $parse_digit)))
    (if (local.get $negative)
      (then (local.set $value (call $int_neg (local.get $value)))))
    (return (local.get $value) (i32.const 1)))
  (ref.i31 (i32.const 0)) (i32.const 0))

  (func $bits_new (param $data (ref $bytes)) (param $offset i32) (param $length i64) (result eqref)
    (struct.new $obj
      (i32.const 9) (local.get $offset) (local.get $length) (f64.const 0)
      (local.get $data) (ref.null $values) (ref.null $limbs) (ref.null $call)))

  (func $bits_data (param $value eqref) (result (ref $bytes))
    (ref.as_non_null (struct.get $obj 4 (ref.cast (ref $obj) (local.get $value)))))

  (func $bits_len64 (param $value eqref) (result i64)
    (struct.get $obj 2 (ref.cast (ref $obj) (local.get $value))))

  (func $bits_offset (param $value eqref) (result i64)
    (i64.extend_i32_u (struct.get $obj 1 (ref.cast (ref $obj) (local.get $value)))))

  (func $bits_has (param $value eqref) (param $start i64) (param $length i64) (result i32)
    (local $total i64)
    (local.set $total (call $bits_len64 (local.get $value)))
    (if (i64.lt_s (local.get $start) (i64.const 0)) (then (return (i32.const 0))))
    (if (i64.lt_s (local.get $length) (i64.const 0)) (then (return (i32.const 0))))
    (if (i64.gt_u (local.get $start) (local.get $total)) (then (return (i32.const 0))))
    (i64.le_u (local.get $length) (i64.sub (local.get $total) (local.get $start))))

  (func $bits_alloc (param $length i64) (result (ref $bytes))
    (local $bytes i64)
    (if (i64.lt_s (local.get $length) (i64.const 0)) (then unreachable))
    (local.set $bytes (i64.div_u (local.get $length) (i64.const 8)))
    (if (i64.ne (i64.and (local.get $length) (i64.const 7)) (i64.const 0))
      (then (local.set $bytes (i64.add (local.get $bytes) (i64.const 1)))))
    (if (i64.gt_u (local.get $bytes) (i64.const 4294967295)) (then unreachable))
    (array.new_default $bytes (i32.wrap_i64 (local.get $bytes))))

  (func $bits_empty (result eqref)
    (call $bits_new (array.new_default $bytes (i32.const 0)) (i32.const 0) (i64.const 0)))

  (func $bits_size (param $value eqref) (result eqref)
    (call $int_from_i64 (call $bits_len64 (local.get $value))))

  ;; A negative sentinel makes invalid dynamic pattern sizes ordinary match
  ;; failures (spec §8.4). Construction uses $bits_length instead, which stops
  ;; for a negative size. clamp_negative is kept for older callers.
  (func $bits_width (param $size eqref) (param $unit i64) (param $clamp_negative i32) (result i64)
    (local $limbs (ref null $limbs)) (local $count i32) (local $value i64)
    (if (i32.lt_s (call $int_sign (local.get $size)) (i32.const 0))
      (then (return (if (result i64) (local.get $clamp_negative)
        (then (i64.const 0)) (else (i64.const -1))))))
    (if (i64.le_s (local.get $unit) (i64.const 0))
      (then (return (i64.const -1))))
    (if (ref.test (ref i31) (local.get $size))
      (then (local.set $value (i64.extend_i32_s (i31.get_s (ref.cast (ref i31) (local.get $size))))))
      (else
        (local.set $limbs (call $int_limbs (local.get $size)))
        (local.set $count (array.len (local.get $limbs)))
        (if (i32.gt_u (local.get $count) (i32.const 2))
          (then (return (i64.const -1))))
        (if (i32.eq (local.get $count) (i32.const 2)) (then
          (if (i32.lt_s (array.get $limbs (local.get $limbs) (i32.const 1)) (i32.const 0))
            (then (return (i64.const -1))))))
        (local.set $value (call $int_to_i64 (local.get $size)))))
    (if (i64.gt_u (local.get $value) (i64.div_u (i64.const 9223372036854775807) (local.get $unit)))
      (then (return (i64.const -1))))
    (i64.mul (local.get $value) (local.get $unit)))

  (func $bits_get (param $value eqref) (param $index i64) (result i32)
    (local $absolute i64)
    (if (i32.eqz (call $bits_has (local.get $value) (local.get $index) (i64.const 1)))
      (then unreachable))
    (local.set $absolute (i64.add (call $bits_offset (local.get $value)) (local.get $index)))
    (i32.and
      (i32.shr_u
        (array.get_u $bytes (call $bits_data (local.get $value))
          (i32.wrap_i64 (i64.shr_u (local.get $absolute) (i64.const 3))))
        (i32.sub (i32.const 7) (i32.wrap_i64 (i64.and (local.get $absolute) (i64.const 7)))))
      (i32.const 1)))

  (func $bits_set (param $data (ref $bytes)) (param $index i64) (param $value i32)
    (local $byte i32) (local $shift i32) (local $old i32)
    (local.set $byte (i32.wrap_i64 (i64.shr_u (local.get $index) (i64.const 3))))
    (local.set $shift (i32.sub (i32.const 7) (i32.wrap_i64 (i64.and (local.get $index) (i64.const 7)))))
    (local.set $old (array.get_u $bytes (local.get $data) (local.get $byte)))
    (array.set $bytes (local.get $data) (local.get $byte)
      (i32.or
        (i32.and (local.get $old) (i32.xor (i32.shl (i32.const 1) (local.get $shift)) (i32.const -1)))
        (i32.shl (i32.and (local.get $value) (i32.const 1)) (local.get $shift)))))

  (func $bits_slice (param $value eqref) (param $start i64) (param $length i64) (result eqref)
    (local $absolute i64) (local $data (ref null $bytes)) (local $i i64)
    (if (i32.eqz (call $bits_has (local.get $value) (local.get $start) (local.get $length)))
      (then unreachable))
    (local.set $absolute (i64.add (call $bits_offset (local.get $value)) (local.get $start)))
    (if (i64.le_u (local.get $absolute) (i64.const 4294967295))
      (then (return (call $bits_new (call $bits_data (local.get $value))
        (i32.wrap_i64 (local.get $absolute)) (local.get $length)))))
    ;; Very large backing arrays can have a view offset too large for the
    ;; object's u32 field. Normalize such a view rather than truncating it.
    (local.set $data (call $bits_alloc (local.get $length)))
    (block $done (loop $copy
      (br_if $done (i64.ge_u (local.get $i) (local.get $length)))
      (call $bits_set (ref.as_non_null (local.get $data)) (local.get $i)
        (call $bits_get (local.get $value) (i64.add (local.get $start) (local.get $i))))
      (local.set $i (i64.add (local.get $i) (i64.const 1)))
      (br $copy)))
    (call $bits_new (ref.as_non_null (local.get $data)) (i32.const 0) (local.get $length)))

  (func $bits_concat (param $left eqref) (param $right eqref) (result eqref)
    (local $a i64) (local $b i64) (local $length i64)
    (local $data (ref null $bytes)) (local $i i64)
    (local.set $a (call $bits_len64 (local.get $left)))
    (local.set $b (call $bits_len64 (local.get $right)))
    (if (i64.eqz (local.get $a)) (then (return (local.get $right))))
    (if (i64.eqz (local.get $b)) (then (return (local.get $left))))
    (local.set $length (i64.add (local.get $a) (local.get $b)))
    (if (i64.lt_u (local.get $length) (local.get $a)) (then unreachable))
    (local.set $data (call $bits_alloc (local.get $length)))
    (if (i64.eqz (i64.and
      (i64.or (i64.or (local.get $a) (local.get $b))
        (i64.or (call $bits_offset (local.get $left)) (call $bits_offset (local.get $right))))
      (i64.const 7)))
      (then
        (array.copy $bytes $bytes (local.get $data) (i32.const 0)
          (call $bits_data (local.get $left))
          (i32.wrap_i64 (i64.shr_u (call $bits_offset (local.get $left)) (i64.const 3)))
          (i32.wrap_i64 (i64.shr_u (local.get $a) (i64.const 3))))
        (array.copy $bytes $bytes (local.get $data)
          (i32.wrap_i64 (i64.shr_u (local.get $a) (i64.const 3)))
          (call $bits_data (local.get $right))
          (i32.wrap_i64 (i64.shr_u (call $bits_offset (local.get $right)) (i64.const 3)))
          (i32.wrap_i64 (i64.shr_u (local.get $b) (i64.const 3)))))
      (else
        (block $left_done (loop $copy_left
          (br_if $left_done (i64.ge_u (local.get $i) (local.get $a)))
          (call $bits_set (ref.as_non_null (local.get $data)) (local.get $i)
            (call $bits_get (local.get $left) (local.get $i)))
          (local.set $i (i64.add (local.get $i) (i64.const 1)))
          (br $copy_left)))
        (local.set $i (i64.const 0))
        (block $right_done (loop $copy_right
          (br_if $right_done (i64.ge_u (local.get $i) (local.get $b)))
          (call $bits_set (ref.as_non_null (local.get $data)) (i64.add (local.get $a) (local.get $i))
            (call $bits_get (local.get $right) (local.get $i)))
          (local.set $i (i64.add (local.get $i) (i64.const 1)))
          (br $copy_right)))))
    (call $bits_new (ref.as_non_null (local.get $data)) (i32.const 0) (local.get $length)))

  (func $bits_match_equal (param $left eqref) (param $right eqref) (result i32)
    (local $length i64) (local $i i64) (local $count i64)
    (local $left_byte i32) (local $right_byte i32)
    (local $left_data (ref null $bytes)) (local $right_data (ref null $bytes))
    (local.set $length (call $bits_len64 (local.get $left)))
    (if (i64.ne (local.get $length) (call $bits_len64 (local.get $right)))
      (then (return (i32.const 0))))
    (if (i64.eqz (i64.and
      (i64.or (call $bits_offset (local.get $left)) (call $bits_offset (local.get $right)))
      (i64.const 7))) (then
      (local.set $left_data (call $bits_data (local.get $left)))
      (local.set $right_data (call $bits_data (local.get $right)))
      (local.set $left_byte (i32.wrap_i64 (i64.shr_u (call $bits_offset (local.get $left)) (i64.const 3))))
      (local.set $right_byte (i32.wrap_i64 (i64.shr_u (call $bits_offset (local.get $right)) (i64.const 3))))
      (local.set $count (i64.shr_u (local.get $length) (i64.const 3)))
      (block $bytes_done (loop $compare_bytes
        (br_if $bytes_done (i64.ge_u (local.get $i) (local.get $count)))
        (if (i32.ne
          (array.get_u $bytes (local.get $left_data) (i32.add (local.get $left_byte) (i32.wrap_i64 (local.get $i))))
          (array.get_u $bytes (local.get $right_data) (i32.add (local.get $right_byte) (i32.wrap_i64 (local.get $i)))))
          (then (return (i32.const 0))))
        (local.set $i (i64.add (local.get $i) (i64.const 1)))
        (br $compare_bytes)))
      (local.set $i (i64.shl (local.get $i) (i64.const 3)))))
    (block $done (loop $compare
      (br_if $done (i64.ge_u (local.get $i) (local.get $length)))
      (if (i32.ne (call $bits_get (local.get $left) (local.get $i))
                   (call $bits_get (local.get $right) (local.get $i)))
        (then (return (i32.const 0))))
      (local.set $i (i64.add (local.get $i) (i64.const 1)))
      (br $compare)))
    (i32.const 1))

  ;; Which integer bit is stored at a segment's stream position? Little
  ;; endian reverses bytes, retaining MSB-first order in each (partial) byte.
  (func $bits_value_position (param $index i64) (param $width i64) (param $little i32) (result i64)
    (local $base i64) (local $group i64)
    (if (i32.eqz (local.get $little))
      (then (return (i64.sub (i64.sub (local.get $width) (i64.const 1)) (local.get $index)))))
    (local.set $base (i64.and (local.get $index) (i64.const -8)))
    (local.set $group (i64.sub (local.get $width) (local.get $base)))
    (if (i64.gt_u (local.get $group) (i64.const 8)) (then (local.set $group (i64.const 8))))
    (i64.sub (i64.sub (i64.add (local.get $base) (local.get $group)) (i64.const 1))
      (i64.sub (local.get $index) (local.get $base))))

  (func $bits_magnitude_bit (param $limbs (ref $limbs)) (param $position i64) (result i32)
    (local $limb i64)
    (local.set $limb (i64.shr_u (local.get $position) (i64.const 5)))
    (if (i64.ge_u (local.get $limb) (i64.extend_i32_u (array.len (local.get $limbs))))
      (then (return (i32.const 0))))
    (i32.and
      (i32.shr_u (array.get $limbs (local.get $limbs) (i32.wrap_i64 (local.get $limb)))
        (i32.wrap_i64 (i64.and (local.get $position) (i64.const 31))))
      (i32.const 1)))

  (func $bits_from_int (param $value eqref) (param $width i64) (param $little i32) (result eqref)
    (local $data (ref null $bytes)) (local $limbs (ref null $limbs))
    (local $negative i32) (local $trail i64) (local $limb i32)
    (local $i i64) (local $position i64) (local $bit i32)
    (local.set $data (call $bits_alloc (local.get $width)))
    (local.set $limbs (call $int_limbs (local.get $value)))
    (local.set $negative (i32.lt_s (call $int_sign (local.get $value)) (i32.const 0)))
    (if (local.get $negative) (then
      (block $found (loop $find
        (if (i32.ne (array.get $limbs (local.get $limbs) (local.get $limb)) (i32.const 0))
          (then (br $found)))
        (local.set $limb (i32.add (local.get $limb) (i32.const 1)))
        (br $find)))
      (local.set $trail
        (i64.add (i64.mul (i64.extend_i32_u (local.get $limb)) (i64.const 32))
          (i64.extend_i32_u (i32.ctz (array.get $limbs (local.get $limbs) (local.get $limb))))))))
    (block $done (loop $write
      (br_if $done (i64.ge_u (local.get $i) (local.get $width)))
      (local.set $position (call $bits_value_position (local.get $i) (local.get $width) (local.get $little)))
      (local.set $bit (call $bits_magnitude_bit (ref.as_non_null (local.get $limbs)) (local.get $position)))
      (if (local.get $negative) (then
        (if (i64.gt_u (local.get $position) (local.get $trail))
          (then (local.set $bit (i32.xor (local.get $bit) (i32.const 1)))))))
      (call $bits_set (ref.as_non_null (local.get $data)) (local.get $i) (local.get $bit))
      (local.set $i (i64.add (local.get $i) (i64.const 1)))
      (br $write)))
    (call $bits_new (ref.as_non_null (local.get $data)) (i32.const 0) (local.get $width)))

  (func $bits_read_int (param $value eqref) (param $start i64) (param $width i64)
      (param $signed i32) (param $little i32) (result eqref)
    (local $limbs (ref null $limbs)) (local $count i64) (local $index i32)
    (local $i i64) (local $position i64) (local $negative i32)
    (local $carry i64) (local $word i64) (local $remainder i32)
    (if (i32.eqz (call $bits_has (local.get $value) (local.get $start) (local.get $width)))
      (then unreachable))
    (local.set $count (i64.div_u (i64.add (local.get $width) (i64.const 31)) (i64.const 32)))
    (if (i64.gt_u (local.get $count) (i64.const 4294967295)) (then unreachable))
    (local.set $limbs (array.new_default $limbs (i32.wrap_i64 (local.get $count))))
    (block $done (loop $read
      (br_if $done (i64.ge_u (local.get $i) (local.get $width)))
      (local.set $position (call $bits_value_position (local.get $i) (local.get $width) (local.get $little)))
      (if (call $bits_get (local.get $value) (i64.add (local.get $start) (local.get $i))) (then
        (local.set $index (i32.wrap_i64 (i64.shr_u (local.get $position) (i64.const 5))))
        (array.set $limbs (local.get $limbs) (local.get $index)
          (i32.or (array.get $limbs (local.get $limbs) (local.get $index))
            (i32.shl (i32.const 1) (i32.wrap_i64 (i64.and (local.get $position) (i64.const 31))))))))
      (local.set $i (i64.add (local.get $i) (i64.const 1)))
      (br $read)))
    (if (i32.and (local.get $signed) (i64.ne (local.get $width) (i64.const 0)))
      (then (local.set $negative (call $bits_magnitude_bit (ref.as_non_null (local.get $limbs))
        (i64.sub (local.get $width) (i64.const 1))))))
    (if (local.get $negative) (then
      (local.set $index (i32.const 0))
      (local.set $carry (i64.const 1))
      (local.set $remainder (i32.wrap_i64 (i64.and (local.get $width) (i64.const 31))))
      (block $neg_done (loop $negate
        (br_if $neg_done (i32.ge_u (local.get $index) (array.len (local.get $limbs))))
        (local.set $word (i64.extend_i32_u
          (i32.xor (array.get $limbs (local.get $limbs) (local.get $index)) (i32.const -1))))
        (if (i32.and
          (i32.eq (local.get $index) (i32.sub (array.len (local.get $limbs)) (i32.const 1)))
          (i32.ne (local.get $remainder) (i32.const 0)))
          (then (local.set $word (i64.and (local.get $word)
            (i64.sub (i64.shl (i64.const 1) (i64.extend_i32_u (local.get $remainder))) (i64.const 1))))))
        (local.set $word (i64.add (local.get $word) (local.get $carry)))
        (array.set $limbs (local.get $limbs) (local.get $index) (i32.wrap_i64 (local.get $word)))
        (local.set $carry (i64.shr_u (local.get $word) (i64.const 32)))
        (local.set $index (i32.add (local.get $index) (i32.const 1)))
        (br $negate)))))
    (call $int_pack (if (result i32) (local.get $negative) (then (i32.const -1)) (else (i32.const 1)))
      (ref.as_non_null (local.get $limbs))))

  (func $bits_from_u64 (param $value i64) (param $width i64) (param $little i32) (result eqref)
    (local $data (ref null $bytes)) (local $i i64) (local $position i64)
    (if (i64.gt_u (local.get $width) (i64.const 64)) (then unreachable))
    (local.set $data (call $bits_alloc (local.get $width)))
    (block $done (loop $write
      (br_if $done (i64.ge_u (local.get $i) (local.get $width)))
      (local.set $position (call $bits_value_position (local.get $i) (local.get $width) (local.get $little)))
      (call $bits_set (ref.as_non_null (local.get $data)) (local.get $i)
        (i32.wrap_i64 (i64.and (i64.shr_u (local.get $value) (local.get $position)) (i64.const 1))))
      (local.set $i (i64.add (local.get $i) (i64.const 1)))
      (br $write)))
    (call $bits_new (ref.as_non_null (local.get $data)) (i32.const 0) (local.get $width)))

  (func $bits_read_u64 (param $value eqref) (param $start i64) (param $width i64)
      (param $little i32) (result i64)
    (local $result i64) (local $i i64) (local $position i64)
    (if (i32.or (i64.gt_u (local.get $width) (i64.const 64))
      (i32.eqz (call $bits_has (local.get $value) (local.get $start) (local.get $width))))
      (then unreachable))
    (block $done (loop $read
      (br_if $done (i64.ge_u (local.get $i) (local.get $width)))
      (local.set $position (call $bits_value_position (local.get $i) (local.get $width) (local.get $little)))
      (local.set $result (i64.or (local.get $result)
        (i64.shl (i64.extend_i32_u (call $bits_get (local.get $value)
          (i64.add (local.get $start) (local.get $i)))) (local.get $position))))
      (local.set $i (i64.add (local.get $i) (i64.const 1)))
      (br $read)))
    (local.get $result))

  (func $bits_round_shift (param $value i64) (param $shift i64) (result i64)
    (local $truncated i64) (local $remainder i64) (local $half i64)
    (local.set $truncated (i64.shr_u (local.get $value) (local.get $shift)))
    (local.set $remainder (i64.and (local.get $value)
      (i64.sub (i64.shl (i64.const 1) (local.get $shift)) (i64.const 1))))
    (local.set $half (i64.shl (i64.const 1) (i64.sub (local.get $shift) (i64.const 1))))
    (if (i32.or (i64.gt_u (local.get $remainder) (local.get $half))
      (i32.and (i64.eq (local.get $remainder) (local.get $half))
        (i64.ne (i64.and (local.get $truncated) (i64.const 1)) (i64.const 0))))
      (then (local.set $truncated (i64.add (local.get $truncated) (i64.const 1)))))
    (local.get $truncated))

  ;; Direct f64-to-half rounding avoids the double rounding of f64 -> f32 -> f16.
  (func $bits_half_encode (param $value f64) (result i64)
    (local $raw i64) (local $sign i64) (local $exponent i32)
    (local $significand i64) (local $rounded i64)
    (local.set $raw (i64.reinterpret_f64 (local.get $value)))
    (local.set $sign (i64.and (i64.shr_u (local.get $raw) (i64.const 48)) (i64.const 32768)))
    (local.set $exponent (i32.sub
      (i32.wrap_i64 (i64.and (i64.shr_u (local.get $raw) (i64.const 52)) (i64.const 2047)))
      (i32.const 1023)))
    (if (i32.gt_s (local.get $exponent) (i32.const 15))
      (then (return (i64.or (local.get $sign) (i64.const 31744)))))
    (if (i32.lt_s (local.get $exponent) (i32.const -25))
      (then (return (local.get $sign))))
    (local.set $significand (i64.or (i64.and (local.get $raw) (i64.const 4503599627370495))
      (i64.const 4503599627370496)))
    (if (i32.lt_s (local.get $exponent) (i32.const -14)) (then
      (return (i64.or (local.get $sign) (call $bits_round_shift (local.get $significand)
        (i64.extend_i32_u (i32.sub (i32.const 28) (local.get $exponent))))))))
    (local.set $rounded (call $bits_round_shift (local.get $significand) (i64.const 42)))
    (if (i64.eq (local.get $rounded) (i64.const 2048)) (then
      (local.set $rounded (i64.const 1024))
      (local.set $exponent (i32.add (local.get $exponent) (i32.const 1)))))
    (if (i32.gt_s (local.get $exponent) (i32.const 15))
      (then (return (i64.or (local.get $sign) (i64.const 31744)))))
    (i64.or (local.get $sign)
      (i64.or (i64.shl (i64.extend_i32_u (i32.add (local.get $exponent) (i32.const 15))) (i64.const 10))
        (i64.and (local.get $rounded) (i64.const 1023)))))

  (func $bits_half_decode (param $raw i64) (result f64)
    (local $exponent i64) (local $fraction i64) (local $sign i64)
    (local.set $exponent (i64.and (i64.shr_u (local.get $raw) (i64.const 10)) (i64.const 31)))
    (local.set $fraction (i64.and (local.get $raw) (i64.const 1023)))
    (local.set $sign (i64.shl (i64.and (local.get $raw) (i64.const 32768)) (i64.const 48)))
    (if (i64.eqz (local.get $exponent)) (then
      (return (f64.copysign
        (f64.mul (f64.convert_i64_u (local.get $fraction)) (f64.const 0x1p-24))
        (f64.reinterpret_i64 (local.get $sign))))))
    (if (i64.eq (local.get $exponent) (i64.const 31)) (then
      (return (f64.reinterpret_i64 (i64.or (local.get $sign)
        (i64.or (i64.const 9218868437227405312) (i64.shl (local.get $fraction) (i64.const 42))))))))
    (f64.reinterpret_i64 (i64.or (local.get $sign)
      (i64.or (i64.shl (i64.add (local.get $exponent) (i64.const 1008)) (i64.const 52))
        (i64.shl (local.get $fraction) (i64.const 42))))))

  (func $bits_from_float (param $value eqref) (param $width i64) (param $little i32) (result eqref)
    (local $number f64) (local $raw i64)
    (local.set $number (call $float_get (local.get $value)))
    (if (i64.eq (local.get $width) (i64.const 16))
      (then (local.set $raw (call $bits_half_encode (local.get $number))))
      (else (if (i64.eq (local.get $width) (i64.const 32))
        (then (local.set $raw (i64.extend_i32_u (i32.reinterpret_f32 (f32.demote_f64 (local.get $number))))))
        (else (if (i64.eq (local.get $width) (i64.const 64))
          (then (local.set $raw (i64.reinterpret_f64 (local.get $number))))
          (else unreachable))))))
    ;; Rounding to a smaller format may overflow to infinity (spec §8.3).
    (if (i32.or
          (i32.and (i64.eq (local.get $width) (i64.const 16))
            (i64.eq (i64.and (local.get $raw) (i64.const 31744)) (i64.const 31744)))
          (i32.and (i64.eq (local.get $width) (i64.const 32))
            (i64.eq (i64.and (local.get $raw) (i64.const 2139095040)) (i64.const 2139095040))))
      (then (call $stop_bits_float) unreachable))
    (call $bits_from_u64 (local.get $raw) (local.get $width) (local.get $little)))

  ;; The length of a segment `u(n)`, `bits(n)` or `bytes(n)` in bits, for
  ;; construction: n times unit. A negative n stops (spec §8.3); a length
  ;; that no bit array can have traps as out of memory.
  (func $bits_length (param $size eqref) (param $unit i64) (result i64)
    (if (i32.lt_s (call $int_sign (local.get $size)) (i32.const 0))
      (then (call $stop_bits_negative) unreachable))
    (if (i32.gt_s (call $int_compare (local.get $size)
          (call $int_from_i64 (i64.div_u (i64.const 34359738360) (local.get $unit)))) (i32.const 0))
      (then unreachable))
    (i64.mul (call $int_to_i64 (local.get $size)) (local.get $unit)))

  ;; The length of a `bits(n)` or `bytes(n)` segment in bits: n times unit.
  ;; A negative n stops (spec §8.3). A length that no bit array can have is
  ;; 2^62, so that `$bits_take` stops because the value is too short.
  (func $bits_take_length (param $size eqref) (param $unit i64) (result i64)
    (if (i32.lt_s (call $int_sign (local.get $size)) (i32.const 0))
      (then (call $stop_bits_negative) unreachable))
    (if (i32.gt_s (call $int_compare (local.get $size)
          (call $int_from_i64 (i64.div_u (i64.const 4611686018427387904) (local.get $unit)))) (i32.const 0))
      (then (return (i64.const 4611686018427387904))))
    (i64.mul (call $int_to_i64 (local.get $size)) (local.get $unit)))

  ;; The first `length` bits of a `bits(n)` or `bytes(n)` segment value.
  (func $bits_take (param $value eqref) (param $length i64) (result eqref)
    (if (i64.lt_u (call $bits_len64 (local.get $value)) (local.get $length))
      (then (call $stop_bits_short) unreachable))
    (call $bits_slice (local.get $value) (i64.const 0) (local.get $length)))

  ;; The value of a `bytes` segment, which must be whole bytes.
  (func $bits_whole (param $value eqref) (result eqref)
    (if (i64.ne (i64.and (call $bits_len64 (local.get $value)) (i64.const 7)) (i64.const 0))
      (then (call $stop_bits_not_whole) unreachable))
    (local.get $value))

  (func $bits_float_finite (param $value eqref) (param $start i64) (param $width i64)
      (param $little i32) (result i32)
    (local $raw i64)
    (if (i32.eqz (call $bits_has (local.get $value) (local.get $start) (local.get $width)))
      (then (return (i32.const 0))))
    (if (i32.eqz (i32.or (i64.eq (local.get $width) (i64.const 16))
      (i32.or (i64.eq (local.get $width) (i64.const 32)) (i64.eq (local.get $width) (i64.const 64)))))
      (then (return (i32.const 0))))
    (local.set $raw (call $bits_read_u64 (local.get $value) (local.get $start) (local.get $width) (local.get $little)))
    (if (i64.eq (local.get $width) (i64.const 16))
      (then (return (i64.ne (i64.and (local.get $raw) (i64.const 31744)) (i64.const 31744)))))
    (if (i64.eq (local.get $width) (i64.const 32))
      (then (return (i64.ne (i64.and (local.get $raw) (i64.const 2139095040)) (i64.const 2139095040)))))
    (i64.ne (i64.and (local.get $raw) (i64.const 9218868437227405312)) (i64.const 9218868437227405312)))

  (func $bits_read_float (param $value eqref) (param $start i64) (param $width i64)
      (param $little i32) (result eqref)
    (local $raw i64)
    (if (i32.eqz (call $bits_float_finite (local.get $value) (local.get $start) (local.get $width) (local.get $little)))
      (then unreachable))
    (local.set $raw (call $bits_read_u64 (local.get $value) (local.get $start) (local.get $width) (local.get $little)))
    (if (i64.eq (local.get $width) (i64.const 16))
      (then (return (call $float_new (call $bits_half_decode (local.get $raw))))))
    (if (i64.eq (local.get $width) (i64.const 32))
      (then (return (call $float_new (f64.promote_f32 (f32.reinterpret_i32 (i32.wrap_i64 (local.get $raw))))))))
    (call $float_new (f64.reinterpret_i64 (local.get $raw))))

  (func $bits_from_string (param $value eqref) (result eqref)
    (local $data (ref null $bytes))
    (local.set $data (struct.get $obj 4 (ref.cast (ref $obj) (local.get $value))))
    (call $bits_new (ref.as_non_null (local.get $data)) (i32.const 0)
      (i64.mul (i64.extend_i32_u (array.len (local.get $data))) (i64.const 8))))

  (func $bits_byte (param $value eqref) (param $start i64) (result i32)
    (local $absolute i64)
    (if (i32.eqz (call $bits_has (local.get $value) (local.get $start) (i64.const 8)))
      (then unreachable))
    (local.set $absolute (i64.add (call $bits_offset (local.get $value)) (local.get $start)))
    (if (i64.eqz (i64.and (local.get $absolute) (i64.const 7))) (then
      (return (array.get_u $bytes (call $bits_data (local.get $value))
        (i32.wrap_i64 (i64.shr_u (local.get $absolute) (i64.const 3)))))))
    (i32.wrap_i64 (call $bits_read_u64 (local.get $value) (local.get $start) (i64.const 8) (i32.const 0))))

  (func $bits_utf8_size (param $value eqref) (param $start i64) (result i32)
    (local $first i32) (local $second i32) (local $byte i32) (local $count i32) (local $i i32)
    (if (i32.eqz (call $bits_has (local.get $value) (local.get $start) (i64.const 8)))
      (then (return (i32.const 0))))
    (local.set $first (call $bits_byte (local.get $value) (local.get $start)))
    (if (i32.lt_u (local.get $first) (i32.const 128)) (then (return (i32.const 1))))
    (if (i32.and (i32.ge_u (local.get $first) (i32.const 194)) (i32.le_u (local.get $first) (i32.const 223)))
      (then (local.set $count (i32.const 2)))
      (else (if (i32.and (i32.ge_u (local.get $first) (i32.const 224)) (i32.le_u (local.get $first) (i32.const 239)))
        (then (local.set $count (i32.const 3)))
        (else (if (i32.and (i32.ge_u (local.get $first) (i32.const 240)) (i32.le_u (local.get $first) (i32.const 244)))
          (then (local.set $count (i32.const 4)))
          (else (return (i32.const 0))))))))
    (if (i32.eqz (call $bits_has (local.get $value) (local.get $start)
      (i64.mul (i64.extend_i32_u (local.get $count)) (i64.const 8))))
      (then (return (i32.const 0))))
    (local.set $second (call $bits_byte (local.get $value) (i64.add (local.get $start) (i64.const 8))))
    (if (i32.or
      (i32.and (i32.eq (local.get $first) (i32.const 224)) (i32.lt_u (local.get $second) (i32.const 160)))
      (i32.or
        (i32.and (i32.eq (local.get $first) (i32.const 237)) (i32.ge_u (local.get $second) (i32.const 160)))
        (i32.or
          (i32.and (i32.eq (local.get $first) (i32.const 240)) (i32.lt_u (local.get $second) (i32.const 144)))
          (i32.and (i32.eq (local.get $first) (i32.const 244)) (i32.ge_u (local.get $second) (i32.const 144))))))
      (then (return (i32.const 0))))
    (local.set $i (i32.const 1))
    (block $done (loop $continuations
      (br_if $done (i32.ge_u (local.get $i) (local.get $count)))
      (local.set $byte (call $bits_byte (local.get $value)
        (i64.add (local.get $start) (i64.mul (i64.extend_i32_u (local.get $i)) (i64.const 8)))))
      (if (i32.ne (i32.and (local.get $byte) (i32.const 192)) (i32.const 128))
        (then (return (i32.const 0))))
      (local.set $i (i32.add (local.get $i) (i32.const 1)))
      (br $continuations)))
    (local.get $count))

  (func $bits_codepoint_new (param $codepoint i32) (result eqref)
    (call $codepoint_new (local.get $codepoint)))

  (func $bits_codepoint_get (param $value eqref) (result i32)
    (struct.get $obj 1 (ref.cast (ref $obj) (local.get $value))))

  (func $bits_read_utf8 (param $value eqref) (param $start i64) (result eqref)
    (local $count i32) (local $codepoint i32) (local $i i32) (local $mask i32)
    (local.set $count (call $bits_utf8_size (local.get $value) (local.get $start)))
    (if (i32.eqz (local.get $count)) (then unreachable))
    (local.set $mask (if (result i32) (i32.eq (local.get $count) (i32.const 1))
      (then (i32.const 127))
      (else (i32.sub (i32.shl (i32.const 1) (i32.sub (i32.const 7) (local.get $count))) (i32.const 1)))))
    (local.set $codepoint (i32.and (call $bits_byte (local.get $value) (local.get $start)) (local.get $mask)))
    (local.set $i (i32.const 1))
    (block $done (loop $read
      (br_if $done (i32.ge_u (local.get $i) (local.get $count)))
      (local.set $codepoint (i32.or (i32.shl (local.get $codepoint) (i32.const 6))
        (i32.and (call $bits_byte (local.get $value)
          (i64.add (local.get $start) (i64.mul (i64.extend_i32_u (local.get $i)) (i64.const 8)))) (i32.const 63))))
      (local.set $i (i32.add (local.get $i) (i32.const 1)))
      (br $read)))
    (call $bits_codepoint_new (local.get $codepoint)))

  (func $bits_utf16_size (param $value eqref) (param $start i64) (param $little i32) (result i32)
    (local $first i32) (local $second i32)
    (if (i32.eqz (call $bits_has (local.get $value) (local.get $start) (i64.const 16)))
      (then (return (i32.const 0))))
    (local.set $first (i32.wrap_i64 (call $bits_read_u64 (local.get $value) (local.get $start)
      (i64.const 16) (local.get $little))))
    (if (i32.and (i32.ge_u (local.get $first) (i32.const 55296)) (i32.le_u (local.get $first) (i32.const 56319)))
      (then
        (if (i32.eqz (call $bits_has (local.get $value) (local.get $start) (i64.const 32)))
          (then (return (i32.const 0))))
        (local.set $second (i32.wrap_i64 (call $bits_read_u64 (local.get $value)
          (i64.add (local.get $start) (i64.const 16)) (i64.const 16) (local.get $little))))
        (if (i32.and (i32.ge_u (local.get $second) (i32.const 56320)) (i32.le_u (local.get $second) (i32.const 57343)))
          (then (return (i32.const 4))) (else (return (i32.const 0))))))
    (if (i32.and (i32.ge_u (local.get $first) (i32.const 56320)) (i32.le_u (local.get $first) (i32.const 57343)))
      (then (return (i32.const 0))))
    (i32.const 2))

  (func $bits_read_utf16 (param $value eqref) (param $start i64) (param $little i32) (result eqref)
    (local $count i32) (local $codepoint i32) (local $second i32)
    (local.set $count (call $bits_utf16_size (local.get $value) (local.get $start) (local.get $little)))
    (if (i32.eqz (local.get $count)) (then unreachable))
    (local.set $codepoint (i32.wrap_i64 (call $bits_read_u64 (local.get $value) (local.get $start)
      (i64.const 16) (local.get $little))))
    (if (i32.eq (local.get $count) (i32.const 4)) (then
      (local.set $second (i32.wrap_i64 (call $bits_read_u64 (local.get $value)
        (i64.add (local.get $start) (i64.const 16)) (i64.const 16) (local.get $little))))
      (local.set $codepoint (i32.add (i32.const 65536)
        (i32.or (i32.shl (i32.sub (local.get $codepoint) (i32.const 55296)) (i32.const 10))
          (i32.sub (local.get $second) (i32.const 56320)))))))
    (call $bits_codepoint_new (local.get $codepoint)))

  (func $bits_valid_codepoint (param $codepoint i32) (result i32)
    (i32.and (i32.le_u (local.get $codepoint) (i32.const 1114111))
      (i32.eqz (i32.and (i32.ge_u (local.get $codepoint) (i32.const 55296))
        (i32.le_u (local.get $codepoint) (i32.const 57343))))))

  (func $bits_utf32_size (param $value eqref) (param $start i64) (param $little i32) (result i32)
    (if (i32.eqz (call $bits_has (local.get $value) (local.get $start) (i64.const 32)))
      (then (return (i32.const 0))))
    (if (result i32) (call $bits_valid_codepoint
      (i32.wrap_i64 (call $bits_read_u64 (local.get $value) (local.get $start) (i64.const 32) (local.get $little))))
      (then (i32.const 4)) (else (i32.const 0))))

  (func $bits_read_utf32 (param $value eqref) (param $start i64) (param $little i32) (result eqref)
    (if (i32.eqz (call $bits_utf32_size (local.get $value) (local.get $start) (local.get $little)))
      (then unreachable))
    (call $bits_codepoint_new (i32.wrap_i64 (call $bits_read_u64 (local.get $value) (local.get $start)
      (i64.const 32) (local.get $little)))))

  (func $bits_read_utf (param $value eqref) (param $start i64) (param $encoding i32) (param $little i32)
      (result eqref i64 i32)
    (local $count i32) (local $codepoint eqref)
    (if (i32.eq (local.get $encoding) (i32.const 8))
      (then
        (local.set $count (call $bits_utf8_size (local.get $value) (local.get $start)))
        (if (local.get $count) (then (local.set $codepoint (call $bits_read_utf8 (local.get $value) (local.get $start))))))
      (else (if (i32.eq (local.get $encoding) (i32.const 16))
        (then
          (local.set $count (call $bits_utf16_size (local.get $value) (local.get $start) (local.get $little)))
          (if (local.get $count) (then (local.set $codepoint (call $bits_read_utf16 (local.get $value) (local.get $start) (local.get $little))))))
        (else (if (i32.eq (local.get $encoding) (i32.const 32))
          (then
            (local.set $count (call $bits_utf32_size (local.get $value) (local.get $start) (local.get $little)))
            (if (local.get $count) (then (local.set $codepoint (call $bits_read_utf32 (local.get $value) (local.get $start) (local.get $little))))))
          (else unreachable))))))
    (local.get $codepoint)
    (i64.mul (i64.extend_i32_u (local.get $count)) (i64.const 8))
    (i32.ne (local.get $count) (i32.const 0)))

  (func $bits_put_word (param $data (ref $bytes)) (param $index i32) (param $word i32) (param $little i32)
    (array.set $bytes (local.get $data) (local.get $index)
      (if (result i32) (local.get $little) (then (local.get $word))
        (else (i32.shr_u (local.get $word) (i32.const 8)))))
    (array.set $bytes (local.get $data) (i32.add (local.get $index) (i32.const 1))
      (if (result i32) (local.get $little) (then (i32.shr_u (local.get $word) (i32.const 8)))
        (else (local.get $word)))))

  (func $bits_from_codepoint (param $value eqref) (param $encoding i32) (param $little i32) (result eqref)
    (local $codepoint i32) (local $count i32) (local $data (ref null $bytes))
    (local.set $codepoint (call $bits_codepoint_get (local.get $value)))
    (if (i32.eqz (call $bits_valid_codepoint (local.get $codepoint))) (then unreachable))
    (if (i32.eq (local.get $encoding) (i32.const 32))
      (then (return (call $bits_from_u64 (i64.extend_i32_u (local.get $codepoint)) (i64.const 32) (local.get $little)))))
    (if (i32.eq (local.get $encoding) (i32.const 16)) (then
      (local.set $count (if (result i32) (i32.lt_u (local.get $codepoint) (i32.const 65536))
        (then (i32.const 2)) (else (i32.const 4))))
      (local.set $data (array.new_default $bytes (local.get $count)))
      (if (i32.eq (local.get $count) (i32.const 2))
        (then (call $bits_put_word (ref.as_non_null (local.get $data)) (i32.const 0) (local.get $codepoint) (local.get $little)))
        (else
          (local.set $codepoint (i32.sub (local.get $codepoint) (i32.const 65536)))
          (call $bits_put_word (ref.as_non_null (local.get $data)) (i32.const 0)
            (i32.add (i32.const 55296) (i32.shr_u (local.get $codepoint) (i32.const 10))) (local.get $little))
          (call $bits_put_word (ref.as_non_null (local.get $data)) (i32.const 2)
            (i32.add (i32.const 56320) (i32.and (local.get $codepoint) (i32.const 1023))) (local.get $little))))
      (return (call $bits_new (ref.as_non_null (local.get $data)) (i32.const 0)
        (i64.mul (i64.extend_i32_u (local.get $count)) (i64.const 8))))))
    (if (i32.ne (local.get $encoding) (i32.const 8)) (then unreachable))
    (local.set $count
      (if (result i32) (i32.lt_u (local.get $codepoint) (i32.const 128)) (then (i32.const 1))
        (else (if (result i32) (i32.lt_u (local.get $codepoint) (i32.const 2048)) (then (i32.const 2))
          (else (if (result i32) (i32.lt_u (local.get $codepoint) (i32.const 65536))
            (then (i32.const 3)) (else (i32.const 4))))))))
    (local.set $data (array.new_default $bytes (local.get $count)))
    (if (i32.eq (local.get $count) (i32.const 1))
      (then (array.set $bytes (local.get $data) (i32.const 0) (local.get $codepoint)))
      (else
        (array.set $bytes (local.get $data) (i32.sub (local.get $count) (i32.const 1))
          (i32.or (i32.const 128) (i32.and (local.get $codepoint) (i32.const 63))))
        (if (i32.eq (local.get $count) (i32.const 2))
          (then (array.set $bytes (local.get $data) (i32.const 0)
            (i32.or (i32.const 192) (i32.shr_u (local.get $codepoint) (i32.const 6)))))
          (else
            (array.set $bytes (local.get $data) (i32.sub (local.get $count) (i32.const 2))
              (i32.or (i32.const 128) (i32.and (i32.shr_u (local.get $codepoint) (i32.const 6)) (i32.const 63))))
            (if (i32.eq (local.get $count) (i32.const 3))
              (then (array.set $bytes (local.get $data) (i32.const 0)
                (i32.or (i32.const 224) (i32.shr_u (local.get $codepoint) (i32.const 12)))))
              (else
                (array.set $bytes (local.get $data) (i32.const 1)
                  (i32.or (i32.const 128) (i32.and (i32.shr_u (local.get $codepoint) (i32.const 12)) (i32.const 63))))
                (array.set $bytes (local.get $data) (i32.const 0)
                  (i32.or (i32.const 240) (i32.shr_u (local.get $codepoint) (i32.const 18))))))))))
    (call $bits_new (ref.as_non_null (local.get $data)) (i32.const 0)
      (i64.mul (i64.extend_i32_u (local.get $count)) (i64.const 8))))

  (func $bits_from_wide_string (param $value eqref) (param $encoding i32) (param $little i32) (result eqref)
    (local $input eqref) (local $data (ref null $bytes)) (local $source i64)
    (local $destination i32) (local $count i32) (local $codepoint i32) (local $capacity i64)
    (local.set $input (call $bits_from_string (local.get $value)))
    (local.set $capacity (i64.mul (i64.div_u (call $bits_len64 (local.get $input)) (i64.const 8))
      (i64.extend_i32_u (i32.div_u (local.get $encoding) (i32.const 8)))))
    (if (i64.gt_u (local.get $capacity) (i64.const 4294967295)) (then unreachable))
    (local.set $data (array.new_default $bytes (i32.wrap_i64 (local.get $capacity))))
    (block $done (loop $encode
      (br_if $done (i64.ge_u (local.get $source) (call $bits_len64 (local.get $input))))
      (local.set $count (call $bits_utf8_size (local.get $input) (local.get $source)))
      (if (i32.eqz (local.get $count)) (then unreachable))
      (local.set $codepoint (call $bits_codepoint_get (call $bits_read_utf8 (local.get $input) (local.get $source))))
      (if (i32.eq (local.get $encoding) (i32.const 16))
        (then
          (if (i32.lt_u (local.get $codepoint) (i32.const 65536))
            (then
              (call $bits_put_word (ref.as_non_null (local.get $data)) (local.get $destination) (local.get $codepoint) (local.get $little))
              (local.set $destination (i32.add (local.get $destination) (i32.const 2))))
            (else
              (local.set $codepoint (i32.sub (local.get $codepoint) (i32.const 65536)))
              (call $bits_put_word (ref.as_non_null (local.get $data)) (local.get $destination)
                (i32.add (i32.const 55296) (i32.shr_u (local.get $codepoint) (i32.const 10))) (local.get $little))
              (call $bits_put_word (ref.as_non_null (local.get $data)) (i32.add (local.get $destination) (i32.const 2))
                (i32.add (i32.const 56320) (i32.and (local.get $codepoint) (i32.const 1023))) (local.get $little))
              (local.set $destination (i32.add (local.get $destination) (i32.const 4))))))
        (else
          (if (local.get $little)
            (then
              (call $bits_put_word (ref.as_non_null (local.get $data)) (local.get $destination) (local.get $codepoint) (i32.const 1))
              (call $bits_put_word (ref.as_non_null (local.get $data)) (i32.add (local.get $destination) (i32.const 2))
                (i32.shr_u (local.get $codepoint) (i32.const 16)) (i32.const 1)))
            (else
              (call $bits_put_word (ref.as_non_null (local.get $data)) (local.get $destination)
                (i32.shr_u (local.get $codepoint) (i32.const 16)) (i32.const 0))
              (call $bits_put_word (ref.as_non_null (local.get $data)) (i32.add (local.get $destination) (i32.const 2))
                (local.get $codepoint) (i32.const 0))))
          (local.set $destination (i32.add (local.get $destination) (i32.const 4)))))
      (local.set $source (i64.add (local.get $source) (i64.mul (i64.extend_i32_u (local.get $count)) (i64.const 8))))
      (br $encode)))
    (call $bits_new (ref.as_non_null (local.get $data)) (i32.const 0)
      (i64.mul (i64.extend_i32_u (local.get $destination)) (i64.const 8))))

  (func $bits_from_utf16_string (param $value eqref) (param $little i32) (result eqref)
    (call $bits_from_wide_string (local.get $value) (i32.const 16) (local.get $little)))

  (func $bits_from_utf32_string (param $value eqref) (param $little i32) (result eqref)
    (call $bits_from_wide_string (local.get $value) (i32.const 32) (local.get $little)))

  (func $bits_from_utf (param $value eqref) (param $encoding i32) (param $little i32) (param $codepoint i32) (result eqref)
    (if (local.get $codepoint) (then
      (return (call $bits_from_codepoint (local.get $value) (local.get $encoding) (local.get $little)))))
    (if (i32.eq (local.get $encoding) (i32.const 8))
      (then (return (call $bits_from_string (local.get $value)))))
    (if (i32.eqz (i32.or (i32.eq (local.get $encoding) (i32.const 16)) (i32.eq (local.get $encoding) (i32.const 32))))
      (then unreachable))
    (call $bits_from_wide_string (local.get $value) (local.get $encoding) (local.get $little)))

  (func $bits_to_string (param $value eqref) (result eqref)
    (local $length i64) (local $count i32) (local $data (ref null $bytes))
    (local $index i32) (local $position i64) (local $size i32)
    (local.set $length (call $bits_len64 (local.get $value)))
    (if (i64.ne (i64.and (local.get $length) (i64.const 7)) (i64.const 0)) (then unreachable))
    (local.set $count (i32.wrap_i64 (i64.shr_u (local.get $length) (i64.const 3))))
    (local.set $data (array.new_default $bytes (local.get $count)))
    (if (i64.eqz (i64.and (call $bits_offset (local.get $value)) (i64.const 7)))
      (then
        (array.copy $bytes $bytes (local.get $data) (i32.const 0)
          (call $bits_data (local.get $value))
          (i32.wrap_i64 (i64.shr_u (call $bits_offset (local.get $value)) (i64.const 3))) (local.get $count)))
      (else
        (block $copy_done (loop $copy
          (br_if $copy_done (i32.ge_u (local.get $index) (local.get $count)))
          (array.set $bytes (local.get $data) (local.get $index)
            (call $bits_byte (local.get $value) (i64.mul (i64.extend_i32_u (local.get $index)) (i64.const 8))))
          (local.set $index (i32.add (local.get $index) (i32.const 1)))
          (br $copy)))))
    (block $valid (loop $validate
      (br_if $valid (i64.ge_u (local.get $position) (local.get $length)))
      (local.set $size (call $bits_utf8_size (local.get $value) (local.get $position)))
      (if (i32.eqz (local.get $size)) (then unreachable))
      (local.set $position (i64.add (local.get $position) (i64.mul (i64.extend_i32_u (local.get $size)) (i64.const 8))))
      (br $validate)))
    (call $string_from_bytes (ref.as_non_null (local.get $data))))


(global $value_false eqref
  (struct.new $obj (i32.const 1) (i32.const 0) (i64.const 0) (f64.const 0)
    (ref.null $bytes) (ref.null $values) (ref.null $limbs) (ref.null $call)))
(global $value_true eqref
  (struct.new $obj (i32.const 1) (i32.const 1) (i64.const 0) (f64.const 0)
    (ref.null $bytes) (ref.null $values) (ref.null $limbs) (ref.null $call)))
(global $value_empty_list eqref
  (struct.new $obj (i32.const 6) (i32.const 0) (i64.const 0) (f64.const 0)
    (ref.null $bytes) (ref.null $values) (ref.null $limbs) (ref.null $call)))
(func $value_tag (param $v eqref) (result i32)
  (if (ref.is_null (local.get $v)) (then (return (i32.const 0))))
  (if (ref.test (ref i31) (local.get $v)) (then (return (i32.const 8))))
  (if (ref.test (ref $hostref) (local.get $v)) (then (return (i32.const 10))))
  (struct.get $val 0 (ref.cast (ref $val) (local.get $v))))
;; A host reference keeps its reference_id, which never changes (spec B.4).
(func $host_ref_new (param $r externref) (result eqref)
  (struct.new $hostref (local.get $r) (call $host_reference_id (local.get $r))))
(func $host_ref_get (param $v eqref) (result externref)
  (struct.get $hostref 0 (ref.cast (ref $hostref) (local.get $v))))
(func $host_id (param $v eqref) (result i64)
  (struct.get $hostref 1 (ref.cast (ref $hostref) (local.get $v))))
(func $codepoint_new (param $c i32) (result eqref)
  (struct.new $obj (i32.const 13) (local.get $c) (i64.const 0) (f64.const 0)
    (ref.null $bytes) (ref.null $values) (ref.null $limbs) (ref.null $call)))
(func $codepoint_get (param $v eqref) (result i32)
  (struct.get $obj 1 (ref.cast (ref $obj) (local.get $v))))
(func $bool_new (param $v i32) (result eqref)
  (if (result eqref) (local.get $v)
    (then (global.get $value_true)) (else (global.get $value_false))))
(func $bool_get (param $v eqref) (result i32)
  (struct.get $obj 1 (ref.cast (ref $obj) (local.get $v))))
(func $float_new (param $v f64) (result eqref)
  ;; NaN fails the comparison as well (spec §11.3).
  (if (i32.eqz (f64.le (f64.abs (local.get $v)) (f64.const 0x1.fffffffffffffp+1023)))
    (then (call $stop_float) unreachable))
  (struct.new $obj (i32.const 2) (i32.const 0) (i64.const 0) (local.get $v)
    (ref.null $bytes) (ref.null $values) (ref.null $limbs) (ref.null $call)))
(func $float_get (param $v eqref) (result f64)
  (struct.get $obj 3 (ref.cast (ref $obj) (local.get $v))))
(func $list_nil (result eqref)
  (global.get $value_empty_list))
(func $list_cons (param $head eqref) (param $tail eqref) (result eqref)
  (struct.new $cons (i32.const 6) (local.get $head) (local.get $tail)))
(func $field (param $v eqref) (param $index i32) (result eqref)
  (array.get $values
    (ref.as_non_null (struct.get $obj 5 (ref.cast (ref $obj) (local.get $v))))
    (local.get $index)))
(func $record (param $ctor i32) (param $type i64) (param $fields (ref $values)) (result eqref)
  (struct.new $obj (i32.const 5) (local.get $ctor) (local.get $type) (f64.const 0)
    (ref.null $bytes) (local.get $fields) (ref.null $limbs) (ref.null $call)))
(func $tuple (param $fields (ref $values)) (result eqref)
  (struct.new $obj (i32.const 4) (i32.const 0) (i64.const 0) (f64.const 0)
    (ref.null $bytes) (local.get $fields) (ref.null $limbs) (ref.null $call)))
(func $list_is_empty (param $v eqref) (result i32)
  (i32.eqz (ref.test (ref $cons) (local.get $v))))
(func $list_head (param $v eqref) (result eqref)
  (struct.get $cons 1 (ref.cast (ref $cons) (local.get $v))))
(func $list_tail (param $v eqref) (result eqref)
  (struct.get $cons 2 (ref.cast (ref $cons) (local.get $v))))
(func $list_length (param $v eqref) (result eqref) (local $n i64)
  (block $done (loop $each
    (br_if $done (call $list_is_empty (local.get $v)))
    (local.set $n (i64.add (local.get $n) (i64.const 1)))
    (local.set $v (call $list_tail (local.get $v))) (br $each)))
  (call $int_from_i64 (local.get $n)))
(func $list_reverse (param $v eqref) (result eqref) (local $out eqref)
  (local.set $out (call $list_nil))
  (block $done (loop $each
    (br_if $done (call $list_is_empty (local.get $v)))
    (local.set $out (call $list_cons (call $list_head (local.get $v)) (local.get $out)))
    (local.set $v (call $list_tail (local.get $v))) (br $each)))
  (local.get $out))
(func $value_bit (param $v eqref) (param $at i64) (result i32)
  (local $o (ref null $obj)) (local $p i64)
  (local.set $o (ref.cast (ref $obj) (local.get $v)))
  (local.set $p (i64.add (local.get $at) (i64.extend_i32_u (struct.get $obj 1 (local.get $o)))))
  (i32.and
    (i32.shr_u
      (array.get_u $bytes (struct.get $obj 4 (local.get $o))
        (i32.wrap_i64 (i64.shr_u (local.get $p) (i64.const 3))))
      (i32.sub (i32.const 7) (i32.wrap_i64 (i64.and (local.get $p) (i64.const 7)))))
    (i32.const 1)))
;; The siblings still to compare: the parts of two `$values` arrays from
;; index `i`, or, when the arrays are null, the tails of two lists.
(type $compare_frame (struct
  (field (ref null $compare_frame))
  (field (ref null $values)) (field (ref null $values)) (field (mut i32))
  (field eqref) (field eqref)))
(func $value_eq (param $a eqref) (param $b eqref) (result i32)
  (if (ref.eq (local.get $a) (local.get $b)) (then (return (i32.const 1))))
  ;; Two small Ints are equal only when they are the same `i31ref`.
  (if (call $both_small (local.get $a) (local.get $b)) (then (return (i32.const 0))))
  (i32.eqz (call $value_compare_i32 (local.get $a) (local.get $b))))
;; `==` (spec §11.4).
(func $equal (param $a eqref) (param $b eqref) (result i32)
  (call $value_eq (local.get $a) (local.get $b)))
;; Functions, boundaries and continuations are the same only as themselves.
(func $value_identity_kind (param $kind i32) (result i32)
  (i32.or (i32.eq (local.get $kind) (i32.const 7))
    (i32.or (i32.eq (local.get $kind) (i32.const 11)) (i32.eq (local.get $kind) (i32.const 12)))))
(func $compare_i32 (param $a i32) (param $b i32) (result i32)
  (i32.sub (i32.gt_s (local.get $a) (local.get $b)) (i32.lt_s (local.get $a) (local.get $b))))
(func $compare_i64 (param $a i64) (param $b i64) (result i32)
  (i32.sub (i64.gt_s (local.get $a) (local.get $b)) (i64.lt_s (local.get $a) (local.get $b))))
;; Two small Ints that are not the same are compared at once, without
;; a frame.
(func $both_small (param $a eqref) (param $b eqref) (result i32)
  (i32.and (ref.test (ref i31) (local.get $a)) (ref.test (ref i31) (local.get $b))))
(func $compare_small (param $a eqref) (param $b eqref) (result i32)
  (call $compare_i32 (i31.get_s (ref.cast (ref i31) (local.get $a)))
                     (i31.get_s (ref.cast (ref i31) (local.get $b)))))
(func $value_compare_i32 (param $a eqref) (param $b eqref) (result i32)
  (local $ka i32) (local $kb i32) (local $c i32)
  (local $oa (ref null $obj)) (local $ob (ref null $obj))
  (local $xa (ref null $values)) (local $xb (ref null $values))
  (local $pending (ref null $compare_frame))
  (local $ca (ref null $cons)) (local $cb (ref null $cons)) (local $ha eqref) (local $hb eqref)
  (local $i i32) (local $na i32) (local $nb i32) (local $bit i64) (local $ba i64) (local $bb i64)
  ;; A frame records siblings still to compare, never a native call frame.
  ;; Primitive values and single remaining children need no allocation.
  (loop $again
    (local.set $c (i32.const 0))
    (block $compared
      (br_if $compared (ref.eq (local.get $a) (local.get $b)))
      (local.set $ka (call $value_tag (local.get $a)))
      (local.set $kb (call $value_tag (local.get $b)))
      (if (i32.ne (local.get $ka) (local.get $kb))
        (then (return (call $compare_i32 (local.get $ka) (local.get $kb)))))
      (br_if $compared (i32.eqz (local.get $ka)))
      (if (i32.eq (local.get $ka) (i32.const 8)) (then
        (local.set $c (call $int_compare (local.get $a) (local.get $b)))
        (br $compared)))
      (if (i32.eq (local.get $ka) (i32.const 10)) (then
        (local.set $c (call $compare_i64 (call $host_id (local.get $a)) (call $host_id (local.get $b))))
        (br $compared)))
      ;; Identity values that are not ref.eq are different. The order between
      ;; them only has to be consistent within one comparison.
      (if (call $value_identity_kind (local.get $ka)) (then (return (i32.const 1))))
      ;; Lists: [] comes first; else the heads, then the tails. Equal heads
      ;; and small Ints need no frame.
      (if (i32.eq (local.get $ka) (i32.const 6)) (then
        (local.set $na (ref.test (ref $cons) (local.get $a)))
        (local.set $nb (ref.test (ref $cons) (local.get $b)))
        (if (i32.eqz (i32.and (local.get $na) (local.get $nb))) (then
          (local.set $c (call $compare_i32 (local.get $na) (local.get $nb)))
          (br $compared)))
        (local.set $ca (ref.cast (ref $cons) (local.get $a)))
        (local.set $cb (ref.cast (ref $cons) (local.get $b)))
        (local.set $ha (struct.get $cons 1 (local.get $ca)))
        (local.set $hb (struct.get $cons 1 (local.get $cb)))
        (local.set $a (struct.get $cons 2 (local.get $ca)))
        (local.set $b (struct.get $cons 2 (local.get $cb)))
        (br_if $again (ref.eq (local.get $ha) (local.get $hb)))
        (if (call $both_small (local.get $ha) (local.get $hb)) (then
          (return (call $compare_small (local.get $ha) (local.get $hb)))))
        (local.set $pending (struct.new $compare_frame (local.get $pending)
          (ref.null $values) (ref.null $values) (i32.const 0) (local.get $a) (local.get $b)))
        (local.set $a (local.get $ha))
        (local.set $b (local.get $hb))
        (br $again)))
      (local.set $oa (ref.cast (ref $obj) (local.get $a)))
      (local.set $ob (ref.cast (ref $obj) (local.get $b)))
      (if (i32.eq (local.get $ka) (i32.const 2)) (then
        (local.set $c (i32.sub
          (f64.gt (struct.get $obj 3 (local.get $oa)) (struct.get $obj 3 (local.get $ob)))
          (f64.lt (struct.get $obj 3 (local.get $oa)) (struct.get $obj 3 (local.get $ob)))))
        (br $compared)))
      (if (i32.eq (local.get $ka) (i32.const 3)) (then
        (local.set $c (call $string_compare (local.get $a) (local.get $b)))
        (br $compared)))
      (if (i32.eq (local.get $ka) (i32.const 9)) (then
        (local.set $ba (struct.get $obj 2 (local.get $oa)))
        (local.set $bb (struct.get $obj 2 (local.get $ob)))
        (local.set $bit (i64.const 0))
        (block $bits_done (loop $bits_each
          (br_if $bits_done (i32.or (i64.ge_u (local.get $bit) (local.get $ba))
                                   (i64.ge_u (local.get $bit) (local.get $bb))))
          (local.set $c (call $compare_i32 (call $value_bit (local.get $a) (local.get $bit))
                                         (call $value_bit (local.get $b) (local.get $bit))))
          (if (local.get $c) (then (return (local.get $c))))
          (local.set $bit (i64.add (local.get $bit) (i64.const 1))) (br $bits_each)))
        (local.set $c (call $compare_i64 (local.get $ba) (local.get $bb)))
        (br $compared)))
      (local.set $c (call $compare_i64 (struct.get $obj 2 (local.get $oa)) (struct.get $obj 2 (local.get $ob))))
      (if (local.get $c) (then (return (local.get $c))))
      (local.set $c (call $compare_i32 (struct.get $obj 1 (local.get $oa)) (struct.get $obj 1 (local.get $ob))))
      (if (local.get $c) (then (return (local.get $c))))
      (local.set $xa (struct.get $obj 5 (local.get $oa)))
      (local.set $xb (struct.get $obj 5 (local.get $ob)))
      (if (i32.or (ref.is_null (local.get $xa)) (ref.is_null (local.get $xb))) (then
        (local.set $c (i32.sub (ref.is_null (local.get $xb)) (ref.is_null (local.get $xa))))
        (br $compared)))
      (local.set $na (array.len (local.get $xa)))
      (local.set $nb (array.len (local.get $xb)))
      (local.set $i (i32.const 0))
      (block $first_found (loop $first
        (br_if $first_found (i32.or (i32.ge_u (local.get $i) (local.get $na))
                                   (i32.ge_u (local.get $i) (local.get $nb))))
        (br_if $first_found (i32.eqz (ref.eq
          (array.get $values (local.get $xa) (local.get $i))
          (array.get $values (local.get $xb) (local.get $i)))))
        (local.set $i (i32.add (local.get $i) (i32.const 1))) (br $first)))
      (if (i32.or (i32.ge_u (local.get $i) (local.get $na))
                  (i32.ge_u (local.get $i) (local.get $nb))) (then
        (local.set $c (call $compare_i32 (local.get $na) (local.get $nb)))
        (br $compared)))
      (local.set $a (array.get $values (local.get $xa) (local.get $i)))
      (local.set $b (array.get $values (local.get $xb) (local.get $i)))
      (if (call $both_small (local.get $a) (local.get $b)) (then
        (return (call $compare_small (local.get $a) (local.get $b)))))
      (local.set $i (i32.add (local.get $i) (i32.const 1)))
      (if (i32.or (i32.ne (local.get $na) (local.get $nb)) (i32.lt_u (local.get $i) (local.get $na)))
        (then (local.set $pending (struct.new $compare_frame (local.get $pending)
          (local.get $xa) (local.get $xb) (local.get $i) (ref.null eq) (ref.null eq)))))
      (br $again))
    (if (local.get $c) (then (return (local.get $c))))
    (loop $resume
      (if (ref.is_null (local.get $pending)) (then (return (i32.const 0))))
      (if (ref.is_null (struct.get $compare_frame 1 (local.get $pending))) (then
        (local.set $a (struct.get $compare_frame 4 (local.get $pending)))
        (local.set $b (struct.get $compare_frame 5 (local.get $pending)))
        (local.set $pending (struct.get $compare_frame 0 (local.get $pending)))
        (br $again)))
      (local.set $xa (struct.get $compare_frame 1 (local.get $pending)))
      (local.set $xb (struct.get $compare_frame 2 (local.get $pending)))
      (local.set $i (struct.get $compare_frame 3 (local.get $pending)))
      (local.set $na (array.len (local.get $xa)))
      (local.set $nb (array.len (local.get $xb)))
      (block $next_found (loop $next
        (br_if $next_found (i32.or (i32.ge_u (local.get $i) (local.get $na))
                                  (i32.ge_u (local.get $i) (local.get $nb))))
        (br_if $next_found (i32.eqz (ref.eq
          (array.get $values (local.get $xa) (local.get $i))
          (array.get $values (local.get $xb) (local.get $i)))))
        (local.set $i (i32.add (local.get $i) (i32.const 1))) (br $next)))
      (if (i32.or (i32.ge_u (local.get $i) (local.get $na))
                  (i32.ge_u (local.get $i) (local.get $nb))) (then
        (local.set $pending (struct.get $compare_frame 0 (local.get $pending)))
        (local.set $c (call $compare_i32 (local.get $na) (local.get $nb)))
        (if (local.get $c) (then (return (local.get $c))))
        (br $resume)))
      (local.set $a (array.get $values (local.get $xa) (local.get $i)))
      (local.set $b (array.get $values (local.get $xb) (local.get $i)))
      (if (call $both_small (local.get $a) (local.get $b)) (then
        (return (call $compare_small (local.get $a) (local.get $b)))))
      (local.set $i (i32.add (local.get $i) (i32.const 1)))
      (if (i32.and (i32.eq (local.get $na) (local.get $nb)) (i32.eq (local.get $i) (local.get $na)))
        (then (local.set $pending (struct.get $compare_frame 0 (local.get $pending))))
        (else (struct.set $compare_frame 3 (local.get $pending) (local.get $i))))
      (br $again)))
  unreachable)
(func $value_compare (param $a eqref) (param $b eqref) (result eqref)
  (ref.i31 (call $value_compare_i32 (local.get $a) (local.get $b))))
(func $hash_mix (param $hash i32) (param $word i32) (result i32)
  (i32.mul (i32.xor (local.get $hash) (local.get $word)) (i32.const 16777619)))
(func $value_hash_u32 (param $v eqref) (result i32)
  (local $kind i32) (local $h i32) (local $i i32)
  (local $o (ref null $obj)) (local $xs (ref null $values))
  (local $bs (ref null $bytes)) (local $ls (ref null $limbs))
  (local $n i32) (local $word i64) (local $bit i64)
  ;; The parts still to hash, on a growable stack: linear in the size.
  (local $stack (ref $values)) (local $sp i32) (local $grown (ref $values))
  (local.set $h (i32.const -2128831035))
  (local.set $stack (array.new_default $values (i32.const 16)))
  (array.set $values (local.get $stack) (i32.const 0) (local.get $v))
  (local.set $sp (i32.const 1))
  (block $done (loop $walk
    (br_if $done (i32.eqz (local.get $sp)))
    (local.set $sp (i32.sub (local.get $sp) (i32.const 1)))
    (local.set $v (array.get $values (local.get $stack) (local.get $sp)))
    (local.set $kind (call $value_tag (local.get $v)))
    (local.set $h (call $hash_mix (local.get $h) (local.get $kind)))
    (br_if $walk (i32.eqz (local.get $kind)))
    ;; Identity values hash by their tag alone: equal values are one value.
    (br_if $walk (call $value_identity_kind (local.get $kind)))
    (local.set $i (i32.const 0))
    ;; A small Int: as below, with its one part, without making the parts.
    (if (ref.test (ref i31) (local.get $v)) (then
      (local.set $n (i31.get_s (ref.cast (ref i31) (local.get $v))))
      (local.set $h (call $hash_mix (local.get $h) (call $int_sign (local.get $v))))
      (if (i32.eqz (local.get $n))
        (then (local.set $h (call $hash_mix (local.get $h) (i32.const 0))))
        (else
          (local.set $h (call $hash_mix (local.get $h) (i32.const 1)))
          (if (i32.lt_s (local.get $n) (i32.const 0))
            (then (local.set $n (i32.sub (i32.const 0) (local.get $n)))))
          (local.set $h (call $hash_mix (local.get $h) (local.get $n)))))
      (br $walk)))
    (if (i32.eq (local.get $kind) (i32.const 8)) (then
      (local.set $h (call $hash_mix (local.get $h) (call $int_sign (local.get $v))))
      (local.set $ls (call $int_limbs (local.get $v)))
      (local.set $n (array.len (local.get $ls)))
      (local.set $h (call $hash_mix (local.get $h) (local.get $n)))
      (block $int_done (loop $int_each
        (br_if $int_done (i32.ge_u (local.get $i) (local.get $n)))
        (local.set $h (call $hash_mix (local.get $h) (array.get $limbs (local.get $ls) (local.get $i))))
        (local.set $i (i32.add (local.get $i) (i32.const 1))) (br $int_each)))
      (br $walk)))
    (if (i32.eq (local.get $kind) (i32.const 10)) (then
      (local.set $word (call $host_id (local.get $v)))
      (local.set $h (call $hash_mix (call $hash_mix (local.get $h) (i32.wrap_i64 (local.get $word)))
        (i32.wrap_i64 (i64.shr_u (local.get $word) (i64.const 32)))))
      (br $walk)))
    ;; A list hashes as it did when its cells were `$obj` values with two
    ;; parts: constructor id 0, type id 0, then the parts unless it is [].
    (if (i32.eq (local.get $kind) (i32.const 6)) (then
      (local.set $h (call $hash_mix (local.get $h) (i32.const 0)))
      (local.set $h (call $hash_mix (call $hash_mix (local.get $h) (i32.const 0)) (i32.const 0)))
      (br_if $walk (call $list_is_empty (local.get $v)))
      (local.set $h (call $hash_mix (local.get $h) (i32.const 2)))
      (if (i32.gt_u (i32.add (local.get $sp) (i32.const 2)) (array.len (local.get $stack))) (then
        (if (i32.ge_u (array.len (local.get $stack)) (i32.const 268435456)) (then unreachable))
        (local.set $grown (array.new_default $values
          (i32.add (i32.shl (array.len (local.get $stack)) (i32.const 1)) (i32.const 2))))
        (array.copy $values $values (local.get $grown) (i32.const 0) (local.get $stack) (i32.const 0) (local.get $sp))
        (local.set $stack (local.get $grown))))
      (array.set $values (local.get $stack) (local.get $sp) (call $list_tail (local.get $v)))
      (array.set $values (local.get $stack) (i32.add (local.get $sp) (i32.const 1)) (call $list_head (local.get $v)))
      (local.set $sp (i32.add (local.get $sp) (i32.const 2)))
      (br $walk)))
    (local.set $o (ref.cast (ref $obj) (local.get $v)))
    (if (i32.eq (local.get $kind) (i32.const 2)) (then
      (local.set $word (i64.const 0))
      (if (f64.ne (struct.get $obj 3 (local.get $o)) (f64.const 0))
        (then (local.set $word (i64.reinterpret_f64 (struct.get $obj 3 (local.get $o))))))
      (local.set $h (call $hash_mix (call $hash_mix (local.get $h) (i32.wrap_i64 (local.get $word)))
        (i32.wrap_i64 (i64.shr_u (local.get $word) (i64.const 32)))))
      (br $walk)))
    (if (i32.eq (local.get $kind) (i32.const 3)) (then
      (local.set $bs (struct.get $obj 4 (local.get $o)))
      (local.set $n (array.len (local.get $bs)))
      (block $string_done (loop $string_each
        (br_if $string_done (i32.ge_u (local.get $i) (local.get $n)))
        (local.set $h (call $hash_mix (local.get $h) (array.get_u $bytes (local.get $bs) (local.get $i))))
        (local.set $i (i32.add (local.get $i) (i32.const 1))) (br $string_each)))
      (br $walk)))
    (if (i32.eq (local.get $kind) (i32.const 9)) (then
      (local.set $word (struct.get $obj 2 (local.get $o)))
      (local.set $h (call $hash_mix (call $hash_mix (local.get $h) (i32.wrap_i64 (local.get $word)))
        (i32.wrap_i64 (i64.shr_u (local.get $word) (i64.const 32)))))
      (local.set $bit (i64.const 0))
      (block $bits_done (loop $bits_each
        (br_if $bits_done (i64.ge_u (local.get $bit) (local.get $word)))
        (local.set $h (call $hash_mix (local.get $h) (call $value_bit (local.get $v) (local.get $bit))))
        (local.set $bit (i64.add (local.get $bit) (i64.const 1))) (br $bits_each)))
      (br $walk)))
    (local.set $h (call $hash_mix (local.get $h) (struct.get $obj 1 (local.get $o))))
    (local.set $word (struct.get $obj 2 (local.get $o)))
    (local.set $h (call $hash_mix (call $hash_mix (local.get $h) (i32.wrap_i64 (local.get $word)))
      (i32.wrap_i64 (i64.shr_u (local.get $word) (i64.const 32)))))
    (local.set $xs (struct.get $obj 5 (local.get $o)))
    (br_if $walk (ref.is_null (local.get $xs)))
    (local.set $n (array.len (local.get $xs)))
    (local.set $h (call $hash_mix (local.get $h) (local.get $n)))
    ;; The parts go on the stack last first, so the first is hashed first.
    (if (i32.gt_u (i32.add (local.get $sp) (local.get $n)) (array.len (local.get $stack))) (then
      (if (i32.ge_u (array.len (local.get $stack)) (i32.const 268435456)) (then unreachable))
      (local.set $grown (array.new_default $values
        (i32.add (i32.shl (array.len (local.get $stack)) (i32.const 1)) (local.get $n))))
      (array.copy $values $values (local.get $grown) (i32.const 0) (local.get $stack) (i32.const 0) (local.get $sp))
      (local.set $stack (local.get $grown))))
    (local.set $i (local.get $n))
    (block $fields_done (loop $fields_each
      (br_if $fields_done (i32.eqz (local.get $i)))
      (local.set $i (i32.sub (local.get $i) (i32.const 1)))
      (array.set $values (local.get $stack) (local.get $sp) (array.get $values (local.get $xs) (local.get $i)))
      (local.set $sp (i32.add (local.get $sp) (i32.const 1)))
      (br $fields_each)))
    (br $walk)))
  (local.get $h))
(func $value_hash (param $v eqref) (result eqref)
  (call $int_from_i64 (i64.extend_i32_u (call $value_hash_u32 (local.get $v)))))

(func $string_bytes (param $s eqref) (result (ref $bytes))
  (ref.as_non_null (struct.get $obj 4 (ref.cast (ref $obj) (local.get $s)))))
(func $string_from_bytes (param $b (ref $bytes)) (result eqref)
  (struct.new $obj (i32.const 3) (i32.const 0) (i64.const 0) (f64.const 0)
    (local.get $b) (ref.null $values) (ref.null $limbs) (ref.null $call)))
(func $string_length_bytes (param $s eqref) (result i32)
  (array.len (call $string_bytes (local.get $s))))
(func $string_byte_size (param $s eqref) (result eqref)
  (call $int_from_i64 (i64.extend_i32_u (call $string_length_bytes (local.get $s)))))

;; Return -1 for malformed or truncated UTF-8, without reading outside the array.
(func $utf8_decode_checked (param $b (ref $bytes)) (param $at i32) (result i32)
  (local $n i32) (local $lead i32) (local $width i32) (local $cp i32)
  (local $i i32) (local $x i32)
  (local.set $n (array.len (local.get $b)))
  (if (i32.ge_u (local.get $at) (local.get $n)) (then (return (i32.const -1))))
  (local.set $lead (array.get_u $bytes (local.get $b) (local.get $at)))
  (if (i32.lt_u (local.get $lead) (i32.const 128)) (then (return (local.get $lead))))
  (if (i32.and (i32.ge_u (local.get $lead) (i32.const 194))
               (i32.le_u (local.get $lead) (i32.const 223)))
    (then (local.set $width (i32.const 2)) (local.set $cp (i32.and (local.get $lead) (i32.const 31))))
    (else
      (if (i32.and (i32.ge_u (local.get $lead) (i32.const 224))
                   (i32.le_u (local.get $lead) (i32.const 239)))
        (then (local.set $width (i32.const 3)) (local.set $cp (i32.and (local.get $lead) (i32.const 15))))
        (else
          (if (i32.and (i32.ge_u (local.get $lead) (i32.const 240))
                       (i32.le_u (local.get $lead) (i32.const 244)))
            (then (local.set $width (i32.const 4)) (local.set $cp (i32.and (local.get $lead) (i32.const 7))))
            (else (return (i32.const -1))))))))
  (if (i32.gt_u (local.get $width) (i32.sub (local.get $n) (local.get $at)))
    (then (return (i32.const -1))))
  (local.set $i (i32.const 1))
  (block $done (loop $more
    (br_if $done (i32.ge_u (local.get $i) (local.get $width)))
    (local.set $x (array.get_u $bytes (local.get $b) (i32.add (local.get $at) (local.get $i))))
    (if (i32.ne (i32.and (local.get $x) (i32.const 192)) (i32.const 128))
      (then (return (i32.const -1))))
    (local.set $cp (i32.or (i32.shl (local.get $cp) (i32.const 6)) (i32.and (local.get $x) (i32.const 63))))
    (local.set $i (i32.add (local.get $i) (i32.const 1)))
    (br $more)))
  (if (i32.or
        (i32.and (i32.eq (local.get $width) (i32.const 3)) (i32.lt_u (local.get $cp) (i32.const 2048)))
        (i32.and (i32.eq (local.get $width) (i32.const 4)) (i32.lt_u (local.get $cp) (i32.const 65536))))
    (then (return (i32.const -1))))
  (if (i32.or (i32.gt_u (local.get $cp) (i32.const 1114111))
               (i32.and (i32.ge_u (local.get $cp) (i32.const 55296))
                        (i32.le_u (local.get $cp) (i32.const 57343))))
    (then (return (i32.const -1))))
  (local.get $cp))
(func $utf8_decode (param $b (ref $bytes)) (param $at i32) (result i32)
  (local $cp i32)
  (local.set $cp (call $utf8_decode_checked (local.get $b) (local.get $at)))
  (if (i32.lt_s (local.get $cp) (i32.const 0)) (then unreachable))
  (local.get $cp))
(func $utf8_width (param $cp i32) (result i32)
  (if (i32.or (i32.gt_u (local.get $cp) (i32.const 1114111))
               (i32.and (i32.ge_u (local.get $cp) (i32.const 55296))
                        (i32.le_u (local.get $cp) (i32.const 57343))))
    (then unreachable))
  (if (i32.lt_u (local.get $cp) (i32.const 128)) (then (return (i32.const 1))))
  (if (i32.lt_u (local.get $cp) (i32.const 2048)) (then (return (i32.const 2))))
  (if (i32.lt_u (local.get $cp) (i32.const 65536)) (then (return (i32.const 3))))
  (i32.const 4))
(func $utf8_valid (param $b (ref $bytes)) (result i32)
  (local $at i32) (local $cp i32)
  (block $done (loop $more
    (br_if $done (i32.ge_u (local.get $at) (array.len (local.get $b))))
    (local.set $cp (call $utf8_decode_checked (local.get $b) (local.get $at)))
    (if (i32.lt_s (local.get $cp) (i32.const 0)) (then (return (i32.const 0))))
    (local.set $at (i32.add (local.get $at) (call $utf8_width (local.get $cp))))
    (br $more)))
  (i32.const 1))
(func $utf8_write (param $b (ref $bytes)) (param $at i32) (param $cp i32) (result i32)
  (local $w i32) (local $i i32) (local $x i32)
  (local.set $w (call $utf8_width (local.get $cp)))
  (if (i32.gt_u (local.get $at) (array.len (local.get $b))) (then unreachable))
  (if (i32.gt_u (local.get $w) (i32.sub (array.len (local.get $b)) (local.get $at))) (then unreachable))
  (if (i32.eq (local.get $w) (i32.const 1))
    (then (array.set $bytes (local.get $b) (local.get $at) (local.get $cp)))
    (else
      (local.set $x (local.get $cp))
      (local.set $i (i32.sub (local.get $w) (i32.const 1)))
      (loop $more
        (array.set $bytes (local.get $b) (i32.add (local.get $at) (local.get $i))
          (i32.or (i32.const 128) (i32.and (local.get $x) (i32.const 63))))
        (local.set $x (i32.shr_u (local.get $x) (i32.const 6)))
        (local.set $i (i32.sub (local.get $i) (i32.const 1)))
        (br_if $more (i32.ne (local.get $i) (i32.const 0))))
      (array.set $bytes (local.get $b) (local.get $at)
        (i32.or (local.get $x)
          (if (result i32) (i32.eq (local.get $w) (i32.const 2))
            (then (i32.const 192))
            (else (if (result i32) (i32.eq (local.get $w) (i32.const 3))
                    (then (i32.const 224)) (else (i32.const 240)))))))))
  (i32.add (local.get $at) (local.get $w)))

(func $string_from_mem (param $ptr i32) (param $len i32) (result eqref)
  (local $b (ref $bytes)) (local $i i32)
  ;; Check memory bounds before allocating, including the zero-length case.
  (if (i64.gt_u (i64.add (i64.extend_i32_u (local.get $ptr)) (i64.extend_i32_u (local.get $len)))
                (i64.shl (i64.extend_i32_u (memory.size)) (i64.const 16))) (then unreachable))
  (local.set $b (array.new_default $bytes (local.get $len)))
  (block $done (loop $more
    (br_if $done (i32.ge_u (local.get $i) (local.get $len)))
    (array.set $bytes (local.get $b) (local.get $i) (i32.load8_u (i32.add (local.get $ptr) (local.get $i))))
    (local.set $i (i32.add (local.get $i) (i32.const 1))) (br $more)))
  (if (i32.eqz (call $utf8_valid (local.get $b))) (then unreachable))
  (call $string_from_bytes (local.get $b)))
(func $string_to_mem (param $s eqref) (result i32)
  (local $b (ref $bytes)) (local $p i32) (local $n i32) (local $i i32)
  (local.set $b (call $string_bytes (local.get $s)))
  (local.set $n (array.len (local.get $b)))
  (local.set $p (call $alloc (local.get $n)))
  (block $done (loop $more
    (br_if $done (i32.ge_u (local.get $i) (local.get $n)))
    (i32.store8 (i32.add (local.get $p) (local.get $i)) (array.get_u $bytes (local.get $b) (local.get $i)))
    (local.set $i (i32.add (local.get $i) (i32.const 1))) (br $more)))
  (local.get $p))
(func $string_append (param $a eqref) (param $b eqref) (result eqref)
  (local $x (ref $bytes)) (local $y (ref $bytes)) (local $z (ref $bytes))
  (local $nx i32) (local $ny i32) (local $n i64)
  (local.set $x (call $string_bytes (local.get $a)))
  (local.set $y (call $string_bytes (local.get $b)))
  (local.set $nx (array.len (local.get $x))) (local.set $ny (array.len (local.get $y)))
  (local.set $n (i64.add (i64.extend_i32_u (local.get $nx)) (i64.extend_i32_u (local.get $ny))))
  (if (i64.gt_u (local.get $n) (i64.const 4294967295)) (then unreachable))
  (local.set $z (array.new_default $bytes (i32.wrap_i64 (local.get $n))))
  (array.copy $bytes $bytes (local.get $z) (i32.const 0) (local.get $x) (i32.const 0) (local.get $nx))
  (array.copy $bytes $bytes (local.get $z) (local.get $nx) (local.get $y) (i32.const 0) (local.get $ny))
  (call $string_from_bytes (local.get $z)))
(func $string_compare (param $a eqref) (param $b eqref) (result i32)
  (local $x (ref $bytes)) (local $y (ref $bytes)) (local $i i32) (local $u i32) (local $v i32)
  (local.set $x (call $string_bytes (local.get $a)))
  (local.set $y (call $string_bytes (local.get $b)))
  (block $end (loop $more
    (br_if $end (i32.or (i32.ge_u (local.get $i) (array.len (local.get $x)))
                         (i32.ge_u (local.get $i) (array.len (local.get $y)))))
    (local.set $u (array.get_u $bytes (local.get $x) (local.get $i)))
    (local.set $v (array.get_u $bytes (local.get $y) (local.get $i)))
    (if (i32.lt_u (local.get $u) (local.get $v)) (then (return (i32.const -1))))
    (if (i32.gt_u (local.get $u) (local.get $v)) (then (return (i32.const 1))))
    (local.set $i (i32.add (local.get $i) (i32.const 1))) (br $more)))
  (i32.sub (i32.gt_u (array.len (local.get $x)) (array.len (local.get $y)))
           (i32.lt_u (array.len (local.get $x)) (array.len (local.get $y)))))
(func $string_slice_bytes (param $s eqref) (param $start i32) (param $len i32) (result eqref)
  (local $b (ref $bytes)) (local $out (ref $bytes))
  (local.set $b (call $string_bytes (local.get $s)))
  (if (i32.gt_u (local.get $start) (array.len (local.get $b))) (then unreachable))
  (if (i32.gt_u (local.get $len) (i32.sub (array.len (local.get $b)) (local.get $start))) (then unreachable))
  (local.set $out (array.new_default $bytes (local.get $len)))
  (array.copy $bytes $bytes (local.get $out) (i32.const 0) (local.get $b) (local.get $start) (local.get $len))
  (if (i32.eqz (call $utf8_valid (local.get $out))) (then unreachable))
  (call $string_from_bytes (local.get $out)))
(func $string_suffix (param $s eqref) (param $start i32) (result eqref)
  (call $string_slice_bytes (local.get $s) (local.get $start)
    (i32.sub (call $string_length_bytes (local.get $s)) (local.get $start))))
;; Is `at` a character boundary of valid UTF-8 (the start, the end, or a byte
;; that is not a continuation byte)?
(func $string_boundary (param $b (ref $bytes)) (param $at i32) (result i32)
  (if (i32.ge_u (local.get $at) (array.len (local.get $b)))
    (then (return (i32.eq (local.get $at) (array.len (local.get $b))))))
  (i32.ne (i32.and (array.get_u $bytes (local.get $b) (local.get $at)) (i32.const 192)) (i32.const 128)))

(func $string_match_at (param $s (ref $bytes)) (param $needle (ref $bytes)) (param $at i32) (result i32)
  (local $i i32)
  (if (i32.gt_u (local.get $at) (array.len (local.get $s))) (then (return (i32.const 0))))
  (if (i32.gt_u (array.len (local.get $needle)) (i32.sub (array.len (local.get $s)) (local.get $at)))
    (then (return (i32.const 0))))
  (block $done (loop $more
    (br_if $done (i32.ge_u (local.get $i) (array.len (local.get $needle))))
    (if (i32.ne (array.get_u $bytes (local.get $needle) (local.get $i))
                 (array.get_u $bytes (local.get $s) (i32.add (local.get $at) (local.get $i))))
      (then (return (i32.const 0))))
    (local.set $i (i32.add (local.get $i) (i32.const 1))) (br $more)))
  (i32.const 1))
(func $string_starts_with (param $s eqref) (param $prefix eqref) (result i32)
  (call $string_match_at (call $string_bytes (local.get $s)) (call $string_bytes (local.get $prefix)) (i32.const 0)))
(func $string_ends_with (param $s eqref) (param $suffix eqref) (result i32)
  (local $a (ref $bytes)) (local $b (ref $bytes))
  (local.set $a (call $string_bytes (local.get $s))) (local.set $b (call $string_bytes (local.get $suffix)))
  (if (i32.gt_u (array.len (local.get $b)) (array.len (local.get $a))) (then (return (i32.const 0))))
  (call $string_match_at (local.get $a) (local.get $b) (i32.sub (array.len (local.get $a)) (array.len (local.get $b)))))
(func $string_index_of (param $s eqref) (param $needle eqref) (result i32)
  (local $a (ref $bytes)) (local $b (ref $bytes)) (local $at i32) (local $last i32)
  (local.set $a (call $string_bytes (local.get $s))) (local.set $b (call $string_bytes (local.get $needle)))
  (if (i32.gt_u (array.len (local.get $b)) (array.len (local.get $a))) (then (return (i32.const -1))))
  (local.set $last (i32.sub (array.len (local.get $a)) (array.len (local.get $b))))
  (loop $more
    (if (call $string_match_at (local.get $a) (local.get $b) (local.get $at)) (then (return (local.get $at))))
    (if (i32.eq (local.get $at) (local.get $last)) (then (return (i32.const -1))))
    (local.set $at (i32.add (local.get $at) (i32.const 1))) (br $more))
  unreachable)

(func $string_codepoints (param $s eqref) (result eqref)
  (local $b (ref $bytes)) (local $end i32) (local $at i32) (local $cp i32) (local $out eqref)
  (local.set $b (call $string_bytes (local.get $s)))
  (local.set $end (array.len (local.get $b)))
  (local.set $out (call $list_nil))
  (block $done (loop $more
    (br_if $done (i32.eqz (local.get $end)))
    (local.set $at (i32.sub (local.get $end) (i32.const 1)))
    (block $lead (loop $continuations
      (br_if $lead (i32.ne (i32.and (array.get_u $bytes (local.get $b) (local.get $at)) (i32.const 192)) (i32.const 128)))
      (if (i32.eqz (local.get $at)) (then unreachable))
      (local.set $at (i32.sub (local.get $at) (i32.const 1))) (br $continuations)))
    (local.set $cp (call $utf8_decode (local.get $b) (local.get $at)))
    (if (i32.ne (i32.add (local.get $at) (call $utf8_width (local.get $cp))) (local.get $end)) (then unreachable))
    (local.set $out (call $list_cons (call $codepoint_new (local.get $cp)) (local.get $out)))
    (local.set $end (local.get $at)) (br $more)))
  (local.get $out))
(func $string_from_codepoints (param $list eqref) (result eqref)
  (local $cursor eqref) (local $n i64)
  (local $cp i32) (local $at i32) (local $out (ref $bytes))
  (local.set $cursor (local.get $list))
  (block $counted (loop $count
    (br_if $counted (call $list_is_empty (local.get $cursor)))
    (local.set $cp (call $codepoint_get (call $list_head (local.get $cursor))))
    (local.set $n (i64.add (local.get $n) (i64.extend_i32_u (call $utf8_width (local.get $cp)))))
    (if (i64.gt_u (local.get $n) (i64.const 4294967295)) (then unreachable))
    (local.set $cursor (call $list_tail (local.get $cursor))) (br $count)))
  (local.set $out (array.new_default $bytes (i32.wrap_i64 (local.get $n))))
  (local.set $cursor (local.get $list))
  (block $written (loop $write
    (br_if $written (call $list_is_empty (local.get $cursor)))
    (local.set $cp (call $codepoint_get (call $list_head (local.get $cursor))))
    (local.set $at (call $utf8_write (local.get $out) (local.get $at) (local.get $cp)))
    (local.set $cursor (call $list_tail (local.get $cursor))) (br $write)))
  (call $string_from_bytes (local.get $out)))
(func $string_concat (param $list eqref) (result eqref)
  (local $cursor eqref) (local $n i64)
  (local $s (ref $bytes)) (local $at i32) (local $out (ref $bytes))
  (local.set $cursor (local.get $list))
  (block $counted (loop $count
    (br_if $counted (call $list_is_empty (local.get $cursor)))
    (local.set $s (call $string_bytes (call $list_head (local.get $cursor))))
    (local.set $n (i64.add (local.get $n) (i64.extend_i32_u (array.len (local.get $s)))))
    (if (i64.gt_u (local.get $n) (i64.const 4294967295)) (then unreachable))
    (local.set $cursor (call $list_tail (local.get $cursor))) (br $count)))
  (local.set $out (array.new_default $bytes (i32.wrap_i64 (local.get $n))))
  (local.set $cursor (local.get $list))
  (block $written (loop $write
    (br_if $written (call $list_is_empty (local.get $cursor)))
    (local.set $s (call $string_bytes (call $list_head (local.get $cursor))))
    (array.copy $bytes $bytes (local.get $out) (local.get $at) (local.get $s) (i32.const 0) (array.len (local.get $s)))
    (local.set $at (i32.add (local.get $at) (array.len (local.get $s))))
    (local.set $cursor (call $list_tail (local.get $cursor))) (br $write)))
  (call $string_from_bytes (local.get $out)))

  ;; The text form is written into one growable byte buffer, `$ib` with the
  ;; length `$il`, so that inspecting is linear in the size of the text. Only
  ;; `$inspect` uses it, and it runs no i code.
  (global $ib (mut (ref null $bytes)) (ref.null $bytes))
  (global $il (mut i32) (i32.const 0))

  ;; Room for `n` more bytes.
  (func $ib_reserve (param $n i32)
    (local $old (ref $bytes)) (local $need i64) (local $cap i64) (local $new (ref $bytes))
    (local.set $old (ref.as_non_null (global.get $ib)))
    (local.set $need (i64.add (i64.extend_i32_u (global.get $il)) (i64.extend_i32_u (local.get $n))))
    (if (i64.le_u (local.get $need) (i64.extend_i32_u (array.len (local.get $old)))) (then (return)))
    (if (i64.gt_u (local.get $need) (i64.const 2147483647)) (then unreachable))
    (local.set $cap (i64.shl (i64.extend_i32_u (array.len (local.get $old))) (i64.const 1)))
    (if (i64.gt_u (local.get $cap) (i64.const 2147483647)) (then (local.set $cap (i64.const 2147483647))))
    (if (i64.lt_u (local.get $cap) (local.get $need)) (then (local.set $cap (local.get $need))))
    (local.set $new (array.new_default $bytes (i32.wrap_i64 (local.get $cap))))
    (array.copy $bytes $bytes (local.get $new) (i32.const 0) (local.get $old) (i32.const 0) (global.get $il))
    (global.set $ib (local.get $new)))

  (func $ib_byte (param $b i32)
    (call $ib_reserve (i32.const 1))
    (array.set $bytes (ref.as_non_null (global.get $ib)) (global.get $il) (local.get $b))
    (global.set $il (i32.add (global.get $il) (i32.const 1))))

  (func $ib_comma
    (call $ib_reserve (i32.const 2))
    (array.set $bytes (ref.as_non_null (global.get $ib)) (global.get $il) (i32.const 44))
    (array.set $bytes (ref.as_non_null (global.get $ib)) (i32.add (global.get $il) (i32.const 1)) (i32.const 32))
    (global.set $il (i32.add (global.get $il) (i32.const 2))))

  ;; The bytes of a String value.
  (func $ib_string (param $s eqref)
    (local $b (ref $bytes)) (local $n i32)
    (local.set $b (call $string_bytes (local.get $s)))
    (local.set $n (array.len (local.get $b)))
    (call $ib_reserve (local.get $n))
    (array.copy $bytes $bytes (ref.as_non_null (global.get $ib)) (global.get $il)
      (local.get $b) (i32.const 0) (local.get $n))
    (global.set $il (i32.add (global.get $il) (local.get $n))))

  ;; The decimal digits of a small Int, with `-` when it is negative.
  (func $ib_small_int (param $v i32)
    (local $u i64) (local $buf (ref $bytes)) (local $start i32) (local $i i32) (local $j i32) (local $t i32)
    (call $ib_reserve (i32.const 12))
    (local.set $buf (ref.as_non_null (global.get $ib)))
    (local.set $i (global.get $il))
    (local.set $u (i64.extend_i32_s (local.get $v)))
    (if (i64.lt_s (local.get $u) (i64.const 0))
      (then
        (array.set $bytes (local.get $buf) (local.get $i) (i32.const 45))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (local.set $u (i64.sub (i64.const 0) (local.get $u)))))
    (local.set $start (local.get $i))
    (loop $digits
      (array.set $bytes (local.get $buf) (local.get $i)
        (i32.add (i32.const 48) (i32.wrap_i64 (i64.rem_u (local.get $u) (i64.const 10)))))
      (local.set $i (i32.add (local.get $i) (i32.const 1)))
      (local.set $u (i64.div_u (local.get $u) (i64.const 10)))
      (br_if $digits (i64.ne (local.get $u) (i64.const 0))))
    (global.set $il (local.get $i))
    (local.set $j (i32.sub (local.get $i) (i32.const 1)))
    (block $reversed (loop $reverse
      (br_if $reversed (i32.ge_s (local.get $start) (local.get $j)))
      (local.set $t (array.get_u $bytes (local.get $buf) (local.get $start)))
      (array.set $bytes (local.get $buf) (local.get $start) (array.get_u $bytes (local.get $buf) (local.get $j)))
      (array.set $bytes (local.get $buf) (local.get $j) (local.get $t))
      (local.set $start (i32.add (local.get $start) (i32.const 1)))
      (local.set $j (i32.sub (local.get $j) (i32.const 1)))
      (br $reverse))))

  ;; Quoting works on UTF-8 bytes. C0, DEL and the UTF-8 encoded C1 controls use
  ;; i escapes; all other valid UTF-8 is copied unchanged.
  (func $ib_quote (param $s eqref)
    (local $bytes (ref $bytes)) (local $n i32) (local $buf (ref $bytes))
    (local $out i32) (local $i i32) (local $b i32)
    (local $escape i32) (local $control i32)
    (local.set $bytes (call $string_bytes (local.get $s)))
    (local.set $n (array.len (local.get $bytes)))
    (if (i32.gt_u (local.get $n) (i32.const 268435455)) (then unreachable))
    (call $ib_reserve (i32.add (i32.mul (local.get $n) (i32.const 8)) (i32.const 2)))
    (local.set $buf (ref.as_non_null (global.get $ib)))
    (local.set $out (global.get $il))
    (array.set $bytes (local.get $buf) (local.get $out) (i32.const 34))
    (local.set $out (i32.add (local.get $out) (i32.const 1)))
    (block $done
      (loop $next
        (br_if $done (i32.ge_u (local.get $i) (local.get $n)))
        (local.set $b (array.get_u $bytes (local.get $bytes) (local.get $i)))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (local.set $control (i32.or (i32.lt_u (local.get $b) (i32.const 32)) (i32.eq (local.get $b) (i32.const 127))))
        (local.set $escape (i32.const 0))
        (if (i32.eq (local.get $b) (i32.const 10)) (then (local.set $escape (i32.const 110))))
        (if (i32.eq (local.get $b) (i32.const 13)) (then (local.set $escape (i32.const 114))))
        (if (i32.eq (local.get $b) (i32.const 9)) (then (local.set $escape (i32.const 116))))
        (if (i32.eq (local.get $b) (i32.const 12)) (then (local.set $escape (i32.const 102))))
        (if (i32.or (i32.eq (local.get $b) (i32.const 34)) (i32.eq (local.get $b) (i32.const 92)))
          (then (local.set $escape (local.get $b))))
        (if (local.get $escape)
          (then
            (array.set $bytes (local.get $buf) (local.get $out) (i32.const 92))
            (array.set $bytes (local.get $buf) (i32.add (local.get $out) (i32.const 1)) (local.get $escape))
            (local.set $out (i32.add (local.get $out) (i32.const 2))))
          (else
            (if (i32.and (i32.eq (local.get $b) (i32.const 194)) (i32.lt_u (local.get $i) (local.get $n)))
              (then
                (if (i32.lt_u (array.get_u $bytes (local.get $bytes) (local.get $i)) (i32.const 160))
                  (then
                    (local.set $b (array.get_u $bytes (local.get $bytes) (local.get $i)))
                    (local.set $control (i32.const 1))
                    (local.set $i (i32.add (local.get $i) (i32.const 1)))))))
            (if (local.get $control)
              (then
                (array.set $bytes (local.get $buf) (local.get $out) (i32.const 92))
                (array.set $bytes (local.get $buf) (i32.add (local.get $out) (i32.const 1)) (i32.const 117))
                (array.set $bytes (local.get $buf) (i32.add (local.get $out) (i32.const 2)) (i32.const 123))
                (array.set $bytes (local.get $buf) (i32.add (local.get $out) (i32.const 3)) (i32.const 48))
                (array.set $bytes (local.get $buf) (i32.add (local.get $out) (i32.const 4)) (i32.const 48))
                (array.set $bytes (local.get $buf) (i32.add (local.get $out) (i32.const 5))
                  (call $inspect_hex (i32.shr_u (local.get $b) (i32.const 4))))
                (array.set $bytes (local.get $buf) (i32.add (local.get $out) (i32.const 6))
                  (call $inspect_hex (i32.and (local.get $b) (i32.const 15))))
                (array.set $bytes (local.get $buf) (i32.add (local.get $out) (i32.const 7)) (i32.const 125))
                (local.set $out (i32.add (local.get $out) (i32.const 8))))
              (else
                (array.set $bytes (local.get $buf) (local.get $out) (local.get $b))
                (local.set $out (i32.add (local.get $out) (i32.const 1)))))))
        (br $next)))
    (array.set $bytes (local.get $buf) (local.get $out) (i32.const 34))
    (global.set $il (i32.add (local.get $out) (i32.const 1))))

  (func $inspect_hex (param $n i32) (result i32)
    (i32.add (local.get $n) (if (result i32) (i32.lt_u (local.get $n) (i32.const 10))
      (then (i32.const 48)) (else (i32.const 55)))))

  (func $inspect_bits (param $value eqref) (result eqref)
    (local $obj (ref $obj)) (local $bytes (ref $bytes)) (local $length i64)
    (local $offset i64) (local $i i64) (local $pos i64) (local $j i32) (local $v i32) (local $out eqref)
    (local.set $obj (ref.cast (ref $obj) (local.get $value)))
    (local.set $bytes (ref.as_non_null (struct.get $obj 4 (local.get $obj))))
    (local.set $length (struct.get $obj 2 (local.get $obj)))
    (local.set $offset (i64.extend_i32_u (struct.get $obj 1 (local.get $obj))))
    (local.set $out (call $list_cons (call $string_from_bytes (array.new_fixed $bytes 2 (i32.const 60) (i32.const 60))) (call $list_nil)))
    (block $done
      (loop $next
        (br_if $done (i64.ge_u (local.get $i) (local.get $length)))
        (if (i64.ne (local.get $i) (i64.const 0)) (then (local.set $out (call $list_cons (call $string_from_bytes (array.new_fixed $bytes 2 (i32.const 44) (i32.const 32))) (local.get $out)))))
        (local.set $j (i32.const 0))
        (local.set $v (i32.const 0))
        (block $group_done
          (loop $group
            (br_if $group_done (i32.eq (local.get $j) (i32.const 8)))
            (br_if $group_done (i64.ge_u (local.get $i) (local.get $length)))
            (local.set $pos (i64.add (local.get $offset) (local.get $i)))
            (local.set $v (i32.or (i32.shl (local.get $v) (i32.const 1))
              (i32.and
                (i32.shr_u (array.get_u $bytes (local.get $bytes) (i32.wrap_i64 (i64.shr_u (local.get $pos) (i64.const 3))))
                  (i32.sub (i32.const 7) (i32.wrap_i64 (i64.and (local.get $pos) (i64.const 7))))) (i32.const 1))))
            (local.set $i (i64.add (local.get $i) (i64.const 1)))
            (local.set $j (i32.add (local.get $j) (i32.const 1)))
            (br $group)))
        (local.set $out (call $list_cons (call $int_to_string (call $int_from_i64 (i64.extend_i32_u (local.get $v)))) (local.get $out)))
        ;; The last k < 8 bits are written `value:uk` (spec §11.8).
        (if (i32.lt_u (local.get $j) (i32.const 8))
          (then
            (local.set $out (call $list_cons (call $string_from_bytes (array.new_fixed $bytes 2 (i32.const 58) (i32.const 117))) (local.get $out)))
            (local.set $out (call $list_cons (call $int_to_string (call $int_from_i64 (i64.extend_i32_u (local.get $j)))) (local.get $out)))))
        (br $next)))
    (call $string_concat (call $list_reverse (call $list_cons (call $string_from_bytes (array.new_fixed $bytes 2 (i32.const 62) (i32.const 62))) (local.get $out)))))

  ;; Parameter i is the letter a + (i mod 26), then i div 26 when i >= 26.
  (func $inspect_function (param $arity i32) (result eqref)
    (local $n i32) (local $i i32) (local $out eqref)
    (local.set $n (local.get $arity))
    (local.set $out (call $list_cons (call $string_from_bytes (array.new_fixed $bytes 5 (i32.const 47) (i32.const 47) (i32.const 102) (i32.const 110) (i32.const 40))) (call $list_nil)))
    (block $done
      (loop $next
        (br_if $done (i32.ge_u (local.get $i) (local.get $n)))
        (if (local.get $i) (then (local.set $out (call $list_cons (call $string_from_bytes (array.new_fixed $bytes 2 (i32.const 44) (i32.const 32))) (local.get $out)))))
        (local.set $out (call $list_cons
          (call $string_from_bytes (array.new_fixed $bytes 1
            (i32.add (i32.const 97) (i32.rem_u (local.get $i) (i32.const 26))))) (local.get $out)))
        (if (i32.ge_u (local.get $i) (i32.const 26))
          (then (local.set $out (call $list_cons
            (call $int_to_string (call $int_from_i64
              (i64.extend_i32_u (i32.div_u (local.get $i) (i32.const 26))))) (local.get $out)))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $next)))
    (call $string_concat (call $list_reverse (call $list_cons (call $string_from_bytes (array.new_fixed $bytes 9 (i32.const 41) (i32.const 32) (i32.const 123) (i32.const 32) (i32.const 46) (i32.const 46) (i32.const 46) (i32.const 32) (i32.const 125))) (local.get $out)))))

  ;; Constructor names: the runtime's own below FIRST_CONSTRUCTOR (16), then the
  ;; names that the code generator installs, in order.
  (global $constructor_names (mut (ref null $values)) (ref.null $values))
  (func $constructors_install (param $names (ref $values))
    (global.set $constructor_names (local.get $names)))
  (func $constructor_name (param $ctor i32) (result eqref)
    (local $names (ref null $values)) (local $index i32)
    (if (i32.eqz (local.get $ctor)) (then (return (call $string_from_bytes (array.new_fixed $bytes 2 (i32.const 79) (i32.const 107))))))
    (if (i32.eq (local.get $ctor) (i32.const 1)) (then (return (call $string_from_bytes (array.new_fixed $bytes 5 (i32.const 69) (i32.const 114) (i32.const 114) (i32.const 111) (i32.const 114))))))
    (if (i32.eq (local.get $ctor) (i32.const 2)) (then (return (call $string_from_bytes (array.new_fixed $bytes 16 (i32.const 65) (i32.const 115) (i32.const 115) (i32.const 101) (i32.const 114) (i32.const 116) (i32.const 105) (i32.const 111) (i32.const 110) (i32.const 70) (i32.const 97) (i32.const 105) (i32.const 108) (i32.const 117) (i32.const 114) (i32.const 101))))))
    (if (i32.eq (local.get $ctor) (i32.const 3)) (then (return (call $string_from_bytes (array.new_fixed $bytes 6 (i32.const 65) (i32.const 115) (i32.const 115) (i32.const 101) (i32.const 114) (i32.const 116))))))
    (local.set $names (global.get $constructor_names))
    (local.set $index (i32.sub (local.get $ctor) (i32.const 16)))
    (if (i32.or (ref.is_null (local.get $names)) (i32.lt_s (local.get $index) (i32.const 0)))
      (then (return (call $string_from_bytes (array.new_fixed $bytes 15 (i32.const 47) (i32.const 47) (i32.const 119) (i32.const 97) (i32.const 115) (i32.const 109) (i32.const 40) (i32.const 117) (i32.const 110) (i32.const 107) (i32.const 110) (i32.const 111) (i32.const 119) (i32.const 110) (i32.const 41))))))
    (if (i32.ge_u (local.get $index) (array.len (local.get $names)))
      (then (return (call $string_from_bytes (array.new_fixed $bytes 15 (i32.const 47) (i32.const 47) (i32.const 119) (i32.const 97) (i32.const 115) (i32.const 109) (i32.const 40) (i32.const 117) (i32.const 110) (i32.const 107) (i32.const 110) (i32.const 111) (i32.const 119) (i32.const 110) (i32.const 41))))))
    (array.get $values (local.get $names) (local.get $index)))

  ;; Aggregate traversal uses an explicit stack of tasks rather than Wasm
  ;; calls, so deep values do not use the Wasm stack. A task is a value and a
  ;; number, `kind | index << 2`, in two growable arrays.
  (global $it_values (mut (ref null $values)) (ref.null $values))
  (global $it_meta (mut (ref null $frames)) (ref.null $frames))
  (global $it_sp (mut i32) (i32.const 0))

  (func $inspect_push (param $kind i32) (param $value eqref) (param $index i32)
    (local $sp i32) (local $n i32) (local $values (ref $values)) (local $meta (ref $frames))
    (local.set $sp (global.get $it_sp))
    (local.set $n (array.len (ref.as_non_null (global.get $it_values))))
    (if (i32.ge_u (local.get $sp) (local.get $n))
      (then
        (if (i32.ge_u (local.get $n) (i32.const 268435456)) (then unreachable))
        (local.set $values (array.new_default $values (i32.shl (local.get $n) (i32.const 1))))
        (local.set $meta (array.new_default $frames (i32.shl (local.get $n) (i32.const 1))))
        (array.copy $values $values (local.get $values) (i32.const 0)
          (ref.as_non_null (global.get $it_values)) (i32.const 0) (local.get $n))
        (array.copy $frames $frames (local.get $meta) (i32.const 0)
          (ref.as_non_null (global.get $it_meta)) (i32.const 0) (local.get $n))
        (global.set $it_values (local.get $values))
        (global.set $it_meta (local.get $meta))))
    (array.set $values (ref.as_non_null (global.get $it_values)) (local.get $sp) (local.get $value))
    (array.set $frames (ref.as_non_null (global.get $it_meta)) (local.get $sp)
      (i32.or (local.get $kind) (i32.shl (local.get $index) (i32.const 2))))
    (global.set $it_sp (i32.add (local.get $sp) (i32.const 1))))

  ;; Values with no parts that are inspected; not Ints and Strings, which
  ;; `$ib_atom` writes itself.
  (func $inspect_atom (param $value eqref) (param $tag i32) (result eqref)
    (local $obj (ref $obj))
    (if (i32.eqz (local.get $tag)) (then (return (call $string_from_bytes (array.new_fixed $bytes 3 (i32.const 78) (i32.const 105) (i32.const 108))))))
    (if (i32.eq (local.get $tag) (i32.const 8)) (then (return_call $int_to_string (local.get $value))))
    (if (i32.eq (local.get $tag) (i32.const 1))
      (then (return (if (result eqref) (call $bool_get (local.get $value)) (then (call $string_from_bytes (array.new_fixed $bytes 4 (i32.const 84) (i32.const 114) (i32.const 117) (i32.const 101)))) (else (call $string_from_bytes (array.new_fixed $bytes 5 (i32.const 70) (i32.const 97) (i32.const 108) (i32.const 115) (i32.const 101))))))))
    (if (i32.eq (local.get $tag) (i32.const 2)) (then (return_call $float_to_string (local.get $value))))
    (if (i32.eq (local.get $tag) (i32.const 9)) (then (return_call $inspect_bits (local.get $value))))
    (if (i32.eq (local.get $tag) (i32.const 10)) (then (return (call $string_from_bytes (array.new_fixed $bytes 14 (i32.const 47) (i32.const 47) (i32.const 119) (i32.const 97) (i32.const 115) (i32.const 109) (i32.const 40) (i32.const 111) (i32.const 112) (i32.const 97) (i32.const 113) (i32.const 117) (i32.const 101) (i32.const 41))))))
    (if (i32.eq (local.get $tag) (i32.const 11)) (then (return (call $string_from_bytes (array.new_fixed $bytes 10 (i32.const 47) (i32.const 47) (i32.const 98) (i32.const 111) (i32.const 117) (i32.const 110) (i32.const 100) (i32.const 97) (i32.const 114) (i32.const 121))))))
    (if (i32.eq (local.get $tag) (i32.const 12)) (then (return (call $string_from_bytes (array.new_fixed $bytes 14 (i32.const 47) (i32.const 47) (i32.const 99) (i32.const 111) (i32.const 110) (i32.const 116) (i32.const 105) (i32.const 110) (i32.const 117) (i32.const 97) (i32.const 116) (i32.const 105) (i32.const 111) (i32.const 110))))))
    (local.set $obj (ref.cast (ref $obj) (local.get $value)))
    (if (i32.eq (local.get $tag) (i32.const 7))
      (then (return_call $inspect_function (i32.wrap_i64 (struct.get $obj 2 (local.get $obj))))))
    (if (i32.eq (local.get $tag) (i32.const 13))
      (then (return (call $string_append (call $string_append (call $string_from_bytes (array.new_fixed $bytes 15 (i32.const 47) (i32.const 47) (i32.const 117) (i32.const 116) (i32.const 102) (i32.const 99) (i32.const 111) (i32.const 100) (i32.const 101) (i32.const 112) (i32.const 111) (i32.const 105) (i32.const 110) (i32.const 116) (i32.const 40)))
        (call $string_from_codepoints (call $list_cons (local.get $value) (call $list_nil)))) (call $string_from_bytes (array.new_fixed $bytes 1 (i32.const 41)))))))
    (call $string_from_bytes (array.new_fixed $bytes 15 (i32.const 47) (i32.const 47) (i32.const 119) (i32.const 97) (i32.const 115) (i32.const 109) (i32.const 40) (i32.const 117) (i32.const 110) (i32.const 107) (i32.const 110) (i32.const 111) (i32.const 119) (i32.const 110) (i32.const 41))))

  (func $ib_atom (param $value eqref) (param $tag i32)
    (if (ref.test (ref i31) (local.get $value))
      (then (return_call $ib_small_int (i31.get_s (ref.cast (ref i31) (local.get $value))))))
    (if (i32.eq (local.get $tag) (i32.const 3)) (then (return_call $ib_quote (local.get $value))))
    (return_call $ib_string (call $inspect_atom (local.get $value) (local.get $tag))))

  (func $inspect (param $value eqref) (result eqref)
    (local $obj (ref $obj)) (local $fields (ref null $values)) (local $out (ref $bytes))
    (local $kind i32) (local $index i32) (local $tag i32) (local $sp i32) (local $meta i32)
    (global.set $ib (array.new_default $bytes (i32.const 64)))
    (global.set $il (i32.const 0))
    (global.set $it_values (array.new_default $values (i32.const 64)))
    (global.set $it_meta (array.new_default $frames (i32.const 64)))
    (global.set $it_sp (i32.const 0))
    (call $inspect_push (i32.const 0) (local.get $value) (i32.const 0))
    (block $done (loop $next
      (local.set $sp (global.get $it_sp))
      (br_if $done (i32.eqz (local.get $sp)))
      (local.set $sp (i32.sub (local.get $sp) (i32.const 1)))
      (global.set $it_sp (local.get $sp))
      (local.set $value (array.get $values (ref.as_non_null (global.get $it_values)) (local.get $sp)))
      (array.set $values (ref.as_non_null (global.get $it_values)) (local.get $sp) (ref.null eq))
      (local.set $meta (array.get $frames (ref.as_non_null (global.get $it_meta)) (local.get $sp)))
      (local.set $kind (i32.and (local.get $meta) (i32.const 3)))
      (local.set $index (i32.shr_u (local.get $meta) (i32.const 2)))

      ;; 1: the byte `index`.
      (if (i32.eq (local.get $kind) (i32.const 1))
        (then (call $ib_byte (local.get $index)) (br $next)))

      ;; 2: the parts of a tuple or a custom value, from `index` on.
      (if (i32.eq (local.get $kind) (i32.const 2))
        (then
          (local.set $fields (struct.get $obj 5 (ref.cast (ref $obj) (local.get $value))))
          (if (ref.is_null (local.get $fields)) (then (br $next)))
          (br_if $next (i32.ge_u (local.get $index) (array.len (ref.as_non_null (local.get $fields)))))
          (if (local.get $index) (then (call $ib_comma)))
          (call $inspect_push (i32.const 2) (local.get $value) (i32.add (local.get $index) (i32.const 1)))
          (call $inspect_push (i32.const 0)
            (array.get $values (ref.as_non_null (local.get $fields)) (local.get $index)) (i32.const 0))
          (br $next)))

      ;; 3: the rest of a list; `index` is 1 after its first part.
      (if (i32.eq (local.get $kind) (i32.const 3))
        (then
          (if (call $list_is_empty (local.get $value))
            (then (call $ib_byte (i32.const 93)) (br $next)))
          (if (local.get $index) (then (call $ib_comma)))
          (call $inspect_push (i32.const 3) (call $list_tail (local.get $value)) (i32.const 1))
          (call $inspect_push (i32.const 0) (call $list_head (local.get $value)) (i32.const 0))
          (br $next)))

      ;; 0: a value.
      (local.set $tag (call $value_tag (local.get $value)))
      (if (i32.or (i32.eq (local.get $tag) (i32.const 4)) (i32.eq (local.get $tag) (i32.const 5)))
        (then
          (local.set $obj (ref.cast (ref $obj) (local.get $value)))
          (local.set $fields (struct.get $obj 5 (local.get $obj)))
          (if (i32.eq (local.get $tag) (i32.const 5))
            (then
              ;; A constructor without fields is only its name: `Nil`, `True`.
              (call $ib_string (call $constructor_name (struct.get $obj 1 (local.get $obj))))
              (if (ref.is_null (local.get $fields)) (then (br $next)))
              (br_if $next (i32.eqz (array.len (ref.as_non_null (local.get $fields)))))))
          (call $ib_byte (i32.const 40))
          (call $inspect_push (i32.const 1) (ref.null eq) (i32.const 41))
          (call $inspect_push (i32.const 2) (local.get $value) (i32.const 0))
          (br $next)))
      (if (i32.eq (local.get $tag) (i32.const 6))
        (then
          (call $ib_byte (i32.const 91))
          (call $inspect_push (i32.const 3) (local.get $value) (i32.const 0))
          (br $next)))
      (call $ib_atom (local.get $value) (local.get $tag))
      (br $next)))
    (local.set $out (array.new_default $bytes (global.get $il)))
    (array.copy $bytes $bytes (local.get $out) (i32.const 0)
      (ref.as_non_null (global.get $ib)) (i32.const 0) (global.get $il))
    (global.set $ib (ref.null $bytes))
    (global.set $it_values (ref.null $values))
    (global.set $it_meta (ref.null $frames))
    (call $string_from_bytes (local.get $out)))

  (func $value_inspect (param $value eqref) (result eqref)
    (return_call $inspect (local.get $value)))

(func $float_to_string (param $value eqref) (result eqref)
  (call $f64_to_string (call $float_get (local.get $value))))

(func $f64_to_string (param $value f64) (result eqref)
  (local $bits i64) (local $fraction i64) (local $mantissa i64)
  (local $encoded_exp i32) (local $exp i32) (local $negative i32)
  (local $inclusive i32) (local $k i32) (local $n i32) (local $i i32)
  (local $digit i32) (local $comparison i32) (local $down i32) (local $up i32)
  (local $round_up i32) (local $decimal_exp i32) (local $lower i32) (local $upper i32)
  (local $scientific i32) (local $exponent i32) (local $power i32)
  (local $r eqref) (local $s eqref) (local $minus eqref) (local $plus eqref)
  (local $ten eqref) (local $tmp eqref) (local $shift eqref)
  (local $digits (ref null $bytes)) (local $out (ref null $bytes))
  (local $result (ref null $bytes)) (local $at i32)
  (local.set $bits (i64.reinterpret_f64 (local.get $value)))
  (local.set $negative (i32.wrap_i64 (i64.shr_u (local.get $bits) (i64.const 63))))
  (local.set $encoded_exp
    (i32.and (i32.wrap_i64 (i64.shr_u (local.get $bits) (i64.const 52))) (i32.const 2047)))
  (if (i32.eq (local.get $encoded_exp) (i32.const 2047)) (then unreachable))
  (local.set $fraction (i64.and (local.get $bits) (i64.const 4503599627370495)))
  (local.set $out (array.new_default $bytes (i32.const 64)))
  (if (local.get $negative)
    (then
      (array.set $bytes (local.get $out) (i32.const 0) (i32.const 45))
      (local.set $at (i32.const 1))))
  (if (f64.eq (local.get $value) (f64.const 0))
    (then
      (array.set $bytes (local.get $out) (local.get $at) (i32.const 48))
      (array.set $bytes (local.get $out) (i32.add (local.get $at) (i32.const 1)) (i32.const 46))
      (array.set $bytes (local.get $out) (i32.add (local.get $at) (i32.const 2)) (i32.const 48))
      (local.set $result (array.new_default $bytes (i32.add (local.get $at) (i32.const 3))))
      (array.copy $bytes $bytes (local.get $result) (i32.const 0)
        (local.get $out) (i32.const 0) (i32.add (local.get $at) (i32.const 3)))
      (return (call $string_from_bytes (ref.as_non_null (local.get $result))))))

  ;; Decode v = r * 2^exp, with midpoint distances minus/plus. At an
  ;; exact normal power of two, the preceding binade has half the spacing.
  (if (i32.eqz (local.get $encoded_exp))
    (then
      (local.set $mantissa (local.get $fraction))
      (local.set $r (call $int_from_i64 (i64.shl (local.get $mantissa) (i64.const 1))))
      (local.set $minus (ref.i31 (i32.const 1)))
      (local.set $plus (ref.i31 (i32.const 1)))
      (local.set $exp (i32.const -1075)))
    (else
      (local.set $mantissa (i64.or (local.get $fraction) (i64.const 4503599627370496)))
      (local.set $r (call $int_from_i64 (i64.shl (local.get $mantissa) (i64.const 2))))
      (local.set $plus (ref.i31 (i32.const 2)))
      (local.set $minus
        (ref.i31
          (if (result i32)
            (i32.and (i64.eqz (local.get $fraction))
                     (i32.gt_u (local.get $encoded_exp) (i32.const 1)))
            (then (i32.const 1)) (else (i32.const 2)))))
      (local.set $exp (i32.sub (local.get $encoded_exp) (i32.const 1077)))))
  (local.set $inclusive (i64.eqz (i64.and (local.get $mantissa) (i64.const 1))))
  (local.set $s (ref.i31 (i32.const 1)))
  (if (i32.lt_s (local.get $exp) (i32.const 0))
    (then
      (local.set $s
        (call $int_shl (local.get $s)
          (ref.i31 (i32.sub (i32.const 0) (local.get $exp))))))
    (else
      (local.set $shift (ref.i31 (local.get $exp)))
      (local.set $r (call $int_shl (local.get $r) (local.get $shift)))
      (local.set $minus (call $int_shl (local.get $minus) (local.get $shift)))
      (local.set $plus (call $int_shl (local.get $plus) (local.get $shift)))))
  (local.set $ten (ref.i31 (i32.const 10)))
  (local.set $k (i32.const 1))

  ;; Exact normalization: 1 <= r/s < 10. Multiplying numerator and both
  ;; margins together preserves the round-to-nearest acceptance interval.
  (block $large_done
    (loop $large
      (local.set $tmp (call $int_mul (local.get $s) (local.get $ten)))
      (br_if $large_done (i32.lt_s (call $int_compare (local.get $r) (local.get $tmp)) (i32.const 0)))
      (local.set $s (local.get $tmp))
      (local.set $k (i32.add (local.get $k) (i32.const 1)))
      (br $large)))
  (block $small_done
    (loop $small
      (br_if $small_done (i32.ge_s (call $int_compare (local.get $r) (local.get $s)) (i32.const 0)))
      (local.set $r (call $int_mul (local.get $r) (local.get $ten)))
      (local.set $minus (call $int_mul (local.get $minus) (local.get $ten)))
      (local.set $plus (call $int_mul (local.get $plus) (local.get $ten)))
      (local.set $k (i32.sub (local.get $k) (i32.const 1)))
      (br $small)))

  (local.set $digits (array.new_default $bytes (i32.const 32)))
  (block $digits_done
    (loop $generate
      (if (i32.ge_u (local.get $n) (i32.const 32)) (then unreachable))
      (local.set $digit
        (i32.wrap_i64 (call $int_to_i64 (call $int_div (local.get $r) (local.get $s)))))
      (local.set $r (call $int_rem (local.get $r) (local.get $s)))
      (array.set $bytes (local.get $digits) (local.get $n) (local.get $digit))
      (local.set $n (i32.add (local.get $n) (i32.const 1)))
      (local.set $comparison (call $int_compare (local.get $r) (local.get $minus)))
      (local.set $down
        (i32.or (i32.lt_s (local.get $comparison) (i32.const 0))
                (i32.and (local.get $inclusive) (i32.eqz (local.get $comparison)))))
      (local.set $comparison
        (call $int_compare (call $int_add (local.get $r) (local.get $plus)) (local.get $s)))
      (local.set $up
        (i32.or (i32.gt_s (local.get $comparison) (i32.const 0))
                (i32.and (local.get $inclusive) (i32.eqz (local.get $comparison)))))
      (br_if $digits_done (i32.or (local.get $down) (local.get $up)))
      (local.set $r (call $int_mul (local.get $r) (local.get $ten)))
      (local.set $minus (call $int_mul (local.get $minus) (local.get $ten)))
      (local.set $plus (call $int_mul (local.get $plus) (local.get $ten)))
      (br $generate)))

  ;; Where both shortest candidates are valid, choose the closer one. An
  ;; exact halfway decimal tie is resolved towards an even final digit.
  (local.set $comparison
    (call $int_compare (call $int_add (local.get $r) (local.get $r)) (local.get $s)))
  (local.set $round_up
    (i32.and (local.get $up)
      (i32.or (i32.eqz (local.get $down))
        (i32.or (i32.gt_s (local.get $comparison) (i32.const 0))
          (i32.and (i32.eqz (local.get $comparison))
                   (i32.and (local.get $digit) (i32.const 1)))))))
  (if (local.get $round_up)
    (then
      (local.set $i (local.get $n))
      (block $rounded
        (loop $carry
          (local.set $i (i32.sub (local.get $i) (i32.const 1)))
          (local.set $digit (array.get_u $bytes (local.get $digits) (local.get $i)))
          (if (i32.lt_u (local.get $digit) (i32.const 9))
            (then
              (array.set $bytes (local.get $digits) (local.get $i) (i32.add (local.get $digit) (i32.const 1)))
              (br $rounded)))
          (array.set $bytes (local.get $digits) (local.get $i) (i32.const 0))
          (if (i32.eqz (local.get $i))
            (then
              (array.set $bytes (local.get $digits) (i32.const 0) (i32.const 1))
              (local.set $k (i32.add (local.get $k) (i32.const 1)))
              (br $rounded)))
          (br $carry)))))
  (block $trim_done
    (loop $trim
      (br_if $trim_done (i32.le_u (local.get $n) (i32.const 1)))
      (br_if $trim_done
        (array.get_u $bytes (local.get $digits) (i32.sub (local.get $n) (i32.const 1))))
      (local.set $n (i32.sub (local.get $n) (i32.const 1)))
      (br $trim)))

  ;; The text form of a Float (spec §11.8): the shortest digits, in scientific
  ;; notation once integral decimal output would imply precision above 2^53.
  (local.set $decimal_exp (i32.sub (local.get $k) (local.get $n)))
  (local.set $lower
    (if (result i32) (i32.eq (local.get $n) (i32.const 1))
      (then (i32.const -4))
      (else (i32.sub (i32.const -2) (local.get $n)))))
  (local.set $upper
    (if (result i32)
      (i32.or (i32.eq (local.get $n) (i32.const 1))
              (i32.ge_s (local.get $k) (i32.const 11)))
      (then (i32.const 2)) (else (i32.const 1))))
  (local.set $scientific
    (i32.or
      (i32.or (i32.lt_s (local.get $decimal_exp) (local.get $lower))
              (i32.gt_s (local.get $decimal_exp) (local.get $upper)))
      (i32.and (i32.ge_s (local.get $decimal_exp) (i32.const 0))
               (f64.ge (f64.abs (local.get $value)) (f64.const 9007199254740992)))))
  (if (local.get $scientific)
    (then
      (array.set $bytes (local.get $out) (local.get $at)
        (i32.add (array.get_u $bytes (local.get $digits) (i32.const 0)) (i32.const 48)))
      (local.set $at (i32.add (local.get $at) (i32.const 1)))
      (array.set $bytes (local.get $out) (local.get $at) (i32.const 46))
      (local.set $at (i32.add (local.get $at) (i32.const 1)))
      (local.set $i (i32.const 1))
      (block $fraction_done
        (loop $fraction_digits
          (br_if $fraction_done (i32.ge_u (local.get $i) (local.get $n)))
          (array.set $bytes (local.get $out) (local.get $at)
            (i32.add (array.get_u $bytes (local.get $digits) (local.get $i)) (i32.const 48)))
          (local.set $at (i32.add (local.get $at) (i32.const 1)))
          (local.set $i (i32.add (local.get $i) (i32.const 1)))
          (br $fraction_digits)))
      (if (i32.eq (local.get $n) (i32.const 1))
        (then
          (array.set $bytes (local.get $out) (local.get $at) (i32.const 48))
          (local.set $at (i32.add (local.get $at) (i32.const 1)))))
      (array.set $bytes (local.get $out) (local.get $at) (i32.const 101))
      (local.set $at (i32.add (local.get $at) (i32.const 1)))
      (local.set $exponent (i32.sub (local.get $k) (i32.const 1)))
      (if (i32.lt_s (local.get $exponent) (i32.const 0))
        (then
          (array.set $bytes (local.get $out) (local.get $at) (i32.const 45))
          (local.set $at (i32.add (local.get $at) (i32.const 1)))
          (local.set $exponent (i32.sub (i32.const 0) (local.get $exponent)))))
      (local.set $power
        (if (result i32) (i32.ge_u (local.get $exponent) (i32.const 100))
          (then (i32.const 100))
          (else (if (result i32) (i32.ge_u (local.get $exponent) (i32.const 10))
            (then (i32.const 10)) (else (i32.const 1))))))
      (loop $exponent_digits
        (array.set $bytes (local.get $out) (local.get $at)
          (i32.add (i32.div_u (local.get $exponent) (local.get $power)) (i32.const 48)))
        (local.set $at (i32.add (local.get $at) (i32.const 1)))
        (local.set $exponent (i32.rem_u (local.get $exponent) (local.get $power)))
        (local.set $power (i32.div_u (local.get $power) (i32.const 10)))
        (br_if $exponent_digits (local.get $power))))
    (else
      (if (i32.le_s (local.get $k) (i32.const 0))
        (then
          (array.set $bytes (local.get $out) (local.get $at) (i32.const 48))
          (array.set $bytes (local.get $out) (i32.add (local.get $at) (i32.const 1)) (i32.const 46))
          (local.set $at (i32.add (local.get $at) (i32.const 2)))
          (local.set $i (local.get $k))
          (block $zeroes_done
            (loop $zeroes
              (br_if $zeroes_done (i32.ge_s (local.get $i) (i32.const 0)))
              (array.set $bytes (local.get $out) (local.get $at) (i32.const 48))
              (local.set $at (i32.add (local.get $at) (i32.const 1)))
              (local.set $i (i32.add (local.get $i) (i32.const 1)))
              (br $zeroes)))))
      (local.set $i (i32.const 0))
      (block $fixed_done
        (loop $fixed_digits
          (br_if $fixed_done (i32.ge_u (local.get $i) (local.get $n)))
          (if (i32.and (i32.gt_s (local.get $k) (i32.const 0))
                       (i32.eq (local.get $i) (local.get $k)))
            (then
              (array.set $bytes (local.get $out) (local.get $at) (i32.const 46))
              (local.set $at (i32.add (local.get $at) (i32.const 1)))))
          (array.set $bytes (local.get $out) (local.get $at)
            (i32.add (array.get_u $bytes (local.get $digits) (local.get $i)) (i32.const 48)))
          (local.set $at (i32.add (local.get $at) (i32.const 1)))
          (local.set $i (i32.add (local.get $i) (i32.const 1)))
          (br $fixed_digits)))
      (if (i32.ge_s (local.get $k) (local.get $n))
        (then
          (block $integer_zeroes_done
            (loop $integer_zeroes
              (br_if $integer_zeroes_done (i32.ge_s (local.get $i) (local.get $k)))
              (array.set $bytes (local.get $out) (local.get $at) (i32.const 48))
              (local.set $at (i32.add (local.get $at) (i32.const 1)))
              (local.set $i (i32.add (local.get $i) (i32.const 1)))
              (br $integer_zeroes)))
          (array.set $bytes (local.get $out) (local.get $at) (i32.const 46))
          (array.set $bytes (local.get $out) (i32.add (local.get $at) (i32.const 1)) (i32.const 48))
          (local.set $at (i32.add (local.get $at) (i32.const 2)))))))
  (local.set $result (array.new_default $bytes (local.get $at)))
  (array.copy $bytes $bytes (local.get $result) (i32.const 0)
    (local.get $out) (i32.const 0) (local.get $at))
  (call $string_from_bytes (ref.as_non_null (local.get $result))))

(global $shadow (mut (ref $frames)) (array.new_default $frames (i32.const 256)))
(global $sp (mut i32) (i32.const 0))
(global $site_table (mut (ref null $sites)) (ref.null $sites))
(global $site_modules (mut (ref null $values)) (ref.null $values))
(global $stopping (mut i32) (i32.const 0))

(func $sites_install (param $table (ref $sites)) (param $modules (ref $values))
  (global.set $site_table (local.get $table))
  (global.set $site_modules (local.get $modules)))

(func $shadow_grow
  (local $old (ref $frames)) (local $new (ref $frames)) (local $size i32)
  (local.set $old (global.get $shadow))
  (local.set $size (array.len (local.get $old)))
  ;; 2^28 frames is far beyond any Wasm call stack; trap as out of stack.
  (if (i32.ge_u (local.get $size) (i32.const 268435456)) (then unreachable))
  (local.set $new (array.new_default $frames (i32.shl (local.get $size) (i32.const 1))))
  (array.copy $frames $frames (local.get $new) (i32.const 0) (local.get $old) (i32.const 0) (local.get $size))
  (global.set $shadow (local.get $new)))

(func $shadow_enter (param $site i32)
  (local $sp i32)
  (local.set $sp (global.get $sp))
  (if (i32.ge_u (local.get $sp) (array.len (global.get $shadow))) (then (call $shadow_grow)))
  (array.set $frames (global.get $shadow) (local.get $sp) (local.get $site))
  (global.set $sp (i32.add (local.get $sp) (i32.const 1))))

(func $shadow_leave
  (global.set $sp (i32.sub (global.get $sp) (i32.const 1))))

;; A tail site (spec §11.7) is kept as `-2 - site`: the position of a call in
;; tail position. It is printed only when no position is printed before it.
;; `$tail` holds the tail site of the tail call being made, until the callee
;; enters with `$shadow_enter_call`; -1 when there is none.
(global $tail (mut i32) (i32.const -1))

;; The entry of a function: its slot holds the tail site of the tail call
;; that called it, if there is one.
(func $shadow_enter_call
  (call $shadow_enter (global.get $tail))
  (global.set $tail (i32.const -1)))

;; Before a tail call: the frame is gone, and the tail site goes to the
;; callee. Library code has no site (-1): it passes on its own tail site.
(func $shadow_leave_tail (param $site i32)
  (local $sp i32)
  (local.set $sp (i32.sub (global.get $sp) (i32.const 1)))
  (if (i32.eq (local.get $site) (i32.const -1))
    (then
      (local.set $site (array.get $frames (global.get $shadow) (local.get $sp)))
      (if (i32.gt_s (local.get $site) (i32.const -2)) (then (local.set $site (i32.const -1))))))
  (global.set $tail (local.get $site))
  (global.set $sp (local.get $sp)))

(func $shadow_site (param $site i32)
  (if (i32.eqz (global.get $sp)) (then (return_call $shadow_enter (local.get $site))))
  (array.set $frames (global.get $shadow) (i32.sub (global.get $sp) (i32.const 1)) (local.get $site)))

;; The slot of the innermost frame. Only called inside a frame.
(func $shadow_top (result i32)
  (array.get $frames (global.get $shadow) (i32.sub (global.get $sp) (i32.const 1))))

(func $shadow_depth (result i32) (global.get $sp))

(func $shadow_restore (param $depth i32) (global.set $sp (local.get $depth)))

;; `module path:line:column`, or null when the site is not in the table.
(func $site_text (param $site i32) (result eqref)
  (local $table (ref null $sites)) (local $modules (ref null $values))
  (local $at i32) (local $module i32)
  (local.set $table (global.get $site_table))
  (local.set $modules (global.get $site_modules))
  (if (i32.or (ref.is_null (local.get $table)) (ref.is_null (local.get $modules)))
    (then (return (ref.null eq))))
  (if (i32.lt_s (local.get $site) (i32.const 0)) (then (return (ref.null eq))))
  (if (i32.ge_u (local.get $site) (i32.div_u (array.len (local.get $table)) (i32.const 3)))
    (then (return (ref.null eq))))
  (local.set $at (i32.mul (local.get $site) (i32.const 3)))
  (local.set $module (array.get $sites (local.get $table) (local.get $at)))
  (if (i32.ge_u (local.get $module) (array.len (local.get $modules)))
    (then (return (ref.null eq))))
  (return_call $site_line (local.get $site)))

;; Callers check the site first; see $site_text.
(func $site_line (param $site i32) (result eqref)
  (local $table (ref $sites)) (local $at i32)
  (local.set $table (ref.as_non_null (global.get $site_table)))
  (local.set $at (i32.mul (local.get $site) (i32.const 3)))
  (call $string_concat
    (call $list_cons
      (array.get $values (ref.as_non_null (global.get $site_modules))
        (array.get $sites (local.get $table) (local.get $at)))
    (call $list_cons (call $string_from_bytes (array.new_fixed $bytes 1 (i32.const 58)))
    (call $list_cons (call $int_to_string (call $int_from_i64 (i64.extend_i32_u
      (array.get $sites (local.get $table) (i32.add (local.get $at) (i32.const 1))))))
    (call $list_cons (call $string_from_bytes (array.new_fixed $bytes 1 (i32.const 58)))
    (call $list_cons (call $int_to_string (call $int_from_i64 (i64.extend_i32_u
      (array.get $sites (local.get $table) (i32.add (local.get $at) (i32.const 2))))))
    (call $list_nil))))))))

;; Append a line for the site to the reversed list of parts.
(func $stop_add_site (param $parts eqref) (param $site i32) (result eqref)
  (local $line eqref)
  (local.set $line (call $site_text (local.get $site)))
  (if (ref.is_null (local.get $line)) (then (return (local.get $parts))))
  (call $list_cons (local.get $line) (call $list_cons (call $string_from_bytes (array.new_fixed $bytes 1 (i32.const 10))) (local.get $parts))))

;; Copy the message into memory, give it to the host once, and trap.
(func $stop_send (param $message eqref)
  (local $length i32)
  (local.set $length (call $string_length_bytes (local.get $message)))
  (call $host_panic (call $string_to_mem (local.get $message)) (local.get $length))
  unreachable)

;; The reason, then every position on the shadow stack, innermost first.
(func $stop (param $reason eqref)
  (local $parts eqref) (local $i i32) (local $site i32) (local $printed i32)
  (if (global.get $stopping) (then unreachable))
  (global.set $stopping (i32.const 1))
  (local.set $parts (call $list_cons (local.get $reason) (call $list_nil)))
  ;; A tail call whose callee has not entered yet.
  (if (i32.le_s (global.get $tail) (i32.const -2))
    (then
      (local.set $parts (call $stop_add_site (local.get $parts)
        (i32.sub (i32.const -2) (global.get $tail))))
      (local.set $printed (i32.const 1))))
  (local.set $i (global.get $sp))
  (if (i32.gt_u (local.get $i) (array.len (global.get $shadow)))
    (then (local.set $i (array.len (global.get $shadow)))))
  (block $done (loop $each
    (br_if $done (i32.eqz (local.get $i)))
    (local.set $i (i32.sub (local.get $i) (i32.const 1)))
    (local.set $site (array.get $frames (global.get $shadow) (local.get $i)))
    ;; A tail site is only the first position (§11.7).
    (if (i32.le_s (local.get $site) (i32.const -2))
      (then
        (local.set $site (if (result i32) (local.get $printed)
          (then (i32.const -1))
          (else (i32.sub (i32.const -2) (local.get $site)))))))
    (if (i32.ge_s (local.get $site) (i32.const 0))
      (then
        (local.set $parts (call $stop_add_site (local.get $parts) (local.get $site)))
        (local.set $printed (i32.const 1))))
    (br $each)))
  (call $stop_send (call $string_concat (call $list_reverse (local.get $parts))))
  unreachable)

;; The reason and one position.
(func $stop_at (param $reason eqref) (param $site i32)
  (if (global.get $stopping) (then unreachable))
  (global.set $stopping (i32.const 1))
  (call $stop_send (call $string_concat (call $list_reverse
    (call $stop_add_site (call $list_cons (local.get $reason) (call $list_nil)) (local.get $site)))))
  unreachable)

(func $panic (param $message eqref)
  (call $stop (call $string_append (call $string_from_bytes (array.new_fixed $bytes 7 (i32.const 112) (i32.const 97) (i32.const 110) (i32.const 105) (i32.const 99) (i32.const 58) (i32.const 32))) (local.get $message))) unreachable)
(func $stop_assertion (param $message eqref)
  (call $stop (call $string_append (call $string_from_bytes (array.new_fixed $bytes 18 (i32.const 97) (i32.const 115) (i32.const 115) (i32.const 101) (i32.const 114) (i32.const 116) (i32.const 105) (i32.const 111) (i32.const 110) (i32.const 32) (i32.const 102) (i32.const 97) (i32.const 105) (i32.const 108) (i32.const 101) (i32.const 100) (i32.const 58) (i32.const 32))) (local.get $message))) unreachable)
(func $stop_error (param $error eqref) (param $site i32)
  (call $stop_at (call $string_append (call $string_from_bytes (array.new_fixed $bytes 7 (i32.const 101) (i32.const 114) (i32.const 114) (i32.const 111) (i32.const 114) (i32.const 58) (i32.const 32))) (call $inspect (local.get $error))) (local.get $site))
  unreachable)
(func $stop_boundary_inactive (call $stop (call $string_from_bytes (array.new_fixed $bytes 31 (i32.const 99) (i32.const 111) (i32.const 110) (i32.const 116) (i32.const 114) (i32.const 111) (i32.const 108) (i32.const 58) (i32.const 32) (i32.const 98) (i32.const 111) (i32.const 117) (i32.const 110) (i32.const 100) (i32.const 97) (i32.const 114) (i32.const 121) (i32.const 32) (i32.const 105) (i32.const 115) (i32.const 32) (i32.const 110) (i32.const 111) (i32.const 116) (i32.const 32) (i32.const 97) (i32.const 99) (i32.const 116) (i32.const 105) (i32.const 118) (i32.const 101)))) unreachable)
(func $stop_continuation_used (call $stop (call $string_from_bytes (array.new_fixed $bytes 38 (i32.const 99) (i32.const 111) (i32.const 110) (i32.const 116) (i32.const 114) (i32.const 111) (i32.const 108) (i32.const 58) (i32.const 32) (i32.const 99) (i32.const 111) (i32.const 110) (i32.const 116) (i32.const 105) (i32.const 110) (i32.const 117) (i32.const 97) (i32.const 116) (i32.const 105) (i32.const 111) (i32.const 110) (i32.const 32) (i32.const 119) (i32.const 97) (i32.const 115) (i32.const 32) (i32.const 97) (i32.const 108) (i32.const 114) (i32.const 101) (i32.const 97) (i32.const 100) (i32.const 121) (i32.const 32) (i32.const 117) (i32.const 115) (i32.const 101) (i32.const 100)))) unreachable)
(func $stop_float (call $stop (call $string_from_bytes (array.new_fixed $bytes 27 (i32.const 102) (i32.const 108) (i32.const 111) (i32.const 97) (i32.const 116) (i32.const 58) (i32.const 32) (i32.const 114) (i32.const 101) (i32.const 115) (i32.const 117) (i32.const 108) (i32.const 116) (i32.const 32) (i32.const 105) (i32.const 115) (i32.const 32) (i32.const 110) (i32.const 111) (i32.const 116) (i32.const 32) (i32.const 102) (i32.const 105) (i32.const 110) (i32.const 105) (i32.const 116) (i32.const 101)))) unreachable)
(func $stop_bits_short (call $stop (call $string_from_bytes (array.new_fixed $bytes 29 (i32.const 98) (i32.const 105) (i32.const 116) (i32.const 32) (i32.const 97) (i32.const 114) (i32.const 114) (i32.const 97) (i32.const 121) (i32.const 58) (i32.const 32) (i32.const 118) (i32.const 97) (i32.const 108) (i32.const 117) (i32.const 101) (i32.const 32) (i32.const 105) (i32.const 115) (i32.const 32) (i32.const 116) (i32.const 111) (i32.const 111) (i32.const 32) (i32.const 115) (i32.const 104) (i32.const 111) (i32.const 114) (i32.const 116)))) unreachable)
(func $stop_bits_negative (call $stop (call $string_from_bytes (array.new_fixed $bytes 29 (i32.const 98) (i32.const 105) (i32.const 116) (i32.const 32) (i32.const 97) (i32.const 114) (i32.const 114) (i32.const 97) (i32.const 121) (i32.const 58) (i32.const 32) (i32.const 108) (i32.const 101) (i32.const 110) (i32.const 103) (i32.const 116) (i32.const 104) (i32.const 32) (i32.const 105) (i32.const 115) (i32.const 32) (i32.const 110) (i32.const 101) (i32.const 103) (i32.const 97) (i32.const 116) (i32.const 105) (i32.const 118) (i32.const 101)))) unreachable)
(func $stop_bits_not_whole (call $stop (call $string_from_bytes (array.new_fixed $bytes 30 (i32.const 98) (i32.const 105) (i32.const 116) (i32.const 32) (i32.const 97) (i32.const 114) (i32.const 114) (i32.const 97) (i32.const 121) (i32.const 58) (i32.const 32) (i32.const 98) (i32.const 121) (i32.const 116) (i32.const 101) (i32.const 115) (i32.const 32) (i32.const 97) (i32.const 114) (i32.const 101) (i32.const 32) (i32.const 110) (i32.const 111) (i32.const 116) (i32.const 32) (i32.const 119) (i32.const 104) (i32.const 111) (i32.const 108) (i32.const 101)))) unreachable)
(func $stop_bits_float (call $stop (call $string_from_bytes (array.new_fixed $bytes 30 (i32.const 98) (i32.const 105) (i32.const 116) (i32.const 32) (i32.const 97) (i32.const 114) (i32.const 114) (i32.const 97) (i32.const 121) (i32.const 58) (i32.const 32) (i32.const 102) (i32.const 108) (i32.const 111) (i32.const 97) (i32.const 116) (i32.const 32) (i32.const 105) (i32.const 115) (i32.const 32) (i32.const 110) (i32.const 111) (i32.const 116) (i32.const 32) (i32.const 102) (i32.const 105) (i32.const 110) (i32.const 105) (i32.const 116) (i32.const 101)))) unreachable)
(func $stop_host_value (call $stop (call $string_from_bytes (array.new_fixed $bytes 26 (i32.const 104) (i32.const 111) (i32.const 115) (i32.const 116) (i32.const 58) (i32.const 32) (i32.const 118) (i32.const 97) (i32.const 108) (i32.const 117) (i32.const 101) (i32.const 32) (i32.const 105) (i32.const 115) (i32.const 32) (i32.const 110) (i32.const 111) (i32.const 116) (i32.const 32) (i32.const 99) (i32.const 111) (i32.const 114) (i32.const 114) (i32.const 101) (i32.const 99) (i32.const 116)))) unreachable)

;; Float operators. $float_new stops when a result is not finite.
(func $float_add (param $a eqref) (param $b eqref) (result eqref)
  (call $float_new (f64.add (call $float_get (local.get $a)) (call $float_get (local.get $b)))))
(func $float_sub (param $a eqref) (param $b eqref) (result eqref)
  (call $float_new (f64.sub (call $float_get (local.get $a)) (call $float_get (local.get $b)))))
(func $float_mul (param $a eqref) (param $b eqref) (result eqref)
  (call $float_new (f64.mul (call $float_get (local.get $a)) (call $float_get (local.get $b)))))
;; a /. 0.0 is 0.0, also for -0.0 (spec §11.3).
(func $float_div (param $a eqref) (param $b eqref) (result eqref)
  (if (result eqref) (f64.eq (call $float_get (local.get $b)) (f64.const 0))
    (then (call $float_new (f64.const 0)))
    (else (call $float_new (f64.div (call $float_get (local.get $a)) (call $float_get (local.get $b)))))))
(func $float_neg (param $a eqref) (result eqref)
  (call $float_new (f64.neg (call $float_get (local.get $a)))))

;; Ok and Error: constructors 0 and 1 of the type Result (type id 0).
(func $result_ok (param $v eqref) (result eqref)
  (call $record (i32.const 0) (i64.const 0) (array.new_fixed $values 1 (local.get $v))))
(func $result_error (param $v eqref) (result eqref)
  (call $record (i32.const 1) (i64.const 0) (array.new_fixed $values 1 (local.get $v))))

(func $core_not (param $v eqref) (result eqref)
  (call $bool_new (i32.eqz (call $bool_get (local.get $v)))))
(func $core_to_float (param $v eqref) (result eqref)
  (call $int_to_float (local.get $v)))
(func $core_truncate (param $v eqref) (result eqref)
  (call $float_truncate (local.get $v)))
(func $core_floor (param $v eqref) (result eqref)
  (call $float_new (f64.floor (call $float_get (local.get $v)))))
(func $core_ceiling (param $v eqref) (result eqref)
  (call $float_new (f64.ceil (call $float_get (local.get $v)))))
;; The square root of a negative number is NaN, which stops.
(func $core_sqrt (param $v eqref) (result eqref)
  (call $float_new (f64.sqrt (call $float_get (local.get $v)))))
(func $core_float_to_bits (param $v eqref) (result eqref)
  (call $float_to_bits (local.get $v)))
(func $core_float_from_bits (param $v eqref) (result eqref)
  (call $float_from_bits (local.get $v)))
(func $core_bit_and (param $a eqref) (param $b eqref) (result eqref)
  (call $int_and (local.get $a) (local.get $b)))
(func $core_bit_or (param $a eqref) (param $b eqref) (result eqref)
  (call $int_or (local.get $a) (local.get $b)))
(func $core_bit_xor (param $a eqref) (param $b eqref) (result eqref)
  (call $int_xor (local.get $a) (local.get $b)))
(func $core_bit_not (param $a eqref) (result eqref)
  (call $int_not (local.get $a)))
(func $core_shift_left (param $v eqref) (param $n eqref) (result eqref)
  (call $int_shl (local.get $v) (local.get $n)))
(func $core_shift_right (param $v eqref) (param $n eqref) (result eqref)
  (call $int_shr (local.get $v) (local.get $n)))
(func $core_concat (param $list eqref) (result eqref)
  (call $string_concat (local.get $list)))
(func $core_byte_size (param $s eqref) (result eqref)
  (call $string_byte_size (local.get $s)))

;; Error(Nil) when a byte is not in the string, or when the start or the end
;; is inside a character. Any Int may come in, also negative and huge ones.
(func $core_byte_slice (param $s eqref) (param $start eqref) (param $length eqref) (result eqref)
  (local $b (ref $bytes)) (local $size i64) (local $from i64) (local $count i64)
  (local.set $b (call $string_bytes (local.get $s)))
  (local.set $size (i64.extend_i32_u (array.len (local.get $b))))
  (if (i32.or (i32.lt_s (call $int_sign (local.get $start)) (i32.const 0))
               (i32.lt_s (call $int_sign (local.get $length)) (i32.const 0)))
    (then (return (call $result_error (ref.null eq)))))
  (if (i32.or
        (i32.gt_s (call $int_compare (local.get $start) (call $int_from_i64 (local.get $size))) (i32.const 0))
        (i32.gt_s (call $int_compare (local.get $length) (call $int_from_i64 (local.get $size))) (i32.const 0)))
    (then (return (call $result_error (ref.null eq)))))
  (local.set $from (call $int_to_i64 (local.get $start)))
  (local.set $count (call $int_to_i64 (local.get $length)))
  (if (i64.gt_u (i64.add (local.get $from) (local.get $count)) (local.get $size))
    (then (return (call $result_error (ref.null eq)))))
  (if (i32.eqz (i32.and
        (call $string_boundary (local.get $b) (i32.wrap_i64 (local.get $from)))
        (call $string_boundary (local.get $b) (i32.wrap_i64 (i64.add (local.get $from) (local.get $count))))))
    (then (return (call $result_error (ref.null eq)))))
  (call $result_ok (call $string_slice_bytes (local.get $s)
    (i32.wrap_i64 (local.get $from)) (i32.wrap_i64 (local.get $count)))))

(func $core_codepoints (param $s eqref) (result eqref)
  (call $string_codepoints (local.get $s)))
(func $core_from_codepoints (param $list eqref) (result eqref)
  (call $string_from_codepoints (local.get $list)))
(func $core_codepoint_to_int (param $c eqref) (result eqref)
  (call $int_from_i64 (i64.extend_i32_u (call $codepoint_get (local.get $c)))))
;; Error(Nil) unless the number is a Unicode scalar value.
(func $core_int_to_codepoint (param $n eqref) (result eqref)
  (local $value i32)
  (if (i32.eqz (ref.test (ref i31) (local.get $n))) (then (return (call $result_error (ref.null eq)))))
  (local.set $value (i31.get_s (ref.cast (ref i31) (local.get $n))))
  (if (i32.or (i32.or (i32.lt_s (local.get $value) (i32.const 0))
                      (i32.gt_s (local.get $value) (i32.const 1114111)))
              (i32.and (i32.ge_s (local.get $value) (i32.const 55296))
                       (i32.le_s (local.get $value) (i32.const 57343))))
    (then (return (call $result_error (ref.null eq)))))
  (call $result_ok (call $codepoint_new (local.get $value))))
(func $core_bit_size (param $b eqref) (result eqref)
  (call $bits_size (local.get $b)))
;; Error(Nil) unless the bits are whole bytes of correct UTF-8.
(func $core_bit_array_to_string (param $value eqref) (result eqref)
  (local $length i64) (local $data (ref $bytes)) (local $count i32) (local $index i32)
  (local.set $length (call $bits_len64 (local.get $value)))
  (if (i64.ne (i64.and (local.get $length) (i64.const 7)) (i64.const 0))
    (then (return (call $result_error (ref.null eq)))))
  (local.set $count (i32.wrap_i64 (i64.shr_u (local.get $length) (i64.const 3))))
  (local.set $data (array.new_default $bytes (local.get $count)))
  (block $copied (loop $copy
    (br_if $copied (i32.ge_u (local.get $index) (local.get $count)))
    (array.set $bytes (local.get $data) (local.get $index)
      (call $bits_byte (local.get $value) (i64.shl (i64.extend_i32_u (local.get $index)) (i64.const 3))))
    (local.set $index (i32.add (local.get $index) (i32.const 1)))
    (br $copy)))
  (if (i32.eqz (call $utf8_valid (local.get $data)))
    (then (return (call $result_error (ref.null eq)))))
  (call $result_ok (call $string_from_bytes (local.get $data))))
(func $core_inspect (param $v eqref) (result eqref)
  (call $inspect (local.get $v)))
(func $core_hash (param $v eqref) (result eqref)
  (call $value_hash (local.get $v)))
;; Simple case mappings of Unicode 18.0.0.
(type $case_ranges (array i32))

(data $case_upper_data "\61\00\00\00\7a\00\00\00\e0\ff\ff\ff\b5\00\00\00\b5\00\00\00\e7\02\00\00\e0\00\00\00\f6\00\00\00\e0\ff\ff\ff\f8\00\00\00\fe\00\00\00\e0\ff\ff\ff\ff\00\00\00\ff\00\00\00\79\00\00\00\01\01\00\00\2f\01\00\01\ff\ff\ff\ff\31\01\00\00\31\01\00\00\18\ff\ff\ff\33\01\00\00\37\01\00\01\ff\ff\ff\ff\3a\01\00\00\48\01\00\01\ff\ff\ff\ff\4b\01\00\00\77\01\00\01\ff\ff\ff\ff\7a\01\00\00\7e\01\00\01\ff\ff\ff\ff\7f\01\00\00\7f\01\00\00\d4\fe\ff\ff\80\01\00\00\80\01\00\00\c3\00\00\00\83\01\00\00\85\01\00\01\ff\ff\ff\ff\88\01\00\00\88\01\00\00\ff\ff\ff\ff\8c\01\00\00\8c\01\00\00\ff\ff\ff\ff\92\01\00\00\92\01\00\00\ff\ff\ff\ff\95\01\00\00\95\01\00\00\61\00\00\00\99\01\00\00\99\01\00\00\ff\ff\ff\ff\9a\01\00\00\9a\01\00\00\a3\00\00\00\9b\01\00\00\9b\01\00\00\41\a6\00\00\9e\01\00\00\9e\01\00\00\82\00\00\00\a1\01\00\00\a5\01\00\01\ff\ff\ff\ff\a8\01\00\00\a8\01\00\00\ff\ff\ff\ff\ad\01\00\00\ad\01\00\00\ff\ff\ff\ff\b0\01\00\00\b0\01\00\00\ff\ff\ff\ff\b4\01\00\00\b6\01\00\01\ff\ff\ff\ff\b9\01\00\00\b9\01\00\00\ff\ff\ff\ff\bd\01\00\00\bd\01\00\00\ff\ff\ff\ff\bf\01\00\00\bf\01\00\00\38\00\00\00\c5\01\00\00\c5\01\00\00\ff\ff\ff\ff\c6\01\00\00\c6\01\00\00\fe\ff\ff\ff\c8\01\00\00\c8\01\00\00\ff\ff\ff\ff\c9\01\00\00\c9\01\00\00\fe\ff\ff\ff\cb\01\00\00\cb\01\00\00\ff\ff\ff\ff\cc\01\00\00\cc\01\00\00\fe\ff\ff\ff\ce\01\00\00\dc\01\00\01\ff\ff\ff\ff\dd\01\00\00\dd\01\00\00\b1\ff\ff\ff\df\01\00\00\ef\01\00\01\ff\ff\ff\ff\f2\01\00\00\f2\01\00\00\ff\ff\ff\ff\f3\01\00\00\f3\01\00\00\fe\ff\ff\ff\f5\01\00\00\f5\01\00\00\ff\ff\ff\ff\f9\01\00\00\1f\02\00\01\ff\ff\ff\ff\23\02\00\00\33\02\00\01\ff\ff\ff\ff\3c\02\00\00\3c\02\00\00\ff\ff\ff\ff\3f\02\00\00\40\02\00\00\3f\2a\00\00\42\02\00\00\42\02\00\00\ff\ff\ff\ff\47\02\00\00\4f\02\00\01\ff\ff\ff\ff\50\02\00\00\50\02\00\00\1f\2a\00\00\51\02\00\00\51\02\00\00\1c\2a\00\00\52\02\00\00\52\02\00\00\1e\2a\00\00\53\02\00\00\53\02\00\00\2e\ff\ff\ff\54\02\00\00\54\02\00\00\32\ff\ff\ff\56\02\00\00\57\02\00\00\33\ff\ff\ff\59\02\00\00\59\02\00\00\36\ff\ff\ff\5b\02\00\00\5b\02\00\00\35\ff\ff\ff\5c\02\00\00\5c\02\00\00\4f\a5\00\00\60\02\00\00\60\02\00\00\33\ff\ff\ff\61\02\00\00\61\02\00\00\4b\a5\00\00\63\02\00\00\63\02\00\00\31\ff\ff\ff\64\02\00\00\64\02\00\00\67\a5\00\00\65\02\00\00\65\02\00\00\28\a5\00\00\66\02\00\00\66\02\00\00\44\a5\00\00\68\02\00\00\68\02\00\00\2f\ff\ff\ff\69\02\00\00\69\02\00\00\2d\ff\ff\ff\6a\02\00\00\6a\02\00\00\44\a5\00\00\6b\02\00\00\6b\02\00\00\f7\29\00\00\6c\02\00\00\6c\02\00\00\41\a5\00\00\6f\02\00\00\6f\02\00\00\2d\ff\ff\ff\71\02\00\00\71\02\00\00\fd\29\00\00\72\02\00\00\72\02\00\00\2b\ff\ff\ff\75\02\00\00\75\02\00\00\2a\ff\ff\ff\77\02\00\00\77\02\00\00\66\a5\00\00\7c\02\00\00\7c\02\00\00\66\a5\00\00\7d\02\00\00\7d\02\00\00\e7\29\00\00\80\02\00\00\80\02\00\00\26\ff\ff\ff\82\02\00\00\82\02\00\00\43\a5\00\00\83\02\00\00\83\02\00\00\26\ff\ff\ff\87\02\00\00\87\02\00\00\2a\a5\00\00\88\02\00\00\88\02\00\00\26\ff\ff\ff\89\02\00\00\89\02\00\00\bb\ff\ff\ff\8a\02\00\00\8b\02\00\00\27\ff\ff\ff\8c\02\00\00\8c\02\00\00\b9\ff\ff\ff\92\02\00\00\92\02\00\00\25\ff\ff\ff\9d\02\00\00\9d\02\00\00\15\a5\00\00\9e\02\00\00\9e\02\00\00\12\a5\00\00\45\03\00\00\45\03\00\00\54\00\00\00\71\03\00\00\73\03\00\01\ff\ff\ff\ff\77\03\00\00\77\03\00\00\ff\ff\ff\ff\7b\03\00\00\7d\03\00\00\82\00\00\00\ac\03\00\00\ac\03\00\00\da\ff\ff\ff\ad\03\00\00\af\03\00\00\db\ff\ff\ff\b1\03\00\00\c1\03\00\00\e0\ff\ff\ff\c2\03\00\00\c2\03\00\00\e1\ff\ff\ff\c3\03\00\00\cb\03\00\00\e0\ff\ff\ff\cc\03\00\00\cc\03\00\00\c0\ff\ff\ff\cd\03\00\00\ce\03\00\00\c1\ff\ff\ff\d0\03\00\00\d0\03\00\00\c2\ff\ff\ff\d1\03\00\00\d1\03\00\00\c7\ff\ff\ff\d5\03\00\00\d5\03\00\00\d1\ff\ff\ff\d6\03\00\00\d6\03\00\00\ca\ff\ff\ff\d7\03\00\00\d7\03\00\00\f8\ff\ff\ff\d9\03\00\00\ef\03\00\01\ff\ff\ff\ff\f0\03\00\00\f0\03\00\00\aa\ff\ff\ff\f1\03\00\00\f1\03\00\00\b0\ff\ff\ff\f2\03\00\00\f2\03\00\00\07\00\00\00\f3\03\00\00\f3\03\00\00\8c\ff\ff\ff\f5\03\00\00\f5\03\00\00\a0\ff\ff\ff\f8\03\00\00\f8\03\00\00\ff\ff\ff\ff\fb\03\00\00\fb\03\00\00\ff\ff\ff\ff\30\04\00\00\4f\04\00\00\e0\ff\ff\ff\50\04\00\00\5f\04\00\00\b0\ff\ff\ff\61\04\00\00\81\04\00\01\ff\ff\ff\ff\8b\04\00\00\bf\04\00\01\ff\ff\ff\ff\c2\04\00\00\ce\04\00\01\ff\ff\ff\ff\cf\04\00\00\cf\04\00\00\f1\ff\ff\ff\d1\04\00\00\2f\05\00\01\ff\ff\ff\ff\61\05\00\00\86\05\00\00\d0\ff\ff\ff\d0\10\00\00\fa\10\00\00\c0\0b\00\00\fd\10\00\00\ff\10\00\00\c0\0b\00\00\f8\13\00\00\fd\13\00\00\f8\ff\ff\ff\80\1c\00\00\80\1c\00\00\92\e7\ff\ff\81\1c\00\00\81\1c\00\00\93\e7\ff\ff\82\1c\00\00\82\1c\00\00\9c\e7\ff\ff\83\1c\00\00\84\1c\00\00\9e\e7\ff\ff\85\1c\00\00\85\1c\00\00\9d\e7\ff\ff\86\1c\00\00\86\1c\00\00\a4\e7\ff\ff\87\1c\00\00\87\1c\00\00\db\e7\ff\ff\88\1c\00\00\88\1c\00\00\c2\89\00\00\8a\1c\00\00\8a\1c\00\00\ff\ff\ff\ff\79\1d\00\00\79\1d\00\00\04\8a\00\00\7d\1d\00\00\7d\1d\00\00\e6\0e\00\00\8e\1d\00\00\8e\1d\00\00\38\8a\00\00\01\1e\00\00\95\1e\00\01\ff\ff\ff\ff\9b\1e\00\00\9b\1e\00\00\c5\ff\ff\ff\a1\1e\00\00\ff\1e\00\01\ff\ff\ff\ff\00\1f\00\00\07\1f\00\00\08\00\00\00\10\1f\00\00\15\1f\00\00\08\00\00\00\20\1f\00\00\27\1f\00\00\08\00\00\00\30\1f\00\00\37\1f\00\00\08\00\00\00\40\1f\00\00\45\1f\00\00\08\00\00\00\51\1f\00\00\57\1f\00\01\08\00\00\00\60\1f\00\00\67\1f\00\00\08\00\00\00\70\1f\00\00\71\1f\00\00\4a\00\00\00\72\1f\00\00\75\1f\00\00\56\00\00\00\76\1f\00\00\77\1f\00\00\64\00\00\00\78\1f\00\00\79\1f\00\00\80\00\00\00\7a\1f\00\00\7b\1f\00\00\70\00\00\00\7c\1f\00\00\7d\1f\00\00\7e\00\00\00\80\1f\00\00\87\1f\00\00\08\00\00\00\90\1f\00\00\97\1f\00\00\08\00\00\00\a0\1f\00\00\a7\1f\00\00\08\00\00\00\b0\1f\00\00\b1\1f\00\00\08\00\00\00\b3\1f\00\00\b3\1f\00\00\09\00\00\00\be\1f\00\00\be\1f\00\00\db\e3\ff\ff\c3\1f\00\00\c3\1f\00\00\09\00\00\00\d0\1f\00\00\d1\1f\00\00\08\00\00\00\e0\1f\00\00\e1\1f\00\00\08\00\00\00\e5\1f\00\00\e5\1f\00\00\07\00\00\00\f3\1f\00\00\f3\1f\00\00\09\00\00\00\4e\21\00\00\4e\21\00\00\e4\ff\ff\ff\70\21\00\00\7f\21\00\00\f0\ff\ff\ff\84\21\00\00\84\21\00\00\ff\ff\ff\ff\d0\24\00\00\e9\24\00\00\e6\ff\ff\ff\30\2c\00\00\5f\2c\00\00\d0\ff\ff\ff\61\2c\00\00\61\2c\00\00\ff\ff\ff\ff\65\2c\00\00\65\2c\00\00\d5\d5\ff\ff\66\2c\00\00\66\2c\00\00\d8\d5\ff\ff\68\2c\00\00\6c\2c\00\01\ff\ff\ff\ff\73\2c\00\00\73\2c\00\00\ff\ff\ff\ff\76\2c\00\00\76\2c\00\00\ff\ff\ff\ff\81\2c\00\00\e3\2c\00\01\ff\ff\ff\ff\ec\2c\00\00\ee\2c\00\01\ff\ff\ff\ff\f3\2c\00\00\f3\2c\00\00\ff\ff\ff\ff\00\2d\00\00\25\2d\00\00\a0\e3\ff\ff\27\2d\00\00\27\2d\00\00\a0\e3\ff\ff\2d\2d\00\00\2d\2d\00\00\a0\e3\ff\ff\41\a6\00\00\6d\a6\00\01\ff\ff\ff\ff\81\a6\00\00\9b\a6\00\01\ff\ff\ff\ff\23\a7\00\00\2f\a7\00\01\ff\ff\ff\ff\33\a7\00\00\6f\a7\00\01\ff\ff\ff\ff\7a\a7\00\00\7c\a7\00\01\ff\ff\ff\ff\7f\a7\00\00\87\a7\00\01\ff\ff\ff\ff\8c\a7\00\00\8c\a7\00\00\ff\ff\ff\ff\91\a7\00\00\93\a7\00\01\ff\ff\ff\ff\94\a7\00\00\94\a7\00\00\30\00\00\00\97\a7\00\00\a9\a7\00\01\ff\ff\ff\ff\b5\a7\00\00\c3\a7\00\01\ff\ff\ff\ff\c8\a7\00\00\ca\a7\00\01\ff\ff\ff\ff\cd\a7\00\00\db\a7\00\01\ff\ff\ff\ff\f6\a7\00\00\f6\a7\00\00\ff\ff\ff\ff\4b\ab\00\00\4c\ab\00\00\21\00\00\00\53\ab\00\00\53\ab\00\00\60\fc\ff\ff\70\ab\00\00\bf\ab\00\00\30\68\ff\ff\41\ff\00\00\5a\ff\00\00\e0\ff\ff\ff\28\04\01\00\4f\04\01\00\d8\ff\ff\ff\d8\04\01\00\fb\04\01\00\d8\ff\ff\ff\97\05\01\00\a1\05\01\00\d9\ff\ff\ff\a3\05\01\00\b1\05\01\00\d9\ff\ff\ff\b3\05\01\00\b9\05\01\00\d9\ff\ff\ff\bb\05\01\00\bc\05\01\00\d9\ff\ff\ff\c0\0c\01\00\f2\0c\01\00\c0\ff\ff\ff\70\0d\01\00\85\0d\01\00\e0\ff\ff\ff\c0\18\01\00\df\18\01\00\e0\ff\ff\ff\60\6e\01\00\7f\6e\01\00\e0\ff\ff\ff\bb\6e\01\00\d3\6e\01\00\e5\ff\ff\ff\41\df\01\00\41\df\01\00\ff\ff\ff\ff\49\df\01\00\4b\df\01\01\ff\ff\ff\ff\4e\df\01\00\4e\df\01\00\ff\ff\ff\ff\52\df\01\00\52\df\01\00\ff\ff\ff\ff\69\df\01\00\6f\df\01\01\ff\ff\ff\ff\73\df\01\00\7f\df\01\01\ff\ff\ff\ff\22\e9\01\00\43\e9\01\00\de\ff\ff\ff")
(global $case_upper (mut (ref null $case_ranges)) (ref.null $case_ranges))
(func $case_upper_table (result (ref $case_ranges))
  (if (ref.is_null (global.get $case_upper))
    (then (global.set $case_upper
      (array.new_data $case_ranges $case_upper_data (i32.const 0) (i32.const 639)))))
  (ref.as_non_null (global.get $case_upper)))

(data $case_lower_data "\41\00\00\00\5a\00\00\00\20\00\00\00\c0\00\00\00\d6\00\00\00\20\00\00\00\d8\00\00\00\de\00\00\00\20\00\00\00\00\01\00\00\2e\01\00\01\01\00\00\00\30\01\00\00\30\01\00\00\39\ff\ff\ff\32\01\00\00\36\01\00\01\01\00\00\00\39\01\00\00\47\01\00\01\01\00\00\00\4a\01\00\00\76\01\00\01\01\00\00\00\78\01\00\00\78\01\00\00\87\ff\ff\ff\79\01\00\00\7d\01\00\01\01\00\00\00\81\01\00\00\81\01\00\00\d2\00\00\00\82\01\00\00\84\01\00\01\01\00\00\00\86\01\00\00\86\01\00\00\ce\00\00\00\87\01\00\00\87\01\00\00\01\00\00\00\89\01\00\00\8a\01\00\00\cd\00\00\00\8b\01\00\00\8b\01\00\00\01\00\00\00\8e\01\00\00\8e\01\00\00\4f\00\00\00\8f\01\00\00\8f\01\00\00\ca\00\00\00\90\01\00\00\90\01\00\00\cb\00\00\00\91\01\00\00\91\01\00\00\01\00\00\00\93\01\00\00\93\01\00\00\cd\00\00\00\94\01\00\00\94\01\00\00\cf\00\00\00\96\01\00\00\96\01\00\00\d3\00\00\00\97\01\00\00\97\01\00\00\d1\00\00\00\98\01\00\00\98\01\00\00\01\00\00\00\9c\01\00\00\9c\01\00\00\d3\00\00\00\9d\01\00\00\9d\01\00\00\d5\00\00\00\9f\01\00\00\9f\01\00\00\d6\00\00\00\a0\01\00\00\a4\01\00\01\01\00\00\00\a6\01\00\00\a6\01\00\00\da\00\00\00\a7\01\00\00\a7\01\00\00\01\00\00\00\a9\01\00\00\a9\01\00\00\da\00\00\00\ac\01\00\00\ac\01\00\00\01\00\00\00\ae\01\00\00\ae\01\00\00\da\00\00\00\af\01\00\00\af\01\00\00\01\00\00\00\b1\01\00\00\b2\01\00\00\d9\00\00\00\b3\01\00\00\b5\01\00\01\01\00\00\00\b7\01\00\00\b7\01\00\00\db\00\00\00\b8\01\00\00\b8\01\00\00\01\00\00\00\bc\01\00\00\bc\01\00\00\01\00\00\00\c4\01\00\00\c4\01\00\00\02\00\00\00\c5\01\00\00\c5\01\00\00\01\00\00\00\c7\01\00\00\c7\01\00\00\02\00\00\00\c8\01\00\00\c8\01\00\00\01\00\00\00\ca\01\00\00\ca\01\00\00\02\00\00\00\cb\01\00\00\db\01\00\01\01\00\00\00\de\01\00\00\ee\01\00\01\01\00\00\00\f1\01\00\00\f1\01\00\00\02\00\00\00\f2\01\00\00\f4\01\00\01\01\00\00\00\f6\01\00\00\f6\01\00\00\9f\ff\ff\ff\f7\01\00\00\f7\01\00\00\c8\ff\ff\ff\f8\01\00\00\1e\02\00\01\01\00\00\00\20\02\00\00\20\02\00\00\7e\ff\ff\ff\22\02\00\00\32\02\00\01\01\00\00\00\3a\02\00\00\3a\02\00\00\2b\2a\00\00\3b\02\00\00\3b\02\00\00\01\00\00\00\3d\02\00\00\3d\02\00\00\5d\ff\ff\ff\3e\02\00\00\3e\02\00\00\28\2a\00\00\41\02\00\00\41\02\00\00\01\00\00\00\43\02\00\00\43\02\00\00\3d\ff\ff\ff\44\02\00\00\44\02\00\00\45\00\00\00\45\02\00\00\45\02\00\00\47\00\00\00\46\02\00\00\4e\02\00\01\01\00\00\00\70\03\00\00\72\03\00\01\01\00\00\00\76\03\00\00\76\03\00\00\01\00\00\00\7f\03\00\00\7f\03\00\00\74\00\00\00\86\03\00\00\86\03\00\00\26\00\00\00\88\03\00\00\8a\03\00\00\25\00\00\00\8c\03\00\00\8c\03\00\00\40\00\00\00\8e\03\00\00\8f\03\00\00\3f\00\00\00\91\03\00\00\a1\03\00\00\20\00\00\00\a3\03\00\00\ab\03\00\00\20\00\00\00\cf\03\00\00\cf\03\00\00\08\00\00\00\d8\03\00\00\ee\03\00\01\01\00\00\00\f4\03\00\00\f4\03\00\00\c4\ff\ff\ff\f7\03\00\00\f7\03\00\00\01\00\00\00\f9\03\00\00\f9\03\00\00\f9\ff\ff\ff\fa\03\00\00\fa\03\00\00\01\00\00\00\fd\03\00\00\ff\03\00\00\7e\ff\ff\ff\00\04\00\00\0f\04\00\00\50\00\00\00\10\04\00\00\2f\04\00\00\20\00\00\00\60\04\00\00\80\04\00\01\01\00\00\00\8a\04\00\00\be\04\00\01\01\00\00\00\c0\04\00\00\c0\04\00\00\0f\00\00\00\c1\04\00\00\cd\04\00\01\01\00\00\00\d0\04\00\00\2e\05\00\01\01\00\00\00\31\05\00\00\56\05\00\00\30\00\00\00\a0\10\00\00\c5\10\00\00\60\1c\00\00\c7\10\00\00\c7\10\00\00\60\1c\00\00\cd\10\00\00\cd\10\00\00\60\1c\00\00\a0\13\00\00\ef\13\00\00\d0\97\00\00\f0\13\00\00\f5\13\00\00\08\00\00\00\89\1c\00\00\89\1c\00\00\01\00\00\00\90\1c\00\00\ba\1c\00\00\40\f4\ff\ff\bd\1c\00\00\bf\1c\00\00\40\f4\ff\ff\00\1e\00\00\94\1e\00\01\01\00\00\00\9e\1e\00\00\9e\1e\00\00\41\e2\ff\ff\a0\1e\00\00\fe\1e\00\01\01\00\00\00\08\1f\00\00\0f\1f\00\00\f8\ff\ff\ff\18\1f\00\00\1d\1f\00\00\f8\ff\ff\ff\28\1f\00\00\2f\1f\00\00\f8\ff\ff\ff\38\1f\00\00\3f\1f\00\00\f8\ff\ff\ff\48\1f\00\00\4d\1f\00\00\f8\ff\ff\ff\59\1f\00\00\5f\1f\00\01\f8\ff\ff\ff\68\1f\00\00\6f\1f\00\00\f8\ff\ff\ff\88\1f\00\00\8f\1f\00\00\f8\ff\ff\ff\98\1f\00\00\9f\1f\00\00\f8\ff\ff\ff\a8\1f\00\00\af\1f\00\00\f8\ff\ff\ff\b8\1f\00\00\b9\1f\00\00\f8\ff\ff\ff\ba\1f\00\00\bb\1f\00\00\b6\ff\ff\ff\bc\1f\00\00\bc\1f\00\00\f7\ff\ff\ff\c8\1f\00\00\cb\1f\00\00\aa\ff\ff\ff\cc\1f\00\00\cc\1f\00\00\f7\ff\ff\ff\d8\1f\00\00\d9\1f\00\00\f8\ff\ff\ff\da\1f\00\00\db\1f\00\00\9c\ff\ff\ff\e8\1f\00\00\e9\1f\00\00\f8\ff\ff\ff\ea\1f\00\00\eb\1f\00\00\90\ff\ff\ff\ec\1f\00\00\ec\1f\00\00\f9\ff\ff\ff\f8\1f\00\00\f9\1f\00\00\80\ff\ff\ff\fa\1f\00\00\fb\1f\00\00\82\ff\ff\ff\fc\1f\00\00\fc\1f\00\00\f7\ff\ff\ff\26\21\00\00\26\21\00\00\a3\e2\ff\ff\2a\21\00\00\2a\21\00\00\41\df\ff\ff\2b\21\00\00\2b\21\00\00\ba\df\ff\ff\32\21\00\00\32\21\00\00\1c\00\00\00\60\21\00\00\6f\21\00\00\10\00\00\00\83\21\00\00\83\21\00\00\01\00\00\00\b6\24\00\00\cf\24\00\00\1a\00\00\00\00\2c\00\00\2f\2c\00\00\30\00\00\00\60\2c\00\00\60\2c\00\00\01\00\00\00\62\2c\00\00\62\2c\00\00\09\d6\ff\ff\63\2c\00\00\63\2c\00\00\1a\f1\ff\ff\64\2c\00\00\64\2c\00\00\19\d6\ff\ff\67\2c\00\00\6b\2c\00\01\01\00\00\00\6d\2c\00\00\6d\2c\00\00\e4\d5\ff\ff\6e\2c\00\00\6e\2c\00\00\03\d6\ff\ff\6f\2c\00\00\6f\2c\00\00\e1\d5\ff\ff\70\2c\00\00\70\2c\00\00\e2\d5\ff\ff\72\2c\00\00\72\2c\00\00\01\00\00\00\75\2c\00\00\75\2c\00\00\01\00\00\00\7e\2c\00\00\7f\2c\00\00\c1\d5\ff\ff\80\2c\00\00\e2\2c\00\01\01\00\00\00\eb\2c\00\00\ed\2c\00\01\01\00\00\00\f2\2c\00\00\f2\2c\00\00\01\00\00\00\40\a6\00\00\6c\a6\00\01\01\00\00\00\80\a6\00\00\9a\a6\00\01\01\00\00\00\22\a7\00\00\2e\a7\00\01\01\00\00\00\32\a7\00\00\6e\a7\00\01\01\00\00\00\79\a7\00\00\7b\a7\00\01\01\00\00\00\7d\a7\00\00\7d\a7\00\00\fc\75\ff\ff\7e\a7\00\00\86\a7\00\01\01\00\00\00\8b\a7\00\00\8b\a7\00\00\01\00\00\00\8d\a7\00\00\8d\a7\00\00\d8\5a\ff\ff\90\a7\00\00\92\a7\00\01\01\00\00\00\96\a7\00\00\a8\a7\00\01\01\00\00\00\aa\a7\00\00\aa\a7\00\00\bc\5a\ff\ff\ab\a7\00\00\ab\a7\00\00\b1\5a\ff\ff\ac\a7\00\00\ac\a7\00\00\b5\5a\ff\ff\ad\a7\00\00\ad\a7\00\00\bf\5a\ff\ff\ae\a7\00\00\ae\a7\00\00\bc\5a\ff\ff\b0\a7\00\00\b0\a7\00\00\ee\5a\ff\ff\b1\a7\00\00\b1\a7\00\00\d6\5a\ff\ff\b2\a7\00\00\b2\a7\00\00\eb\5a\ff\ff\b3\a7\00\00\b3\a7\00\00\a0\03\00\00\b4\a7\00\00\c2\a7\00\01\01\00\00\00\c4\a7\00\00\c4\a7\00\00\d0\ff\ff\ff\c5\a7\00\00\c5\a7\00\00\bd\5a\ff\ff\c6\a7\00\00\c6\a7\00\00\c8\75\ff\ff\c7\a7\00\00\c9\a7\00\01\01\00\00\00\cb\a7\00\00\cb\a7\00\00\99\5a\ff\ff\cc\a7\00\00\da\a7\00\01\01\00\00\00\dc\a7\00\00\dc\a7\00\00\bf\59\ff\ff\dd\a7\00\00\dd\a7\00\00\9a\5a\ff\ff\e2\a7\00\00\e2\a7\00\00\9a\5a\ff\ff\f5\a7\00\00\f5\a7\00\00\01\00\00\00\6c\ab\00\00\6d\ab\00\00\df\ff\ff\ff\21\ff\00\00\3a\ff\00\00\20\00\00\00\00\04\01\00\27\04\01\00\28\00\00\00\b0\04\01\00\d3\04\01\00\28\00\00\00\70\05\01\00\7a\05\01\00\27\00\00\00\7c\05\01\00\8a\05\01\00\27\00\00\00\8c\05\01\00\92\05\01\00\27\00\00\00\94\05\01\00\95\05\01\00\27\00\00\00\80\0c\01\00\b2\0c\01\00\40\00\00\00\50\0d\01\00\65\0d\01\00\20\00\00\00\a0\18\01\00\bf\18\01\00\20\00\00\00\40\6e\01\00\5f\6e\01\00\20\00\00\00\a0\6e\01\00\b8\6e\01\00\1b\00\00\00\40\df\01\00\40\df\01\00\01\00\00\00\48\df\01\00\4a\df\01\01\01\00\00\00\4d\df\01\00\4d\df\01\00\01\00\00\00\51\df\01\00\51\df\01\00\01\00\00\00\68\df\01\00\6e\df\01\01\01\00\00\00\72\df\01\00\7e\df\01\01\01\00\00\00\00\e9\01\00\21\e9\01\00\22\00\00\00")
(global $case_lower (mut (ref null $case_ranges)) (ref.null $case_ranges))
(func $case_lower_table (result (ref $case_ranges))
  (if (ref.is_null (global.get $case_lower))
    (then (global.set $case_lower
      (array.new_data $case_ranges $case_lower_data (i32.const 0) (i32.const 585)))))
  (ref.as_non_null (global.get $case_lower)))

;; The codepoint that `cp` maps to by the table, or `cp` when no range has it.
(func $case_map (param $table (ref $case_ranges)) (param $cp i32) (result i32)
  (local $low i32) (local $high i32) (local $middle i32) (local $at i32) (local $last i32)
  (local.set $high (i32.div_u (array.len (local.get $table)) (i32.const 3)))
  ;; Ranges before $low start at or before cp; ranges from $high start after it.
  (block $found (loop $search
    (br_if $found (i32.ge_u (local.get $low) (local.get $high)))
    (local.set $middle (i32.shr_u (i32.add (local.get $low) (local.get $high)) (i32.const 1)))
    (if (i32.le_u (array.get $case_ranges (local.get $table) (i32.mul (local.get $middle) (i32.const 3)))
                  (local.get $cp))
      (then (local.set $low (i32.add (local.get $middle) (i32.const 1))))
      (else (local.set $high (local.get $middle))))
    (br $search)))
  (if (i32.eqz (local.get $low)) (then (return (local.get $cp))))
  (local.set $at (i32.mul (i32.sub (local.get $low) (i32.const 1)) (i32.const 3)))
  (local.set $last (array.get $case_ranges (local.get $table) (i32.add (local.get $at) (i32.const 1))))
  (if (i32.gt_u (local.get $cp) (i32.and (local.get $last) (i32.const 0xffffff)))
    (then (return (local.get $cp))))
  (if (i32.and (i32.shr_u (local.get $last) (i32.const 24))
        (i32.sub (local.get $cp) (array.get $case_ranges (local.get $table) (local.get $at))))
    (then (return (local.get $cp))))
  (i32.add (local.get $cp) (array.get $case_ranges (local.get $table) (i32.add (local.get $at) (i32.const 2)))))

;; The scalar value at `at`, a character boundary of correct UTF-8 with at
;; least one more character.
(func $case_decode (param $b (ref $bytes)) (param $at i32) (result i32)
  (local $lead i32)
  (local.set $lead (array.get_u $bytes (local.get $b) (local.get $at)))
  (if (i32.lt_u (local.get $lead) (i32.const 0x80)) (then (return (local.get $lead))))
  (if (i32.lt_u (local.get $lead) (i32.const 0xe0))
    (then (return (i32.or
      (i32.shl (i32.and (local.get $lead) (i32.const 0x1f)) (i32.const 6))
      (i32.and (array.get_u $bytes (local.get $b) (i32.add (local.get $at) (i32.const 1))) (i32.const 0x3f))))))
  (if (i32.lt_u (local.get $lead) (i32.const 0xf0))
    (then (return (i32.or (i32.or
      (i32.shl (i32.and (local.get $lead) (i32.const 0x0f)) (i32.const 12))
      (i32.shl (i32.and (array.get_u $bytes (local.get $b) (i32.add (local.get $at) (i32.const 1))) (i32.const 0x3f)) (i32.const 6)))
      (i32.and (array.get_u $bytes (local.get $b) (i32.add (local.get $at) (i32.const 2))) (i32.const 0x3f))))))
  (i32.or (i32.or (i32.or
    (i32.shl (i32.and (local.get $lead) (i32.const 0x07)) (i32.const 18))
    (i32.shl (i32.and (array.get_u $bytes (local.get $b) (i32.add (local.get $at) (i32.const 1))) (i32.const 0x3f)) (i32.const 12)))
    (i32.shl (i32.and (array.get_u $bytes (local.get $b) (i32.add (local.get $at) (i32.const 2))) (i32.const 0x3f)) (i32.const 6)))
    (i32.and (array.get_u $bytes (local.get $b) (i32.add (local.get $at) (i32.const 3))) (i32.const 0x3f))))

;; Map every character of the string. ASCII letters from `ascii_first` to
;; `ascii_first + 25` move by `ascii_shift`; other characters use the table.
;; A string with nothing to change is given back as it is. Otherwise the
;; part before the first change is copied once, and the rest is mapped into
;; a buffer that grows when needed.
(func $case_string (param $s eqref) (param $table (ref $case_ranges))
  (param $ascii_first i32) (param $ascii_shift i32) (result eqref)
  (local $b (ref $bytes)) (local $n i32) (local $at i32) (local $cp i32) (local $mapped i32)
  (local $out (ref null $bytes)) (local $grown (ref null $bytes)) (local $size i32)
  (local $capacity i64)
  (local.set $b (call $string_bytes (local.get $s)))
  (local.set $n (array.len (local.get $b)))
  ;; Find the first character that changes.
  (block $changed (loop $scan
    (if (i32.ge_u (local.get $at) (local.get $n)) (then (return (local.get $s))))
    (local.set $cp (call $case_decode (local.get $b) (local.get $at)))
    (if (i32.lt_u (local.get $cp) (i32.const 0x80))
      (then
        (br_if $changed (i32.lt_u (i32.sub (local.get $cp) (local.get $ascii_first)) (i32.const 26)))
        (local.set $at (i32.add (local.get $at) (i32.const 1))))
      (else
        (br_if $changed (i32.ne (call $case_map (local.get $table) (local.get $cp)) (local.get $cp)))
        (local.set $at (i32.add (local.get $at) (call $utf8_width (local.get $cp))))))
    (br $scan)))
  ;; Room for the rest at the same size, and some more.
  (local.set $capacity (i64.add (i64.extend_i32_u (local.get $n)) (i64.const 16)))
  (if (i64.gt_u (local.get $capacity) (i64.const 4294967295))
    (then (local.set $capacity (i64.const 4294967295))))
  (local.set $out (array.new_default $bytes (i32.wrap_i64 (local.get $capacity))))
  (array.copy $bytes $bytes (local.get $out) (i32.const 0) (local.get $b) (i32.const 0) (local.get $at))
  (local.set $size (local.get $at))
  (block $done (loop $map
    (br_if $done (i32.ge_u (local.get $at) (local.get $n)))
    (local.set $cp (call $case_decode (local.get $b) (local.get $at)))
    (if (i32.lt_u (local.get $cp) (i32.const 0x80))
      (then
        (local.set $mapped (local.get $cp))
        (if (i32.lt_u (i32.sub (local.get $cp) (local.get $ascii_first)) (i32.const 26))
          (then (local.set $mapped (i32.add (local.get $cp) (local.get $ascii_shift)))))
        (local.set $at (i32.add (local.get $at) (i32.const 1))))
      (else
        (local.set $mapped (call $case_map (local.get $table) (local.get $cp)))
        (local.set $at (i32.add (local.get $at) (call $utf8_width (local.get $cp))))))
    ;; A character takes at most 4 bytes.
    (if (i32.gt_u (i32.add (local.get $size) (i32.const 4)) (array.len (local.get $out)))
      (then
        (local.set $capacity (i64.shl (i64.extend_i32_u (array.len (local.get $out))) (i64.const 1)))
        (if (i64.gt_u (local.get $capacity) (i64.const 4294967295))
          (then (local.set $capacity (i64.const 4294967295))))
        (if (i64.le_u (local.get $capacity) (i64.extend_i32_u (array.len (local.get $out))))
          (then unreachable))
        (local.set $grown (array.new_default $bytes (i32.wrap_i64 (local.get $capacity))))
        (array.copy $bytes $bytes (local.get $grown) (i32.const 0) (local.get $out) (i32.const 0) (local.get $size))
        (local.set $out (local.get $grown))))
    (local.set $size (call $utf8_write (ref.as_non_null (local.get $out)) (local.get $size) (local.get $mapped)))
    (br $map)))
  (local.set $grown (array.new_default $bytes (local.get $size)))
  (array.copy $bytes $bytes (local.get $grown) (i32.const 0) (local.get $out) (i32.const 0) (local.get $size))
  (call $string_from_bytes (ref.as_non_null (local.get $grown))))

(func $core_to_upper (param $s eqref) (result eqref)
  (call $case_string (local.get $s) (call $case_upper_table) (i32.const 97) (i32.const -32)))
(func $core_to_lower (param $s eqref) (result eqref)
  (call $case_string (local.get $s) (call $case_lower_table) (i32.const 65) (i32.const 32)))

(type $search_table (array (mut i32)))

;; Error(Nil) when `from` is negative, after the end, or inside a character,
;; and when `find` is not there. An empty `find` is at `from`.
(func $core_index_of (param $s eqref) (param $find eqref) (param $from eqref) (result eqref)
  (local $text (ref $bytes)) (local $pattern (ref $bytes))
  (local $n i32) (local $m i32) (local $start i32) (local $found i32)
  (local.set $text (call $string_bytes (local.get $s)))
  (local.set $pattern (call $string_bytes (local.get $find)))
  (local.set $n (array.len (local.get $text)))
  (local.set $m (array.len (local.get $pattern)))
  (if (i32.lt_s (call $int_sign (local.get $from)) (i32.const 0))
    (then (return (call $result_error (ref.null eq)))))
  (if (i32.gt_s (call $int_compare (local.get $from) (call $int_from_i64 (i64.extend_i32_u (local.get $n))))
                (i32.const 0))
    (then (return (call $result_error (ref.null eq)))))
  (local.set $start (i32.wrap_i64 (call $int_to_i64 (local.get $from))))
  (if (i32.eqz (call $string_boundary (local.get $text) (local.get $start)))
    (then (return (call $result_error (ref.null eq)))))
  (if (i32.eqz (local.get $m)) (then (return (call $result_ok (local.get $from)))))
  (if (i32.gt_u (local.get $m) (i32.sub (local.get $n) (local.get $start)))
    (then (return (call $result_error (ref.null eq)))))
  (local.set $found
    (if (result i32) (i32.eq (local.get $m) (i32.const 1))
      (then (call $search_byte (local.get $text) (array.get_u $bytes (local.get $pattern) (i32.const 0))
        (local.get $start)))
      (else (call $search_kmp (local.get $text) (local.get $pattern) (local.get $start)))))
  (if (i32.lt_s (local.get $found) (i32.const 0))
    (then (return (call $result_error (ref.null eq)))))
  (call $result_ok (call $int_from_i64 (i64.extend_i32_u (local.get $found)))))

;; The first index at or after `at` that holds `byte`, or -1.
(func $search_byte (param $text (ref $bytes)) (param $byte i32) (param $at i32) (result i32)
  (local $n i32)
  (local.set $n (array.len (local.get $text)))
  (block $none (loop $scan
    (br_if $none (i32.ge_u (local.get $at) (local.get $n)))
    (if (i32.eq (array.get_u $bytes (local.get $text) (local.get $at)) (local.get $byte))
      (then (return (local.get $at))))
    (local.set $at (i32.add (local.get $at) (i32.const 1)))
    (br $scan)))
  (i32.const -1))

;; The first index at or after `at` where `pattern` (two bytes or more) is
;; in `text`, or -1. `border[i]` is the length of the longest proper prefix
;; of pattern[0..=i] that is also its suffix.
(func $search_kmp (param $text (ref $bytes)) (param $pattern (ref $bytes)) (param $at i32) (result i32)
  (local $border (ref $search_table)) (local $n i32) (local $m i32) (local $i i32) (local $k i32)
  (local $byte i32)
  (local.set $n (array.len (local.get $text)))
  (local.set $m (array.len (local.get $pattern)))
  (local.set $border (array.new_default $search_table (local.get $m)))
  (local.set $i (i32.const 1))
  (block $built (loop $build
    (br_if $built (i32.ge_u (local.get $i) (local.get $m)))
    (local.set $byte (array.get_u $bytes (local.get $pattern) (local.get $i)))
    (block $fits (loop $shorter
      (br_if $fits (i32.eqz (local.get $k)))
      (br_if $fits (i32.eq (array.get_u $bytes (local.get $pattern) (local.get $k)) (local.get $byte)))
      (local.set $k (array.get $search_table (local.get $border) (i32.sub (local.get $k) (i32.const 1))))
      (br $shorter)))
    (if (i32.eq (array.get_u $bytes (local.get $pattern) (local.get $k)) (local.get $byte))
      (then (local.set $k (i32.add (local.get $k) (i32.const 1)))))
    (array.set $search_table (local.get $border) (local.get $i) (local.get $k))
    (local.set $i (i32.add (local.get $i) (i32.const 1)))
    (br $build)))
  ;; $k is the number of pattern bytes that match the text before $at.
  (local.set $k (i32.const 0))
  (block $none (loop $scan
    (br_if $none (i32.ge_u (local.get $at) (local.get $n)))
    (local.set $byte (array.get_u $bytes (local.get $text) (local.get $at)))
    (block $fits (loop $shorter
      (br_if $fits (i32.eqz (local.get $k)))
      (br_if $fits (i32.eq (array.get_u $bytes (local.get $pattern) (local.get $k)) (local.get $byte)))
      (local.set $k (array.get $search_table (local.get $border) (i32.sub (local.get $k) (i32.const 1))))
      (br $shorter)))
    (if (i32.eq (array.get_u $bytes (local.get $pattern) (local.get $k)) (local.get $byte))
      (then (local.set $k (i32.add (local.get $k) (i32.const 1)))))
    (local.set $at (i32.add (local.get $at) (i32.const 1)))
    (if (i32.eq (local.get $k) (local.get $m))
      (then (return (i32.sub (local.get $at) (local.get $m)))))
    (br $scan)))
  (i32.const -1))

(func $core_bit_array_concat (param $list eqref) (result eqref)
  (local $cursor eqref) (local $part eqref)
  (local $length i64) (local $at i64) (local $data (ref null $bytes)) (local $count i32)
  (local.set $cursor (local.get $list))
  (block $counted (loop $count
    (br_if $counted (call $list_is_empty (local.get $cursor)))
    (local.set $part (call $list_head (local.get $cursor)))
    (local.set $length (i64.add (local.get $length) (call $bits_len64 (local.get $part))))
    (local.set $count (i32.add (local.get $count) (i32.const 1)))
    (local.set $cursor (call $list_tail (local.get $cursor)))
    (br $count)))
  (if (i32.eqz (local.get $count)) (then (return (call $bits_empty))))
  ;; One part needs no copy: bit arrays cannot be changed.
  (if (i32.eq (local.get $count) (i32.const 1)) (then (return (local.get $part))))
  (local.set $data (call $bits_alloc (local.get $length)))
  (local.set $cursor (local.get $list))
  (block $written (loop $write
    (br_if $written (call $list_is_empty (local.get $cursor)))
    (local.set $part (call $list_head (local.get $cursor)))
    (call $bits_join_part (ref.as_non_null (local.get $data)) (local.get $at) (local.get $part))
    (local.set $at (i64.add (local.get $at) (call $bits_len64 (local.get $part))))
    (local.set $cursor (call $list_tail (local.get $cursor)))
    (br $write)))
  (call $bits_new (ref.as_non_null (local.get $data)) (i32.const 0) (local.get $length)))

;; Put the bits of `part` into `out` from bit `at` on. The bits of `out` from
;; `at` on are 0.
(func $bits_join_part (param $out (ref $bytes)) (param $at i64) (param $part eqref)
  (local $source (ref $bytes)) (local $offset i64) (local $length i64) (local $i i64)
  (local $position i64) (local $q i32) (local $r i32) (local $value i32) (local $take i64)
  (local $p i32) (local $t i32) (local $whole i32)
  (local.set $source (call $bits_data (local.get $part)))
  (local.set $offset (call $bits_offset (local.get $part)))
  (local.set $length (call $bits_len64 (local.get $part)))
  (if (i64.eqz (i64.and (i64.or (local.get $at) (local.get $offset)) (i64.const 7)))
    (then
      (local.set $whole (i32.wrap_i64 (i64.shr_u (local.get $length) (i64.const 3))))
      (array.copy $bytes $bytes
        (local.get $out) (i32.wrap_i64 (i64.shr_u (local.get $at) (i64.const 3)))
        (local.get $source) (i32.wrap_i64 (i64.shr_u (local.get $offset) (i64.const 3)))
        (local.get $whole))
      (local.set $i (i64.shl (i64.extend_i32_u (local.get $whole)) (i64.const 3)))))
  ;; Eight bits at a time: read them from two source bytes, cut off what is
  ;; after the end of the part, and write them over two bytes of `out`.
  (block $done (loop $copy
    (br_if $done (i64.ge_u (local.get $i) (local.get $length)))
    (local.set $position (i64.add (local.get $offset) (local.get $i)))
    (local.set $q (i32.wrap_i64 (i64.shr_u (local.get $position) (i64.const 3))))
    (local.set $r (i32.wrap_i64 (i64.and (local.get $position) (i64.const 7))))
    (local.set $value (i32.shl (array.get_u $bytes (local.get $source) (local.get $q)) (local.get $r)))
    (if (i32.and (i32.ne (local.get $r) (i32.const 0))
                 (i32.lt_u (i32.add (local.get $q) (i32.const 1)) (array.len (local.get $source))))
      (then (local.set $value (i32.or (local.get $value)
        (i32.shr_u (array.get_u $bytes (local.get $source) (i32.add (local.get $q) (i32.const 1)))
                   (i32.sub (i32.const 8) (local.get $r)))))))
    (local.set $take (i64.sub (local.get $length) (local.get $i)))
    (if (i64.gt_u (local.get $take) (i64.const 8)) (then (local.set $take (i64.const 8))))
    (local.set $value (i32.and (local.get $value)
      (i32.and (i32.shl (i32.const 0xff) (i32.sub (i32.const 8) (i32.wrap_i64 (local.get $take))))
               (i32.const 0xff))))
    (local.set $position (i64.add (local.get $at) (local.get $i)))
    (local.set $p (i32.wrap_i64 (i64.shr_u (local.get $position) (i64.const 3))))
    (local.set $t (i32.wrap_i64 (i64.and (local.get $position) (i64.const 7))))
    (array.set $bytes (local.get $out) (local.get $p)
      (i32.or (array.get_u $bytes (local.get $out) (local.get $p))
              (i32.shr_u (local.get $value) (local.get $t))))
    ;; The low bits that do not fit go to the next byte; they are 0 unless
    ;; they are bits of the part, so that byte is then in `out`.
    (if (i32.and (i32.ne (local.get $t) (i32.const 0))
                 (i32.ne (i32.and (i32.shl (local.get $value) (i32.sub (i32.const 8) (local.get $t))) (i32.const 0xff))
                         (i32.const 0)))
      (then (array.set $bytes (local.get $out) (i32.add (local.get $p) (i32.const 1))
        (i32.or (array.get_u $bytes (local.get $out) (i32.add (local.get $p) (i32.const 1)))
                (i32.and (i32.shl (local.get $value) (i32.sub (i32.const 8) (local.get $t))) (i32.const 0xff))))))
    (local.set $i (i64.add (local.get $i) (i64.const 8)))
    (br $copy)))
  )

;; A buffer must lie inside memory.
(func $abi_buffer_check (param $address i32) (param $length i64)
  (if (i64.gt_u (i64.add (i64.extend_i32_u (local.get $address)) (local.get $length))
                (i64.shl (i64.extend_i32_u (memory.size)) (i64.const 16)))
    (then (call $stop_host_value) unreachable)))

(func $abi_bool_in (param $value i32) (result eqref)
  (if (i32.gt_u (local.get $value) (i32.const 1)) (then (call $stop_host_value) unreachable))
  (call $bool_new (local.get $value)))
(func $abi_bool_out (param $value eqref) (result i32)
  (call $bool_get (local.get $value)))

(func $abi_float_in (param $value f64) (result eqref)
  (if (i32.eqz (f64.le (f64.abs (local.get $value)) (f64.const 0x1.fffffffffffffp+1023)))
    (then (call $stop_host_value) unreachable))
  (call $float_new (local.get $value)))
(func $abi_float_out (param $value eqref) (result f64)
  (call $float_get (local.get $value)))

;; Sign -1, 0 or 1; count little-endian u32 parts; the last part is not 0;
;; zero is sign 0 and count 0.
(func $abi_int_in (param $sign i32) (param $address i32) (param $count i32) (result eqref)
  (local $limbs (ref $limbs)) (local $i i32)
  (if (i32.gt_u (i32.add (local.get $sign) (i32.const 1)) (i32.const 2))
    (then (call $stop_host_value) unreachable))
  (if (i32.ne (i32.eqz (local.get $sign)) (i32.eqz (local.get $count)))
    (then (call $stop_host_value) unreachable))
  (call $abi_buffer_check (local.get $address)
    (i64.shl (i64.extend_i32_u (local.get $count)) (i64.const 2)))
  (if (i32.eqz (local.get $count)) (then (return (ref.i31 (i32.const 0)))))
  (local.set $limbs (array.new_default $limbs (local.get $count)))
  (block $done (loop $each
    (br_if $done (i32.ge_u (local.get $i) (local.get $count)))
    (array.set $limbs (local.get $limbs) (local.get $i)
      (i32.load (i32.add (local.get $address) (i32.shl (local.get $i) (i32.const 2)))))
    (local.set $i (i32.add (local.get $i) (i32.const 1)))
    (br $each)))
  (if (i32.eqz (array.get $limbs (local.get $limbs) (i32.sub (local.get $count) (i32.const 1))))
    (then (call $stop_host_value) unreachable))
  (call $int_pack (local.get $sign) (local.get $limbs)))
(func $abi_int_out (param $value eqref) (result i32 i32 i32)
  (local $limbs (ref $limbs)) (local $count i32) (local $address i32) (local $i i32)
  (local.set $limbs (call $int_limbs (local.get $value)))
  (local.set $count (array.len (local.get $limbs)))
  (if (i32.eqz (local.get $count)) (then (return (i32.const 0) (i32.const 0) (i32.const 0))))
  (local.set $address (call $alloc (i32.shl (local.get $count) (i32.const 2))))
  (block $done (loop $each
    (br_if $done (i32.ge_u (local.get $i) (local.get $count)))
    (i32.store (i32.add (local.get $address) (i32.shl (local.get $i) (i32.const 2)))
      (array.get $limbs (local.get $limbs) (local.get $i)))
    (local.set $i (i32.add (local.get $i) (i32.const 1)))
    (br $each)))
  (call $int_sign (local.get $value)) (local.get $address) (local.get $count))

(func $abi_bytes_in (param $address i32) (param $length i32) (result (ref $bytes))
  (local $data (ref $bytes)) (local $i i32)
  (call $abi_buffer_check (local.get $address) (i64.extend_i32_u (local.get $length)))
  (local.set $data (array.new_default $bytes (local.get $length)))
  (block $done (loop $each
    (br_if $done (i32.ge_u (local.get $i) (local.get $length)))
    (array.set $bytes (local.get $data) (local.get $i)
      (i32.load8_u (i32.add (local.get $address) (local.get $i))))
    (local.set $i (i32.add (local.get $i) (i32.const 1)))
    (br $each)))
  (local.get $data))

(func $abi_string_in (param $address i32) (param $length i32) (result eqref)
  (local $data (ref $bytes))
  (local.set $data (call $abi_bytes_in (local.get $address) (local.get $length)))
  (if (i32.eqz (call $utf8_valid (local.get $data))) (then (call $stop_host_value) unreachable))
  (call $string_from_bytes (local.get $data)))
(func $abi_string_out (param $value eqref) (result i32 i32)
  (local $length i32)
  (local.set $length (call $string_length_bytes (local.get $value)))
  (if (i32.eqz (local.get $length)) (then (return (i32.const 0) (i32.const 0))))
  (call $string_to_mem (local.get $value)) (local.get $length))

;; ceil(bits / 8) bytes, highest bit first; the bits after the end are 0.
(func $abi_bits_in (param $address i32) (param $bits i32) (result eqref)
  (local $data (ref $bytes)) (local $count i32) (local $rest i32)
  (local.set $count (i32.wrap_i64 (i64.shr_u
    (i64.add (i64.extend_i32_u (local.get $bits)) (i64.const 7)) (i64.const 3))))
  (local.set $data (call $abi_bytes_in (local.get $address) (local.get $count)))
  (local.set $rest (i32.and (local.get $bits) (i32.const 7)))
  (if (local.get $rest)
    (then
      (if (i32.and
            (array.get_u $bytes (local.get $data) (i32.sub (local.get $count) (i32.const 1)))
            (i32.sub (i32.shl (i32.const 1) (i32.sub (i32.const 8) (local.get $rest))) (i32.const 1)))
        (then (call $stop_host_value) unreachable))))
  (call $bits_new (local.get $data) (i32.const 0) (i64.extend_i32_u (local.get $bits))))
(func $abi_bits_out (param $value eqref) (result i32 i32)
  (local $length i64) (local $count i32) (local $address i32) (local $i i32) (local $rest i64)
  (local.set $length (call $bits_len64 (local.get $value)))
  ;; The interface has 32 bits for the length.
  (if (i64.gt_u (local.get $length) (i64.const 4294967295)) (then unreachable))
  (if (i64.eqz (local.get $length)) (then (return (i32.const 0) (i32.const 0))))
  (local.set $count (i32.wrap_i64 (i64.shr_u (i64.add (local.get $length) (i64.const 7)) (i64.const 3))))
  (local.set $address (call $alloc (local.get $count)))
  (block $done (loop $each
    (br_if $done (i64.gt_u (i64.shl (i64.extend_i32_u (i32.add (local.get $i) (i32.const 1))) (i64.const 3))
                           (local.get $length)))
    (i32.store8 (i32.add (local.get $address) (local.get $i))
      (call $bits_byte (local.get $value) (i64.shl (i64.extend_i32_u (local.get $i)) (i64.const 3))))
    (local.set $i (i32.add (local.get $i) (i32.const 1)))
    (br $each)))
  (local.set $rest (i64.and (local.get $length) (i64.const 7)))
  (if (i64.ne (local.get $rest) (i64.const 0))
    (then
      (i32.store8 (i32.add (local.get $address) (local.get $i))
        (i32.wrap_i64 (i64.shl
          (call $bits_read_u64 (local.get $value)
            (i64.shl (i64.extend_i32_u (local.get $i)) (i64.const 3)) (local.get $rest) (i32.const 0))
          (i64.sub (i64.const 8) (local.get $rest)))))))
  (local.get $address) (i32.wrap_i64 (local.get $length)))

(func $abi_ref_in (param $value externref) (result eqref)
  (call $host_ref_new (local.get $value)))
(func $abi_ref_out (param $value eqref) (result externref)
  (call $host_ref_get (local.get $value)))

;; The results of an import: read, then the buffer is freed (B.3), also
;; when it is empty; `$free` does nothing for the address 0.
(func $abi_int_take (param $sign i32) (param $address i32) (param $count i32) (result eqref)
  (local $v eqref)
  (local.set $v (call $abi_int_in (local.get $sign) (local.get $address) (local.get $count)))
  (call $free (local.get $address) (i32.shl (local.get $count) (i32.const 2)))
  (local.get $v))
(func $abi_string_take (param $address i32) (param $length i32) (result eqref)
  (local $v eqref)
  (local.set $v (call $abi_string_in (local.get $address) (local.get $length)))
  (call $free (local.get $address) (local.get $length))
  (local.get $v))
(func $abi_bits_take (param $address i32) (param $bits i32) (result eqref)
  (local $v eqref)
  (local.set $v (call $abi_bits_in (local.get $address) (local.get $bits)))
  (call $free (local.get $address)
    (i32.wrap_i64 (i64.shr_u (i64.add (i64.extend_i32_u (local.get $bits)) (i64.const 7)) (i64.const 3))))
  (local.get $v))

;; The error tag of a result: 0, or 1 to the number of error members.
(func $abi_tag_in (param $tag i32) (param $members i32) (result i32)
  (if (i32.gt_u (local.get $tag) (local.get $members)) (then (call $stop_host_value) unreachable))
  (local.get $tag))

(func $is_ctor (param $v eqref) (param $c i32) (result i32)
  (if (result i32) (i32.eq (call $value_tag (local.get $v)) (i32.const 5))
    (then (i32.eq (struct.get $obj 1 (ref.cast (ref $obj) (local.get $v))) (local.get $c)))
    (else (i32.const 0))))
(func $is_type (param $v eqref) (param $t i64) (result i32)
  (if (result i32) (i32.eq (call $value_tag (local.get $v)) (i32.const 5))
    (then (i64.eq (struct.get $obj 2 (ref.cast (ref $obj) (local.get $v))) (local.get $t)))
    (else (i32.const 0))))
(func $is_bool (param $v eqref) (param $b i32) (result i32)
  (if (result i32) (i32.eq (call $value_tag (local.get $v)) (i32.const 1))
    (then (i32.eq (struct.get $obj 1 (ref.cast (ref $obj) (local.get $v))) (local.get $b)))
    (else (i32.const 0))))
(func $is_int (param $v eqref) (param $n eqref) (result i32)
  (if (result i32) (i32.eq (call $value_tag (local.get $v)) (i32.const 8))
    (then (i32.eqz (call $int_compare (local.get $v) (local.get $n))))
    (else (i32.const 0))))
(func $is_float (param $v eqref) (param $x f64) (result i32)
  (if (result i32) (i32.eq (call $value_tag (local.get $v)) (i32.const 2))
    (then (f64.eq (struct.get $obj 3 (ref.cast (ref $obj) (local.get $v))) (local.get $x)))
    (else (i32.const 0))))
(func $is_string (param $v eqref) (param $s eqref) (result i32)
  (if (result i32) (i32.eq (call $value_tag (local.get $v)) (i32.const 3))
    (then (i32.eqz (call $string_compare (local.get $v) (local.get $s))))
    (else (i32.const 0))))
(func $is_tuple (param $v eqref) (param $n i32) (result i32)
  (if (result i32) (i32.eq (call $value_tag (local.get $v)) (i32.const 4))
    (then (i32.eq (array.len (ref.as_non_null (struct.get $obj 5 (ref.cast (ref $obj) (local.get $v)))))
      (local.get $n)))
    (else (i32.const 0))))

(tag $error (param eqref))
(tag $cancel (param eqref))
(tag $capture (param (ref $cap)))
(global $root_env (mut (ref null $env)) (ref.null $env))

;; `env` with `v` in the slot `id` (§9.2): a new array.
(func $env_with (param $env (ref null $env)) (param $id i32) (param $v eqref) (result (ref null $env))
  (local $new (ref $env)) (local $n i32)
  (local.set $n (array.len (local.get $env)))
  (local.set $new (array.new_default $env (local.get $n)))
  (array.copy $env $env (local.get $new) (i32.const 0) (local.get $env) (i32.const 0) (local.get $n))
  (array.set $env (local.get $new) (local.get $id) (local.get $v))
  (local.get $new))

;; Calls a computation (§9.1) with the environment.
(func $call0 (param $f eqref) (param $env (ref null $env)) (result eqref)
  (call_ref $fn0 (ref.cast (ref $clo0) (local.get $f)) (local.get $env)
    (struct.get $clo0 8 (ref.cast (ref $clo0) (local.get $f)))))

;; Calls a function of one parameter with the environment.
(func $call1 (param $f eqref) (param $a eqref) (param $env (ref null $env)) (result eqref)
  (call_ref $fn1 (ref.cast (ref $clo1) (local.get $f)) (local.get $env) (local.get $a)
    (struct.get $clo1 8 (ref.cast (ref $clo1) (local.get $f)))))

;; Puts the frame record `f` first in the capture `c`, and throws `x` again.
(func $capture_rethrow (param $c (ref $cap)) (param $x exnref) (param $f (ref $frame))
  (struct.set $cap 2 (local.get $c) (struct.new $flist (local.get $f) (struct.get $cap 2 (local.get $c))))
  (throw_ref (local.get $x)))

;; Re-enters the frames `rest`, the outermost first. At the innermost frame,
;; gives `v`, or unwinds with `$cancel` when `cancel` is not null.
(func $reenter_next (param $rest (ref null $flist)) (param $env (ref null $env)) (param $v eqref)
    (param $cancel eqref) (result eqref)
  (local $f (ref $frame))
  (if (ref.is_null (local.get $rest))
    (then
      (if (i32.eqz (ref.is_null (local.get $cancel))) (then (throw $cancel (local.get $cancel))))
      (return (local.get $v))))
  (local.set $f (struct.get $flist 0 (ref.as_non_null (local.get $rest))))
  (return_call_ref $refn (local.get $f) (struct.get $flist 1 (ref.as_non_null (local.get $rest)))
    (local.get $env) (local.get $v) (local.get $cancel) (struct.get $frame 0 (local.get $f))))

;; `fail.catch(body)` (§9.3).
(func $fail_catch (param $body eqref) (param $env (ref null $env)) (result eqref)
  (return_call $catch_run (local.get $env) (local.get $body) (ref.null $flist) (ref.null eq) (ref.null eq)
    (ref.null $frame)))

;; Calls `body`, or re-enters `rest` when `body` is null, and takes `$error`.
(func $catch_run (param $env (ref null $env)) (param $body eqref) (param $rest (ref null $flist))
    (param $v eqref) (param $cancel eqref) (param $frame (ref null $frame)) (result eqref)
  (local $depth i32) (local $e eqref) (local $r eqref)
  (local.set $depth (call $shadow_depth))
  (block $captured (result (ref $cap) exnref)
    (local.set $e
      (block $caught (result eqref)
        (local.set $r
          (try_table (result eqref) (catch $error $caught) (catch_ref $capture $captured)
            (if (result eqref) (ref.is_null (local.get $body))
              (then (call $reenter_next (local.get $rest) (local.get $env) (local.get $v) (local.get $cancel)))
              (else (call $call0 (local.get $body) (local.get $env))))))
        (return (call $result_ok (local.get $r)))))
    (call $shadow_restore (local.get $depth))
    (return (call $result_error (local.get $e))))
  (call $capture_rethrow
    (if (result (ref $frame)) (ref.is_null (local.get $frame))
      (then (struct.new $frame (ref.func $catch_reenter) (ref.null $values)))
      (else (ref.as_non_null (local.get $frame)))))
  (unreachable))

(func $catch_reenter (type $refn) (param $frame (ref $frame)) (param $rest (ref null $flist))
    (param $env (ref null $env)) (param $v eqref) (param $cancel eqref) (result eqref)
  (return_call $catch_run (local.get $env) (ref.null eq) (local.get $rest) (local.get $v) (local.get $cancel)
    (local.get $frame)))

;; `resource.ensure(body, with)` (§9.5): `with` runs after `body`, also when
;; an error or an unwinding goes through, which then goes on. A capture
;; does not run it: the computation is paused.
(func $resource_ensure (param $body eqref) (param $with eqref) (param $env (ref null $env)) (result eqref)
  (return_call $ensure_run (local.get $with) (local.get $env) (local.get $body) (ref.null $flist)
    (ref.null eq) (ref.null eq) (ref.null $frame)))

(func $ensure_run (param $with eqref) (param $env (ref null $env)) (param $body eqref)
    (param $rest (ref null $flist)) (param $v eqref) (param $cancel eqref) (param $frame (ref null $frame))
    (result eqref)
  (local $depth i32) (local $r eqref)
  (local.set $depth (call $shadow_depth))
  (block $captured (result (ref $cap) exnref)
    (block $cancelled (result eqref)
      (block $failed (result eqref)
        (local.set $r
          (try_table (result eqref) (catch $error $failed) (catch $cancel $cancelled)
              (catch_ref $capture $captured)
            (if (result eqref) (ref.is_null (local.get $body))
              (then (call $reenter_next (local.get $rest) (local.get $env) (local.get $v) (local.get $cancel)))
              (else (call $call0 (local.get $body) (local.get $env))))))
        (return_call $ensure_final (local.get $with) (local.get $env) (i32.const 0) (local.get $r)))
      (local.set $r)
      (call $shadow_restore (local.get $depth))
      (return_call $ensure_final (local.get $with) (local.get $env) (i32.const 1) (local.get $r)))
    (local.set $r)
    (call $shadow_restore (local.get $depth))
    (return_call $ensure_final (local.get $with) (local.get $env) (i32.const 2) (local.get $r)))
  (call $capture_rethrow
    (if (result (ref $frame)) (ref.is_null (local.get $frame))
      (then (struct.new $frame (ref.func $ensure_reenter) (array.new_fixed $values 1 (local.get $with))))
      (else (ref.as_non_null (local.get $frame)))))
  (unreachable))

(func $ensure_reenter (type $refn) (param $frame (ref $frame)) (param $rest (ref null $flist))
    (param $env (ref null $env)) (param $v eqref) (param $cancel eqref) (result eqref)
  (return_call $ensure_run
    (array.get $values (ref.as_non_null (struct.get $frame 1 (local.get $frame))) (i32.const 0))
    (local.get $env) (ref.null eq) (local.get $rest) (local.get $v) (local.get $cancel) (local.get $frame)))

;; Runs the final function, then ends as `body` did: mode 0 gives `r`, 1
;; throws the error `r`, 2 goes on unwinding the continuation `r`.
(func $ensure_final (param $with eqref) (param $env (ref null $env)) (param $mode i32) (param $r eqref)
    (result eqref)
  (block $captured (result (ref $cap) exnref)
    (drop (try_table (result eqref) (catch_ref $capture $captured)
      (call $call0 (local.get $with) (local.get $env))))
    (return_call $ensure_end (local.get $mode) (local.get $r)))
  (call $capture_rethrow (struct.new $frame (ref.func $final_reenter)
    (array.new_fixed $values 2 (local.get $r) (ref.i31 (local.get $mode)))))
  (unreachable))

(func $ensure_end (param $mode i32) (param $r eqref) (result eqref)
  (if (i32.eq (local.get $mode) (i32.const 1)) (then (throw $error (local.get $r))))
  (if (i32.eq (local.get $mode) (i32.const 2)) (then (throw $cancel (local.get $r))))
  (local.get $r))

(func $final_reenter (type $refn) (param $frame (ref $frame)) (param $rest (ref null $flist))
    (param $env (ref null $env)) (param $v eqref) (param $cancel eqref) (result eqref)
  (local $saved (ref $values))
  (local.set $saved (ref.as_non_null (struct.get $frame 1 (local.get $frame))))
  (block $captured (result (ref $cap) exnref)
    (drop (try_table (result eqref) (catch_ref $capture $captured)
      (call $reenter_next (local.get $rest) (local.get $env) (local.get $v) (local.get $cancel))))
    (return_call $ensure_end
      (i31.get_u (ref.cast (ref i31) (array.get $values (local.get $saved) (i32.const 1))))
      (array.get $values (local.get $saved) (i32.const 0))))
  (call $capture_rethrow (local.get $frame))
  (unreachable))

;; `control.boundary(body)` (§9.4).
(func $control_boundary (param $body eqref) (param $env (ref null $env)) (result eqref)
  (return_call $boundary_run
    (struct.new $boundary (i32.const 11) (i32.const 0) (i64.const 0) (f64.const 0)
      (ref.null $bytes) (ref.null $values) (ref.null $limbs) (ref.null $call) (i32.const 1))
    (local.get $env) (local.get $body) (ref.null $flist) (ref.null eq) (ref.null eq) (ref.null $frame)))

;; The active boundary `b`, in the place where `env` is: calls `body(b)`,
;; or re-enters `rest` when `body` is null. It is not active once the code
;; inside it is not on the stack (§9.4 rule 1).
(func $boundary_run (param $b (ref $boundary)) (param $env (ref null $env)) (param $body eqref)
    (param $rest (ref null $flist)) (param $v eqref) (param $cancel eqref) (param $frame (ref null $frame))
    (result eqref)
  (local $depth i32) (local $r eqref) (local $c (ref null $cap)) (local $x exnref) (local $f eqref)
  (local.set $depth (call $shadow_depth))
  (block $unwind (result eqref exnref)
    (block $captured (result (ref $cap) exnref)
      (local.set $r
        (try_table (result eqref) (catch_ref $capture $captured) (catch_ref $error $unwind)
            (catch_ref $cancel $unwind)
          (if (result eqref) (ref.is_null (local.get $body))
            (then (call $reenter_next (local.get $rest) (local.get $env) (local.get $v) (local.get $cancel)))
            (else (call $call1 (local.get $body) (local.get $b) (local.get $env))))))
      (struct.set $boundary 8 (local.get $b) (i32.const 0))
      (return (local.get $r)))
    (local.set $x)
    (local.set $c)
    (struct.set $boundary 8 (local.get $b) (i32.const 0))
    (if (ref.eq (struct.get $cap 0 (local.get $c)) (local.get $b))
      (then
        ;; Rule 2: `f(k)` is done in the place of the boundary.
        (call $shadow_restore (local.get $depth))
        (local.set $f (struct.get $cap 1 (local.get $c)))
        (return_call_ref $fn1 (ref.cast (ref $clo1) (local.get $f)) (local.get $env)
          (struct.new $cont (i32.const 12) (i32.const 0) (i64.const 0) (f64.const 0)
            (ref.null $bytes) (ref.null $values) (ref.null $limbs) (ref.null $call)
            (i32.const 0) (local.get $b) (struct.get $cap 2 (local.get $c)))
          (struct.get $clo1 8 (ref.cast (ref $clo1) (local.get $f))))))
    ;; A capture for an outer boundary: this one is paused with it.
    (if (ref.is_null (local.get $frame))
      (then (local.set $frame
        (struct.new $frame (ref.func $boundary_reenter) (array.new_fixed $values 1 (local.get $b))))))
    (call $capture_rethrow (ref.as_non_null (local.get $c)) (local.get $x) (ref.as_non_null (local.get $frame)))
    (unreachable))
  (local.set $x)
  (drop)
  (struct.set $boundary 8 (local.get $b) (i32.const 0))
  (throw_ref (local.get $x)))

(func $boundary_reenter (type $refn) (param $frame (ref $frame)) (param $rest (ref null $flist))
    (param $env (ref null $env)) (param $v eqref) (param $cancel eqref) (result eqref)
  (local $b (ref $boundary))
  (local.set $b (ref.cast (ref $boundary)
    (array.get $values (ref.as_non_null (struct.get $frame 1 (local.get $frame))) (i32.const 0))))
  (struct.set $boundary 8 (local.get $b) (i32.const 1))
  (return_call $boundary_run (local.get $b) (local.get $env) (ref.null eq) (local.get $rest) (local.get $v)
    (local.get $cancel) (local.get $frame)))

;; `control.suspend(b, f)`: stops when `b` is not active.
(func $control_suspend (param $b eqref) (param $f eqref) (result eqref)
  (local $boundary (ref $boundary))
  (local.set $boundary (ref.cast (ref $boundary) (local.get $b)))
  (if (i32.eqz (struct.get $boundary 8 (local.get $boundary)))
    (then (call $stop_boundary_inactive) (unreachable)))
  (throw $capture (struct.new $cap (local.get $boundary) (local.get $f) (ref.null $flist))))

;; The continuation `k`, used now: stops when it was already used (rule 5).
(func $continuation_take (param $k eqref) (result (ref $cont))
  (local $c (ref $cont))
  (local.set $c (ref.cast (ref $cont) (local.get $k)))
  (if (struct.get $cont 8 (local.get $c)) (then (call $stop_continuation_used) (unreachable)))
  (struct.set $cont 8 (local.get $c) (i32.const 1))
  (local.get $c))

;; `control.resume(k, v)`: the paused computation goes on in a boundary that
;; is active again, on top of this place (rule 3).
(func $control_resume (param $k eqref) (param $v eqref) (param $env (ref null $env)) (result eqref)
  (local $c (ref $cont)) (local $b (ref $boundary))
  (local.set $c (call $continuation_take (local.get $k)))
  (local.set $b (struct.get $cont 9 (local.get $c)))
  (struct.set $boundary 8 (local.get $b) (i32.const 1))
  (return_call $boundary_run (local.get $b) (local.get $env) (ref.null eq) (struct.get $cont 10 (local.get $c))
    (local.get $v) (ref.null eq) (ref.null $frame)))

;; `control.cancel(k)`: unwinds the paused computation (rule 4). Its
;; boundary stays not active.
(func $control_cancel (param $k eqref) (param $env (ref null $env)) (result eqref)
  (local $c (ref $cont))
  (local.set $c (call $continuation_take (local.get $k)))
  (return_call $cancel_run (local.get $c) (struct.get $cont 10 (local.get $c)) (local.get $env)
    (ref.null eq) (local.get $c) (ref.null $frame)))

(func $cancel_run (param $k (ref $cont)) (param $rest (ref null $flist)) (param $env (ref null $env))
    (param $v eqref) (param $cancel eqref) (param $frame (ref null $frame)) (result eqref)
  (local $depth i32) (local $e eqref) (local $x exnref)
  (local.set $depth (call $shadow_depth))
  (block $captured (result (ref $cap) exnref)
    (block $cancelled (result eqref exnref)
      (drop (try_table (result eqref) (catch_ref $cancel $cancelled) (catch_ref $capture $captured)
        (call $reenter_next (local.get $rest) (local.get $env) (local.get $v) (local.get $cancel))))
      (return (ref.null eq)))
    (local.set $x)
    (local.set $e)
    (if (i32.eqz (ref.eq (local.get $e) (local.get $k))) (then (throw_ref (local.get $x))))
    (call $shadow_restore (local.get $depth))
    (return (ref.null eq)))
  (call $capture_rethrow
    (if (result (ref $frame)) (ref.is_null (local.get $frame))
      (then (struct.new $frame (ref.func $cancel_reenter) (array.new_fixed $values 1 (local.get $k))))
      (else (ref.as_non_null (local.get $frame)))))
  (unreachable))

(func $cancel_reenter (type $refn) (param $frame (ref $frame)) (param $rest (ref null $flist))
    (param $env (ref null $env)) (param $v eqref) (param $cancel eqref) (result eqref)
  (return_call $cancel_run
    (ref.cast (ref $cont)
      (array.get $values (ref.as_non_null (struct.get $frame 1 (local.get $frame))) (i32.const 0)))
    (local.get $rest) (local.get $env) (local.get $v) (local.get $cancel) (local.get $frame)))

;; # The host boundary (§9.7, B.4, B.5)
;;
;; An async entry point runs in a root boundary, `$host_root`. A function of
;; an async host service starts the import, records it in `$host_pending`,
;; and suspends to the root boundary, where `$host_wait` keeps the
;; continuation in `$host_k` and gives `$host_waiting`: the status is 1. The
;; export `ilang_resume:…` of the import puts the answer in the globals of the
;; import and resumes the continuation, so the function reads the answer
;; with the frames of the program on the shadow stack (§11.7).
(global $host_root (mut (ref null $boundary)) (ref.null $boundary))
(global $host_k (mut eqref) (ref.null eq))
(global $host_pending (mut i32) (i32.const -1))
(global $host_waiting eqref (struct.new $waiting (i32.const 0)))
;; The async entry point that runs: whether it is a host export, which gives
;; its errors to the host, and the site of its binding (§11.7).
(global $async_export (mut i32) (i32.const 0))
(global $async_site (mut i32) (i32.const -1))
;; How it ended: its value or error, and 1 for an error.
(global $async_value (mut eqref) (ref.null eq))
(global $async_error (mut i32) (i32.const 0))

(func $host_wait (param $k eqref) (result eqref)
  (global.set $host_k (local.get $k))
  (global.get $host_waiting))

(func $host_root_boundary (result eqref)
  (if (ref.is_null (global.get $host_root)) (then unreachable))
  (global.get $host_root))

(func $host_boundary (param $body eqref) (param $env (ref null $env)) (result eqref)
  (local $b (ref $boundary))
  (local.set $b (struct.new $boundary (i32.const 11) (i32.const 0) (i64.const 0) (f64.const 0)
    (ref.null $bytes) (ref.null $values) (ref.null $limbs) (ref.null $call) (i32.const 1)))
  (global.set $host_root (local.get $b))
  (return_call $boundary_run (local.get $b) (local.get $env) (local.get $body) (ref.null $flist)
    (ref.null eq) (ref.null eq) (ref.null $frame)))

(func $host_resume_k (result eqref)
  (local $k eqref)
  (global.set $host_pending (i32.const -1))
  (local.set $k (global.get $host_k))
  (global.set $host_k (ref.null eq))
  (return_call $control_resume (local.get $k) (ref.null eq) (global.get $root_env)))

;; The start of an async entry point. While one waits, the host may only
;; answer it (B.5).
(func $async_begin (param $export i32) (param $site i32)
  (if (i32.ne (global.get $host_pending) (i32.const -1)) (then unreachable))
  (global.set $async_export (local.get $export))
  (global.set $async_site (local.get $site))
  (global.set $async_value (ref.null eq))
  (global.set $async_error (i32.const 0)))

;; `ilang_result:…` may only be called when the entry point has ended.
(func $async_result
  (if (i32.ne (global.get $host_pending) (i32.const -1)) (then unreachable)))

;; Runs the body of an async entry point in a new root boundary, or, when
;; `body` is null, goes on with it after the host answered. Gives the status
;; of B.5: 1 while it waits, 0 when it has ended.
(func $async_run (param $body eqref) (result i32)
  (local $r eqref) (local $e eqref) (local $depth i32)
  (local.set $depth (call $shadow_depth))
  (local.set $e (block $error (result eqref)
    (block $cancel (result eqref)
      (local.set $r (try_table (result eqref) (catch $error $error) (catch $cancel $cancel)
        (if (result eqref) (ref.is_null (local.get $body))
          (then (call $host_resume_k))
          (else (call $host_boundary (local.get $body) (global.get $root_env))))))
      (if (ref.eq (local.get $r) (global.get $host_waiting)) (then (return (i32.const 1))))
      (global.set $async_value (local.get $r))
      (return (i32.const 0)))
    (drop) (call $stop_boundary_inactive) (unreachable)))
  ;; An error stops the program entry and tests; a host export gives it to
  ;; the host (§10.2).
  (if (i32.eqz (global.get $async_export))
    (then (call $stop_error (local.get $e) (global.get $async_site)) (unreachable)))
  (call $shadow_restore (local.get $depth))
  (global.set $async_value (local.get $e))
  (global.set $async_error (i32.const 1))
  (i32.const 0))

;; The `fail` of the `Assert` of an entry point (§9.6, §11.7).
(func $entry_assert_fail (type $fn1) (param $clo (ref null $obj)) (param $env (ref null $env)) (param $failure eqref) (result eqref)
  (call $stop_assertion (call $field (local.get $failure) (i32.const 0)))
  (unreachable))
(data $s0 "\61\73\73\65\72\74")
(data $site_data "\00\00\00\00\04\00\00\00\01\00\00\00\00\00\00\00\05\00\00\00\09\00\00\00\00\00\00\00\05\00\00\00\02\00\00\00")
(data $s1 "\61\70\70\2f\6d\61\69\6e")
(data $s2 "\49\6f")
;; app/main.main
(func $f0 (type $fn0) (param $clo (ref null $obj)) (param $env (ref null $env)) (result eqref)
 (local $l0 eqref) (local $l1 eqref) (local $l2 eqref)
 (local $slot i32)
(call $shadow_enter_call)
(call $shadow_site (i32.const 1))
(local.set $l0 (call $f2 (ref.null $obj) (local.get $env)))
(local.set $l1 (call $bool_new (call $equal (local.get $l0) (ref.i31 (i32.const 84)))))
(call $shadow_site (i32.const 2))
(local.set $slot (call $shadow_top))
(local.set $l2 (block $ok0 (result eqref)
(block $cap0 (result (ref $cap) exnref)
(br $ok0 (try_table (result eqref) (catch_ref $capture $cap0)
(call $f1 (ref.null $obj) (local.get $env) (local.get $l1)))))
(call $capture_rethrow (struct.new $frame (ref.func $r0) (array.new_fixed $values 1 (ref.i31 (local.get $slot))))) (unreachable)))
(call $shadow_leave)
(return (ref.null eq))
(unreachable))
;; i/prelude.assert
(func $f1 (type $fn1) (param $clo (ref null $obj)) (param $env (ref null $env)) (param $l0 eqref) (result eqref)
 (local $l1 eqref) (local $l2 eqref) (local $l3 eqref) (local $l4 eqref)
 (local $slot i32)
(call $shadow_enter_call)
(local.set $l1 (local.get $l0))
(local.set $l2 (array.get $env (local.get $env) (i32.const 0)))
(block $b0
(block $b1
(if (i32.eqz (call $is_bool (local.get $l1) (i32.const 1))) (then
(br $b1)
) (else
))
(call $shadow_leave)
(return (ref.null eq))
)
(block $b2
(if (i32.eqz (call $is_bool (local.get $l1) (i32.const 0))) (then
(br $b2)
) (else
))
(local.set $l3 (call $field (local.get $l2) (i32.const 0)))
(local.set $l4 (call $record (i32.const 2) (i64.const 1) (array.new_fixed $values 1 (call $string_from_bytes (array.new_data $bytes $s0 (i32.const 0) (i32.const 6))))))
(call $shadow_leave_tail (i32.const -1))
(return_call_ref $fn1 (ref.cast (ref $clo1) (local.get $l3)) (local.get $env) (local.get $l4) (struct.get $clo1 8 (ref.cast (ref $clo1) (local.get $l3))))
)
(unreachable)
)
(unreachable))
;; app/main.f
(func $f2 (type $fn0) (param $clo (ref null $obj)) (param $env (ref null $env)) (result eqref)
 (local $l0 eqref) (local $l1 eqref)
 (local $slot i32)
(call $shadow_enter_call)
(local.set $l0 (call $int_add (ref.i31 (i32.const 20)) (ref.i31 (i32.const 22))))
(local.set $l1 (call $int_mul (local.get $l0) (ref.i31 (i32.const 2))))
(call $shadow_leave)
(return (local.get $l1))
(unreachable))
;; app/main.main after point 0
(func $s0 (param $env (ref null $env)) (param $res eqref) (result eqref)
 (local $clo (ref null $obj)) (local $slot i32) (local $l2 eqref)
(local.set $l2 (local.get $res))
(call $shadow_leave)
(return (ref.null eq))
(unreachable))
(func $r0 (type $refn) (param $frame (ref $frame)) (param $rest (ref null $flist)) (param $env (ref null $env)) (param $v eqref) (param $cancel eqref) (result eqref)
 (local $saved (ref null $values)) (local $res eqref)
(local.set $saved (struct.get $frame 1 (local.get $frame)))
(call $shadow_enter (i31.get_s (ref.cast (ref i31) (array.get $values (ref.as_non_null (local.get $saved)) (i32.const 0)))))
(local.set $res (block $ok (result eqref)
(block $cap (result (ref $cap) exnref)
(br $ok (try_table (result eqref) (catch_ref $capture $cap)
(call $reenter_next (local.get $rest) (local.get $env) (local.get $v) (local.get $cancel)))))
(call $capture_rethrow (local.get $frame)) (unreachable)))
(return_call $s0 (local.get $env) (local.get $res)))
;; ilang_start
(func $start (type $fn0) (param $clo (ref null $obj)) (param $env (ref null $env)) (result eqref)

 (local $slot i32)
(call $shadow_enter_call)
(call $shadow_leave)
(return (ref.null eq))
(unreachable))
(func $ilang_start (export "ilang_start")
(call $sites_install (array.new_data $sites $site_data (i32.const 0) (i32.const 9))
(array.new_fixed $values 1 (call $string_from_bytes (array.new_data $bytes $s1 (i32.const 0) (i32.const 8)))))
(call $constructors_install (array.new_fixed $values 1 (call $string_from_bytes (array.new_data $bytes $s2 (i32.const 0) (i32.const 2)))))
(global.set $root_env (array.new $env (ref.null eq) (i32.const 1)))
(array.set $env (ref.as_non_null (global.get $root_env)) (i32.const 0)
(call $record (i32.const 3) (i64.const 2) (array.new_fixed $values 1
(struct.new $clo1 (i32.const 7) (i32.const 0) (i64.const 1) (f64.const 0) (ref.null $bytes)
(ref.null $values) (ref.null $limbs) (ref.null $call) (ref.func $entry_assert_fail)))))
(drop (call $start (ref.null $obj) (global.get $root_env)))
)
(func $entry_app_2fmain_2emain (export "app/main.main") (local $e eqref)
(local.set $e (block $error (result eqref)
(block $cancel (result eqref)
(drop (try_table (result eqref) (catch $error $error) (catch $cancel $cancel) (call $f0 (ref.null $obj) (global.get $root_env))))
(return))
(drop) (call $stop_boundary_inactive) (unreachable)))
(call $stop_error (local.get $e) (i32.const 0)) (unreachable))
(elem declare func $f0 $f1 $f2 $r0 $entry_assert_fail $catch_reenter $ensure_reenter $final_reenter $boundary_reenter $cancel_reenter)
)
