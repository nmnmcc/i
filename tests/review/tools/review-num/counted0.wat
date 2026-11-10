(module
  (type $bytes (;0;) (array (mut i8)))
  (type $values (;1;) (array (mut eqref)))
  (type $limbs (;2;) (array (mut i32)))
  (type $call (;3;) (func (param eqref eqref) (result eqref)))
  (type $val (;4;) (sub (struct (field i32))))
  (type $obj (;5;) (sub $val (struct (field i32) (field i32) (field i64) (field f64) (field (ref null $bytes)) (field (ref null $values)) (field (ref null $limbs)) (field (ref null $call)))))
  (type $cons (;6;) (sub final $val (struct (field i32) (field eqref) (field eqref))))
  (type $hostref (;7;) (struct (field externref) (field i64)))
  (type $sites (;8;) (array i32))
  (type $frames (;9;) (array (mut i32)))
  (type $unpend (;10;) (func))
  (type $env (;11;) (array (mut eqref)))
  (type $boundary (;12;) (sub final $obj (struct (field i32) (field i32) (field i64) (field f64) (field (ref null $bytes)) (field (ref null $values)) (field (ref null $limbs)) (field (ref null $call)) (field (mut i32)))))
  (rec
    (type $frame (;13;) (struct (field (ref $refn)) (field (ref null $values)) (field i32) (field i32) (field (ref null $values))))
    (type $refn (;14;) (func (param (ref $frame) (ref null $env) eqref i32) (result eqref)))
    (type $flist (;15;) (struct (field (ref $frame)) (field (mut (ref null $flist))) (field (mut (ref null $flist))) (field (mut (ref null $env))) (field (mut i32))))
  )
  (type $sub (;16;) (struct (field (ref null $flist)) (field (ref null $flist)) (field (ref null $flist)) (field (ref null $flist))))
  (type $cont (;17;) (sub final $obj (struct (field i32) (field i32) (field i64) (field f64) (field (ref null $bytes)) (field (ref null $values)) (field (ref null $limbs)) (field (ref null $call)) (field (mut i32)) (field (ref $boundary)) (field (ref null $sub)))))
  (type $cap (;18;) (struct (field (ref $boundary)) (field eqref) (field (mut (ref null $flist))) (field (mut (ref null $flist))) (field (mut (ref null $flist))) (field (mut (ref null $flist)))))
  (type $tramp (;19;) (struct (field i32) (field (mut (ref null $flist))) (field (ref null $tramp))))
  (type $box (;20;) (struct (field i64)))
  (type $waiting (;21;) (struct (field i32)))
  (type $fn0 (;22;) (func (param (ref null $obj) (ref null $env)) (result eqref)))
  (type $clo0 (;23;) (sub final $obj (struct (field i32) (field i32) (field i64) (field f64) (field (ref null $bytes)) (field (ref null $values)) (field (ref null $limbs)) (field (ref null $call)) (field (ref $fn0)))))
  (type $fn1 (;24;) (func (param (ref null $obj) (ref null $env) eqref) (result eqref)))
  (type $clo1 (;25;) (sub final $obj (struct (field i32) (field i32) (field i64) (field f64) (field (ref null $bytes)) (field (ref null $values)) (field (ref null $limbs)) (field (ref null $call)) (field (ref $fn1)))))
  (type $fn2 (;26;) (func (param (ref null $obj) (ref null $env) eqref eqref) (result eqref)))
  (type $clo2 (;27;) (sub final $obj (struct (field i32) (field i32) (field i64) (field f64) (field (ref null $bytes)) (field (ref null $values)) (field (ref null $limbs)) (field (ref null $call)) (field (ref $fn2)))))
  (type $fn3 (;28;) (func (param (ref null $obj) (ref null $env) eqref eqref eqref) (result eqref)))
  (type $clo3 (;29;) (sub final $obj (struct (field i32) (field i32) (field i64) (field f64) (field (ref null $bytes)) (field (ref null $values)) (field (ref null $limbs)) (field (ref null $call)) (field (ref $fn3)))))
  (type $compare_frame (;30;) (struct (field (ref null $compare_frame)) (field (ref null $values)) (field (ref null $values)) (field (mut i32)) (field eqref) (field eqref)))
  (type $case_ranges (;31;) (array i32))
  (type $search_table (;32;) (array (mut i32)))
  (type (;33;) (func (param i32 i32)))
  (type (;34;) (func (param externref) (result i64)))
  (type (;35;) (func (param i32) (result i32)))
  (type (;36;) (func (param eqref) (result i32)))
  (type (;37;) (func (param eqref) (result (ref $limbs))))
  (type (;38;) (func (param (ref $limbs)) (result i32)))
  (type (;39;) (func (param i32 (ref $limbs)) (result eqref)))
  (type (;40;) (func (param i64) (result eqref)))
  (type (;41;) (func (param eqref) (result i64)))
  (type (;42;) (func (param (ref $limbs) (ref $limbs)) (result i32)))
  (type (;43;) (func (param eqref eqref) (result i32)))
  (type (;44;) (func (param (ref $limbs) (ref $limbs)) (result (ref $limbs))))
  (type (;45;) (func (param eqref) (result eqref)))
  (type (;46;) (func (param (ref $limbs) (ref $limbs)) (result (ref $limbs) (ref $limbs))))
  (type (;47;) (func (param eqref i32) (result (ref $limbs))))
  (type (;48;) (func (param (ref $limbs)) (result eqref)))
  (type (;49;) (func (param eqref eqref i32) (result eqref)))
  (type (;50;) (func (param (ref $limbs) i32) (result i32)))
  (type (;51;) (func (param (ref $bytes)) (result eqref)))
  (type (;52;) (func (param eqref) (result f64)))
  (type (;53;) (func (param f64) (result eqref)))
  (type (;54;) (func (param eqref eqref) (result eqref i32)))
  (type (;55;) (func (param (ref $bytes) i32 i64) (result eqref)))
  (type (;56;) (func (param eqref) (result (ref $bytes))))
  (type (;57;) (func (param eqref i64 i64) (result i32)))
  (type (;58;) (func (param i64) (result (ref $bytes))))
  (type (;59;) (func (result eqref)))
  (type (;60;) (func (param eqref i64 i32) (result i64)))
  (type (;61;) (func (param eqref i64) (result i32)))
  (type (;62;) (func (param (ref $bytes) i64 i32)))
  (type (;63;) (func (param eqref i64 i64) (result eqref)))
  (type (;64;) (func (param i64 i64 i32) (result i64)))
  (type (;65;) (func (param (ref $limbs) i64) (result i32)))
  (type (;66;) (func (param eqref i64 i32) (result eqref)))
  (type (;67;) (func (param eqref i64 i64 i32 i32) (result eqref)))
  (type (;68;) (func (param i64 i64 i32) (result eqref)))
  (type (;69;) (func (param eqref i64 i64 i32) (result i64)))
  (type (;70;) (func (param i64 i64) (result i64)))
  (type (;71;) (func (param f64) (result i64)))
  (type (;72;) (func (param i64) (result f64)))
  (type (;73;) (func (param eqref i64) (result i64)))
  (type (;74;) (func (param eqref i64) (result eqref)))
  (type (;75;) (func (param eqref i64 i64 i32) (result i32)))
  (type (;76;) (func (param eqref i64 i64 i32) (result eqref)))
  (type (;77;) (func (param i32) (result eqref)))
  (type (;78;) (func (param eqref i64 i32) (result i32)))
  (type (;79;) (func (param eqref i64 i32 i32) (result eqref i64 i32)))
  (type (;80;) (func (param (ref $bytes) i32 i32 i32)))
  (type (;81;) (func (param eqref i32 i32) (result eqref)))
  (type (;82;) (func (param eqref i32) (result eqref)))
  (type (;83;) (func (param eqref i32 i32 i32) (result eqref)))
  (type (;84;) (func (param externref) (result eqref)))
  (type (;85;) (func (param eqref) (result externref)))
  (type (;86;) (func (param i32 i64 (ref $values)) (result eqref)))
  (type (;87;) (func (param (ref $values)) (result eqref)))
  (type (;88;) (func (param i32 i32) (result i32)))
  (type (;89;) (func (param i64 i64) (result i32)))
  (type (;90;) (func (param (ref $bytes) i32) (result i32)))
  (type (;91;) (func (param (ref $bytes)) (result i32)))
  (type (;92;) (func (param (ref $bytes) i32 i32) (result i32)))
  (type (;93;) (func (param i32 i32) (result eqref)))
  (type (;94;) (func (param (ref $bytes) (ref $bytes) i32) (result i32)))
  (type (;95;) (func (param i32)))
  (type (;96;) (func (param eqref)))
  (type (;97;) (func (param (ref $values))))
  (type (;98;) (func (param i32 eqref i32)))
  (type (;99;) (func (param eqref i32)))
  (type (;100;) (func (param (ref $sites) (ref $values))))
  (type (;101;) (func (result i32)))
  (type (;102;) (func (param eqref eqref eqref) (result eqref)))
  (type (;103;) (func (result (ref $case_ranges))))
  (type (;104;) (func (param (ref $case_ranges) i32) (result i32)))
  (type (;105;) (func (param eqref (ref $case_ranges) i32 i32) (result eqref)))
  (type (;106;) (func (param (ref $bytes) i64 eqref)))
  (type (;107;) (func (param i32 i64)))
  (type (;108;) (func (param i32 i32 i32) (result eqref)))
  (type (;109;) (func (param eqref) (result i32 i32 i32)))
  (type (;110;) (func (param i32 i32) (result (ref $bytes))))
  (type (;111;) (func (param eqref) (result i32 i32)))
  (type (;112;) (func (param eqref i32) (result i32)))
  (type (;113;) (func (param eqref f64) (result i32)))
  (type (;114;) (func (param (ref $cap))))
  (type (;115;) (func (param (ref null $env) i32 eqref) (result (ref null $env))))
  (type (;116;) (func (param (ref null $env) (ref null $values)) (result (ref null $env))))
  (type (;117;) (func (param eqref (ref null $env)) (result eqref)))
  (type (;118;) (func (param eqref eqref (ref null $env)) (result eqref)))
  (type (;119;) (func (param (ref $cap) exnref (ref $frame))))
  (type (;120;) (func (param (ref $cap)) (result (ref null $sub))))
  (type (;121;) (func (param (ref $cap) exnref (ref $refn) eqref)))
  (type (;122;) (func (param (ref $cap) (ref null $flist) (ref null $flist) (ref null $flist) (ref null $flist))))
  (type (;123;) (func (param (ref null $flist)) (result i32)))
  (type (;124;) (func (param (ref $frames) i32 (ref null $flist)) (result i32)))
  (type (;125;) (func (param i32 (ref null $flist))))
  (type (;126;) (func (param (ref null $sub) (ref null $env) eqref i32) (result eqref)))
  (type (;127;) (func (result (ref $cap) exnref)))
  (type (;128;) (func (param (ref null $env) eqref) (result eqref)))
  (type (;129;) (func (param eqref (ref null $env) eqref) (result eqref)))
  (type (;130;) (func (param eqref (ref null $env) i32 eqref) (result eqref)))
  (type (;131;) (func (param i32 eqref) (result eqref)))
  (type (;132;) (func (param (ref $boundary) (ref null $env) eqref (ref null $flist) eqref eqref (ref null $frame)) (result eqref)))
  (type (;133;) (func (param (ref $boundary) (ref null $env) eqref (ref null $sub) eqref i32) (result eqref)))
  (type (;134;) (func (result eqref exnref)))
  (type (;135;) (func (param eqref) (result (ref $cont))))
  (type (;136;) (func (param (ref $cont) (ref null $sub) (ref null $env) eqref i32) (result eqref)))
  (type (;137;) (func (param (ref null $env) eqref eqref eqref) (result eqref)))
  (type (;138;) (func (param (ref null $env)) (result eqref)))
  (type (;139;) (func (param f64) (result f64)))
  (import "ilang:host/v1" "panic" (func $host_panic (;0;) (type 33) (param i32 i32)))
  (import "ilang:host/v1" "reference_id" (func $host_reference_id (;1;) (type 34) (param externref) (result i64)))
  (memory (;0;) 1)
  (tag $error (;0;) (type 96) (param eqref))
  (tag $cancel (;1;) (type 96) (param eqref))
  (tag $capture (;2;) (type 114) (param (ref $cap)))
  (global $heap (;0;) (mut i32) i32.const 1024)
  (global $free_head (;1;) (mut i32) i32.const 0)
  (global $value_false (;2;) eqref i32.const 1 i32.const 0 i64.const 0 f64.const 0x0p+0 (;=0;) ref.null $bytes ref.null $values ref.null $limbs ref.null $call struct.new $obj)
  (global $value_true (;3;) eqref i32.const 1 i32.const 1 i64.const 0 f64.const 0x0p+0 (;=0;) ref.null $bytes ref.null $values ref.null $limbs ref.null $call struct.new $obj)
  (global $value_empty_list (;4;) eqref i32.const 6 i32.const 0 i64.const 0 f64.const 0x0p+0 (;=0;) ref.null $bytes ref.null $values ref.null $limbs ref.null $call struct.new $obj)
  (global $ib (;5;) (mut (ref null $bytes)) ref.null $bytes)
  (global $il (;6;) (mut i32) i32.const 0)
  (global $constructor_names (;7;) (mut (ref null $values)) ref.null $values)
  (global $it_values (;8;) (mut (ref null $values)) ref.null $values)
  (global $it_meta (;9;) (mut (ref null $frames)) ref.null $frames)
  (global $it_sp (;10;) (mut i32) i32.const 0)
  (global $shadow (;11;) (mut (ref $frames)) i32.const 256 array.new_default $frames)
  (global $sp (;12;) (mut i32) i32.const 0)
  (global $site_table (;13;) (mut (ref null $sites)) ref.null $sites)
  (global $site_modules (;14;) (mut (ref null $values)) ref.null $values)
  (global $stopping (;15;) (mut i32) i32.const 0)
  (global $stop_unpend (;16;) (mut (ref null $unpend)) ref.null $unpend)
  (global $tail (;17;) (mut i32) i32.const -1)
  (global $case_upper (;18;) (mut (ref null $case_ranges)) ref.null $case_ranges)
  (global $case_lower (;19;) (mut (ref null $case_ranges)) ref.null $case_ranges)
  (global $root_env (;20;) (mut (ref null $env)) ref.null $env)
  (global $pend (;21;) (mut (ref null $tramp)) ref.null $tramp)
  (global $host_root (;22;) (mut (ref null $boundary)) ref.null $boundary)
  (global $host_k (;23;) (mut eqref) ref.null eq)
  (global $host_import (;24;) (mut i32) i32.const -1)
  (global $host_request (;25;) (mut i32) i32.const -1)
  (global $host_next (;26;) (mut i32) i32.const 0)
  (global $async_live (;27;) (mut i32) i32.const 0)
  (global $host_waiting (;28;) eqref i32.const 0 struct.new $waiting)
  (global $async_export (;29;) (mut i32) i32.const 0)
  (global $async_site (;30;) (mut i32) i32.const -1)
  (global $async_value (;31;) (mut eqref) ref.null eq)
  (global $async_error (;32;) (mut i32) i32.const 0)
  (global $k89_49 (;33;) eqref i32.const 5 i32.const 89 i64.const 49 f64.const 0x0p+0 (;=0;) ref.null $bytes ref.null $values ref.null $limbs ref.null $call struct.new $obj)
  (global $g0 (;34;) (mut eqref) ref.null eq)
  (global $g1 (;35;) (mut eqref) ref.null eq)
  (global $g2 (;36;) (mut eqref) ref.null eq)
  (global $g3 (;37;) (mut eqref) ref.null eq)
  (global $g4 (;38;) (mut eqref) ref.null eq)
  (global $g5 (;39;) (mut eqref) ref.null eq)
  (global $g6 (;40;) (mut eqref) ref.null eq)
  (global $g7 (;41;) (mut eqref) ref.null eq)
  (global $g8 (;42;) (mut eqref) ref.null eq)
  (global $g9 (;43;) (mut eqref) ref.null eq)
  (global $g10 (;44;) (mut eqref) ref.null eq)
  (global $g11 (;45;) (mut eqref) ref.null eq)
  (global $g12 (;46;) (mut eqref) ref.null eq)
  (global $g13 (;47;) (mut eqref) ref.null eq)
  (global $g14 (;48;) (mut eqref) ref.null eq)
  (global $g15 (;49;) (mut eqref) ref.null eq)
  (global $g16 (;50;) (mut eqref) ref.null eq)
  (global $g17 (;51;) (mut eqref) ref.null eq)
  (global $g18 (;52;) (mut eqref) ref.null eq)
  (global $g19 (;53;) (mut eqref) ref.null eq)
  (global $g20 (;54;) (mut eqref) ref.null eq)
  (global $g21 (;55;) (mut eqref) ref.null eq)
  (global $g22 (;56;) (mut eqref) ref.null eq)
  (global $g23 (;57;) (mut eqref) ref.null eq)
  (global $g24 (;58;) (mut eqref) ref.null eq)
  (global $g25 (;59;) (mut eqref) ref.null eq)
  (global $g26 (;60;) (mut eqref) ref.null eq)
  (global $g27 (;61;) (mut eqref) ref.null eq)
  (global $g28 (;62;) (mut eqref) ref.null eq)
  (global $g29 (;63;) (mut eqref) ref.null eq)
  (global $g30 (;64;) (mut eqref) ref.null eq)
  (global $g31 (;65;) (mut eqref) ref.null eq)
  (global $g32 (;66;) (mut eqref) ref.null eq)
  (global $g33 (;67;) (mut eqref) ref.null eq)
  (global $g34 (;68;) (mut eqref) ref.null eq)
  (global $g35 (;69;) (mut eqref) ref.null eq)
  (global $g36 (;70;) (mut eqref) ref.null eq)
  (global $g37 (;71;) (mut eqref) ref.null eq)
  (global $g38 (;72;) (mut eqref) ref.null eq)
  (global $g39 (;73;) (mut eqref) ref.null eq)
  (global $g40 (;74;) (mut eqref) ref.null eq)
  (global $g41 (;75;) (mut eqref) ref.null eq)
  (global $g42 (;76;) (mut eqref) ref.null eq)
  (global $g43 (;77;) (mut eqref) ref.null eq)
  (global $g44 (;78;) (mut eqref) ref.null eq)
  (global $g45 (;79;) (mut eqref) ref.null eq)
  (global $g46 (;80;) (mut eqref) ref.null eq)
  (global $g47 (;81;) (mut eqref) ref.null eq)
  (global $g48 (;82;) (mut eqref) ref.null eq)
  (global $g49 (;83;) (mut eqref) ref.null eq)
  (global $g50 (;84;) (mut eqref) ref.null eq)
  (global $g51 (;85;) (mut eqref) ref.null eq)
  (global $g52 (;86;) (mut eqref) ref.null eq)
  (global $g53 (;87;) (mut eqref) ref.null eq)
  (global $g54 (;88;) (mut eqref) ref.null eq)
  (global $g55 (;89;) (mut eqref) ref.null eq)
  (global $g56 (;90;) (mut eqref) ref.null eq)
  (global $g57 (;91;) (mut eqref) ref.null eq)
  (global $g58 (;92;) (mut eqref) ref.null eq)
  (global $g59 (;93;) (mut eqref) ref.null eq)
  (global $g60 (;94;) (mut eqref) ref.null eq)
  (global $g61 (;95;) (mut eqref) ref.null eq)
  (global $g62 (;96;) (mut eqref) ref.null eq)
  (global $g63 (;97;) (mut eqref) ref.null eq)
  (global $g64 (;98;) (mut eqref) ref.null eq)
  (global $g65 (;99;) (mut eqref) ref.null eq)
  (global $g66 (;100;) (mut eqref) ref.null eq)
  (global $g67 (;101;) (mut eqref) ref.null eq)
  (global $g68 (;102;) (mut eqref) ref.null eq)
  (global $g69 (;103;) (mut eqref) ref.null eq)
  (global $g70 (;104;) (mut eqref) ref.null eq)
  (global $g71 (;105;) (mut eqref) ref.null eq)
  (global $g72 (;106;) (mut eqref) ref.null eq)
  (global $g73 (;107;) (mut eqref) ref.null eq)
  (global $g74 (;108;) (mut eqref) ref.null eq)
  (global $g75 (;109;) (mut eqref) ref.null eq)
  (global $g76 (;110;) (mut eqref) ref.null eq)
  (global $g77 (;111;) (mut eqref) ref.null eq)
  (global $g78 (;112;) (mut eqref) ref.null eq)
  (global $g79 (;113;) (mut eqref) ref.null eq)
  (global $g80 (;114;) (mut eqref) ref.null eq)
  (global $g81 (;115;) (mut eqref) ref.null eq)
  (global $g82 (;116;) (mut eqref) ref.null eq)
  (global $g83 (;117;) (mut eqref) ref.null eq)
  (global $g84 (;118;) (mut eqref) ref.null eq)
  (global $g85 (;119;) (mut eqref) ref.null eq)
  (global $g86 (;120;) (mut eqref) ref.null eq)
  (global $g87 (;121;) (mut eqref) ref.null eq)
  (global $g88 (;122;) (mut eqref) ref.null eq)
  (global $g89 (;123;) (mut eqref) ref.null eq)
  (global $g90 (;124;) (mut eqref) ref.null eq)
  (global $g91 (;125;) (mut eqref) ref.null eq)
  (global $g92 (;126;) (mut eqref) ref.null eq)
  (global $g93 (;127;) (mut eqref) ref.null eq)
  (global $g94 (;128;) (mut eqref) ref.null eq)
  (global $g95 (;129;) (mut eqref) ref.null eq)
  (global $g96 (;130;) (mut eqref) ref.null eq)
  (global $g97 (;131;) (mut eqref) ref.null eq)
  (global $g98 (;132;) (mut eqref) ref.null eq)
  (global $g99 (;133;) (mut eqref) ref.null eq)
  (global $g100 (;134;) (mut eqref) ref.null eq)
  (global $g101 (;135;) (mut eqref) ref.null eq)
  (global $g102 (;136;) (mut eqref) ref.null eq)
  (global $g103 (;137;) (mut eqref) ref.null eq)
  (global $g104 (;138;) (mut eqref) ref.null eq)
  (global $g105 (;139;) (mut eqref) ref.null eq)
  (global $g106 (;140;) (mut eqref) ref.null eq)
  (global $g107 (;141;) (mut eqref) ref.null eq)
  (global $g108 (;142;) (mut eqref) ref.null eq)
  (global $g109 (;143;) (mut eqref) ref.null eq)
  (global $g110 (;144;) (mut eqref) ref.null eq)
  (global $g111 (;145;) (mut eqref) ref.null eq)
  (global $g112 (;146;) (mut eqref) ref.null eq)
  (global $g113 (;147;) (mut eqref) ref.null eq)
  (global $g114 (;148;) (mut eqref) ref.null eq)
  (global $g115 (;149;) (mut eqref) ref.null eq)
  (global $g116 (;150;) (mut eqref) ref.null eq)
  (global $g117 (;151;) (mut eqref) ref.null eq)
  (global $g118 (;152;) (mut eqref) ref.null eq)
  (global $g119 (;153;) (mut eqref) ref.null eq)
  (global $g120 (;154;) (mut eqref) ref.null eq)
  (global $g121 (;155;) (mut eqref) ref.null eq)
  (global $g122 (;156;) (mut eqref) ref.null eq)
  (global $g123 (;157;) (mut eqref) ref.null eq)
  (global $g124 (;158;) (mut eqref) ref.null eq)
  (global $g125 (;159;) (mut eqref) ref.null eq)
  (global $g126 (;160;) (mut eqref) ref.null eq)
  (global $g127 (;161;) (mut eqref) ref.null eq)
  (global $g128 (;162;) (mut eqref) ref.null eq)
  (global $g129 (;163;) (mut eqref) ref.null eq)
  (global $g130 (;164;) (mut eqref) ref.null eq)
  (global $g131 (;165;) (mut eqref) ref.null eq)
  (global $g132 (;166;) (mut eqref) ref.null eq)
  (global $g133 (;167;) (mut eqref) ref.null eq)
  (global $g134 (;168;) (mut eqref) ref.null eq)
  (global $g135 (;169;) (mut eqref) ref.null eq)
  (global $g136 (;170;) (mut eqref) ref.null eq)
  (global $g137 (;171;) (mut eqref) ref.null eq)
  (global $g138 (;172;) (mut eqref) ref.null eq)
  (global $g139 (;173;) (mut eqref) ref.null eq)
  (global $g140 (;174;) (mut eqref) ref.null eq)
  (global $g141 (;175;) (mut eqref) ref.null eq)
  (global $g142 (;176;) (mut eqref) ref.null eq)
  (global $g143 (;177;) (mut eqref) ref.null eq)
  (global $g144 (;178;) (mut eqref) ref.null eq)
  (global $g145 (;179;) (mut eqref) ref.null eq)
  (global $g146 (;180;) (mut eqref) ref.null eq)
  (global $g147 (;181;) (mut eqref) ref.null eq)
  (global $g148 (;182;) (mut eqref) ref.null eq)
  (global $g149 (;183;) (mut eqref) ref.null eq)
  (global $g150 (;184;) (mut eqref) ref.null eq)
  (global $g151 (;185;) (mut eqref) ref.null eq)
  (global $g152 (;186;) (mut eqref) ref.null eq)
  (global $g153 (;187;) (mut eqref) ref.null eq)
  (global $g154 (;188;) (mut eqref) ref.null eq)
  (global $g155 (;189;) (mut eqref) ref.null eq)
  (global $prof_0 (mut i64) (i64.const 0))
  (global $prof_1 (mut i64) (i64.const 0))
  (global $prof_2 (mut i64) (i64.const 0))
  (global $prof_3 (mut i64) (i64.const 0))
  (global $prof_4 (mut i64) (i64.const 0))
  (global $prof_5 (mut i64) (i64.const 0))
  (global $prof_6 (mut i64) (i64.const 0))
  (global $prof_7 (mut i64) (i64.const 0))
  (global $prof_8 (mut i64) (i64.const 0))
  (global $prof_9 (mut i64) (i64.const 0))
  (global $prof_10 (mut i64) (i64.const 0))
  (global $prof_11 (mut i64) (i64.const 0))
  (global $prof_12 (mut i64) (i64.const 0))
  (global $prof_13 (mut i64) (i64.const 0))
  (global $prof_14 (mut i64) (i64.const 0))
  (global $prof_15 (mut i64) (i64.const 0))
  (global $prof_16 (mut i64) (i64.const 0))
  (global $prof_17 (mut i64) (i64.const 0))
  (global $prof_18 (mut i64) (i64.const 0))
  (global $prof_19 (mut i64) (i64.const 0))
  (global $prof_20 (mut i64) (i64.const 0))
  (global $prof_21 (mut i64) (i64.const 0))
  (global $prof_22 (mut i64) (i64.const 0))
  (global $prof_23 (mut i64) (i64.const 0))
  (global $prof_24 (mut i64) (i64.const 0))
  (global $prof_25 (mut i64) (i64.const 0))
  (global $prof_26 (mut i64) (i64.const 0))
  (global $prof_27 (mut i64) (i64.const 0))
  (global $prof_28 (mut i64) (i64.const 0))
  (global $prof_29 (mut i64) (i64.const 0))
  (global $prof_30 (mut i64) (i64.const 0))
  (global $prof_31 (mut i64) (i64.const 0))
  (global $prof_32 (mut i64) (i64.const 0))
  (global $prof_33 (mut i64) (i64.const 0))
  (global $prof_34 (mut i64) (i64.const 0))
  (global $prof_35 (mut i64) (i64.const 0))
  (global $prof_36 (mut i64) (i64.const 0))
  (global $prof_37 (mut i64) (i64.const 0))
  (global $prof_38 (mut i64) (i64.const 0))
  (global $prof_39 (mut i64) (i64.const 0))
  (global $prof_40 (mut i64) (i64.const 0))
  (global $prof_41 (mut i64) (i64.const 0))
  (global $prof_42 (mut i64) (i64.const 0))
  (global $prof_43 (mut i64) (i64.const 0))
  (global $prof_44 (mut i64) (i64.const 0))
  (global $prof_45 (mut i64) (i64.const 0))
  (global $prof_46 (mut i64) (i64.const 0))
  (global $prof_47 (mut i64) (i64.const 0))
  (global $prof_48 (mut i64) (i64.const 0))
  (global $prof_49 (mut i64) (i64.const 0))
  (global $prof_50 (mut i64) (i64.const 0))
  (global $prof_51 (mut i64) (i64.const 0))
  (global $prof_52 (mut i64) (i64.const 0))
  (global $prof_53 (mut i64) (i64.const 0))
  (global $prof_54 (mut i64) (i64.const 0))
  (global $prof_55 (mut i64) (i64.const 0))
  (global $prof_56 (mut i64) (i64.const 0))
  (global $prof_57 (mut i64) (i64.const 0))
  (global $prof_58 (mut i64) (i64.const 0))
  (global $prof_59 (mut i64) (i64.const 0))
  (global $prof_60 (mut i64) (i64.const 0))
  (global $prof_61 (mut i64) (i64.const 0))
  (global $prof_62 (mut i64) (i64.const 0))
  (global $prof_63 (mut i64) (i64.const 0))
  (global $prof_64 (mut i64) (i64.const 0))
  (global $prof_65 (mut i64) (i64.const 0))
  (global $prof_66 (mut i64) (i64.const 0))
  (global $prof_67 (mut i64) (i64.const 0))
  (global $prof_68 (mut i64) (i64.const 0))
  (global $prof_69 (mut i64) (i64.const 0))
  (global $prof_70 (mut i64) (i64.const 0))
  (global $prof_71 (mut i64) (i64.const 0))
  (global $prof_72 (mut i64) (i64.const 0))
  (global $prof_73 (mut i64) (i64.const 0))
  (global $prof_74 (mut i64) (i64.const 0))
  (global $prof_75 (mut i64) (i64.const 0))
  (global $prof_76 (mut i64) (i64.const 0))
  (global $prof_77 (mut i64) (i64.const 0))
  (global $prof_78 (mut i64) (i64.const 0))
  (global $prof_79 (mut i64) (i64.const 0))
  (global $prof_80 (mut i64) (i64.const 0))
  (global $prof_81 (mut i64) (i64.const 0))
  (global $prof_82 (mut i64) (i64.const 0))
  (global $prof_83 (mut i64) (i64.const 0))
  (global $prof_84 (mut i64) (i64.const 0))
  (global $prof_85 (mut i64) (i64.const 0))
  (global $prof_86 (mut i64) (i64.const 0))
  (global $prof_87 (mut i64) (i64.const 0))
  (global $prof_88 (mut i64) (i64.const 0))
  (global $prof_89 (mut i64) (i64.const 0))
  (global $prof_90 (mut i64) (i64.const 0))
  (global $prof_91 (mut i64) (i64.const 0))
  (global $prof_92 (mut i64) (i64.const 0))
  (global $prof_93 (mut i64) (i64.const 0))
  (global $prof_94 (mut i64) (i64.const 0))
  (global $prof_95 (mut i64) (i64.const 0))
  (global $prof_96 (mut i64) (i64.const 0))
  (global $prof_97 (mut i64) (i64.const 0))
  (global $prof_98 (mut i64) (i64.const 0))
  (global $prof_99 (mut i64) (i64.const 0))
  (global $prof_100 (mut i64) (i64.const 0))
  (global $prof_101 (mut i64) (i64.const 0))
  (global $prof_102 (mut i64) (i64.const 0))
  (global $prof_103 (mut i64) (i64.const 0))
  (global $prof_104 (mut i64) (i64.const 0))
  (global $prof_105 (mut i64) (i64.const 0))
  (global $prof_106 (mut i64) (i64.const 0))
  (global $prof_107 (mut i64) (i64.const 0))
  (global $prof_108 (mut i64) (i64.const 0))
  (global $prof_109 (mut i64) (i64.const 0))
  (global $prof_110 (mut i64) (i64.const 0))
  (global $prof_111 (mut i64) (i64.const 0))
  (global $prof_112 (mut i64) (i64.const 0))
  (global $prof_113 (mut i64) (i64.const 0))
  (global $prof_114 (mut i64) (i64.const 0))
  (global $prof_115 (mut i64) (i64.const 0))
  (global $prof_116 (mut i64) (i64.const 0))
  (global $prof_117 (mut i64) (i64.const 0))
  (global $prof_118 (mut i64) (i64.const 0))
  (global $prof_119 (mut i64) (i64.const 0))
  (global $prof_120 (mut i64) (i64.const 0))
  (global $prof_121 (mut i64) (i64.const 0))
  (global $prof_122 (mut i64) (i64.const 0))
  (global $prof_123 (mut i64) (i64.const 0))
  (global $prof_124 (mut i64) (i64.const 0))
  (global $prof_125 (mut i64) (i64.const 0))
  (global $prof_126 (mut i64) (i64.const 0))
  (global $prof_127 (mut i64) (i64.const 0))
  (global $prof_128 (mut i64) (i64.const 0))
  (global $prof_129 (mut i64) (i64.const 0))
  (global $prof_130 (mut i64) (i64.const 0))
  (global $prof_131 (mut i64) (i64.const 0))
  (global $prof_132 (mut i64) (i64.const 0))
  (global $prof_133 (mut i64) (i64.const 0))
  (global $prof_134 (mut i64) (i64.const 0))
  (global $prof_135 (mut i64) (i64.const 0))
  (global $prof_136 (mut i64) (i64.const 0))
  (global $prof_137 (mut i64) (i64.const 0))
  (global $prof_138 (mut i64) (i64.const 0))
  (global $prof_139 (mut i64) (i64.const 0))
  (global $prof_140 (mut i64) (i64.const 0))
  (global $prof_141 (mut i64) (i64.const 0))
  (global $prof_142 (mut i64) (i64.const 0))
  (global $prof_143 (mut i64) (i64.const 0))
  (global $prof_144 (mut i64) (i64.const 0))
  (global $prof_145 (mut i64) (i64.const 0))
  (global $prof_146 (mut i64) (i64.const 0))
  (global $prof_147 (mut i64) (i64.const 0))
  (global $prof_148 (mut i64) (i64.const 0))
  (global $prof_149 (mut i64) (i64.const 0))
  (global $prof_150 (mut i64) (i64.const 0))
  (global $prof_151 (mut i64) (i64.const 0))
  (global $prof_152 (mut i64) (i64.const 0))
  (global $prof_153 (mut i64) (i64.const 0))
  (global $prof_154 (mut i64) (i64.const 0))
  (global $prof_155 (mut i64) (i64.const 0))
  (global $prof_156 (mut i64) (i64.const 0))
  (global $prof_157 (mut i64) (i64.const 0))
  (global $prof_158 (mut i64) (i64.const 0))
  (global $prof_159 (mut i64) (i64.const 0))
  (global $prof_160 (mut i64) (i64.const 0))
  (global $prof_161 (mut i64) (i64.const 0))
  (global $prof_162 (mut i64) (i64.const 0))
  (global $prof_163 (mut i64) (i64.const 0))
  (global $prof_164 (mut i64) (i64.const 0))
  (global $prof_165 (mut i64) (i64.const 0))
  (global $prof_166 (mut i64) (i64.const 0))
  (global $prof_167 (mut i64) (i64.const 0))
  (global $prof_168 (mut i64) (i64.const 0))
  (global $prof_169 (mut i64) (i64.const 0))
  (global $prof_170 (mut i64) (i64.const 0))
  (global $prof_171 (mut i64) (i64.const 0))
  (global $prof_172 (mut i64) (i64.const 0))
  (global $prof_173 (mut i64) (i64.const 0))
  (global $prof_174 (mut i64) (i64.const 0))
  (global $prof_175 (mut i64) (i64.const 0))
  (global $prof_176 (mut i64) (i64.const 0))
  (global $prof_177 (mut i64) (i64.const 0))
  (global $prof_178 (mut i64) (i64.const 0))
  (global $prof_179 (mut i64) (i64.const 0))
  (global $prof_180 (mut i64) (i64.const 0))
  (global $prof_181 (mut i64) (i64.const 0))
  (global $prof_182 (mut i64) (i64.const 0))
  (global $prof_183 (mut i64) (i64.const 0))
  (global $prof_184 (mut i64) (i64.const 0))
  (global $prof_185 (mut i64) (i64.const 0))
  (global $prof_186 (mut i64) (i64.const 0))
  (global $prof_187 (mut i64) (i64.const 0))
  (global $prof_188 (mut i64) (i64.const 0))
  (global $prof_189 (mut i64) (i64.const 0))
  (global $prof_190 (mut i64) (i64.const 0))
  (global $prof_191 (mut i64) (i64.const 0))
  (global $prof_192 (mut i64) (i64.const 0))
  (global $prof_193 (mut i64) (i64.const 0))
  (global $prof_194 (mut i64) (i64.const 0))
  (global $prof_195 (mut i64) (i64.const 0))
  (global $prof_196 (mut i64) (i64.const 0))
  (global $prof_197 (mut i64) (i64.const 0))
  (global $prof_198 (mut i64) (i64.const 0))
  (global $prof_199 (mut i64) (i64.const 0))
  (global $prof_200 (mut i64) (i64.const 0))
  (global $prof_201 (mut i64) (i64.const 0))
  (global $prof_202 (mut i64) (i64.const 0))
  (global $prof_203 (mut i64) (i64.const 0))
  (global $prof_204 (mut i64) (i64.const 0))
  (global $prof_205 (mut i64) (i64.const 0))
  (global $prof_206 (mut i64) (i64.const 0))
  (global $prof_207 (mut i64) (i64.const 0))
  (global $prof_208 (mut i64) (i64.const 0))
  (global $prof_209 (mut i64) (i64.const 0))
  (global $prof_210 (mut i64) (i64.const 0))
  (global $prof_211 (mut i64) (i64.const 0))
  (global $prof_212 (mut i64) (i64.const 0))
  (global $prof_213 (mut i64) (i64.const 0))
  (global $prof_214 (mut i64) (i64.const 0))
  (global $prof_215 (mut i64) (i64.const 0))
  (global $prof_216 (mut i64) (i64.const 0))
  (global $prof_217 (mut i64) (i64.const 0))
  (global $prof_218 (mut i64) (i64.const 0))
  (global $prof_219 (mut i64) (i64.const 0))
  (global $prof_220 (mut i64) (i64.const 0))
  (global $prof_221 (mut i64) (i64.const 0))
  (global $prof_222 (mut i64) (i64.const 0))
  (global $prof_223 (mut i64) (i64.const 0))
  (global $prof_224 (mut i64) (i64.const 0))
  (global $prof_225 (mut i64) (i64.const 0))
  (global $prof_226 (mut i64) (i64.const 0))
  (global $prof_227 (mut i64) (i64.const 0))
  (global $prof_228 (mut i64) (i64.const 0))
  (global $prof_229 (mut i64) (i64.const 0))
  (global $prof_230 (mut i64) (i64.const 0))
  (global $prof_231 (mut i64) (i64.const 0))
  (global $prof_232 (mut i64) (i64.const 0))
  (global $prof_233 (mut i64) (i64.const 0))
  (global $prof_234 (mut i64) (i64.const 0))
  (global $prof_235 (mut i64) (i64.const 0))
  (global $prof_236 (mut i64) (i64.const 0))
  (global $prof_237 (mut i64) (i64.const 0))
  (global $prof_238 (mut i64) (i64.const 0))
  (global $prof_239 (mut i64) (i64.const 0))
  (global $prof_240 (mut i64) (i64.const 0))
  (global $prof_241 (mut i64) (i64.const 0))
  (global $prof_242 (mut i64) (i64.const 0))
  (global $prof_243 (mut i64) (i64.const 0))
  (global $prof_244 (mut i64) (i64.const 0))
  (global $prof_245 (mut i64) (i64.const 0))
  (global $prof_246 (mut i64) (i64.const 0))
  (global $prof_247 (mut i64) (i64.const 0))
  (global $prof_248 (mut i64) (i64.const 0))
  (global $prof_249 (mut i64) (i64.const 0))
  (global $prof_250 (mut i64) (i64.const 0))
  (global $prof_251 (mut i64) (i64.const 0))
  (global $prof_252 (mut i64) (i64.const 0))
  (global $prof_253 (mut i64) (i64.const 0))
  (global $prof_254 (mut i64) (i64.const 0))
  (global $prof_255 (mut i64) (i64.const 0))
  (global $prof_256 (mut i64) (i64.const 0))
  (global $prof_257 (mut i64) (i64.const 0))
  (global $prof_258 (mut i64) (i64.const 0))
  (global $prof_259 (mut i64) (i64.const 0))
  (global $prof_260 (mut i64) (i64.const 0))
  (global $prof_261 (mut i64) (i64.const 0))
  (global $prof_262 (mut i64) (i64.const 0))
  (global $prof_263 (mut i64) (i64.const 0))
  (global $prof_264 (mut i64) (i64.const 0))
  (global $prof_265 (mut i64) (i64.const 0))
  (global $prof_266 (mut i64) (i64.const 0))
  (global $prof_267 (mut i64) (i64.const 0))
  (global $prof_268 (mut i64) (i64.const 0))
  (global $prof_269 (mut i64) (i64.const 0))
  (global $prof_270 (mut i64) (i64.const 0))
  (global $prof_271 (mut i64) (i64.const 0))
  (global $prof_272 (mut i64) (i64.const 0))
  (global $prof_273 (mut i64) (i64.const 0))
  (global $prof_274 (mut i64) (i64.const 0))
  (global $prof_275 (mut i64) (i64.const 0))
  (global $prof_276 (mut i64) (i64.const 0))
  (global $prof_277 (mut i64) (i64.const 0))
  (global $prof_278 (mut i64) (i64.const 0))
  (global $prof_279 (mut i64) (i64.const 0))
  (global $prof_280 (mut i64) (i64.const 0))
  (global $prof_281 (mut i64) (i64.const 0))
  (global $prof_282 (mut i64) (i64.const 0))
  (global $prof_283 (mut i64) (i64.const 0))
  (global $prof_284 (mut i64) (i64.const 0))
  (global $prof_285 (mut i64) (i64.const 0))
  (global $prof_286 (mut i64) (i64.const 0))
  (global $prof_287 (mut i64) (i64.const 0))
  (global $prof_288 (mut i64) (i64.const 0))
  (global $prof_289 (mut i64) (i64.const 0))
  (global $prof_290 (mut i64) (i64.const 0))
  (global $prof_291 (mut i64) (i64.const 0))
  (global $prof_292 (mut i64) (i64.const 0))
  (global $prof_293 (mut i64) (i64.const 0))
  (global $prof_294 (mut i64) (i64.const 0))
  (global $prof_295 (mut i64) (i64.const 0))
  (global $prof_296 (mut i64) (i64.const 0))
  (global $prof_297 (mut i64) (i64.const 0))
  (global $prof_298 (mut i64) (i64.const 0))
  (global $prof_299 (mut i64) (i64.const 0))
  (global $prof_300 (mut i64) (i64.const 0))
  (global $prof_301 (mut i64) (i64.const 0))
  (global $prof_302 (mut i64) (i64.const 0))
  (global $prof_303 (mut i64) (i64.const 0))
  (global $prof_304 (mut i64) (i64.const 0))
  (global $prof_305 (mut i64) (i64.const 0))
  (global $prof_306 (mut i64) (i64.const 0))
  (global $prof_307 (mut i64) (i64.const 0))
  (global $prof_308 (mut i64) (i64.const 0))
  (global $prof_309 (mut i64) (i64.const 0))
  (global $prof_310 (mut i64) (i64.const 0))
  (global $prof_311 (mut i64) (i64.const 0))
  (global $prof_312 (mut i64) (i64.const 0))
  (global $prof_313 (mut i64) (i64.const 0))
  (global $prof_314 (mut i64) (i64.const 0))
  (global $prof_315 (mut i64) (i64.const 0))
  (global $prof_316 (mut i64) (i64.const 0))
  (global $prof_317 (mut i64) (i64.const 0))
  (global $prof_318 (mut i64) (i64.const 0))
  (global $prof_319 (mut i64) (i64.const 0))
  (global $prof_320 (mut i64) (i64.const 0))
  (global $prof_321 (mut i64) (i64.const 0))
  (global $prof_322 (mut i64) (i64.const 0))
  (global $prof_323 (mut i64) (i64.const 0))
  (global $prof_324 (mut i64) (i64.const 0))
  (global $prof_325 (mut i64) (i64.const 0))
  (global $prof_326 (mut i64) (i64.const 0))
  (global $prof_327 (mut i64) (i64.const 0))
  (global $prof_328 (mut i64) (i64.const 0))
  (global $prof_329 (mut i64) (i64.const 0))
  (global $prof_330 (mut i64) (i64.const 0))
  (global $prof_331 (mut i64) (i64.const 0))
  (global $prof_332 (mut i64) (i64.const 0))
  (global $prof_333 (mut i64) (i64.const 0))
  (global $prof_334 (mut i64) (i64.const 0))
  (global $prof_335 (mut i64) (i64.const 0))
  (global $prof_336 (mut i64) (i64.const 0))
  (global $prof_337 (mut i64) (i64.const 0))
  (global $prof_338 (mut i64) (i64.const 0))
  (global $prof_339 (mut i64) (i64.const 0))
  (global $prof_340 (mut i64) (i64.const 0))
  (global $prof_341 (mut i64) (i64.const 0))
  (global $prof_342 (mut i64) (i64.const 0))
  (global $prof_343 (mut i64) (i64.const 0))
  (global $prof_344 (mut i64) (i64.const 0))
  (global $prof_345 (mut i64) (i64.const 0))
  (global $prof_346 (mut i64) (i64.const 0))
  (global $prof_347 (mut i64) (i64.const 0))
  (global $prof_348 (mut i64) (i64.const 0))
  (global $prof_349 (mut i64) (i64.const 0))
  (global $prof_350 (mut i64) (i64.const 0))
  (global $prof_351 (mut i64) (i64.const 0))
  (global $prof_352 (mut i64) (i64.const 0))
  (global $prof_353 (mut i64) (i64.const 0))
  (global $prof_354 (mut i64) (i64.const 0))
  (global $prof_355 (mut i64) (i64.const 0))
  (global $prof_356 (mut i64) (i64.const 0))
  (global $prof_357 (mut i64) (i64.const 0))
  (global $prof_358 (mut i64) (i64.const 0))
  (global $prof_359 (mut i64) (i64.const 0))
  (global $prof_360 (mut i64) (i64.const 0))
  (global $prof_361 (mut i64) (i64.const 0))
  (global $prof_362 (mut i64) (i64.const 0))
  (global $prof_363 (mut i64) (i64.const 0))
  (global $prof_364 (mut i64) (i64.const 0))
  (global $prof_365 (mut i64) (i64.const 0))
  (global $prof_366 (mut i64) (i64.const 0))
  (global $prof_367 (mut i64) (i64.const 0))
  (global $prof_368 (mut i64) (i64.const 0))
  (global $prof_369 (mut i64) (i64.const 0))
  (global $prof_370 (mut i64) (i64.const 0))
  (global $prof_371 (mut i64) (i64.const 0))
  (global $prof_372 (mut i64) (i64.const 0))
  (global $prof_373 (mut i64) (i64.const 0))
  (global $prof_374 (mut i64) (i64.const 0))
  (global $prof_375 (mut i64) (i64.const 0))
  (global $prof_376 (mut i64) (i64.const 0))
  (global $prof_377 (mut i64) (i64.const 0))
  (global $prof_378 (mut i64) (i64.const 0))
  (global $prof_379 (mut i64) (i64.const 0))
  (global $prof_380 (mut i64) (i64.const 0))
  (global $prof_381 (mut i64) (i64.const 0))
  (global $prof_382 (mut i64) (i64.const 0))
  (global $prof_383 (mut i64) (i64.const 0))
  (global $prof_384 (mut i64) (i64.const 0))
  (global $prof_385 (mut i64) (i64.const 0))
  (global $prof_386 (mut i64) (i64.const 0))
  (global $prof_387 (mut i64) (i64.const 0))
  (global $prof_388 (mut i64) (i64.const 0))
  (global $prof_389 (mut i64) (i64.const 0))
  (global $prof_390 (mut i64) (i64.const 0))
  (global $prof_391 (mut i64) (i64.const 0))
  (global $prof_392 (mut i64) (i64.const 0))
  (global $prof_393 (mut i64) (i64.const 0))
  (global $prof_394 (mut i64) (i64.const 0))
  (global $prof_395 (mut i64) (i64.const 0))
  (global $prof_396 (mut i64) (i64.const 0))
  (global $prof_397 (mut i64) (i64.const 0))
  (global $prof_398 (mut i64) (i64.const 0))
  (global $prof_399 (mut i64) (i64.const 0))
  (global $prof_400 (mut i64) (i64.const 0))
  (global $prof_401 (mut i64) (i64.const 0))
  (global $prof_402 (mut i64) (i64.const 0))
  (global $prof_403 (mut i64) (i64.const 0))
  (global $prof_404 (mut i64) (i64.const 0))
  (global $prof_405 (mut i64) (i64.const 0))
  (global $prof_406 (mut i64) (i64.const 0))
  (global $prof_407 (mut i64) (i64.const 0))
  (global $prof_408 (mut i64) (i64.const 0))
  (global $prof_409 (mut i64) (i64.const 0))
  (global $prof_410 (mut i64) (i64.const 0))
  (global $prof_411 (mut i64) (i64.const 0))
  (global $prof_412 (mut i64) (i64.const 0))
  (global $prof_413 (mut i64) (i64.const 0))
  (global $prof_414 (mut i64) (i64.const 0))
  (global $prof_415 (mut i64) (i64.const 0))
  (global $prof_416 (mut i64) (i64.const 0))
  (global $prof_417 (mut i64) (i64.const 0))
  (global $prof_418 (mut i64) (i64.const 0))
  (global $prof_419 (mut i64) (i64.const 0))
  (global $prof_420 (mut i64) (i64.const 0))
  (global $prof_421 (mut i64) (i64.const 0))
  (global $prof_422 (mut i64) (i64.const 0))
  (global $prof_423 (mut i64) (i64.const 0))
  (global $prof_424 (mut i64) (i64.const 0))
  (global $prof_425 (mut i64) (i64.const 0))
  (global $prof_426 (mut i64) (i64.const 0))
  (global $prof_427 (mut i64) (i64.const 0))
  (global $prof_428 (mut i64) (i64.const 0))
  (global $prof_429 (mut i64) (i64.const 0))
  (global $prof_430 (mut i64) (i64.const 0))
  (global $prof_431 (mut i64) (i64.const 0))
  (global $prof_432 (mut i64) (i64.const 0))
  (global $prof_433 (mut i64) (i64.const 0))
  (global $prof_434 (mut i64) (i64.const 0))
  (global $prof_435 (mut i64) (i64.const 0))
  (global $prof_436 (mut i64) (i64.const 0))
  (global $prof_437 (mut i64) (i64.const 0))
  (global $prof_438 (mut i64) (i64.const 0))
  (global $prof_439 (mut i64) (i64.const 0))
  (global $prof_440 (mut i64) (i64.const 0))
  (global $prof_441 (mut i64) (i64.const 0))
  (global $prof_442 (mut i64) (i64.const 0))
  (global $prof_443 (mut i64) (i64.const 0))
  (global $prof_444 (mut i64) (i64.const 0))
  (global $prof_445 (mut i64) (i64.const 0))
  (global $prof_446 (mut i64) (i64.const 0))
  (global $prof_447 (mut i64) (i64.const 0))
  (global $prof_448 (mut i64) (i64.const 0))
  (global $prof_449 (mut i64) (i64.const 0))
  (global $prof_450 (mut i64) (i64.const 0))
  (global $prof_451 (mut i64) (i64.const 0))
  (global $prof_452 (mut i64) (i64.const 0))
  (global $prof_453 (mut i64) (i64.const 0))
  (global $prof_454 (mut i64) (i64.const 0))
  (global $prof_455 (mut i64) (i64.const 0))
  (global $prof_456 (mut i64) (i64.const 0))
  (global $prof_457 (mut i64) (i64.const 0))
  (global $prof_458 (mut i64) (i64.const 0))
  (global $prof_459 (mut i64) (i64.const 0))
  (global $prof_460 (mut i64) (i64.const 0))
  (global $prof_461 (mut i64) (i64.const 0))
  (global $prof_462 (mut i64) (i64.const 0))
  (global $prof_463 (mut i64) (i64.const 0))
  (global $prof_464 (mut i64) (i64.const 0))
  (global $prof_465 (mut i64) (i64.const 0))
  (global $prof_466 (mut i64) (i64.const 0))
  (global $prof_467 (mut i64) (i64.const 0))
  (global $prof_468 (mut i64) (i64.const 0))
  (global $prof_469 (mut i64) (i64.const 0))
  (global $prof_470 (mut i64) (i64.const 0))
  (global $prof_471 (mut i64) (i64.const 0))
  (global $prof_472 (mut i64) (i64.const 0))
  (global $prof_473 (mut i64) (i64.const 0))
  (global $prof_474 (mut i64) (i64.const 0))
  (global $prof_475 (mut i64) (i64.const 0))
  (global $prof_476 (mut i64) (i64.const 0))
  (global $prof_477 (mut i64) (i64.const 0))
  (global $prof_478 (mut i64) (i64.const 0))
  (global $prof_479 (mut i64) (i64.const 0))
  (global $prof_480 (mut i64) (i64.const 0))
  (global $prof_481 (mut i64) (i64.const 0))
  (global $prof_482 (mut i64) (i64.const 0))
  (global $prof_483 (mut i64) (i64.const 0))
  (global $prof_484 (mut i64) (i64.const 0))
  (global $prof_485 (mut i64) (i64.const 0))
  (global $prof_486 (mut i64) (i64.const 0))
  (global $prof_487 (mut i64) (i64.const 0))
  (global $prof_488 (mut i64) (i64.const 0))
  (global $prof_489 (mut i64) (i64.const 0))
  (global $prof_490 (mut i64) (i64.const 0))
  (global $prof_491 (mut i64) (i64.const 0))
  (global $prof_492 (mut i64) (i64.const 0))
  (global $prof_493 (mut i64) (i64.const 0))
  (global $prof_494 (mut i64) (i64.const 0))
  (global $prof_495 (mut i64) (i64.const 0))
  (global $prof_496 (mut i64) (i64.const 0))
  (global $prof_497 (mut i64) (i64.const 0))
  (global $prof_498 (mut i64) (i64.const 0))
  (global $prof_499 (mut i64) (i64.const 0))
  (global $prof_500 (mut i64) (i64.const 0))
  (global $prof_501 (mut i64) (i64.const 0))
  (global $prof_502 (mut i64) (i64.const 0))
  (global $prof_503 (mut i64) (i64.const 0))
  (global $prof_504 (mut i64) (i64.const 0))
  (global $prof_505 (mut i64) (i64.const 0))
  (global $prof_506 (mut i64) (i64.const 0))
  (global $prof_507 (mut i64) (i64.const 0))
  (global $prof_508 (mut i64) (i64.const 0))
  (global $prof_509 (mut i64) (i64.const 0))
  (global $prof_510 (mut i64) (i64.const 0))
  (global $prof_511 (mut i64) (i64.const 0))
  (global $prof_512 (mut i64) (i64.const 0))
  (global $prof_513 (mut i64) (i64.const 0))
  (global $prof_514 (mut i64) (i64.const 0))
  (global $prof_515 (mut i64) (i64.const 0))
  (global $prof_516 (mut i64) (i64.const 0))
  (global $prof_517 (mut i64) (i64.const 0))
  (global $prof_518 (mut i64) (i64.const 0))
  (global $prof_519 (mut i64) (i64.const 0))
  (global $prof_520 (mut i64) (i64.const 0))
  (global $prof_521 (mut i64) (i64.const 0))
  (global $prof_522 (mut i64) (i64.const 0))
  (global $prof_523 (mut i64) (i64.const 0))
  (global $prof_524 (mut i64) (i64.const 0))
  (global $prof_525 (mut i64) (i64.const 0))
  (global $prof_526 (mut i64) (i64.const 0))
  (global $prof_527 (mut i64) (i64.const 0))
  (global $prof_528 (mut i64) (i64.const 0))
  (global $prof_529 (mut i64) (i64.const 0))
  (global $prof_530 (mut i64) (i64.const 0))
  (global $prof_531 (mut i64) (i64.const 0))
  (global $prof_532 (mut i64) (i64.const 0))
  (global $prof_533 (mut i64) (i64.const 0))
  (global $prof_534 (mut i64) (i64.const 0))
  (global $prof_535 (mut i64) (i64.const 0))
  (global $prof_536 (mut i64) (i64.const 0))
  (global $prof_537 (mut i64) (i64.const 0))
  (global $prof_538 (mut i64) (i64.const 0))
  (global $prof_539 (mut i64) (i64.const 0))
  (global $prof_540 (mut i64) (i64.const 0))
  (global $prof_541 (mut i64) (i64.const 0))
  (global $prof_542 (mut i64) (i64.const 0))
  (global $prof_543 (mut i64) (i64.const 0))
  (global $prof_544 (mut i64) (i64.const 0))
  (global $prof_545 (mut i64) (i64.const 0))
  (global $prof_546 (mut i64) (i64.const 0))
  (global $prof_547 (mut i64) (i64.const 0))
  (global $prof_548 (mut i64) (i64.const 0))
  (global $prof_549 (mut i64) (i64.const 0))
  (global $prof_550 (mut i64) (i64.const 0))
  (export "memory" (memory 0))
  (export "ilang_alloc" (func $alloc))
  (export "ilang_free" (func $free))
  (export "ilang_start" (func $ilang_start))
  (export "num/kernels.identity" (func 346))
  (export "num/kernels.fast" (func 347))
  (export "num/kernels.horner" (func 348))
  (export "num/kernels.dot" (func 349))
  (export "num/kernels.integer" (func 350))
  (export "num/kernels.tuples" (func 351))
  (export "num/kernels.closures" (func 352))
  (elem (;0;) declare func $f0 $f1 $f2 $f3 $f4 $f5 $f6 $f7 $f8 $f9 $f10 $f11 $f12 $f13 $f14 $f15 $f16 $f17 $f18 $f19 $f20 $f21 $f22 $f23 $f24 $r0 $entry_assert_fail $catch_reenter $ensure_reenter $final_reenter $boundary_reenter $cancel_reenter $control_unpend)
  (func $alloc (;2;) (type 35) (param $len i32) (result i32)
    (local $size i32) (local $p i32) (local $prev i32) (local $end i64) (local $available i64)
    global.get $prof_0
    i64.const 1
    i64.add
    global.set $prof_0
    local.get $len
    i32.const 2147483632
    i32.gt_u
    if ;; label = @1
      unreachable
    end
    local.get $len
    i32.const 7
    i32.add
    i32.const -8
    i32.and
    local.set $size
    local.get $size
    i32.eqz
    if ;; label = @1
      i32.const 8
      local.set $size
    end
    global.get $free_head
    local.set $p
    block $not_found
      loop $search
        local.get $p
        i32.eqz
        br_if $not_found
        local.get $p
        i32.load
        local.get $size
        i32.ge_u
        if ;; label = @3
          local.get $prev
          if ;; label = @4
            local.get $prev
            local.get $p
            i32.load offset=4
            i32.store offset=4
          else
            local.get $p
            i32.load offset=4
            global.set $free_head
          end
          local.get $p
          i32.const 8
          i32.add
          return
        end
        local.get $p
        local.set $prev
        local.get $p
        i32.load offset=4
        local.set $p
        br $search
      end
    end
    global.get $heap
    local.set $p
    local.get $p
    i64.extend_i32_u
    local.get $size
    i64.extend_i32_u
    i64.const 8
    i64.add
    i64.add
    local.set $end
    local.get $end
    i64.const 4294967295
    i64.gt_u
    if ;; label = @1
      unreachable
    end
    memory.size
    i64.extend_i32_u
    i64.const 16
    i64.shl
    local.set $available
    local.get $end
    local.get $available
    i64.gt_u
    if ;; label = @1
      local.get $end
      local.get $available
      i64.sub
      i64.const 65535
      i64.add
      i64.const 16
      i64.shr_u
      i32.wrap_i64
      memory.grow
      i32.const -1
      i32.eq
      if ;; label = @2
        unreachable
      end
    end
    local.get $p
    local.get $size
    i32.store
    local.get $p
    i32.const 0
    i32.store offset=4
    local.get $end
    i32.wrap_i64
    global.set $heap
    local.get $p
    i32.const 8
    i32.add
  )
  (func $free (;3;) (type 33) (param $ptr i32) (param $len i32)
    (local $p i32)
    global.get $prof_1
    i64.const 1
    i64.add
    global.set $prof_1
    local.get $ptr
    i32.eqz
    if ;; label = @1
      return
    end
    local.get $ptr
    i32.const 8
    i32.sub
    local.set $p
    local.get $p
    global.get $free_head
    i32.store offset=4
    local.get $p
    global.set $free_head
  )
  (func $int_sign (;4;) (type 36) (param $v eqref) (result i32)
    (local $x i32)
    global.get $prof_2
    i64.const 1
    i64.add
    global.set $prof_2
    local.get $v
    global.get $prof_3
    i64.const 1
    i64.add
    global.set $prof_3
    ref.test (ref i31)
    if (result i32) ;; label = @1
      local.get $v
    global.get $prof_4
    i64.const 1
    i64.add
    global.set $prof_4
      ref.cast (ref i31)
    global.get $prof_5
    i64.const 1
    i64.add
    global.set $prof_5
      i31.get_s
      local.set $x
      local.get $x
      i32.const 0
      i32.gt_s
      local.get $x
      i32.const 0
      i32.lt_s
      i32.sub
    else
      local.get $v
    global.get $prof_4
    i64.const 1
    i64.add
    global.set $prof_4
      ref.cast (ref $obj)
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
      struct.get $obj 1
    end
  )
  (func $int_limbs (;5;) (type 37) (param $v eqref) (result (ref $limbs))
    (local $x i32)
    global.get $prof_7
    i64.const 1
    i64.add
    global.set $prof_7
    local.get $v
    global.get $prof_3
    i64.const 1
    i64.add
    global.set $prof_3
    ref.test (ref i31)
    if (result (ref $limbs)) ;; label = @1
      local.get $v
    global.get $prof_4
    i64.const 1
    i64.add
    global.set $prof_4
      ref.cast (ref i31)
    global.get $prof_5
    i64.const 1
    i64.add
    global.set $prof_5
      i31.get_s
      local.set $x
      local.get $x
      i32.eqz
      if (result (ref $limbs)) ;; label = @2
        i32.const 0
    global.get $prof_8
    i64.const 1
    i64.add
    global.set $prof_8
        array.new_default $limbs
      else
        local.get $x
        i32.const 0
        i32.lt_s
        if (result i32) ;; label = @3
          i32.const 0
          local.get $x
          i32.sub
        else
          local.get $x
        end
    global.get $prof_9
    i64.const 1
    i64.add
    global.set $prof_9
        array.new_fixed $limbs 1
      end
    else
      local.get $v
    global.get $prof_4
    i64.const 1
    i64.add
    global.set $prof_4
      ref.cast (ref $obj)
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
      struct.get $obj 6
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
      ref.as_non_null
    end
  )
  (func $mag_len (;6;) (type 38) (param $a (ref $limbs)) (result i32)
    (local $n i32)
    global.get $prof_11
    i64.const 1
    i64.add
    global.set $prof_11
    local.get $a
    array.len
    local.set $n
    block $done
      loop $trim
        local.get $n
        i32.eqz
        br_if $done
        local.get $a
        local.get $n
        i32.const 1
        i32.sub
    global.get $prof_12
    i64.const 1
    i64.add
    global.set $prof_12
        array.get $limbs
        br_if $done
        local.get $n
        i32.const 1
        i32.sub
        local.set $n
        br $trim
      end
    end
    local.get $n
  )
  (func $int_pack (;7;) (type 39) (param $sign i32) (param $a (ref $limbs)) (result eqref)
    (local $n i32) (local $x i32) (local $trimmed (ref null $limbs))
    global.get $prof_13
    i64.const 1
    i64.add
    global.set $prof_13
    local.get $a
    call $mag_len
    local.set $n
    local.get $n
    i32.eqz
    if ;; label = @1
      i32.const 0
    global.get $prof_14
    i64.const 1
    i64.add
    global.set $prof_14
      ref.i31
      return
    end
    local.get $sign
    i32.const 0
    i32.lt_s
    if (result i32) ;; label = @1
      i32.const -1
    else
      i32.const 1
    end
    local.set $sign
    local.get $n
    i32.const 1
    i32.eq
    if ;; label = @1
      local.get $a
      i32.const 0
    global.get $prof_12
    i64.const 1
    i64.add
    global.set $prof_12
      array.get $limbs
      local.set $x
      local.get $sign
      i32.const 0
      i32.gt_s
      local.get $x
      i32.const 1073741823
      i32.le_u
      i32.and
      local.get $sign
      i32.const 0
      i32.lt_s
      local.get $x
      i32.const 1073741824
      i32.le_u
      i32.and
      i32.or
      if ;; label = @2
        local.get $sign
        local.get $x
        i32.mul
    global.get $prof_14
    i64.const 1
    i64.add
    global.set $prof_14
        ref.i31
        return
      end
    end
    local.get $a
    local.set $trimmed
    local.get $n
    local.get $a
    array.len
    i32.ne
    if ;; label = @1
      local.get $n
    global.get $prof_8
    i64.const 1
    i64.add
    global.set $prof_8
      array.new_default $limbs
      local.set $trimmed
      local.get $trimmed
      i32.const 0
      local.get $a
      i32.const 0
      local.get $n
      array.copy $limbs $limbs
    end
    i32.const 8
    local.get $sign
    i64.const 0
    f64.const 0x0p+0 (;=0;)
    ref.null $bytes
    ref.null $values
    local.get $trimmed
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
  )
  (func $int_from_i64 (;8;) (type 40) (param $v i64) (result eqref)
    (local $sign i32) (local $mag i64)
    global.get $prof_16
    i64.const 1
    i64.add
    global.set $prof_16
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
    global.get $prof_14
    i64.const 1
    i64.add
    global.set $prof_14
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
    global.get $prof_9
    i64.const 1
    i64.add
    global.set $prof_9
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
    global.get $prof_17
    i64.const 1
    i64.add
    global.set $prof_17
    array.new_fixed $limbs 2
    call $int_pack
  )
  (func $int_to_i64 (;9;) (type 41) (param $v eqref) (result i64)
    (local $a (ref null $limbs)) (local $n i32) (local $mag i64)
    global.get $prof_18
    i64.const 1
    i64.add
    global.set $prof_18
    local.get $v
    global.get $prof_3
    i64.const 1
    i64.add
    global.set $prof_3
    ref.test (ref i31)
    if ;; label = @1
      local.get $v
    global.get $prof_4
    i64.const 1
    i64.add
    global.set $prof_4
      ref.cast (ref i31)
    global.get $prof_5
    i64.const 1
    i64.add
    global.set $prof_5
      i31.get_s
      i64.extend_i32_s
      return
    end
    local.get $v
    call $int_limbs
    local.set $a
    local.get $a
    array.len
    local.set $n
    local.get $n
    i32.const 2
    i32.gt_u
    if ;; label = @1
      unreachable
    end
    local.get $a
    i32.const 0
    global.get $prof_12
    i64.const 1
    i64.add
    global.set $prof_12
    array.get $limbs
    i64.extend_i32_u
    local.set $mag
    local.get $n
    i32.const 2
    i32.eq
    if ;; label = @1
      local.get $mag
      local.get $a
      i32.const 1
    global.get $prof_12
    i64.const 1
    i64.add
    global.set $prof_12
      array.get $limbs
      i64.extend_i32_u
      i64.const 32
      i64.shl
      i64.or
      local.set $mag
    end
    local.get $v
    call $int_sign
    i32.const 0
    i32.lt_s
    if (result i64) ;; label = @1
      local.get $mag
      i64.const -9223372036854775808
      i64.gt_u
      if ;; label = @2
        unreachable
      end
      i64.const 0
      local.get $mag
      i64.sub
    else
      local.get $mag
      i64.const 9223372036854775807
      i64.gt_u
      if ;; label = @2
        unreachable
      end
      local.get $mag
    end
  )
  (func $mag_compare (;10;) (type 42) (param $a (ref $limbs)) (param $b (ref $limbs)) (result i32)
    (local $an i32) (local $bn i32) (local $x i32) (local $y i32)
    global.get $prof_19
    i64.const 1
    i64.add
    global.set $prof_19
    local.get $a
    call $mag_len
    local.set $an
    local.get $b
    call $mag_len
    local.set $bn
    local.get $an
    local.get $bn
    i32.lt_u
    if ;; label = @1
      i32.const -1
      return
    end
    local.get $an
    local.get $bn
    i32.gt_u
    if ;; label = @1
      i32.const 1
      return
    end
    block $done
      loop $scan
        local.get $an
        i32.eqz
        br_if $done
        local.get $an
        i32.const 1
        i32.sub
        local.set $an
        local.get $a
        local.get $an
    global.get $prof_12
    i64.const 1
    i64.add
    global.set $prof_12
        array.get $limbs
        local.set $x
        local.get $b
        local.get $an
    global.get $prof_12
    i64.const 1
    i64.add
    global.set $prof_12
        array.get $limbs
        local.set $y
        local.get $x
        local.get $y
        i32.lt_u
        if ;; label = @3
          i32.const -1
          return
        end
        local.get $x
        local.get $y
        i32.gt_u
        if ;; label = @3
          i32.const 1
          return
        end
        br $scan
      end
    end
    i32.const 0
  )
  (func $int_compare (;11;) (type 43) (param $a eqref) (param $b eqref) (result i32)
    (local $as i32) (local $bs i32)
    global.get $prof_20
    i64.const 1
    i64.add
    global.set $prof_20
    local.get $a
    global.get $prof_3
    i64.const 1
    i64.add
    global.set $prof_3
    ref.test (ref i31)
    local.get $b
    global.get $prof_3
    i64.const 1
    i64.add
    global.set $prof_3
    ref.test (ref i31)
    i32.and
    if ;; label = @1
      local.get $a
    global.get $prof_4
    i64.const 1
    i64.add
    global.set $prof_4
      ref.cast (ref i31)
    global.get $prof_5
    i64.const 1
    i64.add
    global.set $prof_5
      i31.get_s
      local.set $as
      local.get $b
    global.get $prof_4
    i64.const 1
    i64.add
    global.set $prof_4
      ref.cast (ref i31)
    global.get $prof_5
    i64.const 1
    i64.add
    global.set $prof_5
      i31.get_s
      local.set $bs
      local.get $as
      local.get $bs
      i32.gt_s
      local.get $as
      local.get $bs
      i32.lt_s
      i32.sub
      return
    end
    local.get $a
    call $int_sign
    local.set $as
    local.get $b
    call $int_sign
    local.set $bs
    local.get $as
    local.get $bs
    i32.lt_s
    if ;; label = @1
      i32.const -1
      return
    end
    local.get $as
    local.get $bs
    i32.gt_s
    if ;; label = @1
      i32.const 1
      return
    end
    local.get $as
    i32.eqz
    if ;; label = @1
      i32.const 0
      return
    end
    local.get $as
    local.get $a
    call $int_limbs
    local.get $b
    call $int_limbs
    call $mag_compare
    i32.mul
  )
  (func $mag_add (;12;) (type 44) (param $a (ref $limbs)) (param $b (ref $limbs)) (result (ref $limbs))
    (local $out (ref null $limbs)) (local $an i32) (local $bn i32) (local $n i32) (local $i i32) (local $carry i64) (local $sum i64)
    global.get $prof_21
    i64.const 1
    i64.add
    global.set $prof_21
    local.get $a
    array.len
    local.set $an
    local.get $b
    array.len
    local.set $bn
    local.get $an
    local.get $bn
    i32.gt_u
    if (result i32) ;; label = @1
      local.get $an
    else
      local.get $bn
    end
    local.set $n
    local.get $n
    i32.const 1
    i32.add
    global.get $prof_8
    i64.const 1
    i64.add
    global.set $prof_8
    array.new_default $limbs
    local.set $out
    block $done
      loop $add
        local.get $i
        local.get $n
        i32.ge_u
        br_if $done
        local.get $carry
        local.set $sum
        local.get $i
        local.get $an
        i32.lt_u
        if ;; label = @3
          local.get $sum
          local.get $a
          local.get $i
    global.get $prof_12
    i64.const 1
    i64.add
    global.set $prof_12
          array.get $limbs
          i64.extend_i32_u
          i64.add
          local.set $sum
        end
        local.get $i
        local.get $bn
        i32.lt_u
        if ;; label = @3
          local.get $sum
          local.get $b
          local.get $i
    global.get $prof_12
    i64.const 1
    i64.add
    global.set $prof_12
          array.get $limbs
          i64.extend_i32_u
          i64.add
          local.set $sum
        end
        local.get $out
        local.get $i
        local.get $sum
        i32.wrap_i64
        array.set $limbs
        local.get $sum
        i64.const 32
        i64.shr_u
        local.set $carry
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $add
      end
    end
    local.get $out
    local.get $n
    local.get $carry
    i32.wrap_i64
    array.set $limbs
    local.get $out
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
    ref.as_non_null
  )
  (func $mag_sub (;13;) (type 44) (param $a (ref $limbs)) (param $b (ref $limbs)) (result (ref $limbs))
    (local $out (ref null $limbs)) (local $an i32) (local $bn i32) (local $i i32) (local $borrow i64) (local $x i64) (local $y i64)
    global.get $prof_22
    i64.const 1
    i64.add
    global.set $prof_22
    local.get $a
    array.len
    local.set $an
    local.get $b
    array.len
    local.set $bn
    local.get $an
    global.get $prof_8
    i64.const 1
    i64.add
    global.set $prof_8
    array.new_default $limbs
    local.set $out
    block $done
      loop $sub
        local.get $i
        local.get $an
        i32.ge_u
        br_if $done
        local.get $a
        local.get $i
    global.get $prof_12
    i64.const 1
    i64.add
    global.set $prof_12
        array.get $limbs
        i64.extend_i32_u
        local.set $x
        local.get $borrow
        local.set $y
        local.get $i
        local.get $bn
        i32.lt_u
        if ;; label = @3
          local.get $y
          local.get $b
          local.get $i
    global.get $prof_12
    i64.const 1
    i64.add
    global.set $prof_12
          array.get $limbs
          i64.extend_i32_u
          i64.add
          local.set $y
        end
        local.get $out
        local.get $i
        local.get $x
        local.get $y
        i64.sub
        i32.wrap_i64
        array.set $limbs
        local.get $x
        local.get $y
        i64.lt_u
        i64.extend_i32_u
        local.set $borrow
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $sub
      end
    end
    local.get $out
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
    ref.as_non_null
  )
  (func $int_add (;14;) (type $call) (param $a eqref) (param $b eqref) (result eqref)
    (local $as i32) (local $bs i32) (local $cmp i32) (local $al (ref null $limbs)) (local $bl (ref null $limbs))
    global.get $prof_23
    i64.const 1
    i64.add
    global.set $prof_23
    local.get $a
    global.get $prof_3
    i64.const 1
    i64.add
    global.set $prof_3
    ref.test (ref i31)
    local.get $b
    global.get $prof_3
    i64.const 1
    i64.add
    global.set $prof_3
    ref.test (ref i31)
    i32.and
    if ;; label = @1
      local.get $a
    global.get $prof_4
    i64.const 1
    i64.add
    global.set $prof_4
      ref.cast (ref i31)
    global.get $prof_5
    i64.const 1
    i64.add
    global.set $prof_5
      i31.get_s
      i64.extend_i32_s
      local.get $b
    global.get $prof_4
    i64.const 1
    i64.add
    global.set $prof_4
      ref.cast (ref i31)
    global.get $prof_5
    i64.const 1
    i64.add
    global.set $prof_5
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
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
      ref.as_non_null
      local.get $bl
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
      ref.as_non_null
      call $mag_add
      call $int_pack
      return
    end
    local.get $al
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
    ref.as_non_null
    local.get $bl
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
    ref.as_non_null
    call $mag_compare
    local.set $cmp
    local.get $cmp
    i32.eqz
    if ;; label = @1
      i32.const 0
    global.get $prof_14
    i64.const 1
    i64.add
    global.set $prof_14
      ref.i31
      return
    end
    local.get $cmp
    i32.const 0
    i32.gt_s
    if (result eqref) ;; label = @1
      local.get $as
      local.get $al
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
      ref.as_non_null
      local.get $bl
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
      ref.as_non_null
      call $mag_sub
      call $int_pack
    else
      local.get $bs
      local.get $bl
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
      ref.as_non_null
      local.get $al
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
      ref.as_non_null
      call $mag_sub
      call $int_pack
    end
  )
  (func $int_neg (;15;) (type 45) (param $v eqref) (result eqref)
    global.get $prof_24
    i64.const 1
    i64.add
    global.set $prof_24
    local.get $v
    global.get $prof_3
    i64.const 1
    i64.add
    global.set $prof_3
    ref.test (ref i31)
    if (result eqref) ;; label = @1
      i64.const 0
      local.get $v
    global.get $prof_4
    i64.const 1
    i64.add
    global.set $prof_4
      ref.cast (ref i31)
    global.get $prof_5
    i64.const 1
    i64.add
    global.set $prof_5
      i31.get_s
      i64.extend_i32_s
      i64.sub
      call $int_from_i64
    else
      i32.const 0
      local.get $v
      call $int_sign
      i32.sub
      local.get $v
      call $int_limbs
      call $int_pack
    end
  )
  (func $int_sub (;16;) (type $call) (param $a eqref) (param $b eqref) (result eqref)
    global.get $prof_25
    i64.const 1
    i64.add
    global.set $prof_25
    local.get $a
    global.get $prof_3
    i64.const 1
    i64.add
    global.set $prof_3
    ref.test (ref i31)
    local.get $b
    global.get $prof_3
    i64.const 1
    i64.add
    global.set $prof_3
    ref.test (ref i31)
    i32.and
    if (result eqref) ;; label = @1
      local.get $a
    global.get $prof_4
    i64.const 1
    i64.add
    global.set $prof_4
      ref.cast (ref i31)
    global.get $prof_5
    i64.const 1
    i64.add
    global.set $prof_5
      i31.get_s
      i64.extend_i32_s
      local.get $b
    global.get $prof_4
    i64.const 1
    i64.add
    global.set $prof_4
      ref.cast (ref i31)
    global.get $prof_5
    i64.const 1
    i64.add
    global.set $prof_5
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
  (func $mag_mul (;17;) (type 44) (param $a (ref $limbs)) (param $b (ref $limbs)) (result (ref $limbs))
    (local $out (ref null $limbs)) (local $an i32) (local $bn i32) (local $i i32) (local $j i32) (local $k i32) (local $carry i64) (local $product i64) (local $x i64)
    global.get $prof_26
    i64.const 1
    i64.add
    global.set $prof_26
    local.get $a
    array.len
    local.set $an
    local.get $b
    array.len
    local.set $bn
    local.get $an
    local.get $bn
    i32.add
    global.get $prof_8
    i64.const 1
    i64.add
    global.set $prof_8
    array.new_default $limbs
    local.set $out
    block $done
      loop $rows
        local.get $i
        local.get $an
        i32.ge_u
        br_if $done
        local.get $a
        local.get $i
    global.get $prof_12
    i64.const 1
    i64.add
    global.set $prof_12
        array.get $limbs
        i64.extend_i32_u
        local.set $x
        i64.const 0
        local.set $carry
        i32.const 0
        local.set $j
        block $row_done
          loop $columns
            local.get $j
            local.get $bn
            i32.ge_u
            br_if $row_done
            local.get $i
            local.get $j
            i32.add
            local.set $k
            local.get $x
            local.get $b
            local.get $j
    global.get $prof_12
    i64.const 1
    i64.add
    global.set $prof_12
            array.get $limbs
            i64.extend_i32_u
            i64.mul
            local.get $out
            local.get $k
    global.get $prof_12
    i64.const 1
    i64.add
    global.set $prof_12
            array.get $limbs
            i64.extend_i32_u
            i64.add
            local.get $carry
            i64.add
            local.set $product
            local.get $out
            local.get $k
            local.get $product
            i32.wrap_i64
            array.set $limbs
            local.get $product
            i64.const 32
            i64.shr_u
            local.set $carry
            local.get $j
            i32.const 1
            i32.add
            local.set $j
            br $columns
          end
        end
        local.get $out
        local.get $i
        local.get $bn
        i32.add
        local.get $carry
        i32.wrap_i64
        array.set $limbs
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $rows
      end
    end
    local.get $out
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
    ref.as_non_null
  )
  (func $int_mul (;18;) (type $call) (param $a eqref) (param $b eqref) (result eqref)
    (local $sign i32)
    global.get $prof_27
    i64.const 1
    i64.add
    global.set $prof_27
    local.get $a
    global.get $prof_3
    i64.const 1
    i64.add
    global.set $prof_3
    ref.test (ref i31)
    local.get $b
    global.get $prof_3
    i64.const 1
    i64.add
    global.set $prof_3
    ref.test (ref i31)
    i32.and
    if ;; label = @1
      local.get $a
    global.get $prof_4
    i64.const 1
    i64.add
    global.set $prof_4
      ref.cast (ref i31)
    global.get $prof_5
    i64.const 1
    i64.add
    global.set $prof_5
      i31.get_s
      i64.extend_i32_s
      local.get $b
    global.get $prof_4
    i64.const 1
    i64.add
    global.set $prof_4
      ref.cast (ref i31)
    global.get $prof_5
    i64.const 1
    i64.add
    global.set $prof_5
      i31.get_s
      i64.extend_i32_s
      i64.mul
      call $int_from_i64
      return
    end
    local.get $a
    call $int_sign
    local.get $b
    call $int_sign
    i32.mul
    local.set $sign
    local.get $sign
    i32.eqz
    if ;; label = @1
      i32.const 0
    global.get $prof_14
    i64.const 1
    i64.add
    global.set $prof_14
      ref.i31
      return
    end
    local.get $sign
    local.get $a
    call $int_limbs
    local.get $b
    call $int_limbs
    call $mag_mul
    call $int_pack
  )
  (func $mag_divrem (;19;) (type 46) (param $a (ref $limbs)) (param $b (ref $limbs)) (result (ref $limbs) (ref $limbs))
    (local $q (ref null $limbs)) (local $r (ref null $limbs)) (local $an i32) (local $bn i32) (local $rn i32) (local $bit i64) (local $i i32) (local $word i32) (local $shift i32) (local $carry i64) (local $sum i64) (local $x i64) (local $y i64) (local $borrow i64)
    global.get $prof_28
    i64.const 1
    i64.add
    global.set $prof_28
    local.get $a
    call $mag_len
    local.set $an
    local.get $b
    call $mag_len
    local.set $bn
    local.get $bn
    i32.eqz
    if ;; label = @1
      unreachable
    end
    local.get $a
    local.get $b
    call $mag_compare
    i32.const 0
    i32.lt_s
    if ;; label = @1
      i32.const 0
    global.get $prof_8
    i64.const 1
    i64.add
    global.set $prof_8
      array.new_default $limbs
      local.get $a
      return
    end
    local.get $bn
    i32.const 1
    i32.eq
    if ;; label = @1
      local.get $an
    global.get $prof_8
    i64.const 1
    i64.add
    global.set $prof_8
      array.new_default $limbs
      local.set $q
      local.get $b
      i32.const 0
    global.get $prof_12
    i64.const 1
    i64.add
    global.set $prof_12
      array.get $limbs
      i64.extend_i32_u
      local.set $x
      local.get $an
      local.set $i
      i64.const 0
      local.set $carry
      block $small_done
        loop $small_divide
          local.get $i
          i32.eqz
          br_if $small_done
          local.get $i
          i32.const 1
          i32.sub
          local.set $i
          local.get $carry
          i64.const 32
          i64.shl
          local.get $a
          local.get $i
    global.get $prof_12
    i64.const 1
    i64.add
    global.set $prof_12
          array.get $limbs
          i64.extend_i32_u
          i64.or
          local.set $sum
          local.get $q
          local.get $i
          local.get $sum
          local.get $x
          i64.div_u
          i32.wrap_i64
          array.set $limbs
          local.get $sum
          local.get $x
          i64.rem_u
          local.set $carry
          br $small_divide
        end
      end
      local.get $q
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
      ref.as_non_null
      local.get $carry
      i32.wrap_i64
    global.get $prof_9
    i64.const 1
    i64.add
    global.set $prof_9
      array.new_fixed $limbs 1
      return
    end
    local.get $bn
    i32.const 1
    i32.add
    local.set $rn
    local.get $an
    global.get $prof_8
    i64.const 1
    i64.add
    global.set $prof_8
    array.new_default $limbs
    local.set $q
    local.get $rn
    global.get $prof_8
    i64.const 1
    i64.add
    global.set $prof_8
    array.new_default $limbs
    local.set $r
    local.get $an
    i64.extend_i32_u
    i64.const 32
    i64.mul
    local.set $bit
    block $done
      loop $bits
        local.get $bit
        i64.eqz
        br_if $done
        local.get $bit
        i64.const 1
        i64.sub
        local.set $bit
        local.get $bit
        i64.const 5
        i64.shr_u
        i32.wrap_i64
        local.set $word
        local.get $bit
        i32.wrap_i64
        i32.const 31
        i32.and
        local.set $shift
        local.get $a
        local.get $word
    global.get $prof_12
    i64.const 1
    i64.add
    global.set $prof_12
        array.get $limbs
        local.get $shift
        i32.shr_u
        i32.const 1
        i32.and
        i64.extend_i32_u
        local.set $carry
        i32.const 0
        local.set $i
        block $shift_done
          loop $shift_r
            local.get $i
            local.get $rn
            i32.ge_u
            br_if $shift_done
            local.get $r
            local.get $i
    global.get $prof_12
    i64.const 1
    i64.add
    global.set $prof_12
            array.get $limbs
            i64.extend_i32_u
            i64.const 1
            i64.shl
            local.get $carry
            i64.or
            local.set $sum
            local.get $r
            local.get $i
            local.get $sum
            i32.wrap_i64
            array.set $limbs
            local.get $sum
            i64.const 32
            i64.shr_u
            local.set $carry
            local.get $i
            i32.const 1
            i32.add
            local.set $i
            br $shift_r
          end
        end
        local.get $r
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
        ref.as_non_null
        local.get $b
        call $mag_compare
        i32.const 0
        i32.ge_s
        if ;; label = @3
          i32.const 0
          local.set $i
          i64.const 0
          local.set $borrow
          block $sub_done
            loop $sub_r
              local.get $i
              local.get $rn
              i32.ge_u
              br_if $sub_done
              local.get $r
              local.get $i
    global.get $prof_12
    i64.const 1
    i64.add
    global.set $prof_12
              array.get $limbs
              i64.extend_i32_u
              local.set $x
              local.get $borrow
              local.set $y
              local.get $i
              local.get $bn
              i32.lt_u
              if ;; label = @6
                local.get $y
                local.get $b
                local.get $i
    global.get $prof_12
    i64.const 1
    i64.add
    global.set $prof_12
                array.get $limbs
                i64.extend_i32_u
                i64.add
                local.set $y
              end
              local.get $r
              local.get $i
              local.get $x
              local.get $y
              i64.sub
              i32.wrap_i64
              array.set $limbs
              local.get $x
              local.get $y
              i64.lt_u
              i64.extend_i32_u
              local.set $borrow
              local.get $i
              i32.const 1
              i32.add
              local.set $i
              br $sub_r
            end
          end
          local.get $q
          local.get $word
          local.get $q
          local.get $word
    global.get $prof_12
    i64.const 1
    i64.add
    global.set $prof_12
          array.get $limbs
          i32.const 1
          local.get $shift
          i32.shl
          i32.or
          array.set $limbs
        end
        br $bits
      end
    end
    local.get $q
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
    ref.as_non_null
    local.get $r
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
    ref.as_non_null
  )
  (func $int_div (;20;) (type $call) (param $a eqref) (param $b eqref) (result eqref)
    (local $q (ref null $limbs)) (local $r (ref null $limbs))
    global.get $prof_29
    i64.const 1
    i64.add
    global.set $prof_29
    local.get $b
    call $int_sign
    i32.eqz
    if ;; label = @1
      i32.const 0
    global.get $prof_14
    i64.const 1
    i64.add
    global.set $prof_14
      ref.i31
      return
    end
    local.get $a
    global.get $prof_3
    i64.const 1
    i64.add
    global.set $prof_3
    ref.test (ref i31)
    local.get $b
    global.get $prof_3
    i64.const 1
    i64.add
    global.set $prof_3
    ref.test (ref i31)
    i32.and
    if ;; label = @1
      local.get $a
    global.get $prof_4
    i64.const 1
    i64.add
    global.set $prof_4
      ref.cast (ref i31)
    global.get $prof_5
    i64.const 1
    i64.add
    global.set $prof_5
      i31.get_s
      i64.extend_i32_s
      local.get $b
    global.get $prof_4
    i64.const 1
    i64.add
    global.set $prof_4
      ref.cast (ref i31)
    global.get $prof_5
    i64.const 1
    i64.add
    global.set $prof_5
      i31.get_s
      i64.extend_i32_s
      i64.div_s
      call $int_from_i64
      return
    end
    local.get $a
    call $int_limbs
    local.get $b
    call $int_limbs
    call $mag_divrem
    local.set $r
    local.set $q
    local.get $a
    call $int_sign
    local.get $b
    call $int_sign
    i32.mul
    local.get $q
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
    ref.as_non_null
    call $int_pack
  )
  (func $int_rem (;21;) (type $call) (param $a eqref) (param $b eqref) (result eqref)
    (local $q (ref null $limbs)) (local $r (ref null $limbs))
    global.get $prof_30
    i64.const 1
    i64.add
    global.set $prof_30
    local.get $b
    call $int_sign
    i32.eqz
    if ;; label = @1
      i32.const 0
    global.get $prof_14
    i64.const 1
    i64.add
    global.set $prof_14
      ref.i31
      return
    end
    local.get $a
    global.get $prof_3
    i64.const 1
    i64.add
    global.set $prof_3
    ref.test (ref i31)
    local.get $b
    global.get $prof_3
    i64.const 1
    i64.add
    global.set $prof_3
    ref.test (ref i31)
    i32.and
    if ;; label = @1
      local.get $a
    global.get $prof_4
    i64.const 1
    i64.add
    global.set $prof_4
      ref.cast (ref i31)
    global.get $prof_5
    i64.const 1
    i64.add
    global.set $prof_5
      i31.get_s
      i64.extend_i32_s
      local.get $b
    global.get $prof_4
    i64.const 1
    i64.add
    global.set $prof_4
      ref.cast (ref i31)
    global.get $prof_5
    i64.const 1
    i64.add
    global.set $prof_5
      i31.get_s
      i64.extend_i32_s
      i64.rem_s
      call $int_from_i64
      return
    end
    local.get $a
    call $int_limbs
    local.get $b
    call $int_limbs
    call $mag_divrem
    local.set $r
    local.set $q
    local.get $a
    call $int_sign
    local.get $r
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
    ref.as_non_null
    call $int_pack
  )
  (func $int_twos (;22;) (type 47) (param $v eqref) (param $n i32) (result (ref $limbs))
    (local $m (ref $limbs)) (local $out (ref $limbs)) (local $i i32) (local $size i32) (local $negative i32) (local $carry i64) (local $word i64)
    global.get $prof_31
    i64.const 1
    i64.add
    global.set $prof_31
    local.get $v
    call $int_limbs
    local.set $m
    local.get $m
    array.len
    local.set $size
    local.get $n
    global.get $prof_8
    i64.const 1
    i64.add
    global.set $prof_8
    array.new_default $limbs
    local.set $out
    local.get $v
    call $int_sign
    i32.const 0
    i32.lt_s
    local.set $negative
    local.get $negative
    i64.extend_i32_u
    local.set $carry
    block $done
      loop $next
        local.get $i
        local.get $n
        i32.ge_u
        br_if $done
        local.get $i
        local.get $size
        i32.lt_u
        if (result i64) ;; label = @3
          local.get $m
          local.get $i
    global.get $prof_12
    i64.const 1
    i64.add
    global.set $prof_12
          array.get $limbs
          i64.extend_i32_u
        else
          i64.const 0
        end
        local.set $word
        local.get $negative
        if ;; label = @3
          local.get $word
          i64.const 4294967295
          i64.xor
          local.get $carry
          i64.add
          local.set $word
          local.get $word
          i64.const 32
          i64.shr_u
          local.set $carry
        end
        local.get $out
        local.get $i
        local.get $word
        i32.wrap_i64
        array.set $limbs
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $next
      end
    end
    local.get $out
  )
  (func $int_from_twos (;23;) (type 48) (param $words (ref $limbs)) (result eqref)
    (local $n i32) (local $i i32) (local $carry i64) (local $word i64)
    global.get $prof_32
    i64.const 1
    i64.add
    global.set $prof_32
    local.get $words
    array.len
    local.set $n
    local.get $n
    i32.eqz
    if ;; label = @1
      i32.const 0
    global.get $prof_14
    i64.const 1
    i64.add
    global.set $prof_14
      ref.i31
      return
    end
    local.get $words
    local.get $n
    i32.const 1
    i32.sub
    global.get $prof_12
    i64.const 1
    i64.add
    global.set $prof_12
    array.get $limbs
    i32.const 0
    i32.ge_s
    if ;; label = @1
      i32.const 1
      local.get $words
      call $int_pack
      return
    end
    i64.const 1
    local.set $carry
    block $done
      loop $next
        local.get $i
        local.get $n
        i32.ge_u
        br_if $done
        local.get $words
        local.get $i
    global.get $prof_12
    i64.const 1
    i64.add
    global.set $prof_12
        array.get $limbs
        i32.const -1
        i32.xor
        i64.extend_i32_u
        local.get $carry
        i64.add
        local.set $word
        local.get $words
        local.get $i
        local.get $word
        i32.wrap_i64
        array.set $limbs
        local.get $word
        i64.const 32
        i64.shr_u
        local.set $carry
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $next
      end
    end
    i32.const -1
    local.get $words
    call $int_pack
  )
  (func $int_bitop (;24;) (type 49) (param $a eqref) (param $b eqref) (param $op i32) (result eqref)
    (local $an i32) (local $bn i32) (local $n i32) (local $i i32) (local $aa (ref $limbs)) (local $bb (ref $limbs)) (local $x i32) (local $y i32)
    global.get $prof_33
    i64.const 1
    i64.add
    global.set $prof_33
    local.get $a
    global.get $prof_3
    i64.const 1
    i64.add
    global.set $prof_3
    ref.test (ref i31)
    local.get $b
    global.get $prof_3
    i64.const 1
    i64.add
    global.set $prof_3
    ref.test (ref i31)
    i32.and
    if ;; label = @1
      local.get $a
    global.get $prof_4
    i64.const 1
    i64.add
    global.set $prof_4
      ref.cast (ref i31)
    global.get $prof_5
    i64.const 1
    i64.add
    global.set $prof_5
      i31.get_s
      local.set $x
      local.get $b
    global.get $prof_4
    i64.const 1
    i64.add
    global.set $prof_4
      ref.cast (ref i31)
    global.get $prof_5
    i64.const 1
    i64.add
    global.set $prof_5
      i31.get_s
      local.set $y
      local.get $op
      i32.eqz
      if (result i32) ;; label = @2
        local.get $x
        local.get $y
        i32.and
      else
        local.get $op
        i32.const 1
        i32.eq
        if (result i32) ;; label = @3
          local.get $x
          local.get $y
          i32.or
        else
          local.get $x
          local.get $y
          i32.xor
        end
      end
    global.get $prof_14
    i64.const 1
    i64.add
    global.set $prof_14
      ref.i31
      return
    end
    local.get $a
    call $int_limbs
    array.len
    local.set $an
    local.get $b
    call $int_limbs
    array.len
    local.set $bn
    local.get $an
    local.get $bn
    local.get $an
    local.get $bn
    i32.gt_u
    select
    local.set $n
    local.get $n
    i32.const -1
    i32.eq
    if ;; label = @1
      unreachable
    end
    local.get $n
    i32.const 1
    i32.add
    local.set $n
    local.get $a
    local.get $n
    call $int_twos
    local.set $aa
    local.get $b
    local.get $n
    call $int_twos
    local.set $bb
    block $done
      loop $next
        local.get $i
        local.get $n
        i32.ge_u
        br_if $done
        local.get $aa
        local.get $i
    global.get $prof_12
    i64.const 1
    i64.add
    global.set $prof_12
        array.get $limbs
        local.set $x
        local.get $bb
        local.get $i
    global.get $prof_12
    i64.const 1
    i64.add
    global.set $prof_12
        array.get $limbs
        local.set $y
        local.get $aa
        local.get $i
        local.get $op
        i32.eqz
        if (result i32) ;; label = @3
          local.get $x
          local.get $y
          i32.and
        else
          local.get $op
          i32.const 1
          i32.eq
          if (result i32) ;; label = @4
            local.get $x
            local.get $y
            i32.or
          else
            local.get $x
            local.get $y
            i32.xor
          end
        end
        array.set $limbs
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $next
      end
    end
    local.get $aa
    call $int_from_twos
  )
  (func $int_and (;25;) (type $call) (param $a eqref) (param $b eqref) (result eqref)
    global.get $prof_34
    i64.const 1
    i64.add
    global.set $prof_34
    local.get $a
    global.get $prof_3
    i64.const 1
    i64.add
    global.set $prof_3
    ref.test (ref i31)
    if ;; label = @1
      local.get $a
    global.get $prof_4
    i64.const 1
    i64.add
    global.set $prof_4
      ref.cast (ref i31)
    global.get $prof_5
    i64.const 1
    i64.add
    global.set $prof_5
      i31.get_s
      i32.const 0
      i32.ge_s
      if ;; label = @2
        local.get $a
    global.get $prof_4
    i64.const 1
    i64.add
    global.set $prof_4
        ref.cast (ref i31)
    global.get $prof_5
    i64.const 1
    i64.add
    global.set $prof_5
        i31.get_s
        local.get $b
        call $int_low_i32
        i32.and
    global.get $prof_14
    i64.const 1
    i64.add
    global.set $prof_14
        ref.i31
        return
      end
    end
    local.get $b
    global.get $prof_3
    i64.const 1
    i64.add
    global.set $prof_3
    ref.test (ref i31)
    if ;; label = @1
      local.get $b
    global.get $prof_4
    i64.const 1
    i64.add
    global.set $prof_4
      ref.cast (ref i31)
    global.get $prof_5
    i64.const 1
    i64.add
    global.set $prof_5
      i31.get_s
      i32.const 0
      i32.ge_s
      if ;; label = @2
        local.get $b
    global.get $prof_4
    i64.const 1
    i64.add
    global.set $prof_4
        ref.cast (ref i31)
    global.get $prof_5
    i64.const 1
    i64.add
    global.set $prof_5
        i31.get_s
        local.get $a
        call $int_low_i32
        i32.and
    global.get $prof_14
    i64.const 1
    i64.add
    global.set $prof_14
        ref.i31
        return
      end
    end
    local.get $a
    local.get $b
    i32.const 0
    call $int_bitop
  )
  (func $int_low_i32 (;26;) (type 36) (param $v eqref) (result i32)
    (local $word i32)
    global.get $prof_35
    i64.const 1
    i64.add
    global.set $prof_35
    local.get $v
    global.get $prof_3
    i64.const 1
    i64.add
    global.set $prof_3
    ref.test (ref i31)
    if ;; label = @1
      local.get $v
    global.get $prof_4
    i64.const 1
    i64.add
    global.set $prof_4
      ref.cast (ref i31)
    global.get $prof_5
    i64.const 1
    i64.add
    global.set $prof_5
      i31.get_s
      return
    end
    local.get $v
    global.get $prof_4
    i64.const 1
    i64.add
    global.set $prof_4
    ref.cast (ref $obj)
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
    struct.get $obj 6
    i32.const 0
    global.get $prof_12
    i64.const 1
    i64.add
    global.set $prof_12
    array.get $limbs
    local.set $word
    local.get $v
    call $int_sign
    i32.const 0
    i32.lt_s
    if (result i32) ;; label = @1
      i32.const 0
      local.get $word
      i32.sub
    else
      local.get $word
    end
  )
  (func $int_or (;27;) (type $call) (param $a eqref) (param $b eqref) (result eqref)
    global.get $prof_36
    i64.const 1
    i64.add
    global.set $prof_36
    local.get $a
    local.get $b
    i32.const 1
    call $int_bitop
  )
  (func $int_xor (;28;) (type $call) (param $a eqref) (param $b eqref) (result eqref)
    global.get $prof_37
    i64.const 1
    i64.add
    global.set $prof_37
    local.get $a
    local.get $b
    i32.const 2
    call $int_bitop
  )
  (func $int_not (;29;) (type 45) (param $a eqref) (result eqref)
    global.get $prof_38
    i64.const 1
    i64.add
    global.set $prof_38
    local.get $a
    global.get $prof_3
    i64.const 1
    i64.add
    global.set $prof_3
    ref.test (ref i31)
    if ;; label = @1
      local.get $a
    global.get $prof_4
    i64.const 1
    i64.add
    global.set $prof_4
      ref.cast (ref i31)
    global.get $prof_5
    i64.const 1
    i64.add
    global.set $prof_5
      i31.get_s
      i32.const -1
      i32.xor
    global.get $prof_14
    i64.const 1
    i64.add
    global.set $prof_14
      ref.i31
      return
    end
    local.get $a
    call $int_neg
    i32.const 1
    global.get $prof_14
    i64.const 1
    i64.add
    global.set $prof_14
    ref.i31
    call $int_sub
  )
  (func $int_shift_left_positive (;30;) (type $call) (param $v eqref) (param $count eqref) (result eqref)
    (local $m (ref $limbs)) (local $out (ref $limbs)) (local $size i32) (local $word_shift i32) (local $bit_shift i64) (local $n i64) (local $amount i64) (local $i i32) (local $carry i64) (local $word i64)
    global.get $prof_39
    i64.const 1
    i64.add
    global.set $prof_39
    local.get $v
    call $int_sign
    i32.eqz
    if ;; label = @1
      i32.const 0
    global.get $prof_14
    i64.const 1
    i64.add
    global.set $prof_14
      ref.i31
      return
    end
    local.get $count
    call $int_sign
    i32.eqz
    if ;; label = @1
      local.get $v
      return
    end
    local.get $v
    global.get $prof_3
    i64.const 1
    i64.add
    global.set $prof_3
    ref.test (ref i31)
    local.get $count
    global.get $prof_3
    i64.const 1
    i64.add
    global.set $prof_3
    ref.test (ref i31)
    i32.and
    if ;; label = @1
      local.get $count
    global.get $prof_4
    i64.const 1
    i64.add
    global.set $prof_4
      ref.cast (ref i31)
      i31.get_u
      i32.const 33
      i32.le_u
      if ;; label = @2
        local.get $v
    global.get $prof_4
    i64.const 1
    i64.add
    global.set $prof_4
        ref.cast (ref i31)
    global.get $prof_5
    i64.const 1
    i64.add
    global.set $prof_5
        i31.get_s
        i64.extend_i32_s
        local.get $count
    global.get $prof_4
    i64.const 1
    i64.add
    global.set $prof_4
        ref.cast (ref i31)
        i31.get_u
        i64.extend_i32_u
        i64.shl
        call $int_from_i64
        return
      end
    end
    local.get $count
    i64.const 137438953440
    call $int_from_i64
    call $int_compare
    i32.const 0
    i32.gt_s
    if ;; label = @1
      unreachable
    end
    local.get $count
    call $int_to_i64
    local.set $amount
    local.get $v
    call $int_limbs
    local.set $m
    local.get $m
    array.len
    local.set $size
    local.get $amount
    i64.const 5
    i64.shr_u
    i32.wrap_i64
    local.set $word_shift
    local.get $amount
    i64.const 31
    i64.and
    local.set $bit_shift
    local.get $size
    i64.extend_i32_u
    local.get $word_shift
    i64.extend_i32_u
    i64.add
    i64.const 1
    i64.add
    local.set $n
    local.get $n
    i64.const 4294967295
    i64.gt_u
    if ;; label = @1
      unreachable
    end
    local.get $n
    i32.wrap_i64
    global.get $prof_8
    i64.const 1
    i64.add
    global.set $prof_8
    array.new_default $limbs
    local.set $out
    block $done
      loop $next
        local.get $i
        local.get $size
        i32.ge_u
        br_if $done
        local.get $m
        local.get $i
    global.get $prof_12
    i64.const 1
    i64.add
    global.set $prof_12
        array.get $limbs
        i64.extend_i32_u
        local.get $bit_shift
        i64.shl
        local.get $carry
        i64.or
        local.set $word
        local.get $out
        local.get $word_shift
        local.get $i
        i32.add
        local.get $word
        i32.wrap_i64
        array.set $limbs
        local.get $word
        i64.const 32
        i64.shr_u
        local.set $carry
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $next
      end
    end
    local.get $out
    local.get $word_shift
    local.get $size
    i32.add
    local.get $carry
    i32.wrap_i64
    array.set $limbs
    local.get $v
    call $int_sign
    local.get $out
    call $int_pack
  )
  (func $int_shift_right_positive (;31;) (type $call) (param $v eqref) (param $count eqref) (result eqref)
    (local $m (ref $limbs)) (local $out (ref $limbs)) (local $sign i32) (local $size i32) (local $word_shift i32) (local $bit_shift i32) (local $n i32) (local $amount i64) (local $i i32) (local $source i32) (local $discarded i32) (local $word i64) (local $result eqref)
    global.get $prof_40
    i64.const 1
    i64.add
    global.set $prof_40
    local.get $v
    call $int_sign
    local.set $sign
    local.get $sign
    i32.eqz
    if ;; label = @1
      i32.const 0
    global.get $prof_14
    i64.const 1
    i64.add
    global.set $prof_14
      ref.i31
      return
    end
    local.get $count
    call $int_sign
    i32.eqz
    if ;; label = @1
      local.get $v
      return
    end
    local.get $count
    global.get $prof_3
    i64.const 1
    i64.add
    global.set $prof_3
    ref.test (ref i31)
    if ;; label = @1
      local.get $count
    global.get $prof_4
    i64.const 1
    i64.add
    global.set $prof_4
      ref.cast (ref i31)
      i31.get_u
      local.set $word_shift
      local.get $v
    global.get $prof_3
    i64.const 1
    i64.add
    global.set $prof_3
      ref.test (ref i31)
      if ;; label = @2
        local.get $word_shift
        i32.const 31
        i32.ge_u
        if ;; label = @3
          i32.const -1
          i32.const 0
          local.get $sign
          i32.const 0
          i32.lt_s
          select
    global.get $prof_14
    i64.const 1
    i64.add
    global.set $prof_14
          ref.i31
          return
        end
        local.get $v
    global.get $prof_4
    i64.const 1
    i64.add
    global.set $prof_4
        ref.cast (ref i31)
    global.get $prof_5
    i64.const 1
    i64.add
    global.set $prof_5
        i31.get_s
        local.get $word_shift
        i32.shr_s
    global.get $prof_14
    i64.const 1
    i64.add
    global.set $prof_14
        ref.i31
        return
      end
      local.get $v
    global.get $prof_4
    i64.const 1
    i64.add
    global.set $prof_4
      ref.cast (ref $obj)
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
      struct.get $obj 6
      array.len
      i32.const 1
      i32.eq
      if ;; label = @2
        local.get $word_shift
        i32.const 32
        i32.ge_u
        if ;; label = @3
          i32.const -1
          i32.const 0
          local.get $sign
          i32.const 0
          i32.lt_s
          select
    global.get $prof_14
    i64.const 1
    i64.add
    global.set $prof_14
          ref.i31
          return
        end
        local.get $v
        call $int_to_i64
        local.get $word_shift
        i64.extend_i32_u
        i64.shr_s
        call $int_from_i64
        return
      end
    end
    local.get $v
    call $int_limbs
    local.set $m
    local.get $m
    array.len
    local.set $size
    local.get $count
    local.get $size
    i64.extend_i32_u
    i64.const 5
    i64.shl
    call $int_from_i64
    call $int_compare
    i32.const 0
    i32.ge_s
    if ;; label = @1
      i32.const -1
      i32.const 0
      local.get $sign
      i32.const 0
      i32.lt_s
      select
    global.get $prof_14
    i64.const 1
    i64.add
    global.set $prof_14
      ref.i31
      return
    end
    local.get $count
    call $int_to_i64
    local.set $amount
    local.get $amount
    i64.const 5
    i64.shr_u
    i32.wrap_i64
    local.set $word_shift
    local.get $amount
    i64.const 31
    i64.and
    i32.wrap_i64
    local.set $bit_shift
    local.get $size
    local.get $word_shift
    i32.sub
    local.set $n
    local.get $n
    global.get $prof_8
    i64.const 1
    i64.add
    global.set $prof_8
    array.new_default $limbs
    local.set $out
    block $discard_done
      loop $discard_next
        local.get $i
        local.get $word_shift
        i32.ge_u
        br_if $discard_done
        local.get $discarded
        local.get $m
        local.get $i
    global.get $prof_12
    i64.const 1
    i64.add
    global.set $prof_12
        array.get $limbs
        i32.or
        local.set $discarded
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $discard_next
      end
    end
    local.get $bit_shift
    if ;; label = @1
      local.get $discarded
      local.get $m
      local.get $word_shift
    global.get $prof_12
    i64.const 1
    i64.add
    global.set $prof_12
      array.get $limbs
      i32.const -1
      i32.const 32
      local.get $bit_shift
      i32.sub
      i32.shr_u
      i32.and
      i32.or
      local.set $discarded
    end
    i32.const 0
    local.set $i
    block $done
      loop $next
        local.get $i
        local.get $n
        i32.ge_u
        br_if $done
        local.get $word_shift
        local.get $i
        i32.add
        local.set $source
        local.get $m
        local.get $source
    global.get $prof_12
    i64.const 1
    i64.add
    global.set $prof_12
        array.get $limbs
        i64.extend_i32_u
        local.get $bit_shift
        i64.extend_i32_u
        i64.shr_u
        local.set $word
        local.get $bit_shift
        i32.const 0
        i32.ne
        local.get $source
        i32.const 1
        i32.add
        local.get $size
        i32.lt_u
        i32.and
        if ;; label = @3
          local.get $word
          local.get $m
          local.get $source
          i32.const 1
          i32.add
    global.get $prof_12
    i64.const 1
    i64.add
    global.set $prof_12
          array.get $limbs
          i64.extend_i32_u
          i32.const 32
          local.get $bit_shift
          i32.sub
          i64.extend_i32_u
          i64.shl
          i64.or
          local.set $word
        end
        local.get $out
        local.get $i
        local.get $word
        i32.wrap_i64
        array.set $limbs
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $next
      end
    end
    local.get $sign
    local.get $out
    call $int_pack
    local.set $result
    local.get $sign
    i32.const 0
    i32.lt_s
    local.get $discarded
    i32.const 0
    i32.ne
    i32.and
    if (result eqref) ;; label = @1
      local.get $result
      i32.const 1
    global.get $prof_14
    i64.const 1
    i64.add
    global.set $prof_14
      ref.i31
      call $int_sub
    else
      local.get $result
    end
  )
  (func $int_shl (;32;) (type $call) (param $v eqref) (param $count eqref) (result eqref)
    global.get $prof_41
    i64.const 1
    i64.add
    global.set $prof_41
    local.get $count
    call $int_sign
    i32.const 0
    i32.lt_s
    if (result eqref) ;; label = @1
      local.get $v
      local.get $count
      call $int_neg
      call $int_shift_right_positive
    else
      local.get $v
      local.get $count
      call $int_shift_left_positive
    end
  )
  (func $int_shr (;33;) (type $call) (param $v eqref) (param $count eqref) (result eqref)
    global.get $prof_42
    i64.const 1
    i64.add
    global.set $prof_42
    local.get $count
    call $int_sign
    i32.const 0
    i32.lt_s
    if (result eqref) ;; label = @1
      local.get $v
      local.get $count
      call $int_neg
      call $int_shift_left_positive
    else
      local.get $v
      local.get $count
      call $int_shift_right_positive
    end
  )
  (func $int_bitwise_and (;34;) (type $call) (param $a eqref) (param $b eqref) (result eqref)
    global.get $prof_43
    i64.const 1
    i64.add
    global.set $prof_43
    local.get $a
    local.get $b
    call $int_and
  )
  (func $int_bitwise_or (;35;) (type $call) (param $a eqref) (param $b eqref) (result eqref)
    global.get $prof_44
    i64.const 1
    i64.add
    global.set $prof_44
    local.get $a
    local.get $b
    call $int_or
  )
  (func $int_bitwise_xor (;36;) (type $call) (param $a eqref) (param $b eqref) (result eqref)
    global.get $prof_45
    i64.const 1
    i64.add
    global.set $prof_45
    local.get $a
    local.get $b
    call $int_xor
  )
  (func $int_bitwise_not (;37;) (type 45) (param $a eqref) (result eqref)
    global.get $prof_46
    i64.const 1
    i64.add
    global.set $prof_46
    local.get $a
    call $int_not
  )
  (func $int_bitwise_shift_left (;38;) (type $call) (param $v eqref) (param $count eqref) (result eqref)
    global.get $prof_47
    i64.const 1
    i64.add
    global.set $prof_47
    local.get $v
    local.get $count
    call $int_shl
  )
  (func $int_bitwise_shift_right (;39;) (type $call) (param $v eqref) (param $count eqref) (result eqref)
    global.get $prof_48
    i64.const 1
    i64.add
    global.set $prof_48
    local.get $v
    local.get $count
    call $int_shr
  )
  (func $conv_mag_bit (;40;) (type 50) (param $a (ref $limbs)) (param $bit i32) (result i32)
    global.get $prof_49
    i64.const 1
    i64.add
    global.set $prof_49
    local.get $a
    local.get $bit
    i32.const 5
    i32.shr_u
    global.get $prof_12
    i64.const 1
    i64.add
    global.set $prof_12
    array.get $limbs
    local.get $bit
    i32.const 31
    i32.and
    i32.shr_u
    i32.const 1
    i32.and
  )
  (func $conv_string (;41;) (type 51) (param $bytes (ref $bytes)) (result eqref)
    global.get $prof_50
    i64.const 1
    i64.add
    global.set $prof_50
    i32.const 3
    i32.const 0
    i64.const 0
    f64.const 0x0p+0 (;=0;)
    local.get $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
  )
  (func $int_to_f64 (;42;) (type 52) (param $value eqref) (result f64)
    (local $sign i32) (local $a (ref $limbs)) (local $n i32) (local $bits i32) (local $shift i32) (local $i i32) (local $q i64) (local $sticky i32) (local $number f64)
    global.get $prof_51
    i64.const 1
    i64.add
    global.set $prof_51
    local.get $value
    global.get $prof_3
    i64.const 1
    i64.add
    global.set $prof_3
    ref.test (ref i31)
    if ;; label = @1
      local.get $value
    global.get $prof_4
    i64.const 1
    i64.add
    global.set $prof_4
      ref.cast (ref i31)
    global.get $prof_5
    i64.const 1
    i64.add
    global.set $prof_5
      i31.get_s
      f64.convert_i32_s
      return
    end
    local.get $value
    call $int_sign
    local.set $sign
    local.get $sign
    i32.eqz
    if ;; label = @1
      f64.const 0x0p+0 (;=0;)
      return
    end
    local.get $value
    call $int_limbs
    local.set $a
    local.get $a
    array.len
    local.set $n
    local.get $n
    i32.const 32
    i32.gt_u
    if ;; label = @1
      call $stop_float
      unreachable
    end
    local.get $n
    i32.const 32
    i32.mul
    local.get $a
    local.get $n
    i32.const 1
    i32.sub
    global.get $prof_12
    i64.const 1
    i64.add
    global.set $prof_12
    array.get $limbs
    i32.clz
    i32.sub
    local.set $bits
    local.get $bits
    i32.const 53
    i32.gt_u
    if (result i32) ;; label = @1
      local.get $bits
      i32.const 53
      i32.sub
    else
      i32.const 0
    end
    local.set $shift
    local.get $bits
    local.set $i
    block $significand_done
      loop $significand
        local.get $i
        local.get $shift
        i32.le_u
        br_if $significand_done
        local.get $i
        i32.const 1
        i32.sub
        local.set $i
        local.get $q
        i64.const 1
        i64.shl
        local.get $a
        local.get $i
        call $conv_mag_bit
        i64.extend_i32_u
        i64.or
        local.set $q
        br $significand
      end
    end
    local.get $shift
    i32.const 0
    i32.gt_u
    if ;; label = @1
      local.get $a
      local.get $shift
      i32.const 1
      i32.sub
      call $conv_mag_bit
      if ;; label = @2
        i32.const 0
        local.set $i
        block $sticky_done
          loop $scan_sticky
            local.get $i
            local.get $shift
            i32.const 1
            i32.sub
            i32.ge_u
            br_if $sticky_done
            local.get $a
            local.get $i
            call $conv_mag_bit
            if ;; label = @5
              i32.const 1
              local.set $sticky
              br $sticky_done
            end
            local.get $i
            i32.const 1
            i32.add
            local.set $i
            br $scan_sticky
          end
        end
        local.get $sticky
        local.get $q
        i64.const 1
        i64.and
        i32.wrap_i64
        i32.or
        if ;; label = @3
          local.get $q
          i64.const 1
          i64.add
          local.set $q
        end
      end
    end
    local.get $q
    f64.convert_i64_u
    local.get $shift
    i32.const 1023
    i32.add
    i64.extend_i32_u
    i64.const 52
    i64.shl
    f64.reinterpret_i64
    global.get $prof_52
    i64.const 1
    i64.add
    global.set $prof_52
    f64.mul
    local.set $number
    local.get $number
    f64.const 0x1.fffffffffffffp+1023 (;=179769313486231570000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000;)
    f64.gt
    if ;; label = @1
      call $stop_float
      unreachable
    end
    local.get $sign
    i32.const 0
    i32.lt_s
    if (result f64) ;; label = @1
      local.get $number
    global.get $prof_53
    i64.const 1
    i64.add
    global.set $prof_53
      f64.neg
    else
      local.get $number
    end
  )
  (func $int_from_f64 (;43;) (type 53) (param $number f64) (result eqref)
    (local $absolute f64) (local $bits i64) (local $mantissa i64) (local $exponent i32) (local $shift i32) (local $sign i32) (local $a (ref $limbs)) (local $i i32) (local $position i32) (local $word i32) (local $small i64)
    global.get $prof_54
    i64.const 1
    i64.add
    global.set $prof_54
    local.get $number
    f64.abs
    local.set $absolute
    local.get $number
    local.get $number
    f64.ne
    local.get $absolute
    f64.const 0x1.fffffffffffffp+1023 (;=179769313486231570000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000;)
    f64.gt
    i32.or
    if ;; label = @1
      unreachable
    end
    local.get $absolute
    f64.const 0x1p+0 (;=1;)
    f64.lt
    if ;; label = @1
      i32.const 0
    global.get $prof_14
    i64.const 1
    i64.add
    global.set $prof_14
      ref.i31
      return
    end
    local.get $number
    f64.const 0x0p+0 (;=0;)
    f64.lt
    if (result i32) ;; label = @1
      i32.const -1
    else
      i32.const 1
    end
    local.set $sign
    local.get $absolute
    i64.reinterpret_f64
    local.set $bits
    local.get $bits
    i64.const 52
    i64.shr_u
    i64.const 2047
    i64.and
    i32.wrap_i64
    i32.const 1023
    i32.sub
    local.set $exponent
    local.get $bits
    i64.const 4503599627370495
    i64.and
    i64.const 4503599627370496
    i64.or
    local.set $mantissa
    local.get $exponent
    i32.const 52
    i32.le_s
    if ;; label = @1
      local.get $mantissa
      i32.const 52
      local.get $exponent
      i32.sub
      i64.extend_i32_u
      i64.shr_u
      local.set $small
      local.get $sign
      i32.const 0
      i32.lt_s
      if ;; label = @2
        i64.const 0
        local.get $small
        i64.sub
        local.set $small
      end
      local.get $small
      call $int_from_i64
      return
    end
    local.get $exponent
    i32.const 52
    i32.sub
    local.set $shift
    local.get $exponent
    i32.const 32
    i32.add
    i32.const 5
    i32.shr_u
    global.get $prof_8
    i64.const 1
    i64.add
    global.set $prof_8
    array.new_default $limbs
    local.set $a
    block $decoded
      loop $decode
        local.get $i
        i32.const 53
        i32.ge_u
        br_if $decoded
        local.get $mantissa
        local.get $i
        i64.extend_i32_u
        i64.shr_u
        i64.const 1
        i64.and
        i64.const 0
        i64.ne
        if ;; label = @3
          local.get $i
          local.get $shift
          i32.add
          local.set $position
          local.get $position
          i32.const 5
          i32.shr_u
          local.set $word
          local.get $a
          local.get $word
          local.get $a
          local.get $word
    global.get $prof_12
    i64.const 1
    i64.add
    global.set $prof_12
          array.get $limbs
          i32.const 1
          local.get $position
          i32.const 31
          i32.and
          i32.shl
          i32.or
          array.set $limbs
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $decode
      end
    end
    local.get $sign
    local.get $a
    call $int_pack
  )
  (func $float_truncate (;44;) (type 45) (param $value eqref) (result eqref)
    global.get $prof_55
    i64.const 1
    i64.add
    global.set $prof_55
    local.get $value
    call $float_get
    call $int_from_f64
  )
  (func $int_to_float (;45;) (type 45) (param $value eqref) (result eqref)
    global.get $prof_56
    i64.const 1
    i64.add
    global.set $prof_56
    local.get $value
    call $int_to_f64
    call $float_new
  )
  (func $float_to_bits (;46;) (type 45) (param $value eqref) (result eqref)
    (local $bits i64)
    global.get $prof_57
    i64.const 1
    i64.add
    global.set $prof_57
    local.get $value
    call $float_get
    i64.reinterpret_f64
    local.set $bits
    i32.const 1
    local.get $bits
    i32.wrap_i64
    local.get $bits
    i64.const 32
    i64.shr_u
    i32.wrap_i64
    global.get $prof_17
    i64.const 1
    i64.add
    global.set $prof_17
    array.new_fixed $limbs 2
    call $int_pack
  )
  (func $float_from_bits (;47;) (type 45) (param $value eqref) (result eqref)
    (local $a (ref $limbs))
    global.get $prof_58
    i64.const 1
    i64.add
    global.set $prof_58
    local.get $value
    i32.const 2
    call $int_twos
    local.set $a
    local.get $a
    i32.const 0
    global.get $prof_12
    i64.const 1
    i64.add
    global.set $prof_12
    array.get $limbs
    i64.extend_i32_u
    local.get $a
    i32.const 1
    global.get $prof_12
    i64.const 1
    i64.add
    global.set $prof_12
    array.get $limbs
    i64.extend_i32_u
    i64.const 32
    i64.shl
    i64.or
    f64.reinterpret_i64
    call $float_new
  )
  (func $int_to_string (;48;) (type 45) (param $value eqref) (result eqref)
    global.get $prof_59
    i64.const 1
    i64.add
    global.set $prof_59
    local.get $value
    i32.const 10
    global.get $prof_14
    i64.const 1
    i64.add
    global.set $prof_14
    ref.i31
    call $int_to_base_string
  )
  (func $int_to_base_string (;49;) (type $call) (param $value eqref) (param $base_value eqref) (result eqref)
    (local $sign i32) (local $base i64) (local $a (ref $limbs)) (local $work (ref $limbs)) (local $n i32) (local $i i32) (local $count i32) (local $remainder i64) (local $dividend i64) (local $digit i32) (local $reverse (ref $bytes)) (local $bytes (ref $bytes))
    global.get $prof_60
    i64.const 1
    i64.add
    global.set $prof_60
    local.get $base_value
    i32.const 2
    global.get $prof_14
    i64.const 1
    i64.add
    global.set $prof_14
    ref.i31
    call $int_compare
    i32.const 0
    i32.lt_s
    local.get $base_value
    i32.const 36
    global.get $prof_14
    i64.const 1
    i64.add
    global.set $prof_14
    ref.i31
    call $int_compare
    i32.const 0
    i32.gt_s
    i32.or
    if ;; label = @1
      unreachable
    end
    local.get $base_value
    call $int_to_i64
    local.set $base
    local.get $value
    call $int_sign
    local.set $sign
    local.get $sign
    i32.eqz
    if ;; label = @1
      i32.const 48
    global.get $prof_61
    i64.const 1
    i64.add
    global.set $prof_61
      array.new_fixed $bytes 1
      call $conv_string
      return
    end
    local.get $value
    call $int_limbs
    local.set $a
    local.get $a
    array.len
    local.set $n
    local.get $n
    i32.const 134217727
    i32.gt_u
    if ;; label = @1
      unreachable
    end
    local.get $n
    global.get $prof_8
    i64.const 1
    i64.add
    global.set $prof_8
    array.new_default $limbs
    local.set $work
    local.get $work
    i32.const 0
    local.get $a
    i32.const 0
    local.get $n
    array.copy $limbs $limbs
    local.get $n
    i32.const 32
    i32.mul
    i32.const 1
    i32.add
    global.get $prof_62
    i64.const 1
    i64.add
    global.set $prof_62
    array.new_default $bytes
    local.set $reverse
    block $digits_done
      loop $digits
        local.get $n
        i32.eqz
        br_if $digits_done
        local.get $n
        local.set $i
        i64.const 0
        local.set $remainder
        block $division_done
          loop $division
            local.get $i
            i32.eqz
            br_if $division_done
            local.get $i
            i32.const 1
            i32.sub
            local.set $i
            local.get $remainder
            i64.const 32
            i64.shl
            local.get $work
            local.get $i
    global.get $prof_12
    i64.const 1
    i64.add
    global.set $prof_12
            array.get $limbs
            i64.extend_i32_u
            i64.or
            local.set $dividend
            local.get $work
            local.get $i
            local.get $dividend
            local.get $base
            i64.div_u
            i32.wrap_i64
            array.set $limbs
            local.get $dividend
            local.get $base
            i64.rem_u
            local.set $remainder
            br $division
          end
        end
        local.get $remainder
        i32.wrap_i64
        local.set $digit
        local.get $reverse
        local.get $count
        local.get $digit
        local.get $digit
        i32.const 10
        i32.lt_u
        if (result i32) ;; label = @3
          i32.const 48
        else
          i32.const 55
        end
        i32.add
        array.set $bytes
        local.get $count
        i32.const 1
        i32.add
        local.set $count
        block $trimmed
          loop $trim
            local.get $n
            i32.eqz
            br_if $trimmed
            local.get $work
            local.get $n
            i32.const 1
            i32.sub
    global.get $prof_12
    i64.const 1
    i64.add
    global.set $prof_12
            array.get $limbs
            br_if $trimmed
            local.get $n
            i32.const 1
            i32.sub
            local.set $n
            br $trim
          end
        end
        br $digits
      end
    end
    local.get $sign
    i32.const 0
    i32.lt_s
    if ;; label = @1
      local.get $reverse
      local.get $count
      i32.const 45
      array.set $bytes
      local.get $count
      i32.const 1
      i32.add
      local.set $count
    end
    local.get $count
    global.get $prof_62
    i64.const 1
    i64.add
    global.set $prof_62
    array.new_default $bytes
    local.set $bytes
    i32.const 0
    local.set $i
    block $reversed
      loop $reverse_digits
        local.get $i
        local.get $count
        i32.ge_u
        br_if $reversed
        local.get $bytes
        local.get $i
        local.get $reverse
        local.get $count
        local.get $i
        i32.sub
        i32.const 1
        i32.sub
        array.get_u $bytes
        array.set $bytes
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $reverse_digits
      end
    end
    local.get $bytes
    call $conv_string
  )
  (func $int_parse (;50;) (type 54) (param $string eqref) (param $base eqref) (result eqref i32)
    (local $bytes (ref $bytes)) (local $n i32) (local $i i32) (local $negative i32) (local $digit i32) (local $char i32) (local $radix i32) (local $value eqref)
    global.get $prof_63
    i64.const 1
    i64.add
    global.set $prof_63
    block $invalid
      local.get $base
      i32.const 2
    global.get $prof_14
    i64.const 1
    i64.add
    global.set $prof_14
      ref.i31
      call $int_compare
      i32.const 0
      i32.lt_s
      local.get $base
      i32.const 36
    global.get $prof_14
    i64.const 1
    i64.add
    global.set $prof_14
      ref.i31
      call $int_compare
      i32.const 0
      i32.gt_s
      i32.or
      br_if $invalid
      local.get $base
      call $int_to_i64
      i32.wrap_i64
      local.set $radix
      local.get $string
    global.get $prof_4
    i64.const 1
    i64.add
    global.set $prof_4
      ref.cast (ref $obj)
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
      struct.get $obj 4
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
      ref.as_non_null
      local.set $bytes
      local.get $bytes
      array.len
      local.set $n
      local.get $n
      i32.eqz
      br_if $invalid
      local.get $bytes
      i32.const 0
      array.get_u $bytes
      local.set $char
      local.get $char
      i32.const 43
      i32.eq
      local.get $char
      i32.const 45
      i32.eq
      i32.or
      if ;; label = @2
        local.get $char
        i32.const 45
        i32.eq
        local.set $negative
        i32.const 1
        local.set $i
      end
      local.get $i
      local.get $n
      i32.eq
      br_if $invalid
      i32.const 0
    global.get $prof_14
    i64.const 1
    i64.add
    global.set $prof_14
      ref.i31
      local.set $value
      block $parsed
        loop $parse_digit
          local.get $i
          local.get $n
          i32.ge_u
          br_if $parsed
          local.get $bytes
          local.get $i
          array.get_u $bytes
          local.set $char
          local.get $char
          i32.const 48
          i32.ge_u
          local.get $char
          i32.const 57
          i32.le_u
          i32.and
          if (result i32) ;; label = @4
            local.get $char
            i32.const 48
            i32.sub
          else
            local.get $char
            i32.const 65
            i32.ge_u
            local.get $char
            i32.const 90
            i32.le_u
            i32.and
            if (result i32) ;; label = @5
              local.get $char
              i32.const 55
              i32.sub
            else
              local.get $char
              i32.const 97
              i32.ge_u
              local.get $char
              i32.const 122
              i32.le_u
              i32.and
              if (result i32) ;; label = @6
                local.get $char
                i32.const 87
                i32.sub
              else
                i32.const -1
              end
            end
          end
          local.set $digit
          local.get $digit
          local.get $radix
          i32.ge_u
          br_if $invalid
          local.get $value
          local.get $base
          call $int_mul
          local.get $digit
    global.get $prof_14
    i64.const 1
    i64.add
    global.set $prof_14
          ref.i31
          call $int_add
          local.set $value
          local.get $i
          i32.const 1
          i32.add
          local.set $i
          br $parse_digit
        end
      end
      local.get $negative
      if ;; label = @2
        local.get $value
        call $int_neg
        local.set $value
      end
      local.get $value
      i32.const 1
      return
    end
    i32.const 0
    global.get $prof_14
    i64.const 1
    i64.add
    global.set $prof_14
    ref.i31
    i32.const 0
  )
  (func $bits_new (;51;) (type 55) (param $data (ref $bytes)) (param $offset i32) (param $length i64) (result eqref)
    global.get $prof_64
    i64.const 1
    i64.add
    global.set $prof_64
    i32.const 9
    local.get $offset
    local.get $length
    f64.const 0x0p+0 (;=0;)
    local.get $data
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
  )
  (func $bits_data (;52;) (type 56) (param $value eqref) (result (ref $bytes))
    global.get $prof_65
    i64.const 1
    i64.add
    global.set $prof_65
    local.get $value
    global.get $prof_4
    i64.const 1
    i64.add
    global.set $prof_4
    ref.cast (ref $obj)
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
    struct.get $obj 4
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
    ref.as_non_null
  )
  (func $bits_len64 (;53;) (type 41) (param $value eqref) (result i64)
    global.get $prof_66
    i64.const 1
    i64.add
    global.set $prof_66
    local.get $value
    global.get $prof_4
    i64.const 1
    i64.add
    global.set $prof_4
    ref.cast (ref $obj)
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
    struct.get $obj 2
  )
  (func $bits_offset (;54;) (type 41) (param $value eqref) (result i64)
    global.get $prof_67
    i64.const 1
    i64.add
    global.set $prof_67
    local.get $value
    global.get $prof_4
    i64.const 1
    i64.add
    global.set $prof_4
    ref.cast (ref $obj)
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
    struct.get $obj 1
    i64.extend_i32_u
  )
  (func $bits_has (;55;) (type 57) (param $value eqref) (param $start i64) (param $length i64) (result i32)
    (local $total i64)
    global.get $prof_68
    i64.const 1
    i64.add
    global.set $prof_68
    local.get $value
    call $bits_len64
    local.set $total
    local.get $start
    i64.const 0
    i64.lt_s
    if ;; label = @1
      i32.const 0
      return
    end
    local.get $length
    i64.const 0
    i64.lt_s
    if ;; label = @1
      i32.const 0
      return
    end
    local.get $start
    local.get $total
    i64.gt_u
    if ;; label = @1
      i32.const 0
      return
    end
    local.get $length
    local.get $total
    local.get $start
    i64.sub
    i64.le_u
  )
  (func $bits_alloc (;56;) (type 58) (param $length i64) (result (ref $bytes))
    (local $bytes i64)
    global.get $prof_69
    i64.const 1
    i64.add
    global.set $prof_69
    local.get $length
    i64.const 0
    i64.lt_s
    if ;; label = @1
      unreachable
    end
    local.get $length
    i64.const 8
    i64.div_u
    local.set $bytes
    local.get $length
    i64.const 7
    i64.and
    i64.const 0
    i64.ne
    if ;; label = @1
      local.get $bytes
      i64.const 1
      i64.add
      local.set $bytes
    end
    local.get $bytes
    i64.const 4294967295
    i64.gt_u
    if ;; label = @1
      unreachable
    end
    local.get $bytes
    i32.wrap_i64
    global.get $prof_62
    i64.const 1
    i64.add
    global.set $prof_62
    array.new_default $bytes
  )
  (func $bits_empty (;57;) (type 59) (result eqref)
    global.get $prof_70
    i64.const 1
    i64.add
    global.set $prof_70
    i32.const 0
    global.get $prof_62
    i64.const 1
    i64.add
    global.set $prof_62
    array.new_default $bytes
    i32.const 0
    i64.const 0
    call $bits_new
  )
  (func $bits_size (;58;) (type 45) (param $value eqref) (result eqref)
    global.get $prof_71
    i64.const 1
    i64.add
    global.set $prof_71
    local.get $value
    call $bits_len64
    call $int_from_i64
  )
  (func $bits_width (;59;) (type 60) (param $size eqref) (param $unit i64) (param $clamp_negative i32) (result i64)
    (local $limbs (ref null $limbs)) (local $count i32) (local $value i64)
    global.get $prof_72
    i64.const 1
    i64.add
    global.set $prof_72
    local.get $size
    call $int_sign
    i32.const 0
    i32.lt_s
    if ;; label = @1
      local.get $clamp_negative
      if (result i64) ;; label = @2
        i64.const 0
      else
        i64.const -1
      end
      return
    end
    local.get $unit
    i64.const 0
    i64.le_s
    if ;; label = @1
      i64.const -1
      return
    end
    local.get $size
    global.get $prof_3
    i64.const 1
    i64.add
    global.set $prof_3
    ref.test (ref i31)
    if ;; label = @1
      local.get $size
    global.get $prof_4
    i64.const 1
    i64.add
    global.set $prof_4
      ref.cast (ref i31)
    global.get $prof_5
    i64.const 1
    i64.add
    global.set $prof_5
      i31.get_s
      i64.extend_i32_s
      local.set $value
    else
      local.get $size
      call $int_limbs
      local.set $limbs
      local.get $limbs
      array.len
      local.set $count
      local.get $count
      i32.const 2
      i32.gt_u
      if ;; label = @2
        i64.const -1
        return
      end
      local.get $count
      i32.const 2
      i32.eq
      if ;; label = @2
        local.get $limbs
        i32.const 1
    global.get $prof_12
    i64.const 1
    i64.add
    global.set $prof_12
        array.get $limbs
        i32.const 0
        i32.lt_s
        if ;; label = @3
          i64.const -1
          return
        end
      end
      local.get $size
      call $int_to_i64
      local.set $value
    end
    local.get $value
    i64.const 9223372036854775807
    local.get $unit
    i64.div_u
    i64.gt_u
    if ;; label = @1
      i64.const -1
      return
    end
    local.get $value
    local.get $unit
    i64.mul
  )
  (func $bits_get (;60;) (type 61) (param $value eqref) (param $index i64) (result i32)
    (local $absolute i64)
    global.get $prof_73
    i64.const 1
    i64.add
    global.set $prof_73
    local.get $value
    local.get $index
    i64.const 1
    call $bits_has
    i32.eqz
    if ;; label = @1
      unreachable
    end
    local.get $value
    call $bits_offset
    local.get $index
    i64.add
    local.set $absolute
    local.get $value
    call $bits_data
    local.get $absolute
    i64.const 3
    i64.shr_u
    i32.wrap_i64
    array.get_u $bytes
    i32.const 7
    local.get $absolute
    i64.const 7
    i64.and
    i32.wrap_i64
    i32.sub
    i32.shr_u
    i32.const 1
    i32.and
  )
  (func $bits_set (;61;) (type 62) (param $data (ref $bytes)) (param $index i64) (param $value i32)
    (local $byte i32) (local $shift i32) (local $old i32)
    global.get $prof_74
    i64.const 1
    i64.add
    global.set $prof_74
    local.get $index
    i64.const 3
    i64.shr_u
    i32.wrap_i64
    local.set $byte
    i32.const 7
    local.get $index
    i64.const 7
    i64.and
    i32.wrap_i64
    i32.sub
    local.set $shift
    local.get $data
    local.get $byte
    array.get_u $bytes
    local.set $old
    local.get $data
    local.get $byte
    local.get $old
    i32.const 1
    local.get $shift
    i32.shl
    i32.const -1
    i32.xor
    i32.and
    local.get $value
    i32.const 1
    i32.and
    local.get $shift
    i32.shl
    i32.or
    array.set $bytes
  )
  (func $bits_slice (;62;) (type 63) (param $value eqref) (param $start i64) (param $length i64) (result eqref)
    (local $absolute i64) (local $data (ref null $bytes)) (local $i i64)
    global.get $prof_75
    i64.const 1
    i64.add
    global.set $prof_75
    local.get $value
    local.get $start
    local.get $length
    call $bits_has
    i32.eqz
    if ;; label = @1
      unreachable
    end
    local.get $value
    call $bits_offset
    local.get $start
    i64.add
    local.set $absolute
    local.get $absolute
    i64.const 4294967295
    i64.le_u
    if ;; label = @1
      local.get $value
      call $bits_data
      local.get $absolute
      i32.wrap_i64
      local.get $length
      call $bits_new
      return
    end
    local.get $length
    call $bits_alloc
    local.set $data
    block $done
      loop $copy
        local.get $i
        local.get $length
        i64.ge_u
        br_if $done
        local.get $data
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
        ref.as_non_null
        local.get $i
        local.get $value
        local.get $start
        local.get $i
        i64.add
        call $bits_get
        call $bits_set
        local.get $i
        i64.const 1
        i64.add
        local.set $i
        br $copy
      end
    end
    local.get $data
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
    ref.as_non_null
    i32.const 0
    local.get $length
    call $bits_new
  )
  (func $bits_concat (;63;) (type $call) (param $left eqref) (param $right eqref) (result eqref)
    (local $a i64) (local $b i64) (local $length i64) (local $data (ref null $bytes)) (local $i i64)
    global.get $prof_76
    i64.const 1
    i64.add
    global.set $prof_76
    local.get $left
    call $bits_len64
    local.set $a
    local.get $right
    call $bits_len64
    local.set $b
    local.get $a
    i64.eqz
    if ;; label = @1
      local.get $right
      return
    end
    local.get $b
    i64.eqz
    if ;; label = @1
      local.get $left
      return
    end
    local.get $a
    local.get $b
    i64.add
    local.set $length
    local.get $length
    local.get $a
    i64.lt_u
    if ;; label = @1
      unreachable
    end
    local.get $length
    call $bits_alloc
    local.set $data
    local.get $a
    local.get $b
    i64.or
    local.get $left
    call $bits_offset
    local.get $right
    call $bits_offset
    i64.or
    i64.or
    i64.const 7
    i64.and
    i64.eqz
    if ;; label = @1
      local.get $data
      i32.const 0
      local.get $left
      call $bits_data
      local.get $left
      call $bits_offset
      i64.const 3
      i64.shr_u
      i32.wrap_i64
      local.get $a
      i64.const 3
      i64.shr_u
      i32.wrap_i64
      array.copy $bytes $bytes
      local.get $data
      local.get $a
      i64.const 3
      i64.shr_u
      i32.wrap_i64
      local.get $right
      call $bits_data
      local.get $right
      call $bits_offset
      i64.const 3
      i64.shr_u
      i32.wrap_i64
      local.get $b
      i64.const 3
      i64.shr_u
      i32.wrap_i64
      array.copy $bytes $bytes
    else
      block $left_done
        loop $copy_left
          local.get $i
          local.get $a
          i64.ge_u
          br_if $left_done
          local.get $data
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
          ref.as_non_null
          local.get $i
          local.get $left
          local.get $i
          call $bits_get
          call $bits_set
          local.get $i
          i64.const 1
          i64.add
          local.set $i
          br $copy_left
        end
      end
      i64.const 0
      local.set $i
      block $right_done
        loop $copy_right
          local.get $i
          local.get $b
          i64.ge_u
          br_if $right_done
          local.get $data
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
          ref.as_non_null
          local.get $a
          local.get $i
          i64.add
          local.get $right
          local.get $i
          call $bits_get
          call $bits_set
          local.get $i
          i64.const 1
          i64.add
          local.set $i
          br $copy_right
        end
      end
    end
    local.get $data
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
    ref.as_non_null
    i32.const 0
    local.get $length
    call $bits_new
  )
  (func $bits_match_equal (;64;) (type 43) (param $left eqref) (param $right eqref) (result i32)
    (local $length i64) (local $i i64) (local $count i64) (local $left_byte i32) (local $right_byte i32) (local $left_data (ref null $bytes)) (local $right_data (ref null $bytes))
    global.get $prof_77
    i64.const 1
    i64.add
    global.set $prof_77
    local.get $left
    call $bits_len64
    local.set $length
    local.get $length
    local.get $right
    call $bits_len64
    i64.ne
    if ;; label = @1
      i32.const 0
      return
    end
    local.get $left
    call $bits_offset
    local.get $right
    call $bits_offset
    i64.or
    i64.const 7
    i64.and
    i64.eqz
    if ;; label = @1
      local.get $left
      call $bits_data
      local.set $left_data
      local.get $right
      call $bits_data
      local.set $right_data
      local.get $left
      call $bits_offset
      i64.const 3
      i64.shr_u
      i32.wrap_i64
      local.set $left_byte
      local.get $right
      call $bits_offset
      i64.const 3
      i64.shr_u
      i32.wrap_i64
      local.set $right_byte
      local.get $length
      i64.const 3
      i64.shr_u
      local.set $count
      block $bytes_done
        loop $compare_bytes
          local.get $i
          local.get $count
          i64.ge_u
          br_if $bytes_done
          local.get $left_data
          local.get $left_byte
          local.get $i
          i32.wrap_i64
          i32.add
          array.get_u $bytes
          local.get $right_data
          local.get $right_byte
          local.get $i
          i32.wrap_i64
          i32.add
          array.get_u $bytes
          i32.ne
          if ;; label = @4
            i32.const 0
            return
          end
          local.get $i
          i64.const 1
          i64.add
          local.set $i
          br $compare_bytes
        end
      end
      local.get $i
      i64.const 3
      i64.shl
      local.set $i
    end
    block $done
      loop $compare
        local.get $i
        local.get $length
        i64.ge_u
        br_if $done
        local.get $left
        local.get $i
        call $bits_get
        local.get $right
        local.get $i
        call $bits_get
        i32.ne
        if ;; label = @3
          i32.const 0
          return
        end
        local.get $i
        i64.const 1
        i64.add
        local.set $i
        br $compare
      end
    end
    i32.const 1
  )
  (func $bits_value_position (;65;) (type 64) (param $index i64) (param $width i64) (param $little i32) (result i64)
    (local $base i64) (local $group i64)
    global.get $prof_78
    i64.const 1
    i64.add
    global.set $prof_78
    local.get $little
    i32.eqz
    if ;; label = @1
      local.get $width
      i64.const 1
      i64.sub
      local.get $index
      i64.sub
      return
    end
    local.get $index
    i64.const -8
    i64.and
    local.set $base
    local.get $width
    local.get $base
    i64.sub
    local.set $group
    local.get $group
    i64.const 8
    i64.gt_u
    if ;; label = @1
      i64.const 8
      local.set $group
    end
    local.get $base
    local.get $group
    i64.add
    i64.const 1
    i64.sub
    local.get $index
    local.get $base
    i64.sub
    i64.sub
  )
  (func $bits_magnitude_bit (;66;) (type 65) (param $limbs (ref $limbs)) (param $position i64) (result i32)
    (local $limb i64)
    global.get $prof_79
    i64.const 1
    i64.add
    global.set $prof_79
    local.get $position
    i64.const 5
    i64.shr_u
    local.set $limb
    local.get $limb
    local.get $limbs
    array.len
    i64.extend_i32_u
    i64.ge_u
    if ;; label = @1
      i32.const 0
      return
    end
    local.get $limbs
    local.get $limb
    i32.wrap_i64
    global.get $prof_12
    i64.const 1
    i64.add
    global.set $prof_12
    array.get $limbs
    local.get $position
    i64.const 31
    i64.and
    i32.wrap_i64
    i32.shr_u
    i32.const 1
    i32.and
  )
  (func $bits_from_int (;67;) (type 66) (param $value eqref) (param $width i64) (param $little i32) (result eqref)
    (local $data (ref null $bytes)) (local $limbs (ref null $limbs)) (local $negative i32) (local $trail i64) (local $limb i32) (local $i i64) (local $position i64) (local $bit i32)
    global.get $prof_80
    i64.const 1
    i64.add
    global.set $prof_80
    local.get $width
    call $bits_alloc
    local.set $data
    local.get $value
    call $int_limbs
    local.set $limbs
    local.get $value
    call $int_sign
    i32.const 0
    i32.lt_s
    local.set $negative
    local.get $negative
    if ;; label = @1
      block $found
        loop $find
          local.get $limbs
          local.get $limb
    global.get $prof_12
    i64.const 1
    i64.add
    global.set $prof_12
          array.get $limbs
          i32.const 0
          i32.ne
          if ;; label = @4
            br $found
          end
          local.get $limb
          i32.const 1
          i32.add
          local.set $limb
          br $find
        end
      end
      local.get $limb
      i64.extend_i32_u
      i64.const 32
      i64.mul
      local.get $limbs
      local.get $limb
    global.get $prof_12
    i64.const 1
    i64.add
    global.set $prof_12
      array.get $limbs
      i32.ctz
      i64.extend_i32_u
      i64.add
      local.set $trail
    end
    block $done
      loop $write
        local.get $i
        local.get $width
        i64.ge_u
        br_if $done
        local.get $i
        local.get $width
        local.get $little
        call $bits_value_position
        local.set $position
        local.get $limbs
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
        ref.as_non_null
        local.get $position
        call $bits_magnitude_bit
        local.set $bit
        local.get $negative
        if ;; label = @3
          local.get $position
          local.get $trail
          i64.gt_u
          if ;; label = @4
            local.get $bit
            i32.const 1
            i32.xor
            local.set $bit
          end
        end
        local.get $data
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
        ref.as_non_null
        local.get $i
        local.get $bit
        call $bits_set
        local.get $i
        i64.const 1
        i64.add
        local.set $i
        br $write
      end
    end
    local.get $data
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
    ref.as_non_null
    i32.const 0
    local.get $width
    call $bits_new
  )
  (func $bits_read_int (;68;) (type 67) (param $value eqref) (param $start i64) (param $width i64) (param $signed i32) (param $little i32) (result eqref)
    (local $limbs (ref null $limbs)) (local $count i64) (local $index i32) (local $i i64) (local $position i64) (local $negative i32) (local $carry i64) (local $word i64) (local $remainder i32)
    global.get $prof_81
    i64.const 1
    i64.add
    global.set $prof_81
    local.get $value
    local.get $start
    local.get $width
    call $bits_has
    i32.eqz
    if ;; label = @1
      unreachable
    end
    local.get $width
    i64.const 31
    i64.add
    i64.const 32
    i64.div_u
    local.set $count
    local.get $count
    i64.const 4294967295
    i64.gt_u
    if ;; label = @1
      unreachable
    end
    local.get $count
    i32.wrap_i64
    global.get $prof_8
    i64.const 1
    i64.add
    global.set $prof_8
    array.new_default $limbs
    local.set $limbs
    block $done
      loop $read
        local.get $i
        local.get $width
        i64.ge_u
        br_if $done
        local.get $i
        local.get $width
        local.get $little
        call $bits_value_position
        local.set $position
        local.get $value
        local.get $start
        local.get $i
        i64.add
        call $bits_get
        if ;; label = @3
          local.get $position
          i64.const 5
          i64.shr_u
          i32.wrap_i64
          local.set $index
          local.get $limbs
          local.get $index
          local.get $limbs
          local.get $index
    global.get $prof_12
    i64.const 1
    i64.add
    global.set $prof_12
          array.get $limbs
          i32.const 1
          local.get $position
          i64.const 31
          i64.and
          i32.wrap_i64
          i32.shl
          i32.or
          array.set $limbs
        end
        local.get $i
        i64.const 1
        i64.add
        local.set $i
        br $read
      end
    end
    local.get $signed
    local.get $width
    i64.const 0
    i64.ne
    i32.and
    if ;; label = @1
      local.get $limbs
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
      ref.as_non_null
      local.get $width
      i64.const 1
      i64.sub
      call $bits_magnitude_bit
      local.set $negative
    end
    local.get $negative
    if ;; label = @1
      i32.const 0
      local.set $index
      i64.const 1
      local.set $carry
      local.get $width
      i64.const 31
      i64.and
      i32.wrap_i64
      local.set $remainder
      block $neg_done
        loop $negate
          local.get $index
          local.get $limbs
          array.len
          i32.ge_u
          br_if $neg_done
          local.get $limbs
          local.get $index
    global.get $prof_12
    i64.const 1
    i64.add
    global.set $prof_12
          array.get $limbs
          i32.const -1
          i32.xor
          i64.extend_i32_u
          local.set $word
          local.get $index
          local.get $limbs
          array.len
          i32.const 1
          i32.sub
          i32.eq
          local.get $remainder
          i32.const 0
          i32.ne
          i32.and
          if ;; label = @4
            local.get $word
            i64.const 1
            local.get $remainder
            i64.extend_i32_u
            i64.shl
            i64.const 1
            i64.sub
            i64.and
            local.set $word
          end
          local.get $word
          local.get $carry
          i64.add
          local.set $word
          local.get $limbs
          local.get $index
          local.get $word
          i32.wrap_i64
          array.set $limbs
          local.get $word
          i64.const 32
          i64.shr_u
          local.set $carry
          local.get $index
          i32.const 1
          i32.add
          local.set $index
          br $negate
        end
      end
    end
    local.get $negative
    if (result i32) ;; label = @1
      i32.const -1
    else
      i32.const 1
    end
    local.get $limbs
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
    ref.as_non_null
    call $int_pack
  )
  (func $bits_from_u64 (;69;) (type 68) (param $value i64) (param $width i64) (param $little i32) (result eqref)
    (local $data (ref null $bytes)) (local $i i64) (local $position i64)
    global.get $prof_82
    i64.const 1
    i64.add
    global.set $prof_82
    local.get $width
    i64.const 64
    i64.gt_u
    if ;; label = @1
      unreachable
    end
    local.get $width
    call $bits_alloc
    local.set $data
    block $done
      loop $write
        local.get $i
        local.get $width
        i64.ge_u
        br_if $done
        local.get $i
        local.get $width
        local.get $little
        call $bits_value_position
        local.set $position
        local.get $data
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
        ref.as_non_null
        local.get $i
        local.get $value
        local.get $position
        i64.shr_u
        i64.const 1
        i64.and
        i32.wrap_i64
        call $bits_set
        local.get $i
        i64.const 1
        i64.add
        local.set $i
        br $write
      end
    end
    local.get $data
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
    ref.as_non_null
    i32.const 0
    local.get $width
    call $bits_new
  )
  (func $bits_read_u64 (;70;) (type 69) (param $value eqref) (param $start i64) (param $width i64) (param $little i32) (result i64)
    (local $result i64) (local $i i64) (local $position i64)
    global.get $prof_83
    i64.const 1
    i64.add
    global.set $prof_83
    local.get $width
    i64.const 64
    i64.gt_u
    local.get $value
    local.get $start
    local.get $width
    call $bits_has
    i32.eqz
    i32.or
    if ;; label = @1
      unreachable
    end
    block $done
      loop $read
        local.get $i
        local.get $width
        i64.ge_u
        br_if $done
        local.get $i
        local.get $width
        local.get $little
        call $bits_value_position
        local.set $position
        local.get $result
        local.get $value
        local.get $start
        local.get $i
        i64.add
        call $bits_get
        i64.extend_i32_u
        local.get $position
        i64.shl
        i64.or
        local.set $result
        local.get $i
        i64.const 1
        i64.add
        local.set $i
        br $read
      end
    end
    local.get $result
  )
  (func $bits_round_shift (;71;) (type 70) (param $value i64) (param $shift i64) (result i64)
    (local $truncated i64) (local $remainder i64) (local $half i64)
    global.get $prof_84
    i64.const 1
    i64.add
    global.set $prof_84
    local.get $value
    local.get $shift
    i64.shr_u
    local.set $truncated
    local.get $value
    i64.const 1
    local.get $shift
    i64.shl
    i64.const 1
    i64.sub
    i64.and
    local.set $remainder
    i64.const 1
    local.get $shift
    i64.const 1
    i64.sub
    i64.shl
    local.set $half
    local.get $remainder
    local.get $half
    i64.gt_u
    local.get $remainder
    local.get $half
    i64.eq
    local.get $truncated
    i64.const 1
    i64.and
    i64.const 0
    i64.ne
    i32.and
    i32.or
    if ;; label = @1
      local.get $truncated
      i64.const 1
      i64.add
      local.set $truncated
    end
    local.get $truncated
  )
  (func $bits_half_encode (;72;) (type 71) (param $value f64) (result i64)
    (local $raw i64) (local $sign i64) (local $exponent i32) (local $significand i64) (local $rounded i64)
    global.get $prof_85
    i64.const 1
    i64.add
    global.set $prof_85
    local.get $value
    i64.reinterpret_f64
    local.set $raw
    local.get $raw
    i64.const 48
    i64.shr_u
    i64.const 32768
    i64.and
    local.set $sign
    local.get $raw
    i64.const 52
    i64.shr_u
    i64.const 2047
    i64.and
    i32.wrap_i64
    i32.const 1023
    i32.sub
    local.set $exponent
    local.get $exponent
    i32.const 15
    i32.gt_s
    if ;; label = @1
      local.get $sign
      i64.const 31744
      i64.or
      return
    end
    local.get $exponent
    i32.const -25
    i32.lt_s
    if ;; label = @1
      local.get $sign
      return
    end
    local.get $raw
    i64.const 4503599627370495
    i64.and
    i64.const 4503599627370496
    i64.or
    local.set $significand
    local.get $exponent
    i32.const -14
    i32.lt_s
    if ;; label = @1
      local.get $sign
      local.get $significand
      i32.const 28
      local.get $exponent
      i32.sub
      i64.extend_i32_u
      call $bits_round_shift
      i64.or
      return
    end
    local.get $significand
    i64.const 42
    call $bits_round_shift
    local.set $rounded
    local.get $rounded
    i64.const 2048
    i64.eq
    if ;; label = @1
      i64.const 1024
      local.set $rounded
      local.get $exponent
      i32.const 1
      i32.add
      local.set $exponent
    end
    local.get $exponent
    i32.const 15
    i32.gt_s
    if ;; label = @1
      local.get $sign
      i64.const 31744
      i64.or
      return
    end
    local.get $sign
    local.get $exponent
    i32.const 15
    i32.add
    i64.extend_i32_u
    i64.const 10
    i64.shl
    local.get $rounded
    i64.const 1023
    i64.and
    i64.or
    i64.or
  )
  (func $bits_half_decode (;73;) (type 72) (param $raw i64) (result f64)
    (local $exponent i64) (local $fraction i64) (local $sign i64)
    global.get $prof_86
    i64.const 1
    i64.add
    global.set $prof_86
    local.get $raw
    i64.const 10
    i64.shr_u
    i64.const 31
    i64.and
    local.set $exponent
    local.get $raw
    i64.const 1023
    i64.and
    local.set $fraction
    local.get $raw
    i64.const 32768
    i64.and
    i64.const 48
    i64.shl
    local.set $sign
    local.get $exponent
    i64.eqz
    if ;; label = @1
      local.get $fraction
      f64.convert_i64_u
      f64.const 0x1p-24 (;=0.00000005960464477539063;)
    global.get $prof_52
    i64.const 1
    i64.add
    global.set $prof_52
      f64.mul
      local.get $sign
      f64.reinterpret_i64
      f64.copysign
      return
    end
    local.get $exponent
    i64.const 31
    i64.eq
    if ;; label = @1
      local.get $sign
      i64.const 9218868437227405312
      local.get $fraction
      i64.const 42
      i64.shl
      i64.or
      i64.or
      f64.reinterpret_i64
      return
    end
    local.get $sign
    local.get $exponent
    i64.const 1008
    i64.add
    i64.const 52
    i64.shl
    local.get $fraction
    i64.const 42
    i64.shl
    i64.or
    i64.or
    f64.reinterpret_i64
  )
  (func $bits_from_float (;74;) (type 66) (param $value eqref) (param $width i64) (param $little i32) (result eqref)
    (local $number f64) (local $raw i64)
    global.get $prof_87
    i64.const 1
    i64.add
    global.set $prof_87
    local.get $value
    call $float_get
    local.set $number
    local.get $width
    i64.const 16
    i64.eq
    if ;; label = @1
      local.get $number
      call $bits_half_encode
      local.set $raw
    else
      local.get $width
      i64.const 32
      i64.eq
      if ;; label = @2
        local.get $number
        f32.demote_f64
        i32.reinterpret_f32
        i64.extend_i32_u
        local.set $raw
      else
        local.get $width
        i64.const 64
        i64.eq
        if ;; label = @3
          local.get $number
          i64.reinterpret_f64
          local.set $raw
        else
          unreachable
        end
      end
    end
    local.get $width
    i64.const 16
    i64.eq
    local.get $raw
    i64.const 31744
    i64.and
    i64.const 31744
    i64.eq
    i32.and
    local.get $width
    i64.const 32
    i64.eq
    local.get $raw
    i64.const 2139095040
    i64.and
    i64.const 2139095040
    i64.eq
    i32.and
    i32.or
    if ;; label = @1
      call $stop_bits_float
      unreachable
    end
    local.get $raw
    local.get $width
    local.get $little
    call $bits_from_u64
  )
  (func $bits_length (;75;) (type 73) (param $size eqref) (param $unit i64) (result i64)
    global.get $prof_88
    i64.const 1
    i64.add
    global.set $prof_88
    local.get $size
    call $int_sign
    i32.const 0
    i32.lt_s
    if ;; label = @1
      call $stop_bits_negative
      unreachable
    end
    local.get $size
    i64.const 34359738360
    local.get $unit
    i64.div_u
    call $int_from_i64
    call $int_compare
    i32.const 0
    i32.gt_s
    if ;; label = @1
      unreachable
    end
    local.get $size
    call $int_to_i64
    local.get $unit
    i64.mul
  )
  (func $bits_take_length (;76;) (type 73) (param $size eqref) (param $unit i64) (result i64)
    global.get $prof_89
    i64.const 1
    i64.add
    global.set $prof_89
    local.get $size
    call $int_sign
    i32.const 0
    i32.lt_s
    if ;; label = @1
      call $stop_bits_negative
      unreachable
    end
    local.get $size
    i64.const 4611686018427387904
    local.get $unit
    i64.div_u
    call $int_from_i64
    call $int_compare
    i32.const 0
    i32.gt_s
    if ;; label = @1
      i64.const 4611686018427387904
      return
    end
    local.get $size
    call $int_to_i64
    local.get $unit
    i64.mul
  )
  (func $bits_take (;77;) (type 74) (param $value eqref) (param $length i64) (result eqref)
    global.get $prof_90
    i64.const 1
    i64.add
    global.set $prof_90
    local.get $value
    call $bits_len64
    local.get $length
    i64.lt_u
    if ;; label = @1
      call $stop_bits_short
      unreachable
    end
    local.get $value
    i64.const 0
    local.get $length
    call $bits_slice
  )
  (func $bits_whole (;78;) (type 45) (param $value eqref) (result eqref)
    global.get $prof_91
    i64.const 1
    i64.add
    global.set $prof_91
    local.get $value
    call $bits_len64
    i64.const 7
    i64.and
    i64.const 0
    i64.ne
    if ;; label = @1
      call $stop_bits_not_whole
      unreachable
    end
    local.get $value
  )
  (func $bits_float_finite (;79;) (type 75) (param $value eqref) (param $start i64) (param $width i64) (param $little i32) (result i32)
    (local $raw i64)
    global.get $prof_92
    i64.const 1
    i64.add
    global.set $prof_92
    local.get $value
    local.get $start
    local.get $width
    call $bits_has
    i32.eqz
    if ;; label = @1
      i32.const 0
      return
    end
    local.get $width
    i64.const 16
    i64.eq
    local.get $width
    i64.const 32
    i64.eq
    local.get $width
    i64.const 64
    i64.eq
    i32.or
    i32.or
    i32.eqz
    if ;; label = @1
      i32.const 0
      return
    end
    local.get $value
    local.get $start
    local.get $width
    local.get $little
    call $bits_read_u64
    local.set $raw
    local.get $width
    i64.const 16
    i64.eq
    if ;; label = @1
      local.get $raw
      i64.const 31744
      i64.and
      i64.const 31744
      i64.ne
      return
    end
    local.get $width
    i64.const 32
    i64.eq
    if ;; label = @1
      local.get $raw
      i64.const 2139095040
      i64.and
      i64.const 2139095040
      i64.ne
      return
    end
    local.get $raw
    i64.const 9218868437227405312
    i64.and
    i64.const 9218868437227405312
    i64.ne
  )
  (func $bits_read_float (;80;) (type 76) (param $value eqref) (param $start i64) (param $width i64) (param $little i32) (result eqref)
    (local $raw i64)
    global.get $prof_93
    i64.const 1
    i64.add
    global.set $prof_93
    local.get $value
    local.get $start
    local.get $width
    local.get $little
    call $bits_float_finite
    i32.eqz
    if ;; label = @1
      unreachable
    end
    local.get $value
    local.get $start
    local.get $width
    local.get $little
    call $bits_read_u64
    local.set $raw
    local.get $width
    i64.const 16
    i64.eq
    if ;; label = @1
      local.get $raw
      call $bits_half_decode
      call $float_new
      return
    end
    local.get $width
    i64.const 32
    i64.eq
    if ;; label = @1
      local.get $raw
      i32.wrap_i64
      f32.reinterpret_i32
      f64.promote_f32
      call $float_new
      return
    end
    local.get $raw
    f64.reinterpret_i64
    call $float_new
  )
  (func $bits_from_string (;81;) (type 45) (param $value eqref) (result eqref)
    (local $data (ref null $bytes))
    global.get $prof_94
    i64.const 1
    i64.add
    global.set $prof_94
    local.get $value
    global.get $prof_4
    i64.const 1
    i64.add
    global.set $prof_4
    ref.cast (ref $obj)
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
    struct.get $obj 4
    local.set $data
    local.get $data
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
    ref.as_non_null
    i32.const 0
    local.get $data
    array.len
    i64.extend_i32_u
    i64.const 8
    i64.mul
    call $bits_new
  )
  (func $bits_byte (;82;) (type 61) (param $value eqref) (param $start i64) (result i32)
    (local $absolute i64)
    global.get $prof_95
    i64.const 1
    i64.add
    global.set $prof_95
    local.get $value
    local.get $start
    i64.const 8
    call $bits_has
    i32.eqz
    if ;; label = @1
      unreachable
    end
    local.get $value
    call $bits_offset
    local.get $start
    i64.add
    local.set $absolute
    local.get $absolute
    i64.const 7
    i64.and
    i64.eqz
    if ;; label = @1
      local.get $value
      call $bits_data
      local.get $absolute
      i64.const 3
      i64.shr_u
      i32.wrap_i64
      array.get_u $bytes
      return
    end
    local.get $value
    local.get $start
    i64.const 8
    i32.const 0
    call $bits_read_u64
    i32.wrap_i64
  )
  (func $bits_utf8_size (;83;) (type 61) (param $value eqref) (param $start i64) (result i32)
    (local $first i32) (local $second i32) (local $byte i32) (local $count i32) (local $i i32)
    global.get $prof_96
    i64.const 1
    i64.add
    global.set $prof_96
    local.get $value
    local.get $start
    i64.const 8
    call $bits_has
    i32.eqz
    if ;; label = @1
      i32.const 0
      return
    end
    local.get $value
    local.get $start
    call $bits_byte
    local.set $first
    local.get $first
    i32.const 128
    i32.lt_u
    if ;; label = @1
      i32.const 1
      return
    end
    local.get $first
    i32.const 194
    i32.ge_u
    local.get $first
    i32.const 223
    i32.le_u
    i32.and
    if ;; label = @1
      i32.const 2
      local.set $count
    else
      local.get $first
      i32.const 224
      i32.ge_u
      local.get $first
      i32.const 239
      i32.le_u
      i32.and
      if ;; label = @2
        i32.const 3
        local.set $count
      else
        local.get $first
        i32.const 240
        i32.ge_u
        local.get $first
        i32.const 244
        i32.le_u
        i32.and
        if ;; label = @3
          i32.const 4
          local.set $count
        else
          i32.const 0
          return
        end
      end
    end
    local.get $value
    local.get $start
    local.get $count
    i64.extend_i32_u
    i64.const 8
    i64.mul
    call $bits_has
    i32.eqz
    if ;; label = @1
      i32.const 0
      return
    end
    local.get $value
    local.get $start
    i64.const 8
    i64.add
    call $bits_byte
    local.set $second
    local.get $first
    i32.const 224
    i32.eq
    local.get $second
    i32.const 160
    i32.lt_u
    i32.and
    local.get $first
    i32.const 237
    i32.eq
    local.get $second
    i32.const 160
    i32.ge_u
    i32.and
    local.get $first
    i32.const 240
    i32.eq
    local.get $second
    i32.const 144
    i32.lt_u
    i32.and
    local.get $first
    i32.const 244
    i32.eq
    local.get $second
    i32.const 144
    i32.ge_u
    i32.and
    i32.or
    i32.or
    i32.or
    if ;; label = @1
      i32.const 0
      return
    end
    i32.const 1
    local.set $i
    block $done
      loop $continuations
        local.get $i
        local.get $count
        i32.ge_u
        br_if $done
        local.get $value
        local.get $start
        local.get $i
        i64.extend_i32_u
        i64.const 8
        i64.mul
        i64.add
        call $bits_byte
        local.set $byte
        local.get $byte
        i32.const 192
        i32.and
        i32.const 128
        i32.ne
        if ;; label = @3
          i32.const 0
          return
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $continuations
      end
    end
    local.get $count
  )
  (func $bits_codepoint_new (;84;) (type 77) (param $codepoint i32) (result eqref)
    global.get $prof_97
    i64.const 1
    i64.add
    global.set $prof_97
    local.get $codepoint
    call $codepoint_new
  )
  (func $bits_codepoint_get (;85;) (type 36) (param $value eqref) (result i32)
    global.get $prof_98
    i64.const 1
    i64.add
    global.set $prof_98
    local.get $value
    global.get $prof_4
    i64.const 1
    i64.add
    global.set $prof_4
    ref.cast (ref $obj)
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
    struct.get $obj 1
  )
  (func $bits_read_utf8 (;86;) (type 74) (param $value eqref) (param $start i64) (result eqref)
    (local $count i32) (local $codepoint i32) (local $i i32) (local $mask i32)
    global.get $prof_99
    i64.const 1
    i64.add
    global.set $prof_99
    local.get $value
    local.get $start
    call $bits_utf8_size
    local.set $count
    local.get $count
    i32.eqz
    if ;; label = @1
      unreachable
    end
    local.get $count
    i32.const 1
    i32.eq
    if (result i32) ;; label = @1
      i32.const 127
    else
      i32.const 1
      i32.const 7
      local.get $count
      i32.sub
      i32.shl
      i32.const 1
      i32.sub
    end
    local.set $mask
    local.get $value
    local.get $start
    call $bits_byte
    local.get $mask
    i32.and
    local.set $codepoint
    i32.const 1
    local.set $i
    block $done
      loop $read
        local.get $i
        local.get $count
        i32.ge_u
        br_if $done
        local.get $codepoint
        i32.const 6
        i32.shl
        local.get $value
        local.get $start
        local.get $i
        i64.extend_i32_u
        i64.const 8
        i64.mul
        i64.add
        call $bits_byte
        i32.const 63
        i32.and
        i32.or
        local.set $codepoint
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $read
      end
    end
    local.get $codepoint
    call $bits_codepoint_new
  )
  (func $bits_utf16_size (;87;) (type 78) (param $value eqref) (param $start i64) (param $little i32) (result i32)
    (local $first i32) (local $second i32)
    global.get $prof_100
    i64.const 1
    i64.add
    global.set $prof_100
    local.get $value
    local.get $start
    i64.const 16
    call $bits_has
    i32.eqz
    if ;; label = @1
      i32.const 0
      return
    end
    local.get $value
    local.get $start
    i64.const 16
    local.get $little
    call $bits_read_u64
    i32.wrap_i64
    local.set $first
    local.get $first
    i32.const 55296
    i32.ge_u
    local.get $first
    i32.const 56319
    i32.le_u
    i32.and
    if ;; label = @1
      local.get $value
      local.get $start
      i64.const 32
      call $bits_has
      i32.eqz
      if ;; label = @2
        i32.const 0
        return
      end
      local.get $value
      local.get $start
      i64.const 16
      i64.add
      i64.const 16
      local.get $little
      call $bits_read_u64
      i32.wrap_i64
      local.set $second
      local.get $second
      i32.const 56320
      i32.ge_u
      local.get $second
      i32.const 57343
      i32.le_u
      i32.and
      if ;; label = @2
        i32.const 4
        return
      else
        i32.const 0
        return
      end
    end
    local.get $first
    i32.const 56320
    i32.ge_u
    local.get $first
    i32.const 57343
    i32.le_u
    i32.and
    if ;; label = @1
      i32.const 0
      return
    end
    i32.const 2
  )
  (func $bits_read_utf16 (;88;) (type 66) (param $value eqref) (param $start i64) (param $little i32) (result eqref)
    (local $count i32) (local $codepoint i32) (local $second i32)
    global.get $prof_101
    i64.const 1
    i64.add
    global.set $prof_101
    local.get $value
    local.get $start
    local.get $little
    call $bits_utf16_size
    local.set $count
    local.get $count
    i32.eqz
    if ;; label = @1
      unreachable
    end
    local.get $value
    local.get $start
    i64.const 16
    local.get $little
    call $bits_read_u64
    i32.wrap_i64
    local.set $codepoint
    local.get $count
    i32.const 4
    i32.eq
    if ;; label = @1
      local.get $value
      local.get $start
      i64.const 16
      i64.add
      i64.const 16
      local.get $little
      call $bits_read_u64
      i32.wrap_i64
      local.set $second
      i32.const 65536
      local.get $codepoint
      i32.const 55296
      i32.sub
      i32.const 10
      i32.shl
      local.get $second
      i32.const 56320
      i32.sub
      i32.or
      i32.add
      local.set $codepoint
    end
    local.get $codepoint
    call $bits_codepoint_new
  )
  (func $bits_valid_codepoint (;89;) (type 35) (param $codepoint i32) (result i32)
    global.get $prof_102
    i64.const 1
    i64.add
    global.set $prof_102
    local.get $codepoint
    i32.const 1114111
    i32.le_u
    local.get $codepoint
    i32.const 55296
    i32.ge_u
    local.get $codepoint
    i32.const 57343
    i32.le_u
    i32.and
    i32.eqz
    i32.and
  )
  (func $bits_utf32_size (;90;) (type 78) (param $value eqref) (param $start i64) (param $little i32) (result i32)
    global.get $prof_103
    i64.const 1
    i64.add
    global.set $prof_103
    local.get $value
    local.get $start
    i64.const 32
    call $bits_has
    i32.eqz
    if ;; label = @1
      i32.const 0
      return
    end
    local.get $value
    local.get $start
    i64.const 32
    local.get $little
    call $bits_read_u64
    i32.wrap_i64
    call $bits_valid_codepoint
    if (result i32) ;; label = @1
      i32.const 4
    else
      i32.const 0
    end
  )
  (func $bits_read_utf32 (;91;) (type 66) (param $value eqref) (param $start i64) (param $little i32) (result eqref)
    global.get $prof_104
    i64.const 1
    i64.add
    global.set $prof_104
    local.get $value
    local.get $start
    local.get $little
    call $bits_utf32_size
    i32.eqz
    if ;; label = @1
      unreachable
    end
    local.get $value
    local.get $start
    i64.const 32
    local.get $little
    call $bits_read_u64
    i32.wrap_i64
    call $bits_codepoint_new
  )
  (func $bits_read_utf (;92;) (type 79) (param $value eqref) (param $start i64) (param $encoding i32) (param $little i32) (result eqref i64 i32)
    (local $count i32) (local $codepoint eqref)
    global.get $prof_105
    i64.const 1
    i64.add
    global.set $prof_105
    local.get $encoding
    i32.const 8
    i32.eq
    if ;; label = @1
      local.get $value
      local.get $start
      call $bits_utf8_size
      local.set $count
      local.get $count
      if ;; label = @2
        local.get $value
        local.get $start
        call $bits_read_utf8
        local.set $codepoint
      end
    else
      local.get $encoding
      i32.const 16
      i32.eq
      if ;; label = @2
        local.get $value
        local.get $start
        local.get $little
        call $bits_utf16_size
        local.set $count
        local.get $count
        if ;; label = @3
          local.get $value
          local.get $start
          local.get $little
          call $bits_read_utf16
          local.set $codepoint
        end
      else
        local.get $encoding
        i32.const 32
        i32.eq
        if ;; label = @3
          local.get $value
          local.get $start
          local.get $little
          call $bits_utf32_size
          local.set $count
          local.get $count
          if ;; label = @4
            local.get $value
            local.get $start
            local.get $little
            call $bits_read_utf32
            local.set $codepoint
          end
        else
          unreachable
        end
      end
    end
    local.get $codepoint
    local.get $count
    i64.extend_i32_u
    i64.const 8
    i64.mul
    local.get $count
    i32.const 0
    i32.ne
  )
  (func $bits_put_word (;93;) (type 80) (param $data (ref $bytes)) (param $index i32) (param $word i32) (param $little i32)
    global.get $prof_106
    i64.const 1
    i64.add
    global.set $prof_106
    local.get $data
    local.get $index
    local.get $little
    if (result i32) ;; label = @1
      local.get $word
    else
      local.get $word
      i32.const 8
      i32.shr_u
    end
    array.set $bytes
    local.get $data
    local.get $index
    i32.const 1
    i32.add
    local.get $little
    if (result i32) ;; label = @1
      local.get $word
      i32.const 8
      i32.shr_u
    else
      local.get $word
    end
    array.set $bytes
  )
  (func $bits_from_codepoint (;94;) (type 81) (param $value eqref) (param $encoding i32) (param $little i32) (result eqref)
    (local $codepoint i32) (local $count i32) (local $data (ref null $bytes))
    global.get $prof_107
    i64.const 1
    i64.add
    global.set $prof_107
    local.get $value
    call $bits_codepoint_get
    local.set $codepoint
    local.get $codepoint
    call $bits_valid_codepoint
    i32.eqz
    if ;; label = @1
      unreachable
    end
    local.get $encoding
    i32.const 32
    i32.eq
    if ;; label = @1
      local.get $codepoint
      i64.extend_i32_u
      i64.const 32
      local.get $little
      call $bits_from_u64
      return
    end
    local.get $encoding
    i32.const 16
    i32.eq
    if ;; label = @1
      local.get $codepoint
      i32.const 65536
      i32.lt_u
      if (result i32) ;; label = @2
        i32.const 2
      else
        i32.const 4
      end
      local.set $count
      local.get $count
    global.get $prof_62
    i64.const 1
    i64.add
    global.set $prof_62
      array.new_default $bytes
      local.set $data
      local.get $count
      i32.const 2
      i32.eq
      if ;; label = @2
        local.get $data
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
        ref.as_non_null
        i32.const 0
        local.get $codepoint
        local.get $little
        call $bits_put_word
      else
        local.get $codepoint
        i32.const 65536
        i32.sub
        local.set $codepoint
        local.get $data
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
        ref.as_non_null
        i32.const 0
        i32.const 55296
        local.get $codepoint
        i32.const 10
        i32.shr_u
        i32.add
        local.get $little
        call $bits_put_word
        local.get $data
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
        ref.as_non_null
        i32.const 2
        i32.const 56320
        local.get $codepoint
        i32.const 1023
        i32.and
        i32.add
        local.get $little
        call $bits_put_word
      end
      local.get $data
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
      ref.as_non_null
      i32.const 0
      local.get $count
      i64.extend_i32_u
      i64.const 8
      i64.mul
      call $bits_new
      return
    end
    local.get $encoding
    i32.const 8
    i32.ne
    if ;; label = @1
      unreachable
    end
    local.get $codepoint
    i32.const 128
    i32.lt_u
    if (result i32) ;; label = @1
      i32.const 1
    else
      local.get $codepoint
      i32.const 2048
      i32.lt_u
      if (result i32) ;; label = @2
        i32.const 2
      else
        local.get $codepoint
        i32.const 65536
        i32.lt_u
        if (result i32) ;; label = @3
          i32.const 3
        else
          i32.const 4
        end
      end
    end
    local.set $count
    local.get $count
    global.get $prof_62
    i64.const 1
    i64.add
    global.set $prof_62
    array.new_default $bytes
    local.set $data
    local.get $count
    i32.const 1
    i32.eq
    if ;; label = @1
      local.get $data
      i32.const 0
      local.get $codepoint
      array.set $bytes
    else
      local.get $data
      local.get $count
      i32.const 1
      i32.sub
      i32.const 128
      local.get $codepoint
      i32.const 63
      i32.and
      i32.or
      array.set $bytes
      local.get $count
      i32.const 2
      i32.eq
      if ;; label = @2
        local.get $data
        i32.const 0
        i32.const 192
        local.get $codepoint
        i32.const 6
        i32.shr_u
        i32.or
        array.set $bytes
      else
        local.get $data
        local.get $count
        i32.const 2
        i32.sub
        i32.const 128
        local.get $codepoint
        i32.const 6
        i32.shr_u
        i32.const 63
        i32.and
        i32.or
        array.set $bytes
        local.get $count
        i32.const 3
        i32.eq
        if ;; label = @3
          local.get $data
          i32.const 0
          i32.const 224
          local.get $codepoint
          i32.const 12
          i32.shr_u
          i32.or
          array.set $bytes
        else
          local.get $data
          i32.const 1
          i32.const 128
          local.get $codepoint
          i32.const 12
          i32.shr_u
          i32.const 63
          i32.and
          i32.or
          array.set $bytes
          local.get $data
          i32.const 0
          i32.const 240
          local.get $codepoint
          i32.const 18
          i32.shr_u
          i32.or
          array.set $bytes
        end
      end
    end
    local.get $data
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
    ref.as_non_null
    i32.const 0
    local.get $count
    i64.extend_i32_u
    i64.const 8
    i64.mul
    call $bits_new
  )
  (func $bits_from_wide_string (;95;) (type 81) (param $value eqref) (param $encoding i32) (param $little i32) (result eqref)
    (local $input eqref) (local $data (ref null $bytes)) (local $source i64) (local $destination i32) (local $count i32) (local $codepoint i32) (local $capacity i64)
    global.get $prof_108
    i64.const 1
    i64.add
    global.set $prof_108
    local.get $value
    call $bits_from_string
    local.set $input
    local.get $input
    call $bits_len64
    i64.const 8
    i64.div_u
    local.get $encoding
    i32.const 8
    i32.div_u
    i64.extend_i32_u
    i64.mul
    local.set $capacity
    local.get $capacity
    i64.const 4294967295
    i64.gt_u
    if ;; label = @1
      unreachable
    end
    local.get $capacity
    i32.wrap_i64
    global.get $prof_62
    i64.const 1
    i64.add
    global.set $prof_62
    array.new_default $bytes
    local.set $data
    block $done
      loop $encode
        local.get $source
        local.get $input
        call $bits_len64
        i64.ge_u
        br_if $done
        local.get $input
        local.get $source
        call $bits_utf8_size
        local.set $count
        local.get $count
        i32.eqz
        if ;; label = @3
          unreachable
        end
        local.get $input
        local.get $source
        call $bits_read_utf8
        call $bits_codepoint_get
        local.set $codepoint
        local.get $encoding
        i32.const 16
        i32.eq
        if ;; label = @3
          local.get $codepoint
          i32.const 65536
          i32.lt_u
          if ;; label = @4
            local.get $data
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
            ref.as_non_null
            local.get $destination
            local.get $codepoint
            local.get $little
            call $bits_put_word
            local.get $destination
            i32.const 2
            i32.add
            local.set $destination
          else
            local.get $codepoint
            i32.const 65536
            i32.sub
            local.set $codepoint
            local.get $data
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
            ref.as_non_null
            local.get $destination
            i32.const 55296
            local.get $codepoint
            i32.const 10
            i32.shr_u
            i32.add
            local.get $little
            call $bits_put_word
            local.get $data
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
            ref.as_non_null
            local.get $destination
            i32.const 2
            i32.add
            i32.const 56320
            local.get $codepoint
            i32.const 1023
            i32.and
            i32.add
            local.get $little
            call $bits_put_word
            local.get $destination
            i32.const 4
            i32.add
            local.set $destination
          end
        else
          local.get $little
          if ;; label = @4
            local.get $data
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
            ref.as_non_null
            local.get $destination
            local.get $codepoint
            i32.const 1
            call $bits_put_word
            local.get $data
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
            ref.as_non_null
            local.get $destination
            i32.const 2
            i32.add
            local.get $codepoint
            i32.const 16
            i32.shr_u
            i32.const 1
            call $bits_put_word
          else
            local.get $data
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
            ref.as_non_null
            local.get $destination
            local.get $codepoint
            i32.const 16
            i32.shr_u
            i32.const 0
            call $bits_put_word
            local.get $data
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
            ref.as_non_null
            local.get $destination
            i32.const 2
            i32.add
            local.get $codepoint
            i32.const 0
            call $bits_put_word
          end
          local.get $destination
          i32.const 4
          i32.add
          local.set $destination
        end
        local.get $source
        local.get $count
        i64.extend_i32_u
        i64.const 8
        i64.mul
        i64.add
        local.set $source
        br $encode
      end
    end
    local.get $data
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
    ref.as_non_null
    i32.const 0
    local.get $destination
    i64.extend_i32_u
    i64.const 8
    i64.mul
    call $bits_new
  )
  (func $bits_from_utf16_string (;96;) (type 82) (param $value eqref) (param $little i32) (result eqref)
    global.get $prof_109
    i64.const 1
    i64.add
    global.set $prof_109
    local.get $value
    i32.const 16
    local.get $little
    call $bits_from_wide_string
  )
  (func $bits_from_utf32_string (;97;) (type 82) (param $value eqref) (param $little i32) (result eqref)
    global.get $prof_110
    i64.const 1
    i64.add
    global.set $prof_110
    local.get $value
    i32.const 32
    local.get $little
    call $bits_from_wide_string
  )
  (func $bits_from_utf (;98;) (type 83) (param $value eqref) (param $encoding i32) (param $little i32) (param $codepoint i32) (result eqref)
    global.get $prof_111
    i64.const 1
    i64.add
    global.set $prof_111
    local.get $codepoint
    if ;; label = @1
      local.get $value
      local.get $encoding
      local.get $little
      call $bits_from_codepoint
      return
    end
    local.get $encoding
    i32.const 8
    i32.eq
    if ;; label = @1
      local.get $value
      call $bits_from_string
      return
    end
    local.get $encoding
    i32.const 16
    i32.eq
    local.get $encoding
    i32.const 32
    i32.eq
    i32.or
    i32.eqz
    if ;; label = @1
      unreachable
    end
    local.get $value
    local.get $encoding
    local.get $little
    call $bits_from_wide_string
  )
  (func $bits_to_string (;99;) (type 45) (param $value eqref) (result eqref)
    (local $length i64) (local $count i32) (local $data (ref null $bytes)) (local $index i32) (local $position i64) (local $size i32)
    global.get $prof_112
    i64.const 1
    i64.add
    global.set $prof_112
    local.get $value
    call $bits_len64
    local.set $length
    local.get $length
    i64.const 7
    i64.and
    i64.const 0
    i64.ne
    if ;; label = @1
      unreachable
    end
    local.get $length
    i64.const 3
    i64.shr_u
    i32.wrap_i64
    local.set $count
    local.get $count
    global.get $prof_62
    i64.const 1
    i64.add
    global.set $prof_62
    array.new_default $bytes
    local.set $data
    local.get $value
    call $bits_offset
    i64.const 7
    i64.and
    i64.eqz
    if ;; label = @1
      local.get $data
      i32.const 0
      local.get $value
      call $bits_data
      local.get $value
      call $bits_offset
      i64.const 3
      i64.shr_u
      i32.wrap_i64
      local.get $count
      array.copy $bytes $bytes
    else
      block $copy_done
        loop $copy
          local.get $index
          local.get $count
          i32.ge_u
          br_if $copy_done
          local.get $data
          local.get $index
          local.get $value
          local.get $index
          i64.extend_i32_u
          i64.const 8
          i64.mul
          call $bits_byte
          array.set $bytes
          local.get $index
          i32.const 1
          i32.add
          local.set $index
          br $copy
        end
      end
    end
    block $valid
      loop $validate
        local.get $position
        local.get $length
        i64.ge_u
        br_if $valid
        local.get $value
        local.get $position
        call $bits_utf8_size
        local.set $size
        local.get $size
        i32.eqz
        if ;; label = @3
          unreachable
        end
        local.get $position
        local.get $size
        i64.extend_i32_u
        i64.const 8
        i64.mul
        i64.add
        local.set $position
        br $validate
      end
    end
    local.get $data
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
    ref.as_non_null
    call $string_from_bytes
  )
  (func $value_tag (;100;) (type 36) (param $v eqref) (result i32)
    global.get $prof_113
    i64.const 1
    i64.add
    global.set $prof_113
    local.get $v
    ref.is_null
    if ;; label = @1
      i32.const 0
      return
    end
    local.get $v
    global.get $prof_3
    i64.const 1
    i64.add
    global.set $prof_3
    ref.test (ref i31)
    if ;; label = @1
      i32.const 8
      return
    end
    local.get $v
    global.get $prof_3
    i64.const 1
    i64.add
    global.set $prof_3
    ref.test (ref $hostref)
    if ;; label = @1
      i32.const 10
      return
    end
    local.get $v
    global.get $prof_4
    i64.const 1
    i64.add
    global.set $prof_4
    ref.cast (ref $val)
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
    struct.get $val 0
  )
  (func $host_ref_new (;101;) (type 84) (param $r externref) (result eqref)
    global.get $prof_114
    i64.const 1
    i64.add
    global.set $prof_114
    local.get $r
    local.get $r
    call $host_reference_id
    global.get $prof_115
    i64.const 1
    i64.add
    global.set $prof_115
    struct.new $hostref
  )
  (func $host_ref_get (;102;) (type 85) (param $v eqref) (result externref)
    global.get $prof_116
    i64.const 1
    i64.add
    global.set $prof_116
    local.get $v
    global.get $prof_4
    i64.const 1
    i64.add
    global.set $prof_4
    ref.cast (ref $hostref)
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
    struct.get $hostref 0
  )
  (func $host_id (;103;) (type 41) (param $v eqref) (result i64)
    global.get $prof_117
    i64.const 1
    i64.add
    global.set $prof_117
    local.get $v
    global.get $prof_4
    i64.const 1
    i64.add
    global.set $prof_4
    ref.cast (ref $hostref)
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
    struct.get $hostref 1
  )
  (func $codepoint_new (;104;) (type 77) (param $c i32) (result eqref)
    global.get $prof_118
    i64.const 1
    i64.add
    global.set $prof_118
    i32.const 13
    local.get $c
    i64.const 0
    f64.const 0x0p+0 (;=0;)
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
  )
  (func $codepoint_get (;105;) (type 36) (param $v eqref) (result i32)
    global.get $prof_119
    i64.const 1
    i64.add
    global.set $prof_119
    local.get $v
    global.get $prof_4
    i64.const 1
    i64.add
    global.set $prof_4
    ref.cast (ref $obj)
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
    struct.get $obj 1
  )
  (func $bool_new (;106;) (type 77) (param $v i32) (result eqref)
    global.get $prof_120
    i64.const 1
    i64.add
    global.set $prof_120
    local.get $v
    if (result eqref) ;; label = @1
      global.get $value_true
    else
      global.get $value_false
    end
  )
  (func $bool_get (;107;) (type 36) (param $v eqref) (result i32)
    global.get $prof_121
    i64.const 1
    i64.add
    global.set $prof_121
    local.get $v
    global.get $prof_4
    i64.const 1
    i64.add
    global.set $prof_4
    ref.cast (ref $obj)
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
    struct.get $obj 1
  )
  (func $float_new (;108;) (type 53) (param $v f64) (result eqref)
    global.get $prof_122
    i64.const 1
    i64.add
    global.set $prof_122
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
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
  )
  (func $float_get (;109;) (type 52) (param $v eqref) (result f64)
    global.get $prof_123
    i64.const 1
    i64.add
    global.set $prof_123
    local.get $v
    global.get $prof_4
    i64.const 1
    i64.add
    global.set $prof_4
    ref.cast (ref $obj)
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
    struct.get $obj 3
  )
  (func $list_nil (;110;) (type 59) (result eqref)
    global.get $prof_124
    i64.const 1
    i64.add
    global.set $prof_124
    global.get $value_empty_list
  )
  (func $list_cons (;111;) (type $call) (param $head eqref) (param $tail eqref) (result eqref)
    global.get $prof_125
    i64.const 1
    i64.add
    global.set $prof_125
    i32.const 6
    local.get $head
    local.get $tail
    global.get $prof_126
    i64.const 1
    i64.add
    global.set $prof_126
    struct.new $cons
  )
  (func $field (;112;) (type 82) (param $v eqref) (param $index i32) (result eqref)
    global.get $prof_127
    i64.const 1
    i64.add
    global.set $prof_127
    local.get $v
    global.get $prof_4
    i64.const 1
    i64.add
    global.set $prof_4
    ref.cast (ref $obj)
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
    struct.get $obj 5
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
    ref.as_non_null
    local.get $index
    global.get $prof_12
    i64.const 1
    i64.add
    global.set $prof_12
    array.get $values
  )
  (func $record (;113;) (type 86) (param $ctor i32) (param $type i64) (param $fields (ref $values)) (result eqref)
    global.get $prof_128
    i64.const 1
    i64.add
    global.set $prof_128
    i32.const 5
    local.get $ctor
    local.get $type
    f64.const 0x0p+0 (;=0;)
    ref.null $bytes
    local.get $fields
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
  )
  (func $tuple (;114;) (type 87) (param $fields (ref $values)) (result eqref)
    global.get $prof_129
    i64.const 1
    i64.add
    global.set $prof_129
    i32.const 4
    i32.const 0
    i64.const 0
    f64.const 0x0p+0 (;=0;)
    ref.null $bytes
    local.get $fields
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
  )
  (func $list_is_empty (;115;) (type 36) (param $v eqref) (result i32)
    global.get $prof_130
    i64.const 1
    i64.add
    global.set $prof_130
    local.get $v
    global.get $prof_3
    i64.const 1
    i64.add
    global.set $prof_3
    ref.test (ref $cons)
    i32.eqz
  )
  (func $list_head (;116;) (type 45) (param $v eqref) (result eqref)
    global.get $prof_131
    i64.const 1
    i64.add
    global.set $prof_131
    local.get $v
    global.get $prof_4
    i64.const 1
    i64.add
    global.set $prof_4
    ref.cast (ref $cons)
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
    struct.get $cons 1
  )
  (func $list_tail (;117;) (type 45) (param $v eqref) (result eqref)
    global.get $prof_132
    i64.const 1
    i64.add
    global.set $prof_132
    local.get $v
    global.get $prof_4
    i64.const 1
    i64.add
    global.set $prof_4
    ref.cast (ref $cons)
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
    struct.get $cons 2
  )
  (func $list_length (;118;) (type 45) (param $v eqref) (result eqref)
    (local $n i64)
    global.get $prof_133
    i64.const 1
    i64.add
    global.set $prof_133
    block $done
      loop $each
        local.get $v
        call $list_is_empty
        br_if $done
        local.get $n
        i64.const 1
        i64.add
        local.set $n
        local.get $v
        call $list_tail
        local.set $v
        br $each
      end
    end
    local.get $n
    call $int_from_i64
  )
  (func $list_reverse (;119;) (type 45) (param $v eqref) (result eqref)
    (local $out eqref)
    global.get $prof_134
    i64.const 1
    i64.add
    global.set $prof_134
    call $list_nil
    local.set $out
    block $done
      loop $each
        local.get $v
        call $list_is_empty
        br_if $done
        local.get $v
        call $list_head
        local.get $out
        call $list_cons
        local.set $out
        local.get $v
        call $list_tail
        local.set $v
        br $each
      end
    end
    local.get $out
  )
  (func $value_bit (;120;) (type 61) (param $v eqref) (param $at i64) (result i32)
    (local $o (ref null $obj)) (local $p i64)
    global.get $prof_135
    i64.const 1
    i64.add
    global.set $prof_135
    local.get $v
    global.get $prof_4
    i64.const 1
    i64.add
    global.set $prof_4
    ref.cast (ref $obj)
    local.set $o
    local.get $at
    local.get $o
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
    struct.get $obj 1
    i64.extend_i32_u
    i64.add
    local.set $p
    local.get $o
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
    struct.get $obj 4
    local.get $p
    i64.const 3
    i64.shr_u
    i32.wrap_i64
    array.get_u $bytes
    i32.const 7
    local.get $p
    i64.const 7
    i64.and
    i32.wrap_i64
    i32.sub
    i32.shr_u
    i32.const 1
    i32.and
  )
  (func $value_eq (;121;) (type 43) (param $a eqref) (param $b eqref) (result i32)
    global.get $prof_136
    i64.const 1
    i64.add
    global.set $prof_136
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
  (func $equal (;122;) (type 43) (param $a eqref) (param $b eqref) (result i32)
    global.get $prof_137
    i64.const 1
    i64.add
    global.set $prof_137
    local.get $a
    local.get $b
    call $value_eq
  )
  (func $value_identity_kind (;123;) (type 35) (param $kind i32) (result i32)
    global.get $prof_138
    i64.const 1
    i64.add
    global.set $prof_138
    local.get $kind
    i32.const 7
    i32.eq
    local.get $kind
    i32.const 11
    i32.eq
    local.get $kind
    i32.const 12
    i32.eq
    i32.or
    i32.or
  )
  (func $compare_i32 (;124;) (type 88) (param $a i32) (param $b i32) (result i32)
    global.get $prof_139
    i64.const 1
    i64.add
    global.set $prof_139
    local.get $a
    local.get $b
    i32.gt_s
    local.get $a
    local.get $b
    i32.lt_s
    i32.sub
  )
  (func $compare_i64 (;125;) (type 89) (param $a i64) (param $b i64) (result i32)
    global.get $prof_140
    i64.const 1
    i64.add
    global.set $prof_140
    local.get $a
    local.get $b
    i64.gt_s
    local.get $a
    local.get $b
    i64.lt_s
    i32.sub
  )
  (func $both_small (;126;) (type 43) (param $a eqref) (param $b eqref) (result i32)
    global.get $prof_141
    i64.const 1
    i64.add
    global.set $prof_141
    local.get $a
    global.get $prof_3
    i64.const 1
    i64.add
    global.set $prof_3
    ref.test (ref i31)
    local.get $b
    global.get $prof_3
    i64.const 1
    i64.add
    global.set $prof_3
    ref.test (ref i31)
    i32.and
  )
  (func $compare_small (;127;) (type 43) (param $a eqref) (param $b eqref) (result i32)
    global.get $prof_142
    i64.const 1
    i64.add
    global.set $prof_142
    local.get $a
    global.get $prof_4
    i64.const 1
    i64.add
    global.set $prof_4
    ref.cast (ref i31)
    global.get $prof_5
    i64.const 1
    i64.add
    global.set $prof_5
    i31.get_s
    local.get $b
    global.get $prof_4
    i64.const 1
    i64.add
    global.set $prof_4
    ref.cast (ref i31)
    global.get $prof_5
    i64.const 1
    i64.add
    global.set $prof_5
    i31.get_s
    call $compare_i32
  )
  (func $value_compare_i32 (;128;) (type 43) (param $a eqref) (param $b eqref) (result i32)
    (local $ka i32) (local $kb i32) (local $c i32) (local $oa (ref null $obj)) (local $ob (ref null $obj)) (local $xa (ref null $values)) (local $xb (ref null $values)) (local $pending (ref null $compare_frame)) (local $ca (ref null $cons)) (local $cb (ref null $cons)) (local $ha eqref) (local $hb eqref) (local $i i32) (local $na i32) (local $nb i32) (local $bit i64) (local $ba i64) (local $bb i64)
    global.get $prof_143
    i64.const 1
    i64.add
    global.set $prof_143
    loop $again
      i32.const 0
      local.set $c
      block $compared
        local.get $a
        local.get $b
        ref.eq
        br_if $compared
        local.get $a
        call $value_tag
        local.set $ka
        local.get $b
        call $value_tag
        local.set $kb
        local.get $ka
        local.get $kb
        i32.ne
        if ;; label = @3
          local.get $ka
          local.get $kb
          call $compare_i32
          return
        end
        local.get $ka
        i32.eqz
        br_if $compared
        local.get $ka
        i32.const 8
        i32.eq
        if ;; label = @3
          local.get $a
          local.get $b
          call $int_compare
          local.set $c
          br $compared
        end
        local.get $ka
        i32.const 10
        i32.eq
        if ;; label = @3
          local.get $a
          call $host_id
          local.get $b
          call $host_id
          call $compare_i64
          local.set $c
          br $compared
        end
        local.get $ka
        call $value_identity_kind
        if ;; label = @3
          i32.const 1
          return
        end
        local.get $ka
        i32.const 6
        i32.eq
        if ;; label = @3
          local.get $a
    global.get $prof_3
    i64.const 1
    i64.add
    global.set $prof_3
          ref.test (ref $cons)
          local.set $na
          local.get $b
    global.get $prof_3
    i64.const 1
    i64.add
    global.set $prof_3
          ref.test (ref $cons)
          local.set $nb
          local.get $na
          local.get $nb
          i32.and
          i32.eqz
          if ;; label = @4
            local.get $na
            local.get $nb
            call $compare_i32
            local.set $c
            br $compared
          end
          local.get $a
    global.get $prof_4
    i64.const 1
    i64.add
    global.set $prof_4
          ref.cast (ref $cons)
          local.set $ca
          local.get $b
    global.get $prof_4
    i64.const 1
    i64.add
    global.set $prof_4
          ref.cast (ref $cons)
          local.set $cb
          local.get $ca
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
          struct.get $cons 1
          local.set $ha
          local.get $cb
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
          struct.get $cons 1
          local.set $hb
          local.get $ca
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
          struct.get $cons 2
          local.set $a
          local.get $cb
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
          struct.get $cons 2
          local.set $b
          local.get $ha
          local.get $hb
          ref.eq
          br_if $again
          local.get $ha
          local.get $hb
          call $both_small
          if ;; label = @4
            local.get $ha
            local.get $hb
            call $compare_small
            return
          end
          local.get $pending
          ref.null $values
          ref.null $values
          i32.const 0
          local.get $a
          local.get $b
    global.get $prof_144
    i64.const 1
    i64.add
    global.set $prof_144
          struct.new $compare_frame
          local.set $pending
          local.get $ha
          local.set $a
          local.get $hb
          local.set $b
          br $again
        end
        local.get $a
    global.get $prof_4
    i64.const 1
    i64.add
    global.set $prof_4
        ref.cast (ref $obj)
        local.set $oa
        local.get $b
    global.get $prof_4
    i64.const 1
    i64.add
    global.set $prof_4
        ref.cast (ref $obj)
        local.set $ob
        local.get $ka
        i32.const 2
        i32.eq
        if ;; label = @3
          local.get $oa
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
          struct.get $obj 3
          local.get $ob
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
          struct.get $obj 3
          f64.gt
          local.get $oa
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
          struct.get $obj 3
          local.get $ob
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
          struct.get $obj 3
          f64.lt
          i32.sub
          local.set $c
          br $compared
        end
        local.get $ka
        i32.const 3
        i32.eq
        if ;; label = @3
          local.get $a
          local.get $b
          call $string_compare
          local.set $c
          br $compared
        end
        local.get $ka
        i32.const 9
        i32.eq
        if ;; label = @3
          local.get $oa
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
          struct.get $obj 2
          local.set $ba
          local.get $ob
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
          struct.get $obj 2
          local.set $bb
          i64.const 0
          local.set $bit
          block $bits_done
            loop $bits_each
              local.get $bit
              local.get $ba
              i64.ge_u
              local.get $bit
              local.get $bb
              i64.ge_u
              i32.or
              br_if $bits_done
              local.get $a
              local.get $bit
              call $value_bit
              local.get $b
              local.get $bit
              call $value_bit
              call $compare_i32
              local.set $c
              local.get $c
              if ;; label = @6
                local.get $c
                return
              end
              local.get $bit
              i64.const 1
              i64.add
              local.set $bit
              br $bits_each
            end
          end
          local.get $ba
          local.get $bb
          call $compare_i64
          local.set $c
          br $compared
        end
        local.get $oa
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
        struct.get $obj 2
        local.get $ob
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
        struct.get $obj 2
        call $compare_i64
        local.set $c
        local.get $c
        if ;; label = @3
          local.get $c
          return
        end
        local.get $oa
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
        struct.get $obj 1
        local.get $ob
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
        struct.get $obj 1
        call $compare_i32
        local.set $c
        local.get $c
        if ;; label = @3
          local.get $c
          return
        end
        local.get $oa
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
        struct.get $obj 5
        local.set $xa
        local.get $ob
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
        struct.get $obj 5
        local.set $xb
        local.get $xa
        ref.is_null
        local.get $xb
        ref.is_null
        i32.or
        if ;; label = @3
          local.get $xb
          ref.is_null
          local.get $xa
          ref.is_null
          i32.sub
          local.set $c
          br $compared
        end
        local.get $xa
        array.len
        local.set $na
        local.get $xb
        array.len
        local.set $nb
        i32.const 0
        local.set $i
        block $first_found
          loop $first
            local.get $i
            local.get $na
            i32.ge_u
            local.get $i
            local.get $nb
            i32.ge_u
            i32.or
            br_if $first_found
            local.get $xa
            local.get $i
    global.get $prof_12
    i64.const 1
    i64.add
    global.set $prof_12
            array.get $values
            local.get $xb
            local.get $i
    global.get $prof_12
    i64.const 1
    i64.add
    global.set $prof_12
            array.get $values
            ref.eq
            i32.eqz
            br_if $first_found
            local.get $i
            i32.const 1
            i32.add
            local.set $i
            br $first
          end
        end
        local.get $i
        local.get $na
        i32.ge_u
        local.get $i
        local.get $nb
        i32.ge_u
        i32.or
        if ;; label = @3
          local.get $na
          local.get $nb
          call $compare_i32
          local.set $c
          br $compared
        end
        local.get $xa
        local.get $i
    global.get $prof_12
    i64.const 1
    i64.add
    global.set $prof_12
        array.get $values
        local.set $a
        local.get $xb
        local.get $i
    global.get $prof_12
    i64.const 1
    i64.add
    global.set $prof_12
        array.get $values
        local.set $b
        local.get $a
        local.get $b
        call $both_small
        if ;; label = @3
          local.get $a
          local.get $b
          call $compare_small
          return
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        local.get $na
        local.get $nb
        i32.ne
        local.get $i
        local.get $na
        i32.lt_u
        i32.or
        if ;; label = @3
          local.get $pending
          local.get $xa
          local.get $xb
          local.get $i
          ref.null eq
          ref.null eq
    global.get $prof_144
    i64.const 1
    i64.add
    global.set $prof_144
          struct.new $compare_frame
          local.set $pending
        end
        br $again
      end
      local.get $c
      if ;; label = @2
        local.get $c
        return
      end
      loop $resume
        local.get $pending
        ref.is_null
        if ;; label = @3
          i32.const 0
          return
        end
        local.get $pending
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
        struct.get $compare_frame 1
        ref.is_null
        if ;; label = @3
          local.get $pending
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
          struct.get $compare_frame 4
          local.set $a
          local.get $pending
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
          struct.get $compare_frame 5
          local.set $b
          local.get $pending
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
          struct.get $compare_frame 0
          local.set $pending
          br $again
        end
        local.get $pending
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
        struct.get $compare_frame 1
        local.set $xa
        local.get $pending
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
        struct.get $compare_frame 2
        local.set $xb
        local.get $pending
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
        struct.get $compare_frame 3
        local.set $i
        local.get $xa
        array.len
        local.set $na
        local.get $xb
        array.len
        local.set $nb
        block $next_found
          loop $next
            local.get $i
            local.get $na
            i32.ge_u
            local.get $i
            local.get $nb
            i32.ge_u
            i32.or
            br_if $next_found
            local.get $xa
            local.get $i
    global.get $prof_12
    i64.const 1
    i64.add
    global.set $prof_12
            array.get $values
            local.get $xb
            local.get $i
    global.get $prof_12
    i64.const 1
    i64.add
    global.set $prof_12
            array.get $values
            ref.eq
            i32.eqz
            br_if $next_found
            local.get $i
            i32.const 1
            i32.add
            local.set $i
            br $next
          end
        end
        local.get $i
        local.get $na
        i32.ge_u
        local.get $i
        local.get $nb
        i32.ge_u
        i32.or
        if ;; label = @3
          local.get $pending
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
          struct.get $compare_frame 0
          local.set $pending
          local.get $na
          local.get $nb
          call $compare_i32
          local.set $c
          local.get $c
          if ;; label = @4
            local.get $c
            return
          end
          br $resume
        end
        local.get $xa
        local.get $i
    global.get $prof_12
    i64.const 1
    i64.add
    global.set $prof_12
        array.get $values
        local.set $a
        local.get $xb
        local.get $i
    global.get $prof_12
    i64.const 1
    i64.add
    global.set $prof_12
        array.get $values
        local.set $b
        local.get $a
        local.get $b
        call $both_small
        if ;; label = @3
          local.get $a
          local.get $b
          call $compare_small
          return
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        local.get $na
        local.get $nb
        i32.eq
        local.get $i
        local.get $na
        i32.eq
        i32.and
        if ;; label = @3
          local.get $pending
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
          struct.get $compare_frame 0
          local.set $pending
        else
          local.get $pending
          local.get $i
          struct.set $compare_frame 3
        end
        br $again
      end
    end
    unreachable
  )
  (func $value_compare (;129;) (type $call) (param $a eqref) (param $b eqref) (result eqref)
    global.get $prof_145
    i64.const 1
    i64.add
    global.set $prof_145
    local.get $a
    local.get $b
    call $value_compare_i32
    global.get $prof_14
    i64.const 1
    i64.add
    global.set $prof_14
    ref.i31
  )
  (func $hash_mix (;130;) (type 88) (param $hash i32) (param $word i32) (result i32)
    global.get $prof_146
    i64.const 1
    i64.add
    global.set $prof_146
    local.get $hash
    local.get $word
    i32.xor
    i32.const 16777619
    i32.mul
  )
  (func $value_hash_u32 (;131;) (type 36) (param $v eqref) (result i32)
    (local $kind i32) (local $h i32) (local $i i32) (local $o (ref null $obj)) (local $xs (ref null $values)) (local $bs (ref null $bytes)) (local $ls (ref null $limbs)) (local $n i32) (local $word i64) (local $bit i64) (local $stack (ref $values)) (local $sp i32) (local $grown (ref $values))
    global.get $prof_147
    i64.const 1
    i64.add
    global.set $prof_147
    i32.const -2128831035
    local.set $h
    i32.const 16
    global.get $prof_148
    i64.const 1
    i64.add
    global.set $prof_148
    array.new_default $values
    local.set $stack
    local.get $stack
    i32.const 0
    local.get $v
    array.set $values
    i32.const 1
    local.set $sp
    block $done
      loop $walk
        local.get $sp
        i32.eqz
        br_if $done
        local.get $sp
        i32.const 1
        i32.sub
        local.set $sp
        local.get $stack
        local.get $sp
    global.get $prof_12
    i64.const 1
    i64.add
    global.set $prof_12
        array.get $values
        local.set $v
        local.get $v
        call $value_tag
        local.set $kind
        local.get $h
        local.get $kind
        call $hash_mix
        local.set $h
        local.get $kind
        i32.eqz
        br_if $walk
        local.get $kind
        call $value_identity_kind
        br_if $walk
        i32.const 0
        local.set $i
        local.get $v
    global.get $prof_3
    i64.const 1
    i64.add
    global.set $prof_3
        ref.test (ref i31)
        if ;; label = @3
          local.get $v
    global.get $prof_4
    i64.const 1
    i64.add
    global.set $prof_4
          ref.cast (ref i31)
    global.get $prof_5
    i64.const 1
    i64.add
    global.set $prof_5
          i31.get_s
          local.set $n
          local.get $h
          local.get $v
          call $int_sign
          call $hash_mix
          local.set $h
          local.get $n
          i32.eqz
          if ;; label = @4
            local.get $h
            i32.const 0
            call $hash_mix
            local.set $h
          else
            local.get $h
            i32.const 1
            call $hash_mix
            local.set $h
            local.get $n
            i32.const 0
            i32.lt_s
            if ;; label = @5
              i32.const 0
              local.get $n
              i32.sub
              local.set $n
            end
            local.get $h
            local.get $n
            call $hash_mix
            local.set $h
          end
          br $walk
        end
        local.get $kind
        i32.const 8
        i32.eq
        if ;; label = @3
          local.get $h
          local.get $v
          call $int_sign
          call $hash_mix
          local.set $h
          local.get $v
          call $int_limbs
          local.set $ls
          local.get $ls
          array.len
          local.set $n
          local.get $h
          local.get $n
          call $hash_mix
          local.set $h
          block $int_done
            loop $int_each
              local.get $i
              local.get $n
              i32.ge_u
              br_if $int_done
              local.get $h
              local.get $ls
              local.get $i
    global.get $prof_12
    i64.const 1
    i64.add
    global.set $prof_12
              array.get $limbs
              call $hash_mix
              local.set $h
              local.get $i
              i32.const 1
              i32.add
              local.set $i
              br $int_each
            end
          end
          br $walk
        end
        local.get $kind
        i32.const 10
        i32.eq
        if ;; label = @3
          local.get $v
          call $host_id
          local.set $word
          local.get $h
          local.get $word
          i32.wrap_i64
          call $hash_mix
          local.get $word
          i64.const 32
          i64.shr_u
          i32.wrap_i64
          call $hash_mix
          local.set $h
          br $walk
        end
        local.get $kind
        i32.const 6
        i32.eq
        if ;; label = @3
          local.get $h
          i32.const 0
          call $hash_mix
          local.set $h
          local.get $h
          i32.const 0
          call $hash_mix
          i32.const 0
          call $hash_mix
          local.set $h
          local.get $v
          call $list_is_empty
          br_if $walk
          local.get $h
          i32.const 2
          call $hash_mix
          local.set $h
          local.get $sp
          i32.const 2
          i32.add
          local.get $stack
          array.len
          i32.gt_u
          if ;; label = @4
            local.get $stack
            array.len
            i32.const 268435456
            i32.ge_u
            if ;; label = @5
              unreachable
            end
            local.get $stack
            array.len
            i32.const 1
            i32.shl
            i32.const 2
            i32.add
    global.get $prof_148
    i64.const 1
    i64.add
    global.set $prof_148
            array.new_default $values
            local.set $grown
            local.get $grown
            i32.const 0
            local.get $stack
            i32.const 0
            local.get $sp
            array.copy $values $values
            local.get $grown
            local.set $stack
          end
          local.get $stack
          local.get $sp
          local.get $v
          call $list_tail
          array.set $values
          local.get $stack
          local.get $sp
          i32.const 1
          i32.add
          local.get $v
          call $list_head
          array.set $values
          local.get $sp
          i32.const 2
          i32.add
          local.set $sp
          br $walk
        end
        local.get $v
    global.get $prof_4
    i64.const 1
    i64.add
    global.set $prof_4
        ref.cast (ref $obj)
        local.set $o
        local.get $kind
        i32.const 2
        i32.eq
        if ;; label = @3
          i64.const 0
          local.set $word
          local.get $o
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
          struct.get $obj 3
          f64.const 0x0p+0 (;=0;)
          f64.ne
          if ;; label = @4
            local.get $o
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
            struct.get $obj 3
            i64.reinterpret_f64
            local.set $word
          end
          local.get $h
          local.get $word
          i32.wrap_i64
          call $hash_mix
          local.get $word
          i64.const 32
          i64.shr_u
          i32.wrap_i64
          call $hash_mix
          local.set $h
          br $walk
        end
        local.get $kind
        i32.const 3
        i32.eq
        if ;; label = @3
          local.get $o
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
          struct.get $obj 4
          local.set $bs
          local.get $bs
          array.len
          local.set $n
          block $string_done
            loop $string_each
              local.get $i
              local.get $n
              i32.ge_u
              br_if $string_done
              local.get $h
              local.get $bs
              local.get $i
              array.get_u $bytes
              call $hash_mix
              local.set $h
              local.get $i
              i32.const 1
              i32.add
              local.set $i
              br $string_each
            end
          end
          br $walk
        end
        local.get $kind
        i32.const 9
        i32.eq
        if ;; label = @3
          local.get $o
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
          struct.get $obj 2
          local.set $word
          local.get $h
          local.get $word
          i32.wrap_i64
          call $hash_mix
          local.get $word
          i64.const 32
          i64.shr_u
          i32.wrap_i64
          call $hash_mix
          local.set $h
          i64.const 0
          local.set $bit
          block $bits_done
            loop $bits_each
              local.get $bit
              local.get $word
              i64.ge_u
              br_if $bits_done
              local.get $h
              local.get $v
              local.get $bit
              call $value_bit
              call $hash_mix
              local.set $h
              local.get $bit
              i64.const 1
              i64.add
              local.set $bit
              br $bits_each
            end
          end
          br $walk
        end
        local.get $h
        local.get $o
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
        struct.get $obj 1
        call $hash_mix
        local.set $h
        local.get $o
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
        struct.get $obj 2
        local.set $word
        local.get $h
        local.get $word
        i32.wrap_i64
        call $hash_mix
        local.get $word
        i64.const 32
        i64.shr_u
        i32.wrap_i64
        call $hash_mix
        local.set $h
        local.get $o
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
        struct.get $obj 5
        local.set $xs
        local.get $xs
        ref.is_null
        br_if $walk
        local.get $xs
        array.len
        local.set $n
        local.get $h
        local.get $n
        call $hash_mix
        local.set $h
        local.get $sp
        local.get $n
        i32.add
        local.get $stack
        array.len
        i32.gt_u
        if ;; label = @3
          local.get $stack
          array.len
          i32.const 268435456
          i32.ge_u
          if ;; label = @4
            unreachable
          end
          local.get $stack
          array.len
          i32.const 1
          i32.shl
          local.get $n
          i32.add
    global.get $prof_148
    i64.const 1
    i64.add
    global.set $prof_148
          array.new_default $values
          local.set $grown
          local.get $grown
          i32.const 0
          local.get $stack
          i32.const 0
          local.get $sp
          array.copy $values $values
          local.get $grown
          local.set $stack
        end
        local.get $n
        local.set $i
        block $fields_done
          loop $fields_each
            local.get $i
            i32.eqz
            br_if $fields_done
            local.get $i
            i32.const 1
            i32.sub
            local.set $i
            local.get $stack
            local.get $sp
            local.get $xs
            local.get $i
    global.get $prof_12
    i64.const 1
    i64.add
    global.set $prof_12
            array.get $values
            array.set $values
            local.get $sp
            i32.const 1
            i32.add
            local.set $sp
            br $fields_each
          end
        end
        br $walk
      end
    end
    local.get $h
  )
  (func $value_hash (;132;) (type 45) (param $v eqref) (result eqref)
    global.get $prof_149
    i64.const 1
    i64.add
    global.set $prof_149
    local.get $v
    call $value_hash_u32
    i64.extend_i32_u
    call $int_from_i64
  )
  (func $string_bytes (;133;) (type 56) (param $s eqref) (result (ref $bytes))
    global.get $prof_150
    i64.const 1
    i64.add
    global.set $prof_150
    local.get $s
    global.get $prof_4
    i64.const 1
    i64.add
    global.set $prof_4
    ref.cast (ref $obj)
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
    struct.get $obj 4
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
    ref.as_non_null
  )
  (func $string_from_bytes (;134;) (type 51) (param $b (ref $bytes)) (result eqref)
    global.get $prof_151
    i64.const 1
    i64.add
    global.set $prof_151
    i32.const 3
    i32.const 0
    i64.const 0
    f64.const 0x0p+0 (;=0;)
    local.get $b
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
  )
  (func $string_length_bytes (;135;) (type 36) (param $s eqref) (result i32)
    global.get $prof_152
    i64.const 1
    i64.add
    global.set $prof_152
    local.get $s
    call $string_bytes
    array.len
  )
  (func $string_byte_size (;136;) (type 45) (param $s eqref) (result eqref)
    global.get $prof_153
    i64.const 1
    i64.add
    global.set $prof_153
    local.get $s
    call $string_length_bytes
    i64.extend_i32_u
    call $int_from_i64
  )
  (func $utf8_decode_checked (;137;) (type 90) (param $b (ref $bytes)) (param $at i32) (result i32)
    (local $n i32) (local $lead i32) (local $width i32) (local $cp i32) (local $i i32) (local $x i32)
    global.get $prof_154
    i64.const 1
    i64.add
    global.set $prof_154
    local.get $b
    array.len
    local.set $n
    local.get $at
    local.get $n
    i32.ge_u
    if ;; label = @1
      i32.const -1
      return
    end
    local.get $b
    local.get $at
    array.get_u $bytes
    local.set $lead
    local.get $lead
    i32.const 128
    i32.lt_u
    if ;; label = @1
      local.get $lead
      return
    end
    local.get $lead
    i32.const 194
    i32.ge_u
    local.get $lead
    i32.const 223
    i32.le_u
    i32.and
    if ;; label = @1
      i32.const 2
      local.set $width
      local.get $lead
      i32.const 31
      i32.and
      local.set $cp
    else
      local.get $lead
      i32.const 224
      i32.ge_u
      local.get $lead
      i32.const 239
      i32.le_u
      i32.and
      if ;; label = @2
        i32.const 3
        local.set $width
        local.get $lead
        i32.const 15
        i32.and
        local.set $cp
      else
        local.get $lead
        i32.const 240
        i32.ge_u
        local.get $lead
        i32.const 244
        i32.le_u
        i32.and
        if ;; label = @3
          i32.const 4
          local.set $width
          local.get $lead
          i32.const 7
          i32.and
          local.set $cp
        else
          i32.const -1
          return
        end
      end
    end
    local.get $width
    local.get $n
    local.get $at
    i32.sub
    i32.gt_u
    if ;; label = @1
      i32.const -1
      return
    end
    i32.const 1
    local.set $i
    block $done
      loop $more
        local.get $i
        local.get $width
        i32.ge_u
        br_if $done
        local.get $b
        local.get $at
        local.get $i
        i32.add
        array.get_u $bytes
        local.set $x
        local.get $x
        i32.const 192
        i32.and
        i32.const 128
        i32.ne
        if ;; label = @3
          i32.const -1
          return
        end
        local.get $cp
        i32.const 6
        i32.shl
        local.get $x
        i32.const 63
        i32.and
        i32.or
        local.set $cp
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $more
      end
    end
    local.get $width
    i32.const 3
    i32.eq
    local.get $cp
    i32.const 2048
    i32.lt_u
    i32.and
    local.get $width
    i32.const 4
    i32.eq
    local.get $cp
    i32.const 65536
    i32.lt_u
    i32.and
    i32.or
    if ;; label = @1
      i32.const -1
      return
    end
    local.get $cp
    i32.const 1114111
    i32.gt_u
    local.get $cp
    i32.const 55296
    i32.ge_u
    local.get $cp
    i32.const 57343
    i32.le_u
    i32.and
    i32.or
    if ;; label = @1
      i32.const -1
      return
    end
    local.get $cp
  )
  (func $utf8_decode (;138;) (type 90) (param $b (ref $bytes)) (param $at i32) (result i32)
    (local $cp i32)
    global.get $prof_155
    i64.const 1
    i64.add
    global.set $prof_155
    local.get $b
    local.get $at
    call $utf8_decode_checked
    local.set $cp
    local.get $cp
    i32.const 0
    i32.lt_s
    if ;; label = @1
      unreachable
    end
    local.get $cp
  )
  (func $utf8_width (;139;) (type 35) (param $cp i32) (result i32)
    global.get $prof_156
    i64.const 1
    i64.add
    global.set $prof_156
    local.get $cp
    i32.const 1114111
    i32.gt_u
    local.get $cp
    i32.const 55296
    i32.ge_u
    local.get $cp
    i32.const 57343
    i32.le_u
    i32.and
    i32.or
    if ;; label = @1
      unreachable
    end
    local.get $cp
    i32.const 128
    i32.lt_u
    if ;; label = @1
      i32.const 1
      return
    end
    local.get $cp
    i32.const 2048
    i32.lt_u
    if ;; label = @1
      i32.const 2
      return
    end
    local.get $cp
    i32.const 65536
    i32.lt_u
    if ;; label = @1
      i32.const 3
      return
    end
    i32.const 4
  )
  (func $utf8_valid (;140;) (type 91) (param $b (ref $bytes)) (result i32)
    (local $at i32) (local $cp i32)
    global.get $prof_157
    i64.const 1
    i64.add
    global.set $prof_157
    block $done
      loop $more
        local.get $at
        local.get $b
        array.len
        i32.ge_u
        br_if $done
        local.get $b
        local.get $at
        call $utf8_decode_checked
        local.set $cp
        local.get $cp
        i32.const 0
        i32.lt_s
        if ;; label = @3
          i32.const 0
          return
        end
        local.get $at
        local.get $cp
        call $utf8_width
        i32.add
        local.set $at
        br $more
      end
    end
    i32.const 1
  )
  (func $utf8_write (;141;) (type 92) (param $b (ref $bytes)) (param $at i32) (param $cp i32) (result i32)
    (local $w i32) (local $i i32) (local $x i32)
    global.get $prof_158
    i64.const 1
    i64.add
    global.set $prof_158
    local.get $cp
    call $utf8_width
    local.set $w
    local.get $at
    local.get $b
    array.len
    i32.gt_u
    if ;; label = @1
      unreachable
    end
    local.get $w
    local.get $b
    array.len
    local.get $at
    i32.sub
    i32.gt_u
    if ;; label = @1
      unreachable
    end
    local.get $w
    i32.const 1
    i32.eq
    if ;; label = @1
      local.get $b
      local.get $at
      local.get $cp
      array.set $bytes
    else
      local.get $cp
      local.set $x
      local.get $w
      i32.const 1
      i32.sub
      local.set $i
      loop $more
        local.get $b
        local.get $at
        local.get $i
        i32.add
        i32.const 128
        local.get $x
        i32.const 63
        i32.and
        i32.or
        array.set $bytes
        local.get $x
        i32.const 6
        i32.shr_u
        local.set $x
        local.get $i
        i32.const 1
        i32.sub
        local.set $i
        local.get $i
        i32.const 0
        i32.ne
        br_if $more
      end
      local.get $b
      local.get $at
      local.get $x
      local.get $w
      i32.const 2
      i32.eq
      if (result i32) ;; label = @2
        i32.const 192
      else
        local.get $w
        i32.const 3
        i32.eq
        if (result i32) ;; label = @3
          i32.const 224
        else
          i32.const 240
        end
      end
      i32.or
      array.set $bytes
    end
    local.get $at
    local.get $w
    i32.add
  )
  (func $string_from_mem (;142;) (type 93) (param $ptr i32) (param $len i32) (result eqref)
    (local $b (ref $bytes)) (local $i i32)
    global.get $prof_159
    i64.const 1
    i64.add
    global.set $prof_159
    local.get $ptr
    i64.extend_i32_u
    local.get $len
    i64.extend_i32_u
    i64.add
    memory.size
    i64.extend_i32_u
    i64.const 16
    i64.shl
    i64.gt_u
    if ;; label = @1
      unreachable
    end
    local.get $len
    global.get $prof_62
    i64.const 1
    i64.add
    global.set $prof_62
    array.new_default $bytes
    local.set $b
    block $done
      loop $more
        local.get $i
        local.get $len
        i32.ge_u
        br_if $done
        local.get $b
        local.get $i
        local.get $ptr
        local.get $i
        i32.add
        i32.load8_u
        array.set $bytes
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $more
      end
    end
    local.get $b
    call $utf8_valid
    i32.eqz
    if ;; label = @1
      unreachable
    end
    local.get $b
    call $string_from_bytes
  )
  (func $string_to_mem (;143;) (type 36) (param $s eqref) (result i32)
    (local $b (ref $bytes)) (local $p i32) (local $n i32) (local $i i32)
    global.get $prof_160
    i64.const 1
    i64.add
    global.set $prof_160
    local.get $s
    call $string_bytes
    local.set $b
    local.get $b
    array.len
    local.set $n
    local.get $n
    call $alloc
    local.set $p
    block $done
      loop $more
        local.get $i
        local.get $n
        i32.ge_u
        br_if $done
        local.get $p
        local.get $i
        i32.add
        local.get $b
        local.get $i
        array.get_u $bytes
        i32.store8
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $more
      end
    end
    local.get $p
  )
  (func $string_append (;144;) (type $call) (param $a eqref) (param $b eqref) (result eqref)
    (local $x (ref $bytes)) (local $y (ref $bytes)) (local $z (ref $bytes)) (local $nx i32) (local $ny i32) (local $n i64)
    global.get $prof_161
    i64.const 1
    i64.add
    global.set $prof_161
    local.get $a
    call $string_bytes
    local.set $x
    local.get $b
    call $string_bytes
    local.set $y
    local.get $x
    array.len
    local.set $nx
    local.get $y
    array.len
    local.set $ny
    local.get $nx
    i64.extend_i32_u
    local.get $ny
    i64.extend_i32_u
    i64.add
    local.set $n
    local.get $n
    i64.const 4294967295
    i64.gt_u
    if ;; label = @1
      unreachable
    end
    local.get $n
    i32.wrap_i64
    global.get $prof_62
    i64.const 1
    i64.add
    global.set $prof_62
    array.new_default $bytes
    local.set $z
    local.get $z
    i32.const 0
    local.get $x
    i32.const 0
    local.get $nx
    array.copy $bytes $bytes
    local.get $z
    local.get $nx
    local.get $y
    i32.const 0
    local.get $ny
    array.copy $bytes $bytes
    local.get $z
    call $string_from_bytes
  )
  (func $string_compare (;145;) (type 43) (param $a eqref) (param $b eqref) (result i32)
    (local $x (ref $bytes)) (local $y (ref $bytes)) (local $i i32) (local $u i32) (local $v i32)
    global.get $prof_162
    i64.const 1
    i64.add
    global.set $prof_162
    local.get $a
    call $string_bytes
    local.set $x
    local.get $b
    call $string_bytes
    local.set $y
    block $end
      loop $more
        local.get $i
        local.get $x
        array.len
        i32.ge_u
        local.get $i
        local.get $y
        array.len
        i32.ge_u
        i32.or
        br_if $end
        local.get $x
        local.get $i
        array.get_u $bytes
        local.set $u
        local.get $y
        local.get $i
        array.get_u $bytes
        local.set $v
        local.get $u
        local.get $v
        i32.lt_u
        if ;; label = @3
          i32.const -1
          return
        end
        local.get $u
        local.get $v
        i32.gt_u
        if ;; label = @3
          i32.const 1
          return
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $more
      end
    end
    local.get $x
    array.len
    local.get $y
    array.len
    i32.gt_u
    local.get $x
    array.len
    local.get $y
    array.len
    i32.lt_u
    i32.sub
  )
  (func $string_slice_bytes (;146;) (type 81) (param $s eqref) (param $start i32) (param $len i32) (result eqref)
    (local $b (ref $bytes)) (local $out (ref $bytes))
    global.get $prof_163
    i64.const 1
    i64.add
    global.set $prof_163
    local.get $s
    call $string_bytes
    local.set $b
    local.get $start
    local.get $b
    array.len
    i32.gt_u
    if ;; label = @1
      unreachable
    end
    local.get $len
    local.get $b
    array.len
    local.get $start
    i32.sub
    i32.gt_u
    if ;; label = @1
      unreachable
    end
    local.get $len
    global.get $prof_62
    i64.const 1
    i64.add
    global.set $prof_62
    array.new_default $bytes
    local.set $out
    local.get $out
    i32.const 0
    local.get $b
    local.get $start
    local.get $len
    array.copy $bytes $bytes
    local.get $out
    call $utf8_valid
    i32.eqz
    if ;; label = @1
      unreachable
    end
    local.get $out
    call $string_from_bytes
  )
  (func $string_suffix (;147;) (type 82) (param $s eqref) (param $start i32) (result eqref)
    global.get $prof_164
    i64.const 1
    i64.add
    global.set $prof_164
    local.get $s
    local.get $start
    local.get $s
    call $string_length_bytes
    local.get $start
    i32.sub
    call $string_slice_bytes
  )
  (func $string_boundary (;148;) (type 90) (param $b (ref $bytes)) (param $at i32) (result i32)
    global.get $prof_165
    i64.const 1
    i64.add
    global.set $prof_165
    local.get $at
    local.get $b
    array.len
    i32.ge_u
    if ;; label = @1
      local.get $at
      local.get $b
      array.len
      i32.eq
      return
    end
    local.get $b
    local.get $at
    array.get_u $bytes
    i32.const 192
    i32.and
    i32.const 128
    i32.ne
  )
  (func $string_match_at (;149;) (type 94) (param $s (ref $bytes)) (param $needle (ref $bytes)) (param $at i32) (result i32)
    (local $i i32)
    global.get $prof_166
    i64.const 1
    i64.add
    global.set $prof_166
    local.get $at
    local.get $s
    array.len
    i32.gt_u
    if ;; label = @1
      i32.const 0
      return
    end
    local.get $needle
    array.len
    local.get $s
    array.len
    local.get $at
    i32.sub
    i32.gt_u
    if ;; label = @1
      i32.const 0
      return
    end
    block $done
      loop $more
        local.get $i
        local.get $needle
        array.len
        i32.ge_u
        br_if $done
        local.get $needle
        local.get $i
        array.get_u $bytes
        local.get $s
        local.get $at
        local.get $i
        i32.add
        array.get_u $bytes
        i32.ne
        if ;; label = @3
          i32.const 0
          return
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $more
      end
    end
    i32.const 1
  )
  (func $string_starts_with (;150;) (type 43) (param $s eqref) (param $prefix eqref) (result i32)
    global.get $prof_167
    i64.const 1
    i64.add
    global.set $prof_167
    local.get $s
    call $string_bytes
    local.get $prefix
    call $string_bytes
    i32.const 0
    call $string_match_at
  )
  (func $string_ends_with (;151;) (type 43) (param $s eqref) (param $suffix eqref) (result i32)
    (local $a (ref $bytes)) (local $b (ref $bytes))
    global.get $prof_168
    i64.const 1
    i64.add
    global.set $prof_168
    local.get $s
    call $string_bytes
    local.set $a
    local.get $suffix
    call $string_bytes
    local.set $b
    local.get $b
    array.len
    local.get $a
    array.len
    i32.gt_u
    if ;; label = @1
      i32.const 0
      return
    end
    local.get $a
    local.get $b
    local.get $a
    array.len
    local.get $b
    array.len
    i32.sub
    call $string_match_at
  )
  (func $string_index_of (;152;) (type 43) (param $s eqref) (param $needle eqref) (result i32)
    (local $a (ref $bytes)) (local $b (ref $bytes)) (local $at i32) (local $last i32)
    global.get $prof_169
    i64.const 1
    i64.add
    global.set $prof_169
    local.get $s
    call $string_bytes
    local.set $a
    local.get $needle
    call $string_bytes
    local.set $b
    local.get $b
    array.len
    local.get $a
    array.len
    i32.gt_u
    if ;; label = @1
      i32.const -1
      return
    end
    local.get $a
    array.len
    local.get $b
    array.len
    i32.sub
    local.set $last
    loop $more
      local.get $a
      local.get $b
      local.get $at
      call $string_match_at
      if ;; label = @2
        local.get $at
        return
      end
      local.get $at
      local.get $last
      i32.eq
      if ;; label = @2
        i32.const -1
        return
      end
      local.get $at
      i32.const 1
      i32.add
      local.set $at
      br $more
    end
    unreachable
  )
  (func $string_codepoints (;153;) (type 45) (param $s eqref) (result eqref)
    (local $b (ref $bytes)) (local $end i32) (local $at i32) (local $cp i32) (local $out eqref)
    global.get $prof_170
    i64.const 1
    i64.add
    global.set $prof_170
    local.get $s
    call $string_bytes
    local.set $b
    local.get $b
    array.len
    local.set $end
    call $list_nil
    local.set $out
    block $done
      loop $more
        local.get $end
        i32.eqz
        br_if $done
        local.get $end
        i32.const 1
        i32.sub
        local.set $at
        block $lead
          loop $continuations
            local.get $b
            local.get $at
            array.get_u $bytes
            i32.const 192
            i32.and
            i32.const 128
            i32.ne
            br_if $lead
            local.get $at
            i32.eqz
            if ;; label = @5
              unreachable
            end
            local.get $at
            i32.const 1
            i32.sub
            local.set $at
            br $continuations
          end
        end
        local.get $b
        local.get $at
        call $utf8_decode
        local.set $cp
        local.get $at
        local.get $cp
        call $utf8_width
        i32.add
        local.get $end
        i32.ne
        if ;; label = @3
          unreachable
        end
        local.get $cp
        call $codepoint_new
        local.get $out
        call $list_cons
        local.set $out
        local.get $at
        local.set $end
        br $more
      end
    end
    local.get $out
  )
  (func $string_from_codepoints (;154;) (type 45) (param $list eqref) (result eqref)
    (local $cursor eqref) (local $n i64) (local $cp i32) (local $at i32) (local $out (ref $bytes))
    global.get $prof_171
    i64.const 1
    i64.add
    global.set $prof_171
    local.get $list
    local.set $cursor
    block $counted
      loop $count
        local.get $cursor
        call $list_is_empty
        br_if $counted
        local.get $cursor
        call $list_head
        call $codepoint_get
        local.set $cp
        local.get $n
        local.get $cp
        call $utf8_width
        i64.extend_i32_u
        i64.add
        local.set $n
        local.get $n
        i64.const 4294967295
        i64.gt_u
        if ;; label = @3
          unreachable
        end
        local.get $cursor
        call $list_tail
        local.set $cursor
        br $count
      end
    end
    local.get $n
    i32.wrap_i64
    global.get $prof_62
    i64.const 1
    i64.add
    global.set $prof_62
    array.new_default $bytes
    local.set $out
    local.get $list
    local.set $cursor
    block $written
      loop $write
        local.get $cursor
        call $list_is_empty
        br_if $written
        local.get $cursor
        call $list_head
        call $codepoint_get
        local.set $cp
        local.get $out
        local.get $at
        local.get $cp
        call $utf8_write
        local.set $at
        local.get $cursor
        call $list_tail
        local.set $cursor
        br $write
      end
    end
    local.get $out
    call $string_from_bytes
  )
  (func $string_concat (;155;) (type 45) (param $list eqref) (result eqref)
    (local $cursor eqref) (local $n i64) (local $s (ref $bytes)) (local $at i32) (local $out (ref $bytes))
    global.get $prof_172
    i64.const 1
    i64.add
    global.set $prof_172
    local.get $list
    local.set $cursor
    block $counted
      loop $count
        local.get $cursor
        call $list_is_empty
        br_if $counted
        local.get $cursor
        call $list_head
        call $string_bytes
        local.set $s
        local.get $n
        local.get $s
        array.len
        i64.extend_i32_u
        i64.add
        local.set $n
        local.get $n
        i64.const 4294967295
        i64.gt_u
        if ;; label = @3
          unreachable
        end
        local.get $cursor
        call $list_tail
        local.set $cursor
        br $count
      end
    end
    local.get $n
    i32.wrap_i64
    global.get $prof_62
    i64.const 1
    i64.add
    global.set $prof_62
    array.new_default $bytes
    local.set $out
    local.get $list
    local.set $cursor
    block $written
      loop $write
        local.get $cursor
        call $list_is_empty
        br_if $written
        local.get $cursor
        call $list_head
        call $string_bytes
        local.set $s
        local.get $out
        local.get $at
        local.get $s
        i32.const 0
        local.get $s
        array.len
        array.copy $bytes $bytes
        local.get $at
        local.get $s
        array.len
        i32.add
        local.set $at
        local.get $cursor
        call $list_tail
        local.set $cursor
        br $write
      end
    end
    local.get $out
    call $string_from_bytes
  )
  (func $ib_reserve (;156;) (type 95) (param $n i32)
    (local $old (ref $bytes)) (local $need i64) (local $cap i64) (local $new (ref $bytes))
    global.get $prof_173
    i64.const 1
    i64.add
    global.set $prof_173
    global.get $ib
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
    ref.as_non_null
    local.set $old
    global.get $il
    i64.extend_i32_u
    local.get $n
    i64.extend_i32_u
    i64.add
    local.set $need
    local.get $need
    local.get $old
    array.len
    i64.extend_i32_u
    i64.le_u
    if ;; label = @1
      return
    end
    local.get $need
    i64.const 2147483647
    i64.gt_u
    if ;; label = @1
      unreachable
    end
    local.get $old
    array.len
    i64.extend_i32_u
    i64.const 1
    i64.shl
    local.set $cap
    local.get $cap
    i64.const 2147483647
    i64.gt_u
    if ;; label = @1
      i64.const 2147483647
      local.set $cap
    end
    local.get $cap
    local.get $need
    i64.lt_u
    if ;; label = @1
      local.get $need
      local.set $cap
    end
    local.get $cap
    i32.wrap_i64
    global.get $prof_62
    i64.const 1
    i64.add
    global.set $prof_62
    array.new_default $bytes
    local.set $new
    local.get $new
    i32.const 0
    local.get $old
    i32.const 0
    global.get $il
    array.copy $bytes $bytes
    local.get $new
    global.set $ib
  )
  (func $ib_byte (;157;) (type 95) (param $b i32)
    global.get $prof_174
    i64.const 1
    i64.add
    global.set $prof_174
    i32.const 1
    call $ib_reserve
    global.get $ib
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
    ref.as_non_null
    global.get $il
    local.get $b
    array.set $bytes
    global.get $il
    i32.const 1
    i32.add
    global.set $il
  )
  (func $ib_comma (;158;) (type $unpend)
    global.get $prof_175
    i64.const 1
    i64.add
    global.set $prof_175
    i32.const 2
    call $ib_reserve
    global.get $ib
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
    ref.as_non_null
    global.get $il
    i32.const 44
    array.set $bytes
    global.get $ib
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
    ref.as_non_null
    global.get $il
    i32.const 1
    i32.add
    i32.const 32
    array.set $bytes
    global.get $il
    i32.const 2
    i32.add
    global.set $il
  )
  (func $ib_string (;159;) (type 96) (param $s eqref)
    (local $b (ref $bytes)) (local $n i32)
    global.get $prof_176
    i64.const 1
    i64.add
    global.set $prof_176
    local.get $s
    call $string_bytes
    local.set $b
    local.get $b
    array.len
    local.set $n
    local.get $n
    call $ib_reserve
    global.get $ib
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
    ref.as_non_null
    global.get $il
    local.get $b
    i32.const 0
    local.get $n
    array.copy $bytes $bytes
    global.get $il
    local.get $n
    i32.add
    global.set $il
  )
  (func $ib_small_int (;160;) (type 95) (param $v i32)
    (local $u i64) (local $buf (ref $bytes)) (local $start i32) (local $i i32) (local $j i32) (local $t i32)
    global.get $prof_177
    i64.const 1
    i64.add
    global.set $prof_177
    i32.const 12
    call $ib_reserve
    global.get $ib
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
    ref.as_non_null
    local.set $buf
    global.get $il
    local.set $i
    local.get $v
    i64.extend_i32_s
    local.set $u
    local.get $u
    i64.const 0
    i64.lt_s
    if ;; label = @1
      local.get $buf
      local.get $i
      i32.const 45
      array.set $bytes
      local.get $i
      i32.const 1
      i32.add
      local.set $i
      i64.const 0
      local.get $u
      i64.sub
      local.set $u
    end
    local.get $i
    local.set $start
    loop $digits
      local.get $buf
      local.get $i
      i32.const 48
      local.get $u
      i64.const 10
      i64.rem_u
      i32.wrap_i64
      i32.add
      array.set $bytes
      local.get $i
      i32.const 1
      i32.add
      local.set $i
      local.get $u
      i64.const 10
      i64.div_u
      local.set $u
      local.get $u
      i64.const 0
      i64.ne
      br_if $digits
    end
    local.get $i
    global.set $il
    local.get $i
    i32.const 1
    i32.sub
    local.set $j
    block $reversed
      loop $reverse
        local.get $start
        local.get $j
        i32.ge_s
        br_if $reversed
        local.get $buf
        local.get $start
        array.get_u $bytes
        local.set $t
        local.get $buf
        local.get $start
        local.get $buf
        local.get $j
        array.get_u $bytes
        array.set $bytes
        local.get $buf
        local.get $j
        local.get $t
        array.set $bytes
        local.get $start
        i32.const 1
        i32.add
        local.set $start
        local.get $j
        i32.const 1
        i32.sub
        local.set $j
        br $reverse
      end
    end
  )
  (func $ib_quote (;161;) (type 96) (param $s eqref)
    (local $bytes (ref $bytes)) (local $n i32) (local $buf (ref $bytes)) (local $out i32) (local $i i32) (local $b i32) (local $escape i32) (local $control i32)
    global.get $prof_178
    i64.const 1
    i64.add
    global.set $prof_178
    local.get $s
    call $string_bytes
    local.set $bytes
    local.get $bytes
    array.len
    local.set $n
    local.get $n
    i32.const 268435455
    i32.gt_u
    if ;; label = @1
      unreachable
    end
    local.get $n
    i32.const 8
    i32.mul
    i32.const 2
    i32.add
    call $ib_reserve
    global.get $ib
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
    ref.as_non_null
    local.set $buf
    global.get $il
    local.set $out
    local.get $buf
    local.get $out
    i32.const 34
    array.set $bytes
    local.get $out
    i32.const 1
    i32.add
    local.set $out
    block $done
      loop $next
        local.get $i
        local.get $n
        i32.ge_u
        br_if $done
        local.get $bytes
        local.get $i
        array.get_u $bytes
        local.set $b
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        local.get $b
        i32.const 32
        i32.lt_u
        local.get $b
        i32.const 127
        i32.eq
        i32.or
        local.set $control
        i32.const 0
        local.set $escape
        local.get $b
        i32.const 10
        i32.eq
        if ;; label = @3
          i32.const 110
          local.set $escape
        end
        local.get $b
        i32.const 13
        i32.eq
        if ;; label = @3
          i32.const 114
          local.set $escape
        end
        local.get $b
        i32.const 9
        i32.eq
        if ;; label = @3
          i32.const 116
          local.set $escape
        end
        local.get $b
        i32.const 12
        i32.eq
        if ;; label = @3
          i32.const 102
          local.set $escape
        end
        local.get $b
        i32.const 34
        i32.eq
        local.get $b
        i32.const 92
        i32.eq
        i32.or
        if ;; label = @3
          local.get $b
          local.set $escape
        end
        local.get $escape
        if ;; label = @3
          local.get $buf
          local.get $out
          i32.const 92
          array.set $bytes
          local.get $buf
          local.get $out
          i32.const 1
          i32.add
          local.get $escape
          array.set $bytes
          local.get $out
          i32.const 2
          i32.add
          local.set $out
        else
          local.get $b
          i32.const 194
          i32.eq
          local.get $i
          local.get $n
          i32.lt_u
          i32.and
          if ;; label = @4
            local.get $bytes
            local.get $i
            array.get_u $bytes
            i32.const 160
            i32.lt_u
            if ;; label = @5
              local.get $bytes
              local.get $i
              array.get_u $bytes
              local.set $b
              i32.const 1
              local.set $control
              local.get $i
              i32.const 1
              i32.add
              local.set $i
            end
          end
          local.get $control
          if ;; label = @4
            local.get $buf
            local.get $out
            i32.const 92
            array.set $bytes
            local.get $buf
            local.get $out
            i32.const 1
            i32.add
            i32.const 117
            array.set $bytes
            local.get $buf
            local.get $out
            i32.const 2
            i32.add
            i32.const 123
            array.set $bytes
            local.get $buf
            local.get $out
            i32.const 3
            i32.add
            i32.const 48
            array.set $bytes
            local.get $buf
            local.get $out
            i32.const 4
            i32.add
            i32.const 48
            array.set $bytes
            local.get $buf
            local.get $out
            i32.const 5
            i32.add
            local.get $b
            i32.const 4
            i32.shr_u
            call $inspect_hex
            array.set $bytes
            local.get $buf
            local.get $out
            i32.const 6
            i32.add
            local.get $b
            i32.const 15
            i32.and
            call $inspect_hex
            array.set $bytes
            local.get $buf
            local.get $out
            i32.const 7
            i32.add
            i32.const 125
            array.set $bytes
            local.get $out
            i32.const 8
            i32.add
            local.set $out
          else
            local.get $buf
            local.get $out
            local.get $b
            array.set $bytes
            local.get $out
            i32.const 1
            i32.add
            local.set $out
          end
        end
        br $next
      end
    end
    local.get $buf
    local.get $out
    i32.const 34
    array.set $bytes
    local.get $out
    i32.const 1
    i32.add
    global.set $il
  )
  (func $inspect_hex (;162;) (type 35) (param $n i32) (result i32)
    global.get $prof_179
    i64.const 1
    i64.add
    global.set $prof_179
    local.get $n
    local.get $n
    i32.const 10
    i32.lt_u
    if (result i32) ;; label = @1
      i32.const 48
    else
      i32.const 55
    end
    i32.add
  )
  (func $inspect_bits (;163;) (type 45) (param $value eqref) (result eqref)
    (local $obj (ref $obj)) (local $bytes (ref $bytes)) (local $length i64) (local $offset i64) (local $i i64) (local $pos i64) (local $j i32) (local $v i32) (local $out eqref)
    global.get $prof_180
    i64.const 1
    i64.add
    global.set $prof_180
    local.get $value
    global.get $prof_4
    i64.const 1
    i64.add
    global.set $prof_4
    ref.cast (ref $obj)
    local.set $obj
    local.get $obj
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
    struct.get $obj 4
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
    ref.as_non_null
    local.set $bytes
    local.get $obj
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
    struct.get $obj 2
    local.set $length
    local.get $obj
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
    struct.get $obj 1
    i64.extend_i32_u
    local.set $offset
    i32.const 60
    i32.const 60
    global.get $prof_181
    i64.const 1
    i64.add
    global.set $prof_181
    array.new_fixed $bytes 2
    call $string_from_bytes
    call $list_nil
    call $list_cons
    local.set $out
    block $done
      loop $next
        local.get $i
        local.get $length
        i64.ge_u
        br_if $done
        local.get $i
        i64.const 0
        i64.ne
        if ;; label = @3
          i32.const 44
          i32.const 32
    global.get $prof_181
    i64.const 1
    i64.add
    global.set $prof_181
          array.new_fixed $bytes 2
          call $string_from_bytes
          local.get $out
          call $list_cons
          local.set $out
        end
        i32.const 0
        local.set $j
        i32.const 0
        local.set $v
        block $group_done
          loop $group
            local.get $j
            i32.const 8
            i32.eq
            br_if $group_done
            local.get $i
            local.get $length
            i64.ge_u
            br_if $group_done
            local.get $offset
            local.get $i
            i64.add
            local.set $pos
            local.get $v
            i32.const 1
            i32.shl
            local.get $bytes
            local.get $pos
            i64.const 3
            i64.shr_u
            i32.wrap_i64
            array.get_u $bytes
            i32.const 7
            local.get $pos
            i64.const 7
            i64.and
            i32.wrap_i64
            i32.sub
            i32.shr_u
            i32.const 1
            i32.and
            i32.or
            local.set $v
            local.get $i
            i64.const 1
            i64.add
            local.set $i
            local.get $j
            i32.const 1
            i32.add
            local.set $j
            br $group
          end
        end
        local.get $v
        i64.extend_i32_u
        call $int_from_i64
        call $int_to_string
        local.get $out
        call $list_cons
        local.set $out
        local.get $j
        i32.const 8
        i32.lt_u
        if ;; label = @3
          i32.const 58
          i32.const 117
    global.get $prof_181
    i64.const 1
    i64.add
    global.set $prof_181
          array.new_fixed $bytes 2
          call $string_from_bytes
          local.get $out
          call $list_cons
          local.set $out
          local.get $j
          i64.extend_i32_u
          call $int_from_i64
          call $int_to_string
          local.get $out
          call $list_cons
          local.set $out
        end
        br $next
      end
    end
    i32.const 62
    i32.const 62
    global.get $prof_181
    i64.const 1
    i64.add
    global.set $prof_181
    array.new_fixed $bytes 2
    call $string_from_bytes
    local.get $out
    call $list_cons
    call $list_reverse
    call $string_concat
  )
  (func $inspect_function (;164;) (type 77) (param $arity i32) (result eqref)
    (local $n i32) (local $i i32) (local $out eqref)
    global.get $prof_182
    i64.const 1
    i64.add
    global.set $prof_182
    local.get $arity
    local.set $n
    i32.const 47
    i32.const 47
    i32.const 102
    i32.const 110
    i32.const 40
    global.get $prof_183
    i64.const 1
    i64.add
    global.set $prof_183
    array.new_fixed $bytes 5
    call $string_from_bytes
    call $list_nil
    call $list_cons
    local.set $out
    block $done
      loop $next
        local.get $i
        local.get $n
        i32.ge_u
        br_if $done
        local.get $i
        if ;; label = @3
          i32.const 44
          i32.const 32
    global.get $prof_181
    i64.const 1
    i64.add
    global.set $prof_181
          array.new_fixed $bytes 2
          call $string_from_bytes
          local.get $out
          call $list_cons
          local.set $out
        end
        i32.const 97
        local.get $i
        i32.const 26
        i32.rem_u
        i32.add
    global.get $prof_61
    i64.const 1
    i64.add
    global.set $prof_61
        array.new_fixed $bytes 1
        call $string_from_bytes
        local.get $out
        call $list_cons
        local.set $out
        local.get $i
        i32.const 26
        i32.ge_u
        if ;; label = @3
          local.get $i
          i32.const 26
          i32.div_u
          i64.extend_i32_u
          call $int_from_i64
          call $int_to_string
          local.get $out
          call $list_cons
          local.set $out
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $next
      end
    end
    i32.const 41
    i32.const 32
    i32.const 123
    i32.const 32
    i32.const 46
    i32.const 46
    i32.const 46
    i32.const 32
    i32.const 125
    global.get $prof_184
    i64.const 1
    i64.add
    global.set $prof_184
    array.new_fixed $bytes 9
    call $string_from_bytes
    local.get $out
    call $list_cons
    call $list_reverse
    call $string_concat
  )
  (func $constructors_install (;165;) (type 97) (param $names (ref $values))
    global.get $prof_185
    i64.const 1
    i64.add
    global.set $prof_185
    local.get $names
    global.set $constructor_names
  )
  (func $constructor_name (;166;) (type 77) (param $ctor i32) (result eqref)
    (local $names (ref null $values)) (local $index i32)
    global.get $prof_186
    i64.const 1
    i64.add
    global.set $prof_186
    local.get $ctor
    i32.eqz
    if ;; label = @1
      i32.const 79
      i32.const 107
    global.get $prof_181
    i64.const 1
    i64.add
    global.set $prof_181
      array.new_fixed $bytes 2
      call $string_from_bytes
      return
    end
    local.get $ctor
    i32.const 1
    i32.eq
    if ;; label = @1
      i32.const 69
      i32.const 114
      i32.const 114
      i32.const 111
      i32.const 114
    global.get $prof_183
    i64.const 1
    i64.add
    global.set $prof_183
      array.new_fixed $bytes 5
      call $string_from_bytes
      return
    end
    local.get $ctor
    i32.const 2
    i32.eq
    if ;; label = @1
      i32.const 65
      i32.const 115
      i32.const 115
      i32.const 101
      i32.const 114
      i32.const 116
      i32.const 105
      i32.const 111
      i32.const 110
      i32.const 70
      i32.const 97
      i32.const 105
      i32.const 108
      i32.const 117
      i32.const 114
      i32.const 101
    global.get $prof_187
    i64.const 1
    i64.add
    global.set $prof_187
      array.new_fixed $bytes 16
      call $string_from_bytes
      return
    end
    local.get $ctor
    i32.const 3
    i32.eq
    if ;; label = @1
      i32.const 65
      i32.const 115
      i32.const 115
      i32.const 101
      i32.const 114
      i32.const 116
    global.get $prof_188
    i64.const 1
    i64.add
    global.set $prof_188
      array.new_fixed $bytes 6
      call $string_from_bytes
      return
    end
    global.get $constructor_names
    local.set $names
    local.get $ctor
    i32.const 16
    i32.sub
    local.set $index
    local.get $names
    ref.is_null
    local.get $index
    i32.const 0
    i32.lt_s
    i32.or
    if ;; label = @1
      i32.const 47
      i32.const 47
      i32.const 119
      i32.const 97
      i32.const 115
      i32.const 109
      i32.const 40
      i32.const 117
      i32.const 110
      i32.const 107
      i32.const 110
      i32.const 111
      i32.const 119
      i32.const 110
      i32.const 41
    global.get $prof_189
    i64.const 1
    i64.add
    global.set $prof_189
      array.new_fixed $bytes 15
      call $string_from_bytes
      return
    end
    local.get $index
    local.get $names
    array.len
    i32.ge_u
    if ;; label = @1
      i32.const 47
      i32.const 47
      i32.const 119
      i32.const 97
      i32.const 115
      i32.const 109
      i32.const 40
      i32.const 117
      i32.const 110
      i32.const 107
      i32.const 110
      i32.const 111
      i32.const 119
      i32.const 110
      i32.const 41
    global.get $prof_189
    i64.const 1
    i64.add
    global.set $prof_189
      array.new_fixed $bytes 15
      call $string_from_bytes
      return
    end
    local.get $names
    local.get $index
    global.get $prof_12
    i64.const 1
    i64.add
    global.set $prof_12
    array.get $values
  )
  (func $inspect_push (;167;) (type 98) (param $kind i32) (param $value eqref) (param $index i32)
    (local $sp i32) (local $n i32) (local $values (ref $values)) (local $meta (ref $frames))
    global.get $prof_190
    i64.const 1
    i64.add
    global.set $prof_190
    global.get $it_sp
    local.set $sp
    global.get $it_values
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
    ref.as_non_null
    array.len
    local.set $n
    local.get $sp
    local.get $n
    i32.ge_u
    if ;; label = @1
      local.get $n
      i32.const 268435456
      i32.ge_u
      if ;; label = @2
        unreachable
      end
      local.get $n
      i32.const 1
      i32.shl
    global.get $prof_148
    i64.const 1
    i64.add
    global.set $prof_148
      array.new_default $values
      local.set $values
      local.get $n
      i32.const 1
      i32.shl
    global.get $prof_191
    i64.const 1
    i64.add
    global.set $prof_191
      array.new_default $frames
      local.set $meta
      local.get $values
      i32.const 0
      global.get $it_values
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
      ref.as_non_null
      i32.const 0
      local.get $n
      array.copy $values $values
      local.get $meta
      i32.const 0
      global.get $it_meta
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
      ref.as_non_null
      i32.const 0
      local.get $n
      array.copy $frames $frames
      local.get $values
      global.set $it_values
      local.get $meta
      global.set $it_meta
    end
    global.get $it_values
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
    ref.as_non_null
    local.get $sp
    local.get $value
    array.set $values
    global.get $it_meta
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
    ref.as_non_null
    local.get $sp
    local.get $kind
    local.get $index
    i32.const 2
    i32.shl
    i32.or
    array.set $frames
    local.get $sp
    i32.const 1
    i32.add
    global.set $it_sp
  )
  (func $inspect_atom (;168;) (type 82) (param $value eqref) (param $tag i32) (result eqref)
    (local $obj (ref $obj))
    global.get $prof_192
    i64.const 1
    i64.add
    global.set $prof_192
    local.get $tag
    i32.eqz
    if ;; label = @1
      i32.const 78
      i32.const 105
      i32.const 108
    global.get $prof_193
    i64.const 1
    i64.add
    global.set $prof_193
      array.new_fixed $bytes 3
      call $string_from_bytes
      return
    end
    local.get $tag
    i32.const 8
    i32.eq
    if ;; label = @1
      local.get $value
      return_call $int_to_string
    end
    local.get $tag
    i32.const 1
    i32.eq
    if ;; label = @1
      local.get $value
      call $bool_get
      if (result eqref) ;; label = @2
        i32.const 84
        i32.const 114
        i32.const 117
        i32.const 101
    global.get $prof_194
    i64.const 1
    i64.add
    global.set $prof_194
        array.new_fixed $bytes 4
        call $string_from_bytes
      else
        i32.const 70
        i32.const 97
        i32.const 108
        i32.const 115
        i32.const 101
    global.get $prof_183
    i64.const 1
    i64.add
    global.set $prof_183
        array.new_fixed $bytes 5
        call $string_from_bytes
      end
      return
    end
    local.get $tag
    i32.const 2
    i32.eq
    if ;; label = @1
      local.get $value
      return_call $float_to_string
    end
    local.get $tag
    i32.const 9
    i32.eq
    if ;; label = @1
      local.get $value
      return_call $inspect_bits
    end
    local.get $tag
    i32.const 10
    i32.eq
    if ;; label = @1
      i32.const 47
      i32.const 47
      i32.const 119
      i32.const 97
      i32.const 115
      i32.const 109
      i32.const 40
      i32.const 111
      i32.const 112
      i32.const 97
      i32.const 113
      i32.const 117
      i32.const 101
      i32.const 41
    global.get $prof_195
    i64.const 1
    i64.add
    global.set $prof_195
      array.new_fixed $bytes 14
      call $string_from_bytes
      return
    end
    local.get $tag
    i32.const 11
    i32.eq
    if ;; label = @1
      i32.const 47
      i32.const 47
      i32.const 98
      i32.const 111
      i32.const 117
      i32.const 110
      i32.const 100
      i32.const 97
      i32.const 114
      i32.const 121
    global.get $prof_196
    i64.const 1
    i64.add
    global.set $prof_196
      array.new_fixed $bytes 10
      call $string_from_bytes
      return
    end
    local.get $tag
    i32.const 12
    i32.eq
    if ;; label = @1
      i32.const 47
      i32.const 47
      i32.const 99
      i32.const 111
      i32.const 110
      i32.const 116
      i32.const 105
      i32.const 110
      i32.const 117
      i32.const 97
      i32.const 116
      i32.const 105
      i32.const 111
      i32.const 110
    global.get $prof_195
    i64.const 1
    i64.add
    global.set $prof_195
      array.new_fixed $bytes 14
      call $string_from_bytes
      return
    end
    local.get $value
    global.get $prof_4
    i64.const 1
    i64.add
    global.set $prof_4
    ref.cast (ref $obj)
    local.set $obj
    local.get $tag
    i32.const 7
    i32.eq
    if ;; label = @1
      local.get $obj
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
      struct.get $obj 2
      i32.wrap_i64
      return_call $inspect_function
    end
    local.get $tag
    i32.const 13
    i32.eq
    if ;; label = @1
      i32.const 47
      i32.const 47
      i32.const 117
      i32.const 116
      i32.const 102
      i32.const 99
      i32.const 111
      i32.const 100
      i32.const 101
      i32.const 112
      i32.const 111
      i32.const 105
      i32.const 110
      i32.const 116
      i32.const 40
    global.get $prof_189
    i64.const 1
    i64.add
    global.set $prof_189
      array.new_fixed $bytes 15
      call $string_from_bytes
      local.get $value
      call $list_nil
      call $list_cons
      call $string_from_codepoints
      call $string_append
      i32.const 41
    global.get $prof_61
    i64.const 1
    i64.add
    global.set $prof_61
      array.new_fixed $bytes 1
      call $string_from_bytes
      call $string_append
      return
    end
    i32.const 47
    i32.const 47
    i32.const 119
    i32.const 97
    i32.const 115
    i32.const 109
    i32.const 40
    i32.const 117
    i32.const 110
    i32.const 107
    i32.const 110
    i32.const 111
    i32.const 119
    i32.const 110
    i32.const 41
    global.get $prof_189
    i64.const 1
    i64.add
    global.set $prof_189
    array.new_fixed $bytes 15
    call $string_from_bytes
  )
  (func $ib_atom (;169;) (type 99) (param $value eqref) (param $tag i32)
    global.get $prof_197
    i64.const 1
    i64.add
    global.set $prof_197
    local.get $value
    global.get $prof_3
    i64.const 1
    i64.add
    global.set $prof_3
    ref.test (ref i31)
    if ;; label = @1
      local.get $value
    global.get $prof_4
    i64.const 1
    i64.add
    global.set $prof_4
      ref.cast (ref i31)
    global.get $prof_5
    i64.const 1
    i64.add
    global.set $prof_5
      i31.get_s
      return_call $ib_small_int
    end
    local.get $tag
    i32.const 3
    i32.eq
    if ;; label = @1
      local.get $value
      return_call $ib_quote
    end
    local.get $value
    local.get $tag
    call $inspect_atom
    return_call $ib_string
  )
  (func $inspect (;170;) (type 45) (param $value eqref) (result eqref)
    (local $obj (ref $obj)) (local $fields (ref null $values)) (local $out (ref $bytes)) (local $kind i32) (local $index i32) (local $tag i32) (local $sp i32) (local $meta i32)
    global.get $prof_198
    i64.const 1
    i64.add
    global.set $prof_198
    i32.const 64
    global.get $prof_62
    i64.const 1
    i64.add
    global.set $prof_62
    array.new_default $bytes
    global.set $ib
    i32.const 0
    global.set $il
    i32.const 64
    global.get $prof_148
    i64.const 1
    i64.add
    global.set $prof_148
    array.new_default $values
    global.set $it_values
    i32.const 64
    global.get $prof_191
    i64.const 1
    i64.add
    global.set $prof_191
    array.new_default $frames
    global.set $it_meta
    i32.const 0
    global.set $it_sp
    i32.const 0
    local.get $value
    i32.const 0
    call $inspect_push
    block $done
      loop $next
        global.get $it_sp
        local.set $sp
        local.get $sp
        i32.eqz
        br_if $done
        local.get $sp
        i32.const 1
        i32.sub
        local.set $sp
        local.get $sp
        global.set $it_sp
        global.get $it_values
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
        ref.as_non_null
        local.get $sp
    global.get $prof_12
    i64.const 1
    i64.add
    global.set $prof_12
        array.get $values
        local.set $value
        global.get $it_values
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
        ref.as_non_null
        local.get $sp
        ref.null eq
        array.set $values
        global.get $it_meta
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
        ref.as_non_null
        local.get $sp
    global.get $prof_12
    i64.const 1
    i64.add
    global.set $prof_12
        array.get $frames
        local.set $meta
        local.get $meta
        i32.const 3
        i32.and
        local.set $kind
        local.get $meta
        i32.const 2
        i32.shr_u
        local.set $index
        local.get $kind
        i32.const 1
        i32.eq
        if ;; label = @3
          local.get $index
          call $ib_byte
          br $next
        end
        local.get $kind
        i32.const 2
        i32.eq
        if ;; label = @3
          local.get $value
    global.get $prof_4
    i64.const 1
    i64.add
    global.set $prof_4
          ref.cast (ref $obj)
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
          struct.get $obj 5
          local.set $fields
          local.get $fields
          ref.is_null
          if ;; label = @4
            br $next
          end
          local.get $index
          local.get $fields
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
          ref.as_non_null
          array.len
          i32.ge_u
          br_if $next
          local.get $index
          if ;; label = @4
            call $ib_comma
          end
          i32.const 2
          local.get $value
          local.get $index
          i32.const 1
          i32.add
          call $inspect_push
          i32.const 0
          local.get $fields
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
          ref.as_non_null
          local.get $index
    global.get $prof_12
    i64.const 1
    i64.add
    global.set $prof_12
          array.get $values
          i32.const 0
          call $inspect_push
          br $next
        end
        local.get $kind
        i32.const 3
        i32.eq
        if ;; label = @3
          local.get $value
          call $list_is_empty
          if ;; label = @4
            i32.const 93
            call $ib_byte
            br $next
          end
          local.get $index
          if ;; label = @4
            call $ib_comma
          end
          i32.const 3
          local.get $value
          call $list_tail
          i32.const 1
          call $inspect_push
          i32.const 0
          local.get $value
          call $list_head
          i32.const 0
          call $inspect_push
          br $next
        end
        local.get $value
        call $value_tag
        local.set $tag
        local.get $tag
        i32.const 4
        i32.eq
        local.get $tag
        i32.const 5
        i32.eq
        i32.or
        if ;; label = @3
          local.get $value
    global.get $prof_4
    i64.const 1
    i64.add
    global.set $prof_4
          ref.cast (ref $obj)
          local.set $obj
          local.get $obj
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
          struct.get $obj 5
          local.set $fields
          local.get $tag
          i32.const 5
          i32.eq
          if ;; label = @4
            local.get $obj
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
            struct.get $obj 1
            call $constructor_name
            call $ib_string
            local.get $fields
            ref.is_null
            if ;; label = @5
              br $next
            end
            local.get $fields
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
            ref.as_non_null
            array.len
            i32.eqz
            br_if $next
          end
          i32.const 40
          call $ib_byte
          i32.const 1
          ref.null eq
          i32.const 41
          call $inspect_push
          i32.const 2
          local.get $value
          i32.const 0
          call $inspect_push
          br $next
        end
        local.get $tag
        i32.const 6
        i32.eq
        if ;; label = @3
          i32.const 91
          call $ib_byte
          i32.const 3
          local.get $value
          i32.const 0
          call $inspect_push
          br $next
        end
        local.get $value
        local.get $tag
        call $ib_atom
        br $next
      end
    end
    global.get $il
    global.get $prof_62
    i64.const 1
    i64.add
    global.set $prof_62
    array.new_default $bytes
    local.set $out
    local.get $out
    i32.const 0
    global.get $ib
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
    ref.as_non_null
    i32.const 0
    global.get $il
    array.copy $bytes $bytes
    ref.null $bytes
    global.set $ib
    ref.null $values
    global.set $it_values
    ref.null $frames
    global.set $it_meta
    local.get $out
    call $string_from_bytes
  )
  (func $value_inspect (;171;) (type 45) (param $value eqref) (result eqref)
    global.get $prof_199
    i64.const 1
    i64.add
    global.set $prof_199
    local.get $value
    return_call $inspect
  )
  (func $float_to_string (;172;) (type 45) (param $value eqref) (result eqref)
    global.get $prof_200
    i64.const 1
    i64.add
    global.set $prof_200
    local.get $value
    call $float_get
    call $f64_to_string
  )
  (func $f64_to_string (;173;) (type 53) (param $value f64) (result eqref)
    (local $bits i64) (local $fraction i64) (local $mantissa i64) (local $encoded_exp i32) (local $exp i32) (local $negative i32) (local $inclusive i32) (local $k i32) (local $n i32) (local $i i32) (local $digit i32) (local $comparison i32) (local $down i32) (local $up i32) (local $round_up i32) (local $decimal_exp i32) (local $lower i32) (local $upper i32) (local $scientific i32) (local $exponent i32) (local $power i32) (local $r eqref) (local $s eqref) (local $minus eqref) (local $plus eqref) (local $ten eqref) (local $tmp eqref) (local $shift eqref) (local $digits (ref null $bytes)) (local $out (ref null $bytes)) (local $result (ref null $bytes)) (local $at i32)
    global.get $prof_201
    i64.const 1
    i64.add
    global.set $prof_201
    local.get $value
    i64.reinterpret_f64
    local.set $bits
    local.get $bits
    i64.const 63
    i64.shr_u
    i32.wrap_i64
    local.set $negative
    local.get $bits
    i64.const 52
    i64.shr_u
    i32.wrap_i64
    i32.const 2047
    i32.and
    local.set $encoded_exp
    local.get $encoded_exp
    i32.const 2047
    i32.eq
    if ;; label = @1
      unreachable
    end
    local.get $bits
    i64.const 4503599627370495
    i64.and
    local.set $fraction
    i32.const 64
    global.get $prof_62
    i64.const 1
    i64.add
    global.set $prof_62
    array.new_default $bytes
    local.set $out
    local.get $negative
    if ;; label = @1
      local.get $out
      i32.const 0
      i32.const 45
      array.set $bytes
      i32.const 1
      local.set $at
    end
    local.get $value
    f64.const 0x0p+0 (;=0;)
    f64.eq
    if ;; label = @1
      local.get $out
      local.get $at
      i32.const 48
      array.set $bytes
      local.get $out
      local.get $at
      i32.const 1
      i32.add
      i32.const 46
      array.set $bytes
      local.get $out
      local.get $at
      i32.const 2
      i32.add
      i32.const 48
      array.set $bytes
      local.get $at
      i32.const 3
      i32.add
    global.get $prof_62
    i64.const 1
    i64.add
    global.set $prof_62
      array.new_default $bytes
      local.set $result
      local.get $result
      i32.const 0
      local.get $out
      i32.const 0
      local.get $at
      i32.const 3
      i32.add
      array.copy $bytes $bytes
      local.get $result
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
      ref.as_non_null
      call $string_from_bytes
      return
    end
    local.get $encoded_exp
    i32.eqz
    if ;; label = @1
      local.get $fraction
      local.set $mantissa
      local.get $mantissa
      i64.const 1
      i64.shl
      call $int_from_i64
      local.set $r
      i32.const 1
    global.get $prof_14
    i64.const 1
    i64.add
    global.set $prof_14
      ref.i31
      local.set $minus
      i32.const 1
    global.get $prof_14
    i64.const 1
    i64.add
    global.set $prof_14
      ref.i31
      local.set $plus
      i32.const -1075
      local.set $exp
    else
      local.get $fraction
      i64.const 4503599627370496
      i64.or
      local.set $mantissa
      local.get $mantissa
      i64.const 2
      i64.shl
      call $int_from_i64
      local.set $r
      i32.const 2
    global.get $prof_14
    i64.const 1
    i64.add
    global.set $prof_14
      ref.i31
      local.set $plus
      local.get $fraction
      i64.eqz
      local.get $encoded_exp
      i32.const 1
      i32.gt_u
      i32.and
      if (result i32) ;; label = @2
        i32.const 1
      else
        i32.const 2
      end
    global.get $prof_14
    i64.const 1
    i64.add
    global.set $prof_14
      ref.i31
      local.set $minus
      local.get $encoded_exp
      i32.const 1077
      i32.sub
      local.set $exp
    end
    local.get $mantissa
    i64.const 1
    i64.and
    i64.eqz
    local.set $inclusive
    i32.const 1
    global.get $prof_14
    i64.const 1
    i64.add
    global.set $prof_14
    ref.i31
    local.set $s
    local.get $exp
    i32.const 0
    i32.lt_s
    if ;; label = @1
      local.get $s
      i32.const 0
      local.get $exp
      i32.sub
    global.get $prof_14
    i64.const 1
    i64.add
    global.set $prof_14
      ref.i31
      call $int_shl
      local.set $s
    else
      local.get $exp
    global.get $prof_14
    i64.const 1
    i64.add
    global.set $prof_14
      ref.i31
      local.set $shift
      local.get $r
      local.get $shift
      call $int_shl
      local.set $r
      local.get $minus
      local.get $shift
      call $int_shl
      local.set $minus
      local.get $plus
      local.get $shift
      call $int_shl
      local.set $plus
    end
    i32.const 10
    global.get $prof_14
    i64.const 1
    i64.add
    global.set $prof_14
    ref.i31
    local.set $ten
    i32.const 1
    local.set $k
    block $large_done
      loop $large
        local.get $s
        local.get $ten
        call $int_mul
        local.set $tmp
        local.get $r
        local.get $tmp
        call $int_compare
        i32.const 0
        i32.lt_s
        br_if $large_done
        local.get $tmp
        local.set $s
        local.get $k
        i32.const 1
        i32.add
        local.set $k
        br $large
      end
    end
    block $small_done
      loop $small
        local.get $r
        local.get $s
        call $int_compare
        i32.const 0
        i32.ge_s
        br_if $small_done
        local.get $r
        local.get $ten
        call $int_mul
        local.set $r
        local.get $minus
        local.get $ten
        call $int_mul
        local.set $minus
        local.get $plus
        local.get $ten
        call $int_mul
        local.set $plus
        local.get $k
        i32.const 1
        i32.sub
        local.set $k
        br $small
      end
    end
    i32.const 32
    global.get $prof_62
    i64.const 1
    i64.add
    global.set $prof_62
    array.new_default $bytes
    local.set $digits
    block $digits_done
      loop $generate
        local.get $n
        i32.const 32
        i32.ge_u
        if ;; label = @3
          unreachable
        end
        local.get $r
        local.get $s
        call $int_div
        call $int_to_i64
        i32.wrap_i64
        local.set $digit
        local.get $r
        local.get $s
        call $int_rem
        local.set $r
        local.get $digits
        local.get $n
        local.get $digit
        array.set $bytes
        local.get $n
        i32.const 1
        i32.add
        local.set $n
        local.get $r
        local.get $minus
        call $int_compare
        local.set $comparison
        local.get $comparison
        i32.const 0
        i32.lt_s
        local.get $inclusive
        local.get $comparison
        i32.eqz
        i32.and
        i32.or
        local.set $down
        local.get $r
        local.get $plus
        call $int_add
        local.get $s
        call $int_compare
        local.set $comparison
        local.get $comparison
        i32.const 0
        i32.gt_s
        local.get $inclusive
        local.get $comparison
        i32.eqz
        i32.and
        i32.or
        local.set $up
        local.get $down
        local.get $up
        i32.or
        br_if $digits_done
        local.get $r
        local.get $ten
        call $int_mul
        local.set $r
        local.get $minus
        local.get $ten
        call $int_mul
        local.set $minus
        local.get $plus
        local.get $ten
        call $int_mul
        local.set $plus
        br $generate
      end
    end
    local.get $r
    local.get $r
    call $int_add
    local.get $s
    call $int_compare
    local.set $comparison
    local.get $up
    local.get $down
    i32.eqz
    local.get $comparison
    i32.const 0
    i32.gt_s
    local.get $comparison
    i32.eqz
    local.get $digit
    i32.const 1
    i32.and
    i32.and
    i32.or
    i32.or
    i32.and
    local.set $round_up
    local.get $round_up
    if ;; label = @1
      local.get $n
      local.set $i
      block $rounded
        loop $carry
          local.get $i
          i32.const 1
          i32.sub
          local.set $i
          local.get $digits
          local.get $i
          array.get_u $bytes
          local.set $digit
          local.get $digit
          i32.const 9
          i32.lt_u
          if ;; label = @4
            local.get $digits
            local.get $i
            local.get $digit
            i32.const 1
            i32.add
            array.set $bytes
            br $rounded
          end
          local.get $digits
          local.get $i
          i32.const 0
          array.set $bytes
          local.get $i
          i32.eqz
          if ;; label = @4
            local.get $digits
            i32.const 0
            i32.const 1
            array.set $bytes
            local.get $k
            i32.const 1
            i32.add
            local.set $k
            br $rounded
          end
          br $carry
        end
      end
    end
    block $trim_done
      loop $trim
        local.get $n
        i32.const 1
        i32.le_u
        br_if $trim_done
        local.get $digits
        local.get $n
        i32.const 1
        i32.sub
        array.get_u $bytes
        br_if $trim_done
        local.get $n
        i32.const 1
        i32.sub
        local.set $n
        br $trim
      end
    end
    local.get $k
    local.get $n
    i32.sub
    local.set $decimal_exp
    local.get $n
    i32.const 1
    i32.eq
    if (result i32) ;; label = @1
      i32.const -4
    else
      i32.const -2
      local.get $n
      i32.sub
    end
    local.set $lower
    local.get $n
    i32.const 1
    i32.eq
    local.get $k
    i32.const 11
    i32.ge_s
    i32.or
    if (result i32) ;; label = @1
      i32.const 2
    else
      i32.const 1
    end
    local.set $upper
    local.get $decimal_exp
    local.get $lower
    i32.lt_s
    local.get $decimal_exp
    local.get $upper
    i32.gt_s
    i32.or
    local.get $decimal_exp
    i32.const 0
    i32.ge_s
    local.get $value
    f64.abs
    f64.const 0x1p+53 (;=9007199254740992;)
    f64.ge
    i32.and
    i32.or
    local.set $scientific
    local.get $scientific
    if ;; label = @1
      local.get $out
      local.get $at
      local.get $digits
      i32.const 0
      array.get_u $bytes
      i32.const 48
      i32.add
      array.set $bytes
      local.get $at
      i32.const 1
      i32.add
      local.set $at
      local.get $out
      local.get $at
      i32.const 46
      array.set $bytes
      local.get $at
      i32.const 1
      i32.add
      local.set $at
      i32.const 1
      local.set $i
      block $fraction_done
        loop $fraction_digits
          local.get $i
          local.get $n
          i32.ge_u
          br_if $fraction_done
          local.get $out
          local.get $at
          local.get $digits
          local.get $i
          array.get_u $bytes
          i32.const 48
          i32.add
          array.set $bytes
          local.get $at
          i32.const 1
          i32.add
          local.set $at
          local.get $i
          i32.const 1
          i32.add
          local.set $i
          br $fraction_digits
        end
      end
      local.get $n
      i32.const 1
      i32.eq
      if ;; label = @2
        local.get $out
        local.get $at
        i32.const 48
        array.set $bytes
        local.get $at
        i32.const 1
        i32.add
        local.set $at
      end
      local.get $out
      local.get $at
      i32.const 101
      array.set $bytes
      local.get $at
      i32.const 1
      i32.add
      local.set $at
      local.get $k
      i32.const 1
      i32.sub
      local.set $exponent
      local.get $exponent
      i32.const 0
      i32.lt_s
      if ;; label = @2
        local.get $out
        local.get $at
        i32.const 45
        array.set $bytes
        local.get $at
        i32.const 1
        i32.add
        local.set $at
        i32.const 0
        local.get $exponent
        i32.sub
        local.set $exponent
      end
      local.get $exponent
      i32.const 100
      i32.ge_u
      if (result i32) ;; label = @2
        i32.const 100
      else
        local.get $exponent
        i32.const 10
        i32.ge_u
        if (result i32) ;; label = @3
          i32.const 10
        else
          i32.const 1
        end
      end
      local.set $power
      loop $exponent_digits
        local.get $out
        local.get $at
        local.get $exponent
        local.get $power
        i32.div_u
        i32.const 48
        i32.add
        array.set $bytes
        local.get $at
        i32.const 1
        i32.add
        local.set $at
        local.get $exponent
        local.get $power
        i32.rem_u
        local.set $exponent
        local.get $power
        i32.const 10
        i32.div_u
        local.set $power
        local.get $power
        br_if $exponent_digits
      end
    else
      local.get $k
      i32.const 0
      i32.le_s
      if ;; label = @2
        local.get $out
        local.get $at
        i32.const 48
        array.set $bytes
        local.get $out
        local.get $at
        i32.const 1
        i32.add
        i32.const 46
        array.set $bytes
        local.get $at
        i32.const 2
        i32.add
        local.set $at
        local.get $k
        local.set $i
        block $zeroes_done
          loop $zeroes
            local.get $i
            i32.const 0
            i32.ge_s
            br_if $zeroes_done
            local.get $out
            local.get $at
            i32.const 48
            array.set $bytes
            local.get $at
            i32.const 1
            i32.add
            local.set $at
            local.get $i
            i32.const 1
            i32.add
            local.set $i
            br $zeroes
          end
        end
      end
      i32.const 0
      local.set $i
      block $fixed_done
        loop $fixed_digits
          local.get $i
          local.get $n
          i32.ge_u
          br_if $fixed_done
          local.get $k
          i32.const 0
          i32.gt_s
          local.get $i
          local.get $k
          i32.eq
          i32.and
          if ;; label = @4
            local.get $out
            local.get $at
            i32.const 46
            array.set $bytes
            local.get $at
            i32.const 1
            i32.add
            local.set $at
          end
          local.get $out
          local.get $at
          local.get $digits
          local.get $i
          array.get_u $bytes
          i32.const 48
          i32.add
          array.set $bytes
          local.get $at
          i32.const 1
          i32.add
          local.set $at
          local.get $i
          i32.const 1
          i32.add
          local.set $i
          br $fixed_digits
        end
      end
      local.get $k
      local.get $n
      i32.ge_s
      if ;; label = @2
        block $integer_zeroes_done
          loop $integer_zeroes
            local.get $i
            local.get $k
            i32.ge_s
            br_if $integer_zeroes_done
            local.get $out
            local.get $at
            i32.const 48
            array.set $bytes
            local.get $at
            i32.const 1
            i32.add
            local.set $at
            local.get $i
            i32.const 1
            i32.add
            local.set $i
            br $integer_zeroes
          end
        end
        local.get $out
        local.get $at
        i32.const 46
        array.set $bytes
        local.get $out
        local.get $at
        i32.const 1
        i32.add
        i32.const 48
        array.set $bytes
        local.get $at
        i32.const 2
        i32.add
        local.set $at
      end
    end
    local.get $at
    global.get $prof_62
    i64.const 1
    i64.add
    global.set $prof_62
    array.new_default $bytes
    local.set $result
    local.get $result
    i32.const 0
    local.get $out
    i32.const 0
    local.get $at
    array.copy $bytes $bytes
    local.get $result
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
    ref.as_non_null
    call $string_from_bytes
  )
  (func $sites_install (;174;) (type 100) (param $table (ref $sites)) (param $modules (ref $values))
    global.get $prof_202
    i64.const 1
    i64.add
    global.set $prof_202
    local.get $table
    global.set $site_table
    local.get $modules
    global.set $site_modules
  )
  (func $shadow_grow (;175;) (type $unpend)
    (local $old (ref $frames)) (local $new (ref $frames)) (local $size i32)
    global.get $prof_203
    i64.const 1
    i64.add
    global.set $prof_203
    global.get $shadow
    local.set $old
    local.get $old
    array.len
    local.set $size
    local.get $size
    i32.const 268435456
    i32.ge_u
    if ;; label = @1
      unreachable
    end
    local.get $size
    i32.const 1
    i32.shl
    global.get $prof_191
    i64.const 1
    i64.add
    global.set $prof_191
    array.new_default $frames
    local.set $new
    local.get $new
    i32.const 0
    local.get $old
    i32.const 0
    local.get $size
    array.copy $frames $frames
    local.get $new
    global.set $shadow
  )
  (func $shadow_enter (;176;) (type 95) (param $site i32)
    (local $sp i32)
    global.get $prof_204
    i64.const 1
    i64.add
    global.set $prof_204
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
  (func $shadow_leave (;177;) (type $unpend)
    global.get $prof_205
    i64.const 1
    i64.add
    global.set $prof_205
    global.get $sp
    i32.const 1
    i32.sub
    global.set $sp
  )
  (func $shadow_enter_call (;178;) (type $unpend)
    global.get $prof_206
    i64.const 1
    i64.add
    global.set $prof_206
    global.get $tail
    call $shadow_enter
    i32.const -1
    global.set $tail
  )
  (func $shadow_leave_tail (;179;) (type 95) (param $site i32)
    (local $sp i32)
    global.get $prof_207
    i64.const 1
    i64.add
    global.set $prof_207
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
    global.get $prof_12
    i64.const 1
    i64.add
    global.set $prof_12
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
  (func $shadow_site (;180;) (type 95) (param $site i32)
    global.get $prof_208
    i64.const 1
    i64.add
    global.set $prof_208
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
  (func $shadow_top (;181;) (type 101) (result i32)
    global.get $prof_209
    i64.const 1
    i64.add
    global.set $prof_209
    global.get $shadow
    global.get $sp
    i32.const 1
    i32.sub
    global.get $prof_12
    i64.const 1
    i64.add
    global.set $prof_12
    array.get $frames
  )
  (func $shadow_depth (;182;) (type 101) (result i32)
    global.get $prof_210
    i64.const 1
    i64.add
    global.set $prof_210
    global.get $sp
  )
  (func $shadow_restore (;183;) (type 95) (param $depth i32)
    global.get $prof_211
    i64.const 1
    i64.add
    global.set $prof_211
    local.get $depth
    global.set $sp
  )
  (func $site_text (;184;) (type 77) (param $site i32) (result eqref)
    (local $table (ref null $sites)) (local $modules (ref null $values)) (local $at i32) (local $module i32)
    global.get $prof_212
    i64.const 1
    i64.add
    global.set $prof_212
    global.get $site_table
    local.set $table
    global.get $site_modules
    local.set $modules
    local.get $table
    ref.is_null
    local.get $modules
    ref.is_null
    i32.or
    if ;; label = @1
      ref.null eq
      return
    end
    local.get $site
    i32.const 0
    i32.lt_s
    if ;; label = @1
      ref.null eq
      return
    end
    local.get $site
    local.get $table
    array.len
    i32.const 3
    i32.div_u
    i32.ge_u
    if ;; label = @1
      ref.null eq
      return
    end
    local.get $site
    i32.const 3
    i32.mul
    local.set $at
    local.get $table
    local.get $at
    global.get $prof_12
    i64.const 1
    i64.add
    global.set $prof_12
    array.get $sites
    local.set $module
    local.get $module
    local.get $modules
    array.len
    i32.ge_u
    if ;; label = @1
      ref.null eq
      return
    end
    local.get $site
    return_call $site_line
  )
  (func $site_line (;185;) (type 77) (param $site i32) (result eqref)
    (local $table (ref $sites)) (local $at i32)
    global.get $prof_213
    i64.const 1
    i64.add
    global.set $prof_213
    global.get $site_table
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
    ref.as_non_null
    local.set $table
    local.get $site
    i32.const 3
    i32.mul
    local.set $at
    global.get $site_modules
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
    ref.as_non_null
    local.get $table
    local.get $at
    global.get $prof_12
    i64.const 1
    i64.add
    global.set $prof_12
    array.get $sites
    global.get $prof_12
    i64.const 1
    i64.add
    global.set $prof_12
    array.get $values
    i32.const 58
    global.get $prof_61
    i64.const 1
    i64.add
    global.set $prof_61
    array.new_fixed $bytes 1
    call $string_from_bytes
    local.get $table
    local.get $at
    i32.const 1
    i32.add
    global.get $prof_12
    i64.const 1
    i64.add
    global.set $prof_12
    array.get $sites
    i64.extend_i32_u
    call $int_from_i64
    call $int_to_string
    i32.const 58
    global.get $prof_61
    i64.const 1
    i64.add
    global.set $prof_61
    array.new_fixed $bytes 1
    call $string_from_bytes
    local.get $table
    local.get $at
    i32.const 2
    i32.add
    global.get $prof_12
    i64.const 1
    i64.add
    global.set $prof_12
    array.get $sites
    i64.extend_i32_u
    call $int_from_i64
    call $int_to_string
    call $list_nil
    call $list_cons
    call $list_cons
    call $list_cons
    call $list_cons
    call $list_cons
    call $string_concat
  )
  (func $stop_add_site (;186;) (type 82) (param $parts eqref) (param $site i32) (result eqref)
    (local $line eqref)
    global.get $prof_214
    i64.const 1
    i64.add
    global.set $prof_214
    local.get $site
    call $site_text
    local.set $line
    local.get $line
    ref.is_null
    if ;; label = @1
      local.get $parts
      return
    end
    local.get $line
    i32.const 10
    global.get $prof_61
    i64.const 1
    i64.add
    global.set $prof_61
    array.new_fixed $bytes 1
    call $string_from_bytes
    local.get $parts
    call $list_cons
    call $list_cons
  )
  (func $stop_send (;187;) (type 96) (param $message eqref)
    (local $length i32)
    global.get $prof_215
    i64.const 1
    i64.add
    global.set $prof_215
    local.get $message
    call $string_length_bytes
    local.set $length
    local.get $message
    call $string_to_mem
    local.get $length
    call $host_panic
    unreachable
  )
  (func $stop (;188;) (type 96) (param $reason eqref)
    (local $parts eqref) (local $i i32) (local $site i32) (local $printed i32)
    global.get $prof_216
    i64.const 1
    i64.add
    global.set $prof_216
    global.get $stopping
    if ;; label = @1
      unreachable
    end
    i32.const 1
    global.set $stopping
    global.get $stop_unpend
    ref.is_null
    i32.eqz
    if ;; label = @1
      global.get $stop_unpend
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
      ref.as_non_null
    global.get $prof_217
    i64.const 1
    i64.add
    global.set $prof_217
      call_ref $unpend
    end
    local.get $reason
    call $list_nil
    call $list_cons
    local.set $parts
    global.get $tail
    i32.const -2
    i32.le_s
    if ;; label = @1
      local.get $parts
      i32.const -2
      global.get $tail
      i32.sub
      call $stop_add_site
      local.set $parts
      i32.const 1
      local.set $printed
    end
    global.get $sp
    local.set $i
    local.get $i
    global.get $shadow
    array.len
    i32.gt_u
    if ;; label = @1
      global.get $shadow
      array.len
      local.set $i
    end
    block $done
      loop $each
        local.get $i
        i32.eqz
        br_if $done
        local.get $i
        i32.const 1
        i32.sub
        local.set $i
        global.get $shadow
        local.get $i
    global.get $prof_12
    i64.const 1
    i64.add
    global.set $prof_12
        array.get $frames
        local.set $site
        local.get $site
        i32.const -2
        i32.le_s
        if ;; label = @3
          local.get $printed
          if (result i32) ;; label = @4
            i32.const -1
          else
            i32.const -2
            local.get $site
            i32.sub
          end
          local.set $site
        end
        local.get $site
        i32.const 0
        i32.ge_s
        if ;; label = @3
          local.get $parts
          local.get $site
          call $stop_add_site
          local.set $parts
          i32.const 1
          local.set $printed
        end
        br $each
      end
    end
    local.get $parts
    call $list_reverse
    call $string_concat
    call $stop_send
    unreachable
  )
  (func $stop_at (;189;) (type 99) (param $reason eqref) (param $site i32)
    global.get $prof_218
    i64.const 1
    i64.add
    global.set $prof_218
    global.get $stopping
    if ;; label = @1
      unreachable
    end
    i32.const 1
    global.set $stopping
    local.get $reason
    call $list_nil
    call $list_cons
    local.get $site
    call $stop_add_site
    call $list_reverse
    call $string_concat
    call $stop_send
    unreachable
  )
  (func $panic (;190;) (type 96) (param $message eqref)
    global.get $prof_219
    i64.const 1
    i64.add
    global.set $prof_219
    i32.const 112
    i32.const 97
    i32.const 110
    i32.const 105
    i32.const 99
    i32.const 58
    i32.const 32
    global.get $prof_220
    i64.const 1
    i64.add
    global.set $prof_220
    array.new_fixed $bytes 7
    call $string_from_bytes
    local.get $message
    call $string_append
    call $stop
    unreachable
  )
  (func $stop_assertion (;191;) (type 96) (param $message eqref)
    global.get $prof_221
    i64.const 1
    i64.add
    global.set $prof_221
    i32.const 97
    i32.const 115
    i32.const 115
    i32.const 101
    i32.const 114
    i32.const 116
    i32.const 105
    i32.const 111
    i32.const 110
    i32.const 32
    i32.const 102
    i32.const 97
    i32.const 105
    i32.const 108
    i32.const 101
    i32.const 100
    i32.const 58
    i32.const 32
    global.get $prof_222
    i64.const 1
    i64.add
    global.set $prof_222
    array.new_fixed $bytes 18
    call $string_from_bytes
    local.get $message
    call $string_append
    call $stop
    unreachable
  )
  (func $stop_error (;192;) (type 99) (param $error eqref) (param $site i32)
    global.get $prof_223
    i64.const 1
    i64.add
    global.set $prof_223
    i32.const 101
    i32.const 114
    i32.const 114
    i32.const 111
    i32.const 114
    i32.const 58
    i32.const 32
    global.get $prof_220
    i64.const 1
    i64.add
    global.set $prof_220
    array.new_fixed $bytes 7
    call $string_from_bytes
    local.get $error
    call $inspect
    call $string_append
    local.get $site
    call $stop_at
    unreachable
  )
  (func $stop_boundary_inactive (;193;) (type $unpend)
    global.get $prof_224
    i64.const 1
    i64.add
    global.set $prof_224
    i32.const 99
    i32.const 111
    i32.const 110
    i32.const 116
    i32.const 114
    i32.const 111
    i32.const 108
    i32.const 58
    i32.const 32
    i32.const 98
    i32.const 111
    i32.const 117
    i32.const 110
    i32.const 100
    i32.const 97
    i32.const 114
    i32.const 121
    i32.const 32
    i32.const 105
    i32.const 115
    i32.const 32
    i32.const 110
    i32.const 111
    i32.const 116
    i32.const 32
    i32.const 97
    i32.const 99
    i32.const 116
    i32.const 105
    i32.const 118
    i32.const 101
    global.get $prof_225
    i64.const 1
    i64.add
    global.set $prof_225
    array.new_fixed $bytes 31
    call $string_from_bytes
    call $stop
    unreachable
  )
  (func $stop_continuation_used (;194;) (type $unpend)
    global.get $prof_226
    i64.const 1
    i64.add
    global.set $prof_226
    i32.const 99
    i32.const 111
    i32.const 110
    i32.const 116
    i32.const 114
    i32.const 111
    i32.const 108
    i32.const 58
    i32.const 32
    i32.const 99
    i32.const 111
    i32.const 110
    i32.const 116
    i32.const 105
    i32.const 110
    i32.const 117
    i32.const 97
    i32.const 116
    i32.const 105
    i32.const 111
    i32.const 110
    i32.const 32
    i32.const 119
    i32.const 97
    i32.const 115
    i32.const 32
    i32.const 97
    i32.const 108
    i32.const 114
    i32.const 101
    i32.const 97
    i32.const 100
    i32.const 121
    i32.const 32
    i32.const 117
    i32.const 115
    i32.const 101
    i32.const 100
    global.get $prof_227
    i64.const 1
    i64.add
    global.set $prof_227
    array.new_fixed $bytes 38
    call $string_from_bytes
    call $stop
    unreachable
  )
  (func $stop_float (;195;) (type $unpend)
    global.get $prof_228
    i64.const 1
    i64.add
    global.set $prof_228
    i32.const 102
    i32.const 108
    i32.const 111
    i32.const 97
    i32.const 116
    i32.const 58
    i32.const 32
    i32.const 114
    i32.const 101
    i32.const 115
    i32.const 117
    i32.const 108
    i32.const 116
    i32.const 32
    i32.const 105
    i32.const 115
    i32.const 32
    i32.const 110
    i32.const 111
    i32.const 116
    i32.const 32
    i32.const 102
    i32.const 105
    i32.const 110
    i32.const 105
    i32.const 116
    i32.const 101
    global.get $prof_229
    i64.const 1
    i64.add
    global.set $prof_229
    array.new_fixed $bytes 27
    call $string_from_bytes
    call $stop
    unreachable
  )
  (func $stop_bits_short (;196;) (type $unpend)
    global.get $prof_230
    i64.const 1
    i64.add
    global.set $prof_230
    i32.const 98
    i32.const 105
    i32.const 116
    i32.const 32
    i32.const 97
    i32.const 114
    i32.const 114
    i32.const 97
    i32.const 121
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
    i32.const 116
    i32.const 111
    i32.const 111
    i32.const 32
    i32.const 115
    i32.const 104
    i32.const 111
    i32.const 114
    i32.const 116
    global.get $prof_231
    i64.const 1
    i64.add
    global.set $prof_231
    array.new_fixed $bytes 29
    call $string_from_bytes
    call $stop
    unreachable
  )
  (func $stop_bits_negative (;197;) (type $unpend)
    global.get $prof_232
    i64.const 1
    i64.add
    global.set $prof_232
    i32.const 98
    i32.const 105
    i32.const 116
    i32.const 32
    i32.const 97
    i32.const 114
    i32.const 114
    i32.const 97
    i32.const 121
    i32.const 58
    i32.const 32
    i32.const 108
    i32.const 101
    i32.const 110
    i32.const 103
    i32.const 116
    i32.const 104
    i32.const 32
    i32.const 105
    i32.const 115
    i32.const 32
    i32.const 110
    i32.const 101
    i32.const 103
    i32.const 97
    i32.const 116
    i32.const 105
    i32.const 118
    i32.const 101
    global.get $prof_231
    i64.const 1
    i64.add
    global.set $prof_231
    array.new_fixed $bytes 29
    call $string_from_bytes
    call $stop
    unreachable
  )
  (func $stop_bits_not_whole (;198;) (type $unpend)
    global.get $prof_233
    i64.const 1
    i64.add
    global.set $prof_233
    i32.const 98
    i32.const 105
    i32.const 116
    i32.const 32
    i32.const 97
    i32.const 114
    i32.const 114
    i32.const 97
    i32.const 121
    i32.const 58
    i32.const 32
    i32.const 98
    i32.const 121
    i32.const 116
    i32.const 101
    i32.const 115
    i32.const 32
    i32.const 97
    i32.const 114
    i32.const 101
    i32.const 32
    i32.const 110
    i32.const 111
    i32.const 116
    i32.const 32
    i32.const 119
    i32.const 104
    i32.const 111
    i32.const 108
    i32.const 101
    global.get $prof_234
    i64.const 1
    i64.add
    global.set $prof_234
    array.new_fixed $bytes 30
    call $string_from_bytes
    call $stop
    unreachable
  )
  (func $stop_bits_float (;199;) (type $unpend)
    global.get $prof_235
    i64.const 1
    i64.add
    global.set $prof_235
    i32.const 98
    i32.const 105
    i32.const 116
    i32.const 32
    i32.const 97
    i32.const 114
    i32.const 114
    i32.const 97
    i32.const 121
    i32.const 58
    i32.const 32
    i32.const 102
    i32.const 108
    i32.const 111
    i32.const 97
    i32.const 116
    i32.const 32
    i32.const 105
    i32.const 115
    i32.const 32
    i32.const 110
    i32.const 111
    i32.const 116
    i32.const 32
    i32.const 102
    i32.const 105
    i32.const 110
    i32.const 105
    i32.const 116
    i32.const 101
    global.get $prof_234
    i64.const 1
    i64.add
    global.set $prof_234
    array.new_fixed $bytes 30
    call $string_from_bytes
    call $stop
    unreachable
  )
  (func $stop_host_value (;200;) (type $unpend)
    global.get $prof_236
    i64.const 1
    i64.add
    global.set $prof_236
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
    global.get $prof_237
    i64.const 1
    i64.add
    global.set $prof_237
    array.new_fixed $bytes 26
    call $string_from_bytes
    call $stop
    unreachable
  )
  (func $float_add (;201;) (type $call) (param $a eqref) (param $b eqref) (result eqref)
    global.get $prof_238
    i64.const 1
    i64.add
    global.set $prof_238
    local.get $a
    call $float_get
    local.get $b
    call $float_get
    global.get $prof_239
    i64.const 1
    i64.add
    global.set $prof_239
    f64.add
    call $float_new
  )
  (func $float_sub (;202;) (type $call) (param $a eqref) (param $b eqref) (result eqref)
    global.get $prof_240
    i64.const 1
    i64.add
    global.set $prof_240
    local.get $a
    call $float_get
    local.get $b
    call $float_get
    global.get $prof_241
    i64.const 1
    i64.add
    global.set $prof_241
    f64.sub
    call $float_new
  )
  (func $float_mul (;203;) (type $call) (param $a eqref) (param $b eqref) (result eqref)
    global.get $prof_242
    i64.const 1
    i64.add
    global.set $prof_242
    local.get $a
    call $float_get
    local.get $b
    call $float_get
    global.get $prof_52
    i64.const 1
    i64.add
    global.set $prof_52
    f64.mul
    call $float_new
  )
  (func $float_div (;204;) (type $call) (param $a eqref) (param $b eqref) (result eqref)
    global.get $prof_243
    i64.const 1
    i64.add
    global.set $prof_243
    local.get $b
    call $float_get
    f64.const 0x0p+0 (;=0;)
    f64.eq
    if (result eqref) ;; label = @1
      f64.const 0x0p+0 (;=0;)
      call $float_new
    else
      local.get $a
      call $float_get
      local.get $b
      call $float_get
    global.get $prof_244
    i64.const 1
    i64.add
    global.set $prof_244
      f64.div
      call $float_new
    end
  )
  (func $float_neg (;205;) (type 45) (param $a eqref) (result eqref)
    global.get $prof_245
    i64.const 1
    i64.add
    global.set $prof_245
    local.get $a
    call $float_get
    global.get $prof_53
    i64.const 1
    i64.add
    global.set $prof_53
    f64.neg
    call $float_new
  )
  (func $result_ok (;206;) (type 45) (param $v eqref) (result eqref)
    global.get $prof_246
    i64.const 1
    i64.add
    global.set $prof_246
    i32.const 0
    i64.const 0
    local.get $v
    global.get $prof_247
    i64.const 1
    i64.add
    global.set $prof_247
    array.new_fixed $values 1
    call $record
  )
  (func $result_error (;207;) (type 45) (param $v eqref) (result eqref)
    global.get $prof_248
    i64.const 1
    i64.add
    global.set $prof_248
    i32.const 1
    i64.const 0
    local.get $v
    global.get $prof_247
    i64.const 1
    i64.add
    global.set $prof_247
    array.new_fixed $values 1
    call $record
  )
  (func $core_not (;208;) (type 45) (param $v eqref) (result eqref)
    global.get $prof_249
    i64.const 1
    i64.add
    global.set $prof_249
    local.get $v
    call $bool_get
    i32.eqz
    call $bool_new
  )
  (func $core_to_float (;209;) (type 45) (param $v eqref) (result eqref)
    global.get $prof_250
    i64.const 1
    i64.add
    global.set $prof_250
    local.get $v
    call $int_to_float
  )
  (func $core_truncate (;210;) (type 45) (param $v eqref) (result eqref)
    global.get $prof_251
    i64.const 1
    i64.add
    global.set $prof_251
    local.get $v
    call $float_truncate
  )
  (func $core_floor (;211;) (type 45) (param $v eqref) (result eqref)
    global.get $prof_252
    i64.const 1
    i64.add
    global.set $prof_252
    local.get $v
    call $float_get
    f64.floor
    call $float_new
  )
  (func $core_ceiling (;212;) (type 45) (param $v eqref) (result eqref)
    global.get $prof_253
    i64.const 1
    i64.add
    global.set $prof_253
    local.get $v
    call $float_get
    f64.ceil
    call $float_new
  )
  (func $core_sqrt (;213;) (type 45) (param $v eqref) (result eqref)
    global.get $prof_254
    i64.const 1
    i64.add
    global.set $prof_254
    local.get $v
    call $float_get
    f64.sqrt
    call $float_new
  )
  (func $core_float_to_bits (;214;) (type 45) (param $v eqref) (result eqref)
    global.get $prof_255
    i64.const 1
    i64.add
    global.set $prof_255
    local.get $v
    call $float_to_bits
  )
  (func $core_float_from_bits (;215;) (type 45) (param $v eqref) (result eqref)
    global.get $prof_256
    i64.const 1
    i64.add
    global.set $prof_256
    local.get $v
    call $float_from_bits
  )
  (func $core_bit_and (;216;) (type $call) (param $a eqref) (param $b eqref) (result eqref)
    global.get $prof_257
    i64.const 1
    i64.add
    global.set $prof_257
    local.get $a
    local.get $b
    call $int_and
  )
  (func $core_bit_or (;217;) (type $call) (param $a eqref) (param $b eqref) (result eqref)
    global.get $prof_258
    i64.const 1
    i64.add
    global.set $prof_258
    local.get $a
    local.get $b
    call $int_or
  )
  (func $core_bit_xor (;218;) (type $call) (param $a eqref) (param $b eqref) (result eqref)
    global.get $prof_259
    i64.const 1
    i64.add
    global.set $prof_259
    local.get $a
    local.get $b
    call $int_xor
  )
  (func $core_bit_not (;219;) (type 45) (param $a eqref) (result eqref)
    global.get $prof_260
    i64.const 1
    i64.add
    global.set $prof_260
    local.get $a
    call $int_not
  )
  (func $core_shift_left (;220;) (type $call) (param $v eqref) (param $n eqref) (result eqref)
    global.get $prof_261
    i64.const 1
    i64.add
    global.set $prof_261
    local.get $v
    local.get $n
    call $int_shl
  )
  (func $core_shift_right (;221;) (type $call) (param $v eqref) (param $n eqref) (result eqref)
    global.get $prof_262
    i64.const 1
    i64.add
    global.set $prof_262
    local.get $v
    local.get $n
    call $int_shr
  )
  (func $core_concat (;222;) (type 45) (param $list eqref) (result eqref)
    global.get $prof_263
    i64.const 1
    i64.add
    global.set $prof_263
    local.get $list
    call $string_concat
  )
  (func $core_byte_size (;223;) (type 45) (param $s eqref) (result eqref)
    global.get $prof_264
    i64.const 1
    i64.add
    global.set $prof_264
    local.get $s
    call $string_byte_size
  )
  (func $core_byte_slice (;224;) (type 102) (param $s eqref) (param $start eqref) (param $length eqref) (result eqref)
    (local $b (ref $bytes)) (local $size i64) (local $from i64) (local $count i64)
    global.get $prof_265
    i64.const 1
    i64.add
    global.set $prof_265
    local.get $s
    call $string_bytes
    local.set $b
    local.get $b
    array.len
    i64.extend_i32_u
    local.set $size
    local.get $start
    call $int_sign
    i32.const 0
    i32.lt_s
    local.get $length
    call $int_sign
    i32.const 0
    i32.lt_s
    i32.or
    if ;; label = @1
      ref.null eq
      call $result_error
      return
    end
    local.get $start
    local.get $size
    call $int_from_i64
    call $int_compare
    i32.const 0
    i32.gt_s
    local.get $length
    local.get $size
    call $int_from_i64
    call $int_compare
    i32.const 0
    i32.gt_s
    i32.or
    if ;; label = @1
      ref.null eq
      call $result_error
      return
    end
    local.get $start
    call $int_to_i64
    local.set $from
    local.get $length
    call $int_to_i64
    local.set $count
    local.get $from
    local.get $count
    i64.add
    local.get $size
    i64.gt_u
    if ;; label = @1
      ref.null eq
      call $result_error
      return
    end
    local.get $b
    local.get $from
    i32.wrap_i64
    call $string_boundary
    local.get $b
    local.get $from
    local.get $count
    i64.add
    i32.wrap_i64
    call $string_boundary
    i32.and
    i32.eqz
    if ;; label = @1
      ref.null eq
      call $result_error
      return
    end
    local.get $s
    local.get $from
    i32.wrap_i64
    local.get $count
    i32.wrap_i64
    call $string_slice_bytes
    call $result_ok
  )
  (func $core_codepoints (;225;) (type 45) (param $s eqref) (result eqref)
    global.get $prof_266
    i64.const 1
    i64.add
    global.set $prof_266
    local.get $s
    call $string_codepoints
  )
  (func $core_from_codepoints (;226;) (type 45) (param $list eqref) (result eqref)
    global.get $prof_267
    i64.const 1
    i64.add
    global.set $prof_267
    local.get $list
    call $string_from_codepoints
  )
  (func $core_codepoint_to_int (;227;) (type 45) (param $c eqref) (result eqref)
    global.get $prof_268
    i64.const 1
    i64.add
    global.set $prof_268
    local.get $c
    call $codepoint_get
    i64.extend_i32_u
    call $int_from_i64
  )
  (func $core_int_to_codepoint (;228;) (type 45) (param $n eqref) (result eqref)
    (local $value i32)
    global.get $prof_269
    i64.const 1
    i64.add
    global.set $prof_269
    local.get $n
    global.get $prof_3
    i64.const 1
    i64.add
    global.set $prof_3
    ref.test (ref i31)
    i32.eqz
    if ;; label = @1
      ref.null eq
      call $result_error
      return
    end
    local.get $n
    global.get $prof_4
    i64.const 1
    i64.add
    global.set $prof_4
    ref.cast (ref i31)
    global.get $prof_5
    i64.const 1
    i64.add
    global.set $prof_5
    i31.get_s
    local.set $value
    local.get $value
    i32.const 0
    i32.lt_s
    local.get $value
    i32.const 1114111
    i32.gt_s
    i32.or
    local.get $value
    i32.const 55296
    i32.ge_s
    local.get $value
    i32.const 57343
    i32.le_s
    i32.and
    i32.or
    if ;; label = @1
      ref.null eq
      call $result_error
      return
    end
    local.get $value
    call $codepoint_new
    call $result_ok
  )
  (func $core_bit_size (;229;) (type 45) (param $b eqref) (result eqref)
    global.get $prof_270
    i64.const 1
    i64.add
    global.set $prof_270
    local.get $b
    call $bits_size
  )
  (func $core_bit_array_to_string (;230;) (type 45) (param $value eqref) (result eqref)
    (local $length i64) (local $data (ref $bytes)) (local $count i32) (local $index i32)
    global.get $prof_271
    i64.const 1
    i64.add
    global.set $prof_271
    local.get $value
    call $bits_len64
    local.set $length
    local.get $length
    i64.const 7
    i64.and
    i64.const 0
    i64.ne
    if ;; label = @1
      ref.null eq
      call $result_error
      return
    end
    local.get $length
    i64.const 3
    i64.shr_u
    i32.wrap_i64
    local.set $count
    local.get $count
    global.get $prof_62
    i64.const 1
    i64.add
    global.set $prof_62
    array.new_default $bytes
    local.set $data
    block $copied
      loop $copy
        local.get $index
        local.get $count
        i32.ge_u
        br_if $copied
        local.get $data
        local.get $index
        local.get $value
        local.get $index
        i64.extend_i32_u
        i64.const 3
        i64.shl
        call $bits_byte
        array.set $bytes
        local.get $index
        i32.const 1
        i32.add
        local.set $index
        br $copy
      end
    end
    local.get $data
    call $utf8_valid
    i32.eqz
    if ;; label = @1
      ref.null eq
      call $result_error
      return
    end
    local.get $data
    call $string_from_bytes
    call $result_ok
  )
  (func $core_inspect (;231;) (type 45) (param $v eqref) (result eqref)
    global.get $prof_272
    i64.const 1
    i64.add
    global.set $prof_272
    local.get $v
    call $inspect
  )
  (func $core_hash (;232;) (type 45) (param $v eqref) (result eqref)
    global.get $prof_273
    i64.const 1
    i64.add
    global.set $prof_273
    local.get $v
    call $value_hash
  )
  (func $case_upper_table (;233;) (type 103) (result (ref $case_ranges))
    global.get $prof_274
    i64.const 1
    i64.add
    global.set $prof_274
    global.get $case_upper
    ref.is_null
    if ;; label = @1
      i32.const 0
      i32.const 639
    global.get $prof_275
    i64.const 1
    i64.add
    global.set $prof_275
      array.new_data $case_ranges $case_upper_data
      global.set $case_upper
    end
    global.get $case_upper
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
    ref.as_non_null
  )
  (func $case_lower_table (;234;) (type 103) (result (ref $case_ranges))
    global.get $prof_276
    i64.const 1
    i64.add
    global.set $prof_276
    global.get $case_lower
    ref.is_null
    if ;; label = @1
      i32.const 0
      i32.const 585
    global.get $prof_277
    i64.const 1
    i64.add
    global.set $prof_277
      array.new_data $case_ranges $case_lower_data
      global.set $case_lower
    end
    global.get $case_lower
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
    ref.as_non_null
  )
  (func $case_map (;235;) (type 104) (param $table (ref $case_ranges)) (param $cp i32) (result i32)
    (local $low i32) (local $high i32) (local $middle i32) (local $at i32) (local $last i32)
    global.get $prof_278
    i64.const 1
    i64.add
    global.set $prof_278
    local.get $table
    array.len
    i32.const 3
    i32.div_u
    local.set $high
    block $found
      loop $search
        local.get $low
        local.get $high
        i32.ge_u
        br_if $found
        local.get $low
        local.get $high
        i32.add
        i32.const 1
        i32.shr_u
        local.set $middle
        local.get $table
        local.get $middle
        i32.const 3
        i32.mul
    global.get $prof_12
    i64.const 1
    i64.add
    global.set $prof_12
        array.get $case_ranges
        local.get $cp
        i32.le_u
        if ;; label = @3
          local.get $middle
          i32.const 1
          i32.add
          local.set $low
        else
          local.get $middle
          local.set $high
        end
        br $search
      end
    end
    local.get $low
    i32.eqz
    if ;; label = @1
      local.get $cp
      return
    end
    local.get $low
    i32.const 1
    i32.sub
    i32.const 3
    i32.mul
    local.set $at
    local.get $table
    local.get $at
    i32.const 1
    i32.add
    global.get $prof_12
    i64.const 1
    i64.add
    global.set $prof_12
    array.get $case_ranges
    local.set $last
    local.get $cp
    local.get $last
    i32.const 16777215
    i32.and
    i32.gt_u
    if ;; label = @1
      local.get $cp
      return
    end
    local.get $last
    i32.const 24
    i32.shr_u
    local.get $cp
    local.get $table
    local.get $at
    global.get $prof_12
    i64.const 1
    i64.add
    global.set $prof_12
    array.get $case_ranges
    i32.sub
    i32.and
    if ;; label = @1
      local.get $cp
      return
    end
    local.get $cp
    local.get $table
    local.get $at
    i32.const 2
    i32.add
    global.get $prof_12
    i64.const 1
    i64.add
    global.set $prof_12
    array.get $case_ranges
    i32.add
  )
  (func $case_decode (;236;) (type 90) (param $b (ref $bytes)) (param $at i32) (result i32)
    (local $lead i32)
    global.get $prof_279
    i64.const 1
    i64.add
    global.set $prof_279
    local.get $b
    local.get $at
    array.get_u $bytes
    local.set $lead
    local.get $lead
    i32.const 128
    i32.lt_u
    if ;; label = @1
      local.get $lead
      return
    end
    local.get $lead
    i32.const 224
    i32.lt_u
    if ;; label = @1
      local.get $lead
      i32.const 31
      i32.and
      i32.const 6
      i32.shl
      local.get $b
      local.get $at
      i32.const 1
      i32.add
      array.get_u $bytes
      i32.const 63
      i32.and
      i32.or
      return
    end
    local.get $lead
    i32.const 240
    i32.lt_u
    if ;; label = @1
      local.get $lead
      i32.const 15
      i32.and
      i32.const 12
      i32.shl
      local.get $b
      local.get $at
      i32.const 1
      i32.add
      array.get_u $bytes
      i32.const 63
      i32.and
      i32.const 6
      i32.shl
      i32.or
      local.get $b
      local.get $at
      i32.const 2
      i32.add
      array.get_u $bytes
      i32.const 63
      i32.and
      i32.or
      return
    end
    local.get $lead
    i32.const 7
    i32.and
    i32.const 18
    i32.shl
    local.get $b
    local.get $at
    i32.const 1
    i32.add
    array.get_u $bytes
    i32.const 63
    i32.and
    i32.const 12
    i32.shl
    i32.or
    local.get $b
    local.get $at
    i32.const 2
    i32.add
    array.get_u $bytes
    i32.const 63
    i32.and
    i32.const 6
    i32.shl
    i32.or
    local.get $b
    local.get $at
    i32.const 3
    i32.add
    array.get_u $bytes
    i32.const 63
    i32.and
    i32.or
  )
  (func $case_string (;237;) (type 105) (param $s eqref) (param $table (ref $case_ranges)) (param $ascii_first i32) (param $ascii_shift i32) (result eqref)
    (local $b (ref $bytes)) (local $n i32) (local $at i32) (local $cp i32) (local $mapped i32) (local $out (ref null $bytes)) (local $grown (ref null $bytes)) (local $size i32) (local $capacity i64)
    global.get $prof_280
    i64.const 1
    i64.add
    global.set $prof_280
    local.get $s
    call $string_bytes
    local.set $b
    local.get $b
    array.len
    local.set $n
    block $changed
      loop $scan
        local.get $at
        local.get $n
        i32.ge_u
        if ;; label = @3
          local.get $s
          return
        end
        local.get $b
        local.get $at
        call $case_decode
        local.set $cp
        local.get $cp
        i32.const 128
        i32.lt_u
        if ;; label = @3
          local.get $cp
          local.get $ascii_first
          i32.sub
          i32.const 26
          i32.lt_u
          br_if $changed
          local.get $at
          i32.const 1
          i32.add
          local.set $at
        else
          local.get $table
          local.get $cp
          call $case_map
          local.get $cp
          i32.ne
          br_if $changed
          local.get $at
          local.get $cp
          call $utf8_width
          i32.add
          local.set $at
        end
        br $scan
      end
    end
    local.get $n
    i64.extend_i32_u
    i64.const 16
    i64.add
    local.set $capacity
    local.get $capacity
    i64.const 4294967295
    i64.gt_u
    if ;; label = @1
      i64.const 4294967295
      local.set $capacity
    end
    local.get $capacity
    i32.wrap_i64
    global.get $prof_62
    i64.const 1
    i64.add
    global.set $prof_62
    array.new_default $bytes
    local.set $out
    local.get $out
    i32.const 0
    local.get $b
    i32.const 0
    local.get $at
    array.copy $bytes $bytes
    local.get $at
    local.set $size
    block $done
      loop $map
        local.get $at
        local.get $n
        i32.ge_u
        br_if $done
        local.get $b
        local.get $at
        call $case_decode
        local.set $cp
        local.get $cp
        i32.const 128
        i32.lt_u
        if ;; label = @3
          local.get $cp
          local.set $mapped
          local.get $cp
          local.get $ascii_first
          i32.sub
          i32.const 26
          i32.lt_u
          if ;; label = @4
            local.get $cp
            local.get $ascii_shift
            i32.add
            local.set $mapped
          end
          local.get $at
          i32.const 1
          i32.add
          local.set $at
        else
          local.get $table
          local.get $cp
          call $case_map
          local.set $mapped
          local.get $at
          local.get $cp
          call $utf8_width
          i32.add
          local.set $at
        end
        local.get $size
        i32.const 4
        i32.add
        local.get $out
        array.len
        i32.gt_u
        if ;; label = @3
          local.get $out
          array.len
          i64.extend_i32_u
          i64.const 1
          i64.shl
          local.set $capacity
          local.get $capacity
          i64.const 4294967295
          i64.gt_u
          if ;; label = @4
            i64.const 4294967295
            local.set $capacity
          end
          local.get $capacity
          local.get $out
          array.len
          i64.extend_i32_u
          i64.le_u
          if ;; label = @4
            unreachable
          end
          local.get $capacity
          i32.wrap_i64
    global.get $prof_62
    i64.const 1
    i64.add
    global.set $prof_62
          array.new_default $bytes
          local.set $grown
          local.get $grown
          i32.const 0
          local.get $out
          i32.const 0
          local.get $size
          array.copy $bytes $bytes
          local.get $grown
          local.set $out
        end
        local.get $out
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
        ref.as_non_null
        local.get $size
        local.get $mapped
        call $utf8_write
        local.set $size
        br $map
      end
    end
    local.get $size
    global.get $prof_62
    i64.const 1
    i64.add
    global.set $prof_62
    array.new_default $bytes
    local.set $grown
    local.get $grown
    i32.const 0
    local.get $out
    i32.const 0
    local.get $size
    array.copy $bytes $bytes
    local.get $grown
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
    ref.as_non_null
    call $string_from_bytes
  )
  (func $core_to_upper (;238;) (type 45) (param $s eqref) (result eqref)
    global.get $prof_281
    i64.const 1
    i64.add
    global.set $prof_281
    local.get $s
    call $case_upper_table
    i32.const 97
    i32.const -32
    call $case_string
  )
  (func $core_to_lower (;239;) (type 45) (param $s eqref) (result eqref)
    global.get $prof_282
    i64.const 1
    i64.add
    global.set $prof_282
    local.get $s
    call $case_lower_table
    i32.const 65
    i32.const 32
    call $case_string
  )
  (func $core_index_of (;240;) (type 102) (param $s eqref) (param $find eqref) (param $from eqref) (result eqref)
    (local $text (ref $bytes)) (local $pattern (ref $bytes)) (local $n i32) (local $m i32) (local $start i32) (local $found i32)
    global.get $prof_283
    i64.const 1
    i64.add
    global.set $prof_283
    local.get $s
    call $string_bytes
    local.set $text
    local.get $find
    call $string_bytes
    local.set $pattern
    local.get $text
    array.len
    local.set $n
    local.get $pattern
    array.len
    local.set $m
    local.get $from
    call $int_sign
    i32.const 0
    i32.lt_s
    if ;; label = @1
      ref.null eq
      call $result_error
      return
    end
    local.get $from
    local.get $n
    i64.extend_i32_u
    call $int_from_i64
    call $int_compare
    i32.const 0
    i32.gt_s
    if ;; label = @1
      ref.null eq
      call $result_error
      return
    end
    local.get $from
    call $int_to_i64
    i32.wrap_i64
    local.set $start
    local.get $text
    local.get $start
    call $string_boundary
    i32.eqz
    if ;; label = @1
      ref.null eq
      call $result_error
      return
    end
    local.get $m
    i32.eqz
    if ;; label = @1
      local.get $from
      call $result_ok
      return
    end
    local.get $m
    local.get $n
    local.get $start
    i32.sub
    i32.gt_u
    if ;; label = @1
      ref.null eq
      call $result_error
      return
    end
    local.get $m
    i32.const 1
    i32.eq
    if (result i32) ;; label = @1
      local.get $text
      local.get $pattern
      i32.const 0
      array.get_u $bytes
      local.get $start
      call $search_byte
    else
      local.get $text
      local.get $pattern
      local.get $start
      call $search_kmp
    end
    local.set $found
    local.get $found
    i32.const 0
    i32.lt_s
    if ;; label = @1
      ref.null eq
      call $result_error
      return
    end
    local.get $found
    i64.extend_i32_u
    call $int_from_i64
    call $result_ok
  )
  (func $search_byte (;241;) (type 92) (param $text (ref $bytes)) (param $byte i32) (param $at i32) (result i32)
    (local $n i32)
    global.get $prof_284
    i64.const 1
    i64.add
    global.set $prof_284
    local.get $text
    array.len
    local.set $n
    block $none
      loop $scan
        local.get $at
        local.get $n
        i32.ge_u
        br_if $none
        local.get $text
        local.get $at
        array.get_u $bytes
        local.get $byte
        i32.eq
        if ;; label = @3
          local.get $at
          return
        end
        local.get $at
        i32.const 1
        i32.add
        local.set $at
        br $scan
      end
    end
    i32.const -1
  )
  (func $search_kmp (;242;) (type 94) (param $text (ref $bytes)) (param $pattern (ref $bytes)) (param $at i32) (result i32)
    (local $border (ref $search_table)) (local $n i32) (local $m i32) (local $i i32) (local $k i32) (local $byte i32)
    global.get $prof_285
    i64.const 1
    i64.add
    global.set $prof_285
    local.get $text
    array.len
    local.set $n
    local.get $pattern
    array.len
    local.set $m
    local.get $m
    global.get $prof_286
    i64.const 1
    i64.add
    global.set $prof_286
    array.new_default $search_table
    local.set $border
    i32.const 1
    local.set $i
    block $built
      loop $build
        local.get $i
        local.get $m
        i32.ge_u
        br_if $built
        local.get $pattern
        local.get $i
        array.get_u $bytes
        local.set $byte
        block $fits
          loop $shorter
            local.get $k
            i32.eqz
            br_if $fits
            local.get $pattern
            local.get $k
            array.get_u $bytes
            local.get $byte
            i32.eq
            br_if $fits
            local.get $border
            local.get $k
            i32.const 1
            i32.sub
    global.get $prof_12
    i64.const 1
    i64.add
    global.set $prof_12
            array.get $search_table
            local.set $k
            br $shorter
          end
        end
        local.get $pattern
        local.get $k
        array.get_u $bytes
        local.get $byte
        i32.eq
        if ;; label = @3
          local.get $k
          i32.const 1
          i32.add
          local.set $k
        end
        local.get $border
        local.get $i
        local.get $k
        array.set $search_table
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $build
      end
    end
    i32.const 0
    local.set $k
    block $none
      loop $scan
        local.get $at
        local.get $n
        i32.ge_u
        br_if $none
        local.get $text
        local.get $at
        array.get_u $bytes
        local.set $byte
        block $fits
          loop $shorter
            local.get $k
            i32.eqz
            br_if $fits
            local.get $pattern
            local.get $k
            array.get_u $bytes
            local.get $byte
            i32.eq
            br_if $fits
            local.get $border
            local.get $k
            i32.const 1
            i32.sub
    global.get $prof_12
    i64.const 1
    i64.add
    global.set $prof_12
            array.get $search_table
            local.set $k
            br $shorter
          end
        end
        local.get $pattern
        local.get $k
        array.get_u $bytes
        local.get $byte
        i32.eq
        if ;; label = @3
          local.get $k
          i32.const 1
          i32.add
          local.set $k
        end
        local.get $at
        i32.const 1
        i32.add
        local.set $at
        local.get $k
        local.get $m
        i32.eq
        if ;; label = @3
          local.get $at
          local.get $m
          i32.sub
          return
        end
        br $scan
      end
    end
    i32.const -1
  )
  (func $core_bit_array_concat (;243;) (type 45) (param $list eqref) (result eqref)
    (local $cursor eqref) (local $part eqref) (local $length i64) (local $at i64) (local $data (ref null $bytes)) (local $count i32)
    global.get $prof_287
    i64.const 1
    i64.add
    global.set $prof_287
    local.get $list
    local.set $cursor
    block $counted
      loop $count
        local.get $cursor
        call $list_is_empty
        br_if $counted
        local.get $cursor
        call $list_head
        local.set $part
        local.get $length
        local.get $part
        call $bits_len64
        i64.add
        local.set $length
        local.get $count
        i32.const 1
        i32.add
        local.set $count
        local.get $cursor
        call $list_tail
        local.set $cursor
        br $count
      end
    end
    local.get $count
    i32.eqz
    if ;; label = @1
      call $bits_empty
      return
    end
    local.get $count
    i32.const 1
    i32.eq
    if ;; label = @1
      local.get $part
      return
    end
    local.get $length
    call $bits_alloc
    local.set $data
    local.get $list
    local.set $cursor
    block $written
      loop $write
        local.get $cursor
        call $list_is_empty
        br_if $written
        local.get $cursor
        call $list_head
        local.set $part
        local.get $data
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
        ref.as_non_null
        local.get $at
        local.get $part
        call $bits_join_part
        local.get $at
        local.get $part
        call $bits_len64
        i64.add
        local.set $at
        local.get $cursor
        call $list_tail
        local.set $cursor
        br $write
      end
    end
    local.get $data
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
    ref.as_non_null
    i32.const 0
    local.get $length
    call $bits_new
  )
  (func $bits_join_part (;244;) (type 106) (param $out (ref $bytes)) (param $at i64) (param $part eqref)
    (local $source (ref $bytes)) (local $offset i64) (local $length i64) (local $i i64) (local $position i64) (local $q i32) (local $r i32) (local $value i32) (local $take i64) (local $p i32) (local $t i32) (local $whole i32)
    global.get $prof_288
    i64.const 1
    i64.add
    global.set $prof_288
    local.get $part
    call $bits_data
    local.set $source
    local.get $part
    call $bits_offset
    local.set $offset
    local.get $part
    call $bits_len64
    local.set $length
    local.get $at
    local.get $offset
    i64.or
    i64.const 7
    i64.and
    i64.eqz
    if ;; label = @1
      local.get $length
      i64.const 3
      i64.shr_u
      i32.wrap_i64
      local.set $whole
      local.get $out
      local.get $at
      i64.const 3
      i64.shr_u
      i32.wrap_i64
      local.get $source
      local.get $offset
      i64.const 3
      i64.shr_u
      i32.wrap_i64
      local.get $whole
      array.copy $bytes $bytes
      local.get $whole
      i64.extend_i32_u
      i64.const 3
      i64.shl
      local.set $i
    end
    block $done
      loop $copy
        local.get $i
        local.get $length
        i64.ge_u
        br_if $done
        local.get $offset
        local.get $i
        i64.add
        local.set $position
        local.get $position
        i64.const 3
        i64.shr_u
        i32.wrap_i64
        local.set $q
        local.get $position
        i64.const 7
        i64.and
        i32.wrap_i64
        local.set $r
        local.get $source
        local.get $q
        array.get_u $bytes
        local.get $r
        i32.shl
        local.set $value
        local.get $r
        i32.const 0
        i32.ne
        local.get $q
        i32.const 1
        i32.add
        local.get $source
        array.len
        i32.lt_u
        i32.and
        if ;; label = @3
          local.get $value
          local.get $source
          local.get $q
          i32.const 1
          i32.add
          array.get_u $bytes
          i32.const 8
          local.get $r
          i32.sub
          i32.shr_u
          i32.or
          local.set $value
        end
        local.get $length
        local.get $i
        i64.sub
        local.set $take
        local.get $take
        i64.const 8
        i64.gt_u
        if ;; label = @3
          i64.const 8
          local.set $take
        end
        local.get $value
        i32.const 255
        i32.const 8
        local.get $take
        i32.wrap_i64
        i32.sub
        i32.shl
        i32.const 255
        i32.and
        i32.and
        local.set $value
        local.get $at
        local.get $i
        i64.add
        local.set $position
        local.get $position
        i64.const 3
        i64.shr_u
        i32.wrap_i64
        local.set $p
        local.get $position
        i64.const 7
        i64.and
        i32.wrap_i64
        local.set $t
        local.get $out
        local.get $p
        local.get $out
        local.get $p
        array.get_u $bytes
        local.get $value
        local.get $t
        i32.shr_u
        i32.or
        array.set $bytes
        local.get $t
        i32.const 0
        i32.ne
        local.get $value
        i32.const 8
        local.get $t
        i32.sub
        i32.shl
        i32.const 255
        i32.and
        i32.const 0
        i32.ne
        i32.and
        if ;; label = @3
          local.get $out
          local.get $p
          i32.const 1
          i32.add
          local.get $out
          local.get $p
          i32.const 1
          i32.add
          array.get_u $bytes
          local.get $value
          i32.const 8
          local.get $t
          i32.sub
          i32.shl
          i32.const 255
          i32.and
          i32.or
          array.set $bytes
        end
        local.get $i
        i64.const 8
        i64.add
        local.set $i
        br $copy
      end
    end
  )
  (func $abi_buffer_check (;245;) (type 107) (param $address i32) (param $length i64)
    global.get $prof_289
    i64.const 1
    i64.add
    global.set $prof_289
    local.get $address
    i64.extend_i32_u
    local.get $length
    i64.add
    memory.size
    i64.extend_i32_u
    i64.const 16
    i64.shl
    i64.gt_u
    if ;; label = @1
      call $stop_host_value
      unreachable
    end
  )
  (func $abi_bool_in (;246;) (type 77) (param $value i32) (result eqref)
    global.get $prof_290
    i64.const 1
    i64.add
    global.set $prof_290
    local.get $value
    i32.const 1
    i32.gt_u
    if ;; label = @1
      call $stop_host_value
      unreachable
    end
    local.get $value
    call $bool_new
  )
  (func $abi_bool_out (;247;) (type 36) (param $value eqref) (result i32)
    global.get $prof_291
    i64.const 1
    i64.add
    global.set $prof_291
    local.get $value
    call $bool_get
  )
  (func $abi_float_in (;248;) (type 53) (param $value f64) (result eqref)
    global.get $prof_292
    i64.const 1
    i64.add
    global.set $prof_292
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
  (func $abi_float_out (;249;) (type 52) (param $value eqref) (result f64)
    global.get $prof_293
    i64.const 1
    i64.add
    global.set $prof_293
    local.get $value
    call $float_get
  )
  (func $abi_int_in (;250;) (type 108) (param $sign i32) (param $address i32) (param $count i32) (result eqref)
    (local $limbs (ref $limbs)) (local $i i32)
    global.get $prof_294
    i64.const 1
    i64.add
    global.set $prof_294
    local.get $sign
    i32.const 1
    i32.add
    i32.const 2
    i32.gt_u
    if ;; label = @1
      call $stop_host_value
      unreachable
    end
    local.get $sign
    i32.eqz
    local.get $count
    i32.eqz
    i32.ne
    if ;; label = @1
      call $stop_host_value
      unreachable
    end
    local.get $address
    local.get $count
    i64.extend_i32_u
    i64.const 2
    i64.shl
    call $abi_buffer_check
    local.get $count
    i32.eqz
    if ;; label = @1
      i32.const 0
    global.get $prof_14
    i64.const 1
    i64.add
    global.set $prof_14
      ref.i31
      return
    end
    local.get $count
    global.get $prof_8
    i64.const 1
    i64.add
    global.set $prof_8
    array.new_default $limbs
    local.set $limbs
    block $done
      loop $each
        local.get $i
        local.get $count
        i32.ge_u
        br_if $done
        local.get $limbs
        local.get $i
        local.get $address
        local.get $i
        i32.const 2
        i32.shl
        i32.add
        i32.load
        array.set $limbs
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $each
      end
    end
    local.get $limbs
    local.get $count
    i32.const 1
    i32.sub
    global.get $prof_12
    i64.const 1
    i64.add
    global.set $prof_12
    array.get $limbs
    i32.eqz
    if ;; label = @1
      call $stop_host_value
      unreachable
    end
    local.get $sign
    local.get $limbs
    call $int_pack
  )
  (func $abi_int_out (;251;) (type 109) (param $value eqref) (result i32 i32 i32)
    (local $limbs (ref $limbs)) (local $count i32) (local $address i32) (local $i i32)
    global.get $prof_295
    i64.const 1
    i64.add
    global.set $prof_295
    local.get $value
    call $int_limbs
    local.set $limbs
    local.get $limbs
    array.len
    local.set $count
    local.get $count
    i32.eqz
    if ;; label = @1
      i32.const 0
      i32.const 0
      i32.const 0
      return
    end
    local.get $count
    i32.const 2
    i32.shl
    call $alloc
    local.set $address
    block $done
      loop $each
        local.get $i
        local.get $count
        i32.ge_u
        br_if $done
        local.get $address
        local.get $i
        i32.const 2
        i32.shl
        i32.add
        local.get $limbs
        local.get $i
    global.get $prof_12
    i64.const 1
    i64.add
    global.set $prof_12
        array.get $limbs
        i32.store
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $each
      end
    end
    local.get $value
    call $int_sign
    local.get $address
    local.get $count
  )
  (func $abi_bytes_in (;252;) (type 110) (param $address i32) (param $length i32) (result (ref $bytes))
    (local $data (ref $bytes)) (local $i i32)
    global.get $prof_296
    i64.const 1
    i64.add
    global.set $prof_296
    local.get $address
    local.get $length
    i64.extend_i32_u
    call $abi_buffer_check
    local.get $length
    global.get $prof_62
    i64.const 1
    i64.add
    global.set $prof_62
    array.new_default $bytes
    local.set $data
    block $done
      loop $each
        local.get $i
        local.get $length
        i32.ge_u
        br_if $done
        local.get $data
        local.get $i
        local.get $address
        local.get $i
        i32.add
        i32.load8_u
        array.set $bytes
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $each
      end
    end
    local.get $data
  )
  (func $abi_string_in (;253;) (type 93) (param $address i32) (param $length i32) (result eqref)
    (local $data (ref $bytes))
    global.get $prof_297
    i64.const 1
    i64.add
    global.set $prof_297
    local.get $address
    local.get $length
    call $abi_bytes_in
    local.set $data
    local.get $data
    call $utf8_valid
    i32.eqz
    if ;; label = @1
      call $stop_host_value
      unreachable
    end
    local.get $data
    call $string_from_bytes
  )
  (func $abi_string_out (;254;) (type 111) (param $value eqref) (result i32 i32)
    (local $length i32)
    global.get $prof_298
    i64.const 1
    i64.add
    global.set $prof_298
    local.get $value
    call $string_length_bytes
    local.set $length
    local.get $length
    i32.eqz
    if ;; label = @1
      i32.const 0
      i32.const 0
      return
    end
    local.get $value
    call $string_to_mem
    local.get $length
  )
  (func $abi_bits_in (;255;) (type 93) (param $address i32) (param $bits i32) (result eqref)
    (local $data (ref $bytes)) (local $count i32) (local $rest i32)
    global.get $prof_299
    i64.const 1
    i64.add
    global.set $prof_299
    local.get $bits
    i64.extend_i32_u
    i64.const 7
    i64.add
    i64.const 3
    i64.shr_u
    i32.wrap_i64
    local.set $count
    local.get $address
    local.get $count
    call $abi_bytes_in
    local.set $data
    local.get $bits
    i32.const 7
    i32.and
    local.set $rest
    local.get $rest
    if ;; label = @1
      local.get $data
      local.get $count
      i32.const 1
      i32.sub
      array.get_u $bytes
      i32.const 1
      i32.const 8
      local.get $rest
      i32.sub
      i32.shl
      i32.const 1
      i32.sub
      i32.and
      if ;; label = @2
        call $stop_host_value
        unreachable
      end
    end
    local.get $data
    i32.const 0
    local.get $bits
    i64.extend_i32_u
    call $bits_new
  )
  (func $abi_bits_out (;256;) (type 111) (param $value eqref) (result i32 i32)
    (local $length i64) (local $count i32) (local $address i32) (local $i i32) (local $rest i64)
    global.get $prof_300
    i64.const 1
    i64.add
    global.set $prof_300
    local.get $value
    call $bits_len64
    local.set $length
    local.get $length
    i64.const 4294967295
    i64.gt_u
    if ;; label = @1
      unreachable
    end
    local.get $length
    i64.eqz
    if ;; label = @1
      i32.const 0
      i32.const 0
      return
    end
    local.get $length
    i64.const 7
    i64.add
    i64.const 3
    i64.shr_u
    i32.wrap_i64
    local.set $count
    local.get $count
    call $alloc
    local.set $address
    block $done
      loop $each
        local.get $i
        i32.const 1
        i32.add
        i64.extend_i32_u
        i64.const 3
        i64.shl
        local.get $length
        i64.gt_u
        br_if $done
        local.get $address
        local.get $i
        i32.add
        local.get $value
        local.get $i
        i64.extend_i32_u
        i64.const 3
        i64.shl
        call $bits_byte
        i32.store8
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $each
      end
    end
    local.get $length
    i64.const 7
    i64.and
    local.set $rest
    local.get $rest
    i64.const 0
    i64.ne
    if ;; label = @1
      local.get $address
      local.get $i
      i32.add
      local.get $value
      local.get $i
      i64.extend_i32_u
      i64.const 3
      i64.shl
      local.get $rest
      i32.const 0
      call $bits_read_u64
      i64.const 8
      local.get $rest
      i64.sub
      i64.shl
      i32.wrap_i64
      i32.store8
    end
    local.get $address
    local.get $length
    i32.wrap_i64
  )
  (func $abi_ref_in (;257;) (type 84) (param $value externref) (result eqref)
    global.get $prof_301
    i64.const 1
    i64.add
    global.set $prof_301
    local.get $value
    call $host_ref_new
  )
  (func $abi_ref_out (;258;) (type 85) (param $value eqref) (result externref)
    global.get $prof_302
    i64.const 1
    i64.add
    global.set $prof_302
    local.get $value
    call $host_ref_get
  )
  (func $abi_int_take (;259;) (type 108) (param $sign i32) (param $address i32) (param $count i32) (result eqref)
    (local $v eqref)
    global.get $prof_303
    i64.const 1
    i64.add
    global.set $prof_303
    local.get $sign
    local.get $address
    local.get $count
    call $abi_int_in
    local.set $v
    local.get $address
    local.get $count
    i32.const 2
    i32.shl
    call $free
    local.get $v
  )
  (func $abi_string_take (;260;) (type 93) (param $address i32) (param $length i32) (result eqref)
    (local $v eqref)
    global.get $prof_304
    i64.const 1
    i64.add
    global.set $prof_304
    local.get $address
    local.get $length
    call $abi_string_in
    local.set $v
    local.get $address
    local.get $length
    call $free
    local.get $v
  )
  (func $abi_bits_take (;261;) (type 93) (param $address i32) (param $bits i32) (result eqref)
    (local $v eqref)
    global.get $prof_305
    i64.const 1
    i64.add
    global.set $prof_305
    local.get $address
    local.get $bits
    call $abi_bits_in
    local.set $v
    local.get $address
    local.get $bits
    i64.extend_i32_u
    i64.const 7
    i64.add
    i64.const 3
    i64.shr_u
    i32.wrap_i64
    call $free
    local.get $v
  )
  (func $abi_tag_in (;262;) (type 88) (param $tag i32) (param $members i32) (result i32)
    global.get $prof_306
    i64.const 1
    i64.add
    global.set $prof_306
    local.get $tag
    local.get $members
    i32.gt_u
    if ;; label = @1
      call $stop_host_value
      unreachable
    end
    local.get $tag
  )
  (func $is_ctor (;263;) (type 112) (param $v eqref) (param $c i32) (result i32)
    global.get $prof_307
    i64.const 1
    i64.add
    global.set $prof_307
    local.get $v
    call $value_tag
    i32.const 5
    i32.eq
    if (result i32) ;; label = @1
      local.get $v
    global.get $prof_4
    i64.const 1
    i64.add
    global.set $prof_4
      ref.cast (ref $obj)
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
      struct.get $obj 1
      local.get $c
      i32.eq
    else
      i32.const 0
    end
  )
  (func $is_type (;264;) (type 61) (param $v eqref) (param $t i64) (result i32)
    global.get $prof_308
    i64.const 1
    i64.add
    global.set $prof_308
    local.get $v
    call $value_tag
    i32.const 5
    i32.eq
    if (result i32) ;; label = @1
      local.get $v
    global.get $prof_4
    i64.const 1
    i64.add
    global.set $prof_4
      ref.cast (ref $obj)
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
      struct.get $obj 2
      local.get $t
      i64.eq
    else
      i32.const 0
    end
  )
  (func $is_bool (;265;) (type 112) (param $v eqref) (param $b i32) (result i32)
    global.get $prof_309
    i64.const 1
    i64.add
    global.set $prof_309
    local.get $v
    call $value_tag
    i32.const 1
    i32.eq
    if (result i32) ;; label = @1
      local.get $v
    global.get $prof_4
    i64.const 1
    i64.add
    global.set $prof_4
      ref.cast (ref $obj)
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
      struct.get $obj 1
      local.get $b
      i32.eq
    else
      i32.const 0
    end
  )
  (func $is_int (;266;) (type 43) (param $v eqref) (param $n eqref) (result i32)
    global.get $prof_310
    i64.const 1
    i64.add
    global.set $prof_310
    local.get $v
    call $value_tag
    i32.const 8
    i32.eq
    if (result i32) ;; label = @1
      local.get $v
      local.get $n
      call $int_compare
      i32.eqz
    else
      i32.const 0
    end
  )
  (func $is_float (;267;) (type 113) (param $v eqref) (param $x f64) (result i32)
    global.get $prof_311
    i64.const 1
    i64.add
    global.set $prof_311
    local.get $v
    call $value_tag
    i32.const 2
    i32.eq
    if (result i32) ;; label = @1
      local.get $v
    global.get $prof_4
    i64.const 1
    i64.add
    global.set $prof_4
      ref.cast (ref $obj)
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
      struct.get $obj 3
      local.get $x
      f64.eq
    else
      i32.const 0
    end
  )
  (func $is_string (;268;) (type 43) (param $v eqref) (param $s eqref) (result i32)
    global.get $prof_312
    i64.const 1
    i64.add
    global.set $prof_312
    local.get $v
    call $value_tag
    i32.const 3
    i32.eq
    if (result i32) ;; label = @1
      local.get $v
      local.get $s
      call $string_compare
      i32.eqz
    else
      i32.const 0
    end
  )
  (func $is_tuple (;269;) (type 112) (param $v eqref) (param $n i32) (result i32)
    global.get $prof_313
    i64.const 1
    i64.add
    global.set $prof_313
    local.get $v
    call $value_tag
    i32.const 4
    i32.eq
    if (result i32) ;; label = @1
      local.get $v
    global.get $prof_4
    i64.const 1
    i64.add
    global.set $prof_4
      ref.cast (ref $obj)
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
      struct.get $obj 5
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
      ref.as_non_null
      array.len
      local.get $n
      i32.eq
    else
      i32.const 0
    end
  )
  (func $env_with (;270;) (type 115) (param $env (ref null $env)) (param $id i32) (param $v eqref) (result (ref null $env))
    (local $new (ref $env)) (local $n i32)
    global.get $prof_314
    i64.const 1
    i64.add
    global.set $prof_314
    local.get $env
    array.len
    local.set $n
    local.get $n
    global.get $prof_315
    i64.const 1
    i64.add
    global.set $prof_315
    array.new_default $env
    local.set $new
    local.get $new
    i32.const 0
    local.get $env
    i32.const 0
    local.get $n
    array.copy $env $env
    local.get $new
    local.get $id
    local.get $v
    array.set $env
    local.get $new
  )
  (func $env_apply (;271;) (type 116) (param $env (ref null $env)) (param $delta (ref null $values)) (result (ref null $env))
    (local $new (ref $env)) (local $pairs (ref $values)) (local $n i32) (local $i i32)
    global.get $prof_316
    i64.const 1
    i64.add
    global.set $prof_316
    local.get $delta
    ref.is_null
    if ;; label = @1
      local.get $env
      return
    end
    local.get $delta
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
    ref.as_non_null
    local.set $pairs
    local.get $env
    array.len
    local.set $n
    local.get $n
    global.get $prof_315
    i64.const 1
    i64.add
    global.set $prof_315
    array.new_default $env
    local.set $new
    local.get $new
    i32.const 0
    local.get $env
    i32.const 0
    local.get $n
    array.copy $env $env
    block $done
      loop $each
        local.get $i
        local.get $pairs
        array.len
        i32.ge_u
        br_if $done
        local.get $new
        local.get $pairs
        local.get $i
    global.get $prof_12
    i64.const 1
    i64.add
    global.set $prof_12
        array.get $values
    global.get $prof_4
    i64.const 1
    i64.add
    global.set $prof_4
        ref.cast (ref i31)
        i31.get_u
        local.get $pairs
        local.get $i
        i32.const 1
        i32.add
    global.get $prof_12
    i64.const 1
    i64.add
    global.set $prof_12
        array.get $values
        array.set $env
        local.get $i
        i32.const 2
        i32.add
        local.set $i
        br $each
      end
    end
    local.get $new
  )
  (func $call0 (;272;) (type 117) (param $f eqref) (param $env (ref null $env)) (result eqref)
    global.get $prof_317
    i64.const 1
    i64.add
    global.set $prof_317
    local.get $f
    global.get $prof_4
    i64.const 1
    i64.add
    global.set $prof_4
    ref.cast (ref $clo0)
    local.get $env
    local.get $f
    global.get $prof_4
    i64.const 1
    i64.add
    global.set $prof_4
    ref.cast (ref $clo0)
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
    struct.get $clo0 8
    global.get $prof_217
    i64.const 1
    i64.add
    global.set $prof_217
    call_ref $fn0
  )
  (func $call1 (;273;) (type 118) (param $f eqref) (param $a eqref) (param $env (ref null $env)) (result eqref)
    global.get $prof_318
    i64.const 1
    i64.add
    global.set $prof_318
    local.get $f
    global.get $prof_4
    i64.const 1
    i64.add
    global.set $prof_4
    ref.cast (ref $clo1)
    local.get $env
    local.get $a
    local.get $f
    global.get $prof_4
    i64.const 1
    i64.add
    global.set $prof_4
    ref.cast (ref $clo1)
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
    struct.get $clo1 8
    global.get $prof_217
    i64.const 1
    i64.add
    global.set $prof_217
    call_ref $fn1
  )
  (func $capture_rethrow (;274;) (type 119) (param $c (ref $cap)) (param $x exnref) (param $f (ref $frame))
    (local $n (ref $flist))
    global.get $prof_319
    i64.const 1
    i64.add
    global.set $prof_319
    local.get $f
    ref.null $flist
    ref.null $flist
    ref.null $env
    i32.const 0
    global.get $prof_320
    i64.const 1
    i64.add
    global.set $prof_320
    struct.new $flist
    local.set $n
    local.get $c
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
    struct.get $cap 3
    ref.is_null
    if ;; label = @1
      local.get $c
      local.get $n
      struct.set $cap 2
    else
      local.get $c
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
      struct.get $cap 3
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
      ref.as_non_null
      local.get $n
      struct.set $flist 1
    end
    local.get $c
    local.get $n
    struct.set $cap 3
    local.get $f
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
    struct.get $frame 3
    i32.const 1
    i32.eq
    if ;; label = @1
      local.get $c
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
      struct.get $cap 5
      ref.is_null
      if ;; label = @2
        local.get $c
        local.get $n
        struct.set $cap 4
      else
        local.get $c
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
        struct.get $cap 5
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
        ref.as_non_null
        local.get $n
        struct.set $flist 2
      end
      local.get $c
      local.get $n
      struct.set $cap 5
    end
    local.get $x
    throw_ref
  )
  (func $capture_sub (;275;) (type 120) (param $c (ref $cap)) (result (ref null $sub))
    global.get $prof_321
    i64.const 1
    i64.add
    global.set $prof_321
    local.get $c
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
    struct.get $cap 2
    ref.is_null
    if ;; label = @1
      ref.null $sub
      return
    end
    local.get $c
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
    struct.get $cap 2
    local.get $c
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
    struct.get $cap 3
    local.get $c
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
    struct.get $cap 4
    local.get $c
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
    struct.get $cap 5
    global.get $prof_322
    i64.const 1
    i64.add
    global.set $prof_322
    struct.new $sub
  )
  (func $capture_pack (;276;) (type 121) (param $c (ref $cap)) (param $x exnref) (param $f (ref $refn)) (param $first eqref)
    (local $n (ref $flist))
    global.get $prof_323
    i64.const 1
    i64.add
    global.set $prof_323
    local.get $f
    local.get $first
    local.get $c
    call $capture_sub
    global.get $prof_324
    i64.const 1
    i64.add
    global.set $prof_324
    array.new_fixed $values 2
    i32.const -1
    i32.const 3
    ref.null $values
    global.get $prof_325
    i64.const 1
    i64.add
    global.set $prof_325
    struct.new $frame
    ref.null $flist
    ref.null $flist
    ref.null $env
    i32.const 0
    global.get $prof_320
    i64.const 1
    i64.add
    global.set $prof_320
    struct.new $flist
    local.set $n
    local.get $c
    local.get $n
    struct.set $cap 2
    local.get $c
    local.get $n
    struct.set $cap 3
    local.get $c
    ref.null $flist
    struct.set $cap 4
    local.get $c
    ref.null $flist
    struct.set $cap 5
    local.get $x
    throw_ref
  )
  (func $capture_splice (;277;) (type 122) (param $c (ref $cap)) (param $p (ref null $flist)) (param $pm (ref null $flist)) (param $last (ref null $flist)) (param $mlast (ref null $flist))
    global.get $prof_326
    i64.const 1
    i64.add
    global.set $prof_326
    local.get $p
    ref.is_null
    i32.eqz
    if ;; label = @1
      local.get $c
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
      struct.get $cap 3
      ref.is_null
      if ;; label = @2
        local.get $c
        local.get $p
        struct.set $cap 2
      else
        local.get $c
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
        struct.get $cap 3
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
        ref.as_non_null
        local.get $p
        struct.set $flist 1
      end
      local.get $c
      local.get $last
      struct.set $cap 3
    end
    local.get $pm
    ref.is_null
    i32.eqz
    if ;; label = @1
      local.get $c
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
      struct.get $cap 5
      ref.is_null
      if ;; label = @2
        local.get $c
        local.get $pm
        struct.set $cap 4
      else
        local.get $c
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
        struct.get $cap 5
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
        ref.as_non_null
        local.get $pm
        struct.set $flist 2
      end
      local.get $c
      local.get $mlast
      struct.set $cap 5
    end
  )
  (func $code_count (;278;) (type 123) (param $n (ref null $flist)) (result i32)
    (local $k i32)
    global.get $prof_327
    i64.const 1
    i64.add
    global.set $prof_327
    block $done
      loop $each
        local.get $n
        ref.is_null
        br_if $done
        local.get $n
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
        ref.as_non_null
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
        struct.get $flist 0
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
        struct.get $frame 3
        i32.const 1
        i32.le_u
        if ;; label = @3
          local.get $k
          i32.const 1
          i32.add
          local.set $k
        end
        local.get $n
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
        ref.as_non_null
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
        struct.get $flist 1
        local.set $n
        br $each
      end
    end
    local.get $k
  )
  (func $code_slots (;279;) (type 124) (param $a (ref $frames)) (param $hi i32) (param $n (ref null $flist)) (result i32)
    (local $f (ref $frame))
    global.get $prof_328
    i64.const 1
    i64.add
    global.set $prof_328
    block $done
      loop $each
        local.get $n
        ref.is_null
        br_if $done
        local.get $n
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
        ref.as_non_null
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
        struct.get $flist 0
        local.set $f
        local.get $f
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
        struct.get $frame 3
        i32.const 1
        i32.le_u
        if ;; label = @3
          local.get $hi
          i32.const 1
          i32.sub
          local.set $hi
          local.get $a
          local.get $hi
          local.get $f
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
          struct.get $frame 2
          array.set $frames
        end
        local.get $n
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
        ref.as_non_null
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
        struct.get $flist 1
        local.set $n
        br $each
      end
    end
    local.get $hi
  )
  (func $shadow_insert (;280;) (type 125) (param $at i32) (param $from (ref null $flist))
    (local $n i32) (local $sp i32) (local $need i32) (local $old (ref $frames)) (local $new (ref $frames)) (local $size i32)
    global.get $prof_329
    i64.const 1
    i64.add
    global.set $prof_329
    local.get $from
    call $code_count
    local.set $n
    local.get $n
    i32.eqz
    if ;; label = @1
      return
    end
    global.get $sp
    local.set $sp
    local.get $sp
    local.get $n
    i32.add
    local.set $need
    local.get $need
    i32.const 268435456
    i32.gt_u
    if ;; label = @1
      unreachable
    end
    global.get $shadow
    local.set $old
    local.get $old
    array.len
    local.set $size
    local.get $need
    local.get $size
    i32.gt_u
    if ;; label = @1
      local.get $size
      i32.const 1
      i32.shl
      local.set $size
      local.get $need
      local.get $size
      i32.gt_u
      if ;; label = @2
        local.get $need
        local.set $size
      end
      local.get $size
    global.get $prof_191
    i64.const 1
    i64.add
    global.set $prof_191
      array.new_default $frames
      local.set $new
      local.get $new
      i32.const 0
      local.get $old
      i32.const 0
      local.get $at
      array.copy $frames $frames
      local.get $new
      local.get $at
      local.get $n
      i32.add
      local.get $old
      local.get $at
      local.get $sp
      local.get $at
      i32.sub
      array.copy $frames $frames
      local.get $new
      global.set $shadow
    else
      local.get $old
      local.get $at
      local.get $n
      i32.add
      local.get $old
      local.get $at
      local.get $sp
      local.get $at
      i32.sub
      array.copy $frames $frames
    end
    global.get $shadow
    local.get $at
    local.get $n
    i32.add
    local.get $from
    call $code_slots
    drop
    local.get $need
    global.set $sp
  )
  (func $control_unpend (;281;) (type $unpend)
    (local $sp i32) (local $n i32) (local $top i32) (local $hi i32) (local $len i32) (local $t (ref null $tramp)) (local $new (ref $frames)) (local $old (ref $frames))
    global.get $prof_330
    i64.const 1
    i64.add
    global.set $prof_330
    global.get $pend
    ref.is_null
    if ;; label = @1
      return
    end
    global.get $shadow
    local.set $old
    global.get $sp
    local.set $sp
    local.get $sp
    local.get $old
    array.len
    i32.gt_u
    if ;; label = @1
      local.get $old
      array.len
      local.set $sp
    end
    local.get $sp
    local.set $top
    global.get $pend
    local.set $t
    block $done
      loop $each
        local.get $t
        ref.is_null
        br_if $done
        local.get $t
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
        ref.as_non_null
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
        struct.get $tramp 0
        local.get $top
        i32.le_u
        if ;; label = @3
          local.get $n
          local.get $t
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
          ref.as_non_null
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
          struct.get $tramp 1
          call $code_count
          i32.add
          local.set $n
          local.get $t
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
          ref.as_non_null
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
          struct.get $tramp 0
          local.set $top
        end
        local.get $t
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
        ref.as_non_null
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
        struct.get $tramp 2
        local.set $t
        br $each
      end
    end
    local.get $n
    i32.eqz
    if ;; label = @1
      return
    end
    local.get $sp
    local.get $n
    i32.add
    global.get $prof_191
    i64.const 1
    i64.add
    global.set $prof_191
    array.new_default $frames
    local.set $new
    local.get $sp
    local.get $n
    i32.add
    local.set $hi
    local.get $sp
    local.set $top
    global.get $pend
    local.set $t
    block $done
      loop $each
        local.get $t
        ref.is_null
        br_if $done
        local.get $t
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
        ref.as_non_null
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
        struct.get $tramp 0
        local.get $top
        i32.le_u
        if ;; label = @3
          local.get $top
          local.get $t
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
          ref.as_non_null
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
          struct.get $tramp 0
          i32.sub
          local.set $len
          local.get $hi
          local.get $len
          i32.sub
          local.set $hi
          local.get $new
          local.get $hi
          local.get $old
          local.get $t
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
          ref.as_non_null
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
          struct.get $tramp 0
          local.get $len
          array.copy $frames $frames
          local.get $t
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
          ref.as_non_null
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
          struct.get $tramp 0
          local.set $top
          local.get $new
          local.get $hi
          local.get $t
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
          ref.as_non_null
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
          struct.get $tramp 1
          call $code_slots
          local.set $hi
        end
        local.get $t
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
        ref.as_non_null
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
        struct.get $tramp 2
        local.set $t
        br $each
      end
    end
    local.get $new
    i32.const 0
    local.get $old
    i32.const 0
    local.get $top
    array.copy $frames $frames
    local.get $new
    global.set $shadow
    local.get $sp
    local.get $n
    i32.add
    global.set $sp
  )
  (func $control_reset (;282;) (type $unpend)
    global.get $prof_331
    i64.const 1
    i64.add
    global.set $prof_331
    ref.null $tramp
    global.set $pend
  )
  (func $tramp_run (;283;) (type 126) (param $s (ref null $sub)) (param $env (ref null $env)) (param $v eqref) (param $mode i32) (result eqref)
    (local $node (ref null $flist)) (local $nm (ref null $flist)) (local $last (ref null $flist)) (local $mlast (ref null $flist)) (local $first (ref null $flist)) (local $from (ref null $flist)) (local $m (ref null $flist)) (local $prev (ref null $flist)) (local $next (ref null $flist)) (local $this (ref $flist)) (local $fr (ref $frame)) (local $e (ref null $env)) (local $cur (ref null $env)) (local $given (ref null $env)) (local $d i32) (local $t (ref $tramp)) (local $c (ref $cap)) (local $x exnref)
    global.get $prof_332
    i64.const 1
    i64.add
    global.set $prof_332
    local.get $s
    ref.is_null
    if ;; label = @1
      local.get $mode
      local.get $v
      return_call $ensure_end
    end
    local.get $s
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
    ref.as_non_null
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
    struct.get $sub 0
    local.set $node
    local.get $node
    ref.is_null
    if ;; label = @1
      local.get $mode
      local.get $v
      return_call $ensure_end
    end
    local.get $s
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
    ref.as_non_null
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
    struct.get $sub 1
    local.set $last
    local.get $s
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
    ref.as_non_null
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
    struct.get $sub 2
    local.set $nm
    local.get $s
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
    ref.as_non_null
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
    struct.get $sub 3
    local.set $mlast
    local.get $nm
    local.set $m
    block $done
      loop $each
        local.get $m
        ref.is_null
        br_if $done
        local.get $m
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
        ref.as_non_null
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
        struct.get $flist 2
        local.set $next
        local.get $m
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
        ref.as_non_null
        local.get $prev
        struct.set $flist 2
        local.get $m
        local.set $prev
        local.get $next
        local.set $m
        br $each
      end
    end
    local.get $prev
    local.set $m
    ref.null $flist
    local.set $prev
    local.get $env
    local.set $e
    block $done
      loop $each
        local.get $m
        ref.is_null
        br_if $done
        local.get $m
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
        ref.as_non_null
        local.get $e
        struct.set $flist 3
        local.get $e
        local.get $m
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
        ref.as_non_null
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
        struct.get $flist 0
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
        struct.get $frame 4
        call $env_apply
        local.set $e
        local.get $m
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
        ref.as_non_null
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
        struct.get $flist 2
        local.set $next
        local.get $m
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
        ref.as_non_null
        local.get $prev
        struct.set $flist 2
        local.get $m
        local.set $prev
        local.get $next
        local.set $m
        br $each
      end
    end
    local.get $e
    local.set $cur
    call $shadow_depth
    local.set $d
    local.get $d
    local.get $node
    global.get $pend
    global.get $prof_333
    i64.const 1
    i64.add
    global.set $prof_333
    struct.new $tramp
    local.set $t
    local.get $t
    global.set $pend
    ref.func $control_unpend
    global.set $stop_unpend
    local.get $node
    local.set $first
    local.get $node
    local.set $from
    block $captured (type 127) (result (ref $cap) exnref)
      loop $step
        local.get $node
        ref.is_null
        if ;; label = @3
          local.get $mode
          if ;; label = @4
            local.get $d
            local.get $from
            call $shadow_insert
          end
          local.get $t
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
          struct.get $tramp 2
          global.set $pend
          local.get $mode
          local.get $v
          return_call $ensure_end
        end
        local.get $node
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
        ref.as_non_null
        local.set $this
        local.get $this
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
        struct.get $flist 0
        local.set $fr
        local.get $this
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
        struct.get $flist 4
        if ;; label = @3
          unreachable
        end
        local.get $this
        i32.const 1
        struct.set $flist 4
        local.get $fr
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
        struct.get $frame 3
        i32.const 3
        i32.eq
        local.get $this
        local.get $first
        ref.eq
        i32.eqz
        i32.and
        if ;; label = @3
          unreachable
        end
        local.get $this
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
        struct.get $flist 1
        local.set $node
        local.get $t
        local.get $node
        struct.set $tramp 1
        local.get $this
        local.get $nm
        ref.eq
        if ;; label = @3
          local.get $this
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
          struct.get $flist 3
          local.set $given
          local.get $this
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
          struct.get $flist 2
          local.set $nm
          local.get $given
          local.set $cur
        else
          local.get $cur
          local.set $given
        end
        local.get $mode
        if ;; label = @3
          i32.const 1
          i32.const 28
          i32.const 12
          local.get $mode
          i32.const 1
          i32.eq
          select
          local.get $fr
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
          struct.get $frame 3
          i32.shr_u
          i32.and
          i32.eqz
          br_if $step
          local.get $d
          call $shadow_restore
        end
        block $cancelled (result eqref)
          block $failed (result eqref)
            try_table (result eqref) (catch $error $failed) (catch $cancel $cancelled) (catch_ref $capture $captured) ;; label = @5
              local.get $fr
              local.get $given
              local.get $v
              local.get $mode
              local.get $fr
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
              struct.get $frame 0
    global.get $prof_217
    i64.const 1
    i64.add
    global.set $prof_217
              call_ref $refn
            end
            local.set $v
            i32.const 0
            local.set $mode
            br $step
          end
          local.set $v
          local.get $node
          local.set $from
          i32.const 1
          local.set $mode
          br $step
        end
        local.set $v
        local.get $node
        local.set $from
        i32.const 2
        local.set $mode
        br $step
      end
      unreachable
    end
    local.set $x
    local.set $c
    local.get $c
    local.get $node
    local.get $nm
    local.get $last
    local.get $mlast
    call $capture_splice
    local.get $t
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
    struct.get $tramp 2
    global.set $pend
    local.get $x
    throw_ref
  )
  (func $fail_catch (;284;) (type 117) (param $body eqref) (param $env (ref null $env)) (result eqref)
    global.get $prof_334
    i64.const 1
    i64.add
    global.set $prof_334
    local.get $env
    local.get $body
    return_call $catch_run
  )
  (func $catch_run (;285;) (type 128) (param $env (ref null $env)) (param $body eqref) (result eqref)
    (local $depth i32) (local $e eqref) (local $r eqref)
    global.get $prof_335
    i64.const 1
    i64.add
    global.set $prof_335
    call $shadow_depth
    local.set $depth
    block $captured (type 127) (result (ref $cap) exnref)
      block $caught (result eqref)
        try_table (result eqref) (catch $error $caught) (catch_ref $capture $captured) ;; label = @3
          local.get $body
          local.get $env
          call $call0
        end
        local.set $r
        local.get $r
        call $result_ok
        return
      end
      local.set $e
      local.get $depth
      call $shadow_restore
      local.get $e
      call $result_error
      return
    end
    ref.func $catch_reenter
    ref.null $values
    i32.const -1
    i32.const 4
    ref.null $values
    global.get $prof_325
    i64.const 1
    i64.add
    global.set $prof_325
    struct.new $frame
    call $capture_rethrow
    unreachable
  )
  (func $catch_reenter (;286;) (type $refn) (param $frame (ref $frame)) (param $env (ref null $env)) (param $v eqref) (param $mode i32) (result eqref)
    global.get $prof_336
    i64.const 1
    i64.add
    global.set $prof_336
    local.get $mode
    i32.const 1
    i32.eq
    if ;; label = @1
      local.get $v
      return_call $result_error
    end
    local.get $v
    return_call $result_ok
  )
  (func $resource_ensure (;287;) (type 118) (param $body eqref) (param $with eqref) (param $env (ref null $env)) (result eqref)
    global.get $prof_337
    i64.const 1
    i64.add
    global.set $prof_337
    local.get $with
    local.get $env
    local.get $body
    return_call $ensure_run
  )
  (func $ensure_run (;288;) (type 129) (param $with eqref) (param $env (ref null $env)) (param $body eqref) (result eqref)
    (local $depth i32) (local $r eqref)
    global.get $prof_338
    i64.const 1
    i64.add
    global.set $prof_338
    call $shadow_depth
    local.set $depth
    block $captured (type 127) (result (ref $cap) exnref)
      block $cancelled (result eqref)
        block $failed (result eqref)
          try_table (result eqref) (catch $error $failed) (catch $cancel $cancelled) (catch_ref $capture $captured) ;; label = @4
            local.get $body
            local.get $env
            call $call0
          end
          local.set $r
          local.get $with
          local.get $env
          i32.const 0
          local.get $r
          return_call $ensure_final
        end
        local.set $r
        local.get $depth
        call $shadow_restore
        local.get $with
        local.get $env
        i32.const 1
        local.get $r
        return_call $ensure_final
      end
      local.set $r
      local.get $depth
      call $shadow_restore
      local.get $with
      local.get $env
      i32.const 2
      local.get $r
      return_call $ensure_final
    end
    ref.func $ensure_reenter
    local.get $with
    global.get $prof_247
    i64.const 1
    i64.add
    global.set $prof_247
    array.new_fixed $values 1
    i32.const -1
    i32.const 2
    ref.null $values
    global.get $prof_325
    i64.const 1
    i64.add
    global.set $prof_325
    struct.new $frame
    call $capture_rethrow
    unreachable
  )
  (func $ensure_reenter (;289;) (type $refn) (param $frame (ref $frame)) (param $env (ref null $env)) (param $v eqref) (param $mode i32) (result eqref)
    global.get $prof_339
    i64.const 1
    i64.add
    global.set $prof_339
    local.get $frame
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
    struct.get $frame 1
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
    ref.as_non_null
    i32.const 0
    global.get $prof_12
    i64.const 1
    i64.add
    global.set $prof_12
    array.get $values
    local.get $env
    local.get $mode
    local.get $v
    return_call $ensure_final
  )
  (func $ensure_final (;290;) (type 130) (param $with eqref) (param $env (ref null $env)) (param $mode i32) (param $r eqref) (result eqref)
    global.get $prof_340
    i64.const 1
    i64.add
    global.set $prof_340
    block $captured (type 127) (result (ref $cap) exnref)
      try_table (result eqref) (catch_ref $capture $captured) ;; label = @2
        local.get $with
        local.get $env
        call $call0
      end
      drop
      local.get $mode
      local.get $r
      return_call $ensure_end
    end
    ref.func $final_reenter
    local.get $r
    local.get $mode
    global.get $prof_14
    i64.const 1
    i64.add
    global.set $prof_14
    ref.i31
    global.get $prof_324
    i64.const 1
    i64.add
    global.set $prof_324
    array.new_fixed $values 2
    i32.const -1
    i32.const 5
    ref.null $values
    global.get $prof_325
    i64.const 1
    i64.add
    global.set $prof_325
    struct.new $frame
    call $capture_rethrow
    unreachable
  )
  (func $ensure_end (;291;) (type 131) (param $mode i32) (param $r eqref) (result eqref)
    global.get $prof_341
    i64.const 1
    i64.add
    global.set $prof_341
    local.get $mode
    i32.const 1
    i32.eq
    if ;; label = @1
      local.get $r
      throw $error
    end
    local.get $mode
    i32.const 2
    i32.eq
    if ;; label = @1
      local.get $r
      throw $cancel
    end
    local.get $r
  )
  (func $final_reenter (;292;) (type $refn) (param $frame (ref $frame)) (param $env (ref null $env)) (param $v eqref) (param $mode i32) (result eqref)
    (local $saved (ref $values))
    global.get $prof_342
    i64.const 1
    i64.add
    global.set $prof_342
    local.get $frame
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
    struct.get $frame 1
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
    ref.as_non_null
    local.set $saved
    local.get $saved
    i32.const 1
    global.get $prof_12
    i64.const 1
    i64.add
    global.set $prof_12
    array.get $values
    global.get $prof_4
    i64.const 1
    i64.add
    global.set $prof_4
    ref.cast (ref i31)
    i31.get_u
    local.get $saved
    i32.const 0
    global.get $prof_12
    i64.const 1
    i64.add
    global.set $prof_12
    array.get $values
    return_call $ensure_end
  )
  (func $control_boundary (;293;) (type 117) (param $body eqref) (param $env (ref null $env)) (result eqref)
    global.get $prof_343
    i64.const 1
    i64.add
    global.set $prof_343
    i32.const 11
    i32.const 0
    i64.const 0
    f64.const 0x0p+0 (;=0;)
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    i32.const 1
    global.get $prof_344
    i64.const 1
    i64.add
    global.set $prof_344
    struct.new $boundary
    local.get $env
    local.get $body
    ref.null $sub
    ref.null eq
    i32.const 0
    return_call $boundary_core
  )
  (func $boundary_run (;294;) (type 132) (param $b (ref $boundary)) (param $env (ref null $env)) (param $body eqref) (param $rest (ref null $flist)) (param $v eqref) (param $cancel eqref) (param $frame (ref null $frame)) (result eqref)
    global.get $prof_345
    i64.const 1
    i64.add
    global.set $prof_345
    local.get $rest
    ref.is_null
    i32.eqz
    if ;; label = @1
      unreachable
    end
    local.get $cancel
    ref.is_null
    i32.eqz
    if ;; label = @1
      unreachable
    end
    local.get $frame
    ref.is_null
    i32.eqz
    if ;; label = @1
      unreachable
    end
    local.get $b
    local.get $env
    local.get $body
    ref.null $sub
    local.get $v
    i32.const 0
    return_call $boundary_core
  )
  (func $boundary_core (;295;) (type 133) (param $b (ref $boundary)) (param $env (ref null $env)) (param $body eqref) (param $sub (ref null $sub)) (param $v eqref) (param $mode i32) (result eqref)
    (local $depth i32) (local $r eqref) (local $c (ref null $cap)) (local $x exnref) (local $f eqref)
    global.get $prof_346
    i64.const 1
    i64.add
    global.set $prof_346
    call $shadow_depth
    local.set $depth
    block $unwind (type 134) (result eqref exnref)
      block $captured (type 127) (result (ref $cap) exnref)
        try_table (result eqref) (catch_ref $capture $captured) (catch_ref $error $unwind) (catch_ref $cancel $unwind) ;; label = @3
          local.get $body
          ref.is_null
          if (result eqref) ;; label = @4
            local.get $sub
            local.get $env
            local.get $v
            local.get $mode
            call $tramp_run
          else
            local.get $body
            local.get $b
            local.get $env
            call $call1
          end
        end
        local.set $r
        local.get $b
        i32.const 0
        struct.set $boundary 8
        local.get $r
        return
      end
      local.set $x
      local.set $c
      local.get $b
      i32.const 0
      struct.set $boundary 8
      local.get $c
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
      struct.get $cap 0
      local.get $b
      ref.eq
      if ;; label = @2
        local.get $depth
        call $shadow_restore
        local.get $c
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
        struct.get $cap 1
        local.set $f
        local.get $f
    global.get $prof_4
    i64.const 1
    i64.add
    global.set $prof_4
        ref.cast (ref $clo1)
        local.get $env
        i32.const 12
        i32.const 0
        i64.const 0
        f64.const 0x0p+0 (;=0;)
        ref.null $bytes
        ref.null $values
        ref.null $limbs
        ref.null $call
        i32.const 0
        local.get $b
        local.get $c
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
        ref.as_non_null
        call $capture_sub
    global.get $prof_347
    i64.const 1
    i64.add
    global.set $prof_347
        struct.new $cont
        local.get $f
    global.get $prof_4
    i64.const 1
    i64.add
    global.set $prof_4
        ref.cast (ref $clo1)
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
        struct.get $clo1 8
    global.get $prof_348
    i64.const 1
    i64.add
    global.set $prof_348
        return_call_ref $fn1
      end
      local.get $c
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
      ref.as_non_null
      local.get $x
      ref.func $boundary_reenter
      local.get $b
      call $capture_pack
      unreachable
    end
    local.set $x
    drop
    local.get $b
    i32.const 0
    struct.set $boundary 8
    local.get $x
    throw_ref
  )
  (func $boundary_reenter (;296;) (type $refn) (param $frame (ref $frame)) (param $env (ref null $env)) (param $v eqref) (param $mode i32) (result eqref)
    (local $saved (ref $values)) (local $b (ref $boundary))
    global.get $prof_349
    i64.const 1
    i64.add
    global.set $prof_349
    local.get $frame
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
    struct.get $frame 1
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
    ref.as_non_null
    local.set $saved
    local.get $saved
    i32.const 0
    global.get $prof_12
    i64.const 1
    i64.add
    global.set $prof_12
    array.get $values
    global.get $prof_4
    i64.const 1
    i64.add
    global.set $prof_4
    ref.cast (ref $boundary)
    local.set $b
    local.get $b
    i32.const 1
    struct.set $boundary 8
    local.get $b
    local.get $env
    ref.null eq
    local.get $saved
    i32.const 1
    global.get $prof_12
    i64.const 1
    i64.add
    global.set $prof_12
    array.get $values
    global.get $prof_4
    i64.const 1
    i64.add
    global.set $prof_4
    ref.cast (ref null $sub)
    local.get $v
    local.get $mode
    return_call $boundary_core
  )
  (func $control_suspend (;297;) (type $call) (param $b eqref) (param $f eqref) (result eqref)
    (local $boundary (ref $boundary))
    global.get $prof_350
    i64.const 1
    i64.add
    global.set $prof_350
    local.get $b
    global.get $prof_4
    i64.const 1
    i64.add
    global.set $prof_4
    ref.cast (ref $boundary)
    local.set $boundary
    local.get $boundary
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
    struct.get $boundary 8
    i32.eqz
    if ;; label = @1
      call $stop_boundary_inactive
      unreachable
    end
    local.get $boundary
    local.get $f
    ref.null $flist
    ref.null $flist
    ref.null $flist
    ref.null $flist
    global.get $prof_351
    i64.const 1
    i64.add
    global.set $prof_351
    struct.new $cap
    throw $capture
  )
  (func $continuation_take (;298;) (type 135) (param $k eqref) (result (ref $cont))
    (local $c (ref $cont))
    global.get $prof_352
    i64.const 1
    i64.add
    global.set $prof_352
    local.get $k
    global.get $prof_4
    i64.const 1
    i64.add
    global.set $prof_4
    ref.cast (ref $cont)
    local.set $c
    local.get $c
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
    struct.get $cont 8
    if ;; label = @1
      call $stop_continuation_used
      unreachable
    end
    local.get $c
    i32.const 1
    struct.set $cont 8
    local.get $c
  )
  (func $control_resume (;299;) (type 118) (param $k eqref) (param $v eqref) (param $env (ref null $env)) (result eqref)
    (local $c (ref $cont)) (local $b (ref $boundary))
    global.get $prof_353
    i64.const 1
    i64.add
    global.set $prof_353
    local.get $k
    call $continuation_take
    local.set $c
    local.get $c
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
    struct.get $cont 9
    local.set $b
    local.get $b
    i32.const 1
    struct.set $boundary 8
    local.get $b
    local.get $env
    ref.null eq
    local.get $c
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
    struct.get $cont 10
    local.get $v
    i32.const 0
    return_call $boundary_core
  )
  (func $control_cancel (;300;) (type 117) (param $k eqref) (param $env (ref null $env)) (result eqref)
    (local $c (ref $cont))
    global.get $prof_354
    i64.const 1
    i64.add
    global.set $prof_354
    local.get $k
    call $continuation_take
    local.set $c
    local.get $c
    local.get $c
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
    struct.get $cont 10
    local.get $env
    local.get $c
    i32.const 2
    return_call $cancel_core
  )
  (func $cancel_core (;301;) (type 136) (param $k (ref $cont)) (param $sub (ref null $sub)) (param $env (ref null $env)) (param $v eqref) (param $mode i32) (result eqref)
    (local $depth i32) (local $e eqref) (local $x exnref) (local $c (ref null $cap))
    global.get $prof_355
    i64.const 1
    i64.add
    global.set $prof_355
    call $shadow_depth
    local.set $depth
    block $captured (type 127) (result (ref $cap) exnref)
      block $cancelled (type 134) (result eqref exnref)
        try_table (result eqref) (catch_ref $cancel $cancelled) (catch_ref $capture $captured) ;; label = @3
          local.get $sub
          local.get $env
          local.get $v
          local.get $mode
          call $tramp_run
        end
        drop
        ref.null eq
        return
      end
      local.set $x
      local.set $e
      local.get $e
      local.get $k
      ref.eq
      i32.eqz
      if ;; label = @2
        local.get $x
        throw_ref
      end
      local.get $depth
      call $shadow_restore
      ref.null eq
      return
    end
    local.set $x
    local.set $c
    local.get $c
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
    ref.as_non_null
    local.get $x
    ref.func $cancel_reenter
    local.get $k
    call $capture_pack
    unreachable
  )
  (func $cancel_reenter (;302;) (type $refn) (param $frame (ref $frame)) (param $env (ref null $env)) (param $v eqref) (param $mode i32) (result eqref)
    (local $saved (ref $values))
    global.get $prof_356
    i64.const 1
    i64.add
    global.set $prof_356
    local.get $frame
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
    struct.get $frame 1
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
    ref.as_non_null
    local.set $saved
    local.get $saved
    i32.const 0
    global.get $prof_12
    i64.const 1
    i64.add
    global.set $prof_12
    array.get $values
    global.get $prof_4
    i64.const 1
    i64.add
    global.set $prof_4
    ref.cast (ref $cont)
    local.get $saved
    i32.const 1
    global.get $prof_12
    i64.const 1
    i64.add
    global.set $prof_12
    array.get $values
    global.get $prof_4
    i64.const 1
    i64.add
    global.set $prof_4
    ref.cast (ref null $sub)
    local.get $env
    local.get $v
    local.get $mode
    return_call $cancel_core
  )
  (func $host_wait (;303;) (type 45) (param $k eqref) (result eqref)
    global.get $prof_357
    i64.const 1
    i64.add
    global.set $prof_357
    local.get $k
    global.set $host_k
    global.get $host_waiting
  )
  (func $host_root_boundary (;304;) (type 59) (result eqref)
    global.get $prof_358
    i64.const 1
    i64.add
    global.set $prof_358
    global.get $host_root
    ref.is_null
    if ;; label = @1
      unreachable
    end
    global.get $host_root
  )
  (func $host_boundary (;305;) (type 117) (param $body eqref) (param $env (ref null $env)) (result eqref)
    (local $b (ref $boundary))
    global.get $prof_359
    i64.const 1
    i64.add
    global.set $prof_359
    i32.const 11
    i32.const 0
    i64.const 0
    f64.const 0x0p+0 (;=0;)
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    i32.const 1
    global.get $prof_344
    i64.const 1
    i64.add
    global.set $prof_344
    struct.new $boundary
    local.set $b
    local.get $b
    global.set $host_root
    local.get $b
    local.get $env
    local.get $body
    ref.null $flist
    ref.null eq
    ref.null eq
    ref.null $frame
    return_call $boundary_run
  )
  (func $req_start (;306;) (type 35) (param $import i32) (result i32)
    (local $id i32)
    global.get $prof_360
    i64.const 1
    i64.add
    global.set $prof_360
    global.get $host_import
    i32.const -1
    i32.ne
    if ;; label = @1
      unreachable
    end
    global.get $host_next
    local.set $id
    local.get $id
    i32.const 1
    i32.add
    i32.const 2147483647
    i32.and
    global.set $host_next
    local.get $import
    global.set $host_import
    local.get $id
    global.set $host_request
    local.get $id
  )
  (func $req_answer (;307;) (type 33) (param $import i32) (param $id i32)
    global.get $prof_361
    i64.const 1
    i64.add
    global.set $prof_361
    global.get $host_import
    local.get $import
    i32.ne
    global.get $host_request
    local.get $id
    i32.ne
    i32.or
    if ;; label = @1
      unreachable
    end
    i32.const -1
    global.set $host_import
    i32.const -1
    global.set $host_request
  )
  (func $req_outstanding (;308;) (type 101) (result i32)
    global.get $prof_362
    i64.const 1
    i64.add
    global.set $prof_362
    global.get $host_import
    i32.const -1
    i32.ne
  )
  (func $req_clear (;309;) (type $unpend)
    global.get $prof_363
    i64.const 1
    i64.add
    global.set $prof_363
    i32.const -1
    global.set $host_import
    i32.const -1
    global.set $host_request
  )
  (func $host_resume_k (;310;) (type 59) (result eqref)
    (local $k eqref)
    global.get $prof_364
    i64.const 1
    i64.add
    global.set $prof_364
    global.get $host_k
    local.set $k
    ref.null eq
    global.set $host_k
    local.get $k
    ref.null eq
    global.get $root_env
    return_call $control_resume
  )
  (func $async_begin (;311;) (type 33) (param $export i32) (param $site i32)
    global.get $prof_365
    i64.const 1
    i64.add
    global.set $prof_365
    global.get $async_live
    if ;; label = @1
      unreachable
    end
    i32.const 1
    global.set $async_live
    local.get $export
    global.set $async_export
    local.get $site
    global.set $async_site
    ref.null eq
    global.set $async_value
    i32.const 0
    global.set $async_error
  )
  (func $async_result (;312;) (type $unpend)
    global.get $prof_366
    i64.const 1
    i64.add
    global.set $prof_366
    global.get $async_live
    if ;; label = @1
      unreachable
    end
  )
  (func $async_run (;313;) (type 36) (param $body eqref) (result i32)
    (local $r eqref) (local $e eqref) (local $depth i32)
    global.get $prof_367
    i64.const 1
    i64.add
    global.set $prof_367
    call $shadow_depth
    local.set $depth
    block $error (result eqref)
      block $cancel (result eqref)
        try_table (result eqref) (catch $error $error) (catch $cancel $cancel) ;; label = @3
          local.get $body
          ref.is_null
          if (result eqref) ;; label = @4
            call $host_resume_k
          else
            local.get $body
            global.get $root_env
            call $host_boundary
          end
        end
        local.set $r
        local.get $r
        global.get $host_waiting
        ref.eq
        if ;; label = @3
          i32.const 1
          return
        end
        local.get $r
        global.set $async_value
        call $req_clear
        i32.const 0
        global.set $async_live
        i32.const 0
        return
      end
      drop
      call $stop_boundary_inactive
      unreachable
    end
    local.set $e
    global.get $async_export
    i32.eqz
    if ;; label = @1
      local.get $e
      global.get $async_site
      call $stop_error
      unreachable
    end
    local.get $depth
    call $shadow_restore
    local.get $e
    global.set $async_value
    i32.const 1
    global.set $async_error
    call $req_clear
    i32.const 0
    global.set $async_live
    i32.const 0
  )
  (func $entry_assert_fail (;314;) (type $fn1) (param $clo (ref null $obj)) (param $env (ref null $env)) (param $failure eqref) (result eqref)
    global.get $prof_368
    i64.const 1
    i64.add
    global.set $prof_368
    local.get $failure
    i32.const 0
    call $field
    call $stop_assertion
    unreachable
  )
  (func $f0 (;315;) (type $fn1) (param $clo (ref null $obj)) (param $env (ref null $env)) (param $l0 eqref) (result eqref)
    (local $l1 eqref) (local $slot i32)
    global.get $prof_369
    i64.const 1
    i64.add
    global.set $prof_369
    call $shadow_enter_call
    local.get $l0
    local.set $l1
    call $shadow_leave
    local.get $l1
    return
    unreachable
  )
  (func $f1 (;316;) (type $fn1) (param $clo (ref null $obj)) (param $env (ref null $env)) (param $l0 eqref) (result eqref)
    (local $l1 eqref) (local $slot i32)
    global.get $prof_370
    i64.const 1
    i64.add
    global.set $prof_370
    call $shadow_enter_call
    local.get $l0
    local.set $l1
    i32.const -30
    call $shadow_leave_tail
    ref.null $obj
    local.get $env
    local.get $l1
    return_call $f12
    unreachable
  )
  (func $f2 (;317;) (type $fn1) (param $clo (ref null $obj)) (param $env (ref null $env)) (param $l0 eqref) (result eqref)
    (local $l1 eqref) (local $l2 eqref) (local $l3 eqref) (local $l4 eqref) (local $l5 eqref) (local $l6 eqref) (local $l7 eqref) (local $l8 eqref) (local $l9 eqref) (local $l10 eqref) (local $l11 eqref) (local $l12 eqref) (local $l13 eqref) (local $l14 eqref) (local $l15 eqref) (local $l16 eqref) (local $l17 eqref) (local $l18 eqref) (local $l19 eqref) (local $l20 eqref) (local $l21 eqref) (local $slot i32)
    global.get $prof_371
    i64.const 1
    i64.add
    global.set $prof_371
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
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
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
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
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
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
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
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
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
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
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
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
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
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
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
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    call $float_add
    local.set $l21
    call $shadow_leave
    local.get $l21
    return
    unreachable
  )
  (func $f3 (;318;) (type $fn1) (param $clo (ref null $obj)) (param $env (ref null $env)) (param $l0 eqref) (result eqref)
    (local $l1 eqref) (local $slot i32)
    global.get $prof_372
    i64.const 1
    i64.add
    global.set $prof_372
    call $shadow_enter_call
    local.get $l0
    local.set $l1
    i32.const -19
    call $shadow_leave_tail
    ref.null $obj
    local.get $env
    global.get $g142
    global.get $g142
    local.get $l1
    return_call $f11
    unreachable
  )
  (func $f4 (;319;) (type $fn1) (param $clo (ref null $obj)) (param $env (ref null $env)) (param $l0 eqref) (result eqref)
    (local $l1 eqref) (local $l2 eqref) (local $l3 eqref) (local $l4 eqref) (local $slot i32)
    global.get $prof_373
    i64.const 1
    i64.add
    global.set $prof_373
    call $shadow_enter_call
    local.get $l0
    local.set $l1
    i32.const 13
    call $shadow_site
    local.get $l1
    call $core_truncate
    local.set $l2
    i32.const 14
    call $shadow_site
    ref.null $obj
    local.get $env
    i32.const 128
    global.get $prof_14
    i64.const 1
    i64.add
    global.set $prof_14
    ref.i31
    local.get $l2
    call $f10
    local.set $l3
    i32.const -17
    call $shadow_site
    local.get $l3
    call $core_to_float
    local.set $l4
    call $shadow_leave
    local.get $l4
    return
    unreachable
  )
  (func $f5 (;320;) (type $fn1) (param $clo (ref null $obj)) (param $env (ref null $env)) (param $l0 eqref) (result eqref)
    (local $l1 eqref) (local $l2 eqref) (local $l3 eqref) (local $l4 eqref) (local $l5 eqref) (local $l6 eqref) (local $l7 eqref) (local $l8 eqref) (local $l9 eqref) (local $l10 eqref) (local $l11 eqref) (local $l12 eqref) (local $l13 eqref) (local $l14 eqref) (local $l15 eqref) (local $l16 eqref) (local $l17 eqref) (local $l18 eqref) (local $l19 eqref) (local $slot i32)
    global.get $prof_374
    i64.const 1
    i64.add
    global.set $prof_374
    call $shadow_enter_call
    local.get $l0
    local.set $l1
    i32.const 5
    call $shadow_site
    ref.null $obj
    local.get $env
    local.get $l1
    call $f9
    local.set $l2
    local.get $l2
    i32.const 0
    call $field
    local.set $l3
    local.get $l3
    local.set $l4
    local.get $l2
    i32.const 1
    call $field
    local.set $l5
    local.get $l5
    local.set $l6
    i32.const 6
    call $shadow_site
    local.get $l4
    local.get $l6
    call $float_add
    local.set $l7
    i32.const 7
    call $shadow_site
    ref.null $obj
    local.get $env
    local.get $l7
    call $f9
    local.set $l8
    local.get $l8
    i32.const 0
    call $field
    local.set $l9
    local.get $l9
    local.set $l10
    local.get $l8
    i32.const 1
    call $field
    local.set $l11
    local.get $l11
    local.set $l12
    i32.const 8
    call $shadow_site
    local.get $l10
    local.get $l12
    call $float_add
    local.set $l13
    i32.const 9
    call $shadow_site
    ref.null $obj
    local.get $env
    local.get $l13
    call $f9
    local.set $l14
    local.get $l14
    i32.const 0
    call $field
    local.set $l15
    local.get $l15
    local.set $l16
    local.get $l14
    i32.const 1
    call $field
    local.set $l17
    local.get $l17
    local.set $l18
    i32.const 10
    call $shadow_site
    local.get $l16
    local.get $l18
    call $float_add
    local.set $l19
    call $shadow_leave
    local.get $l19
    return
    unreachable
  )
  (func $f6 (;321;) (type $fn1) (param $clo (ref null $obj)) (param $env (ref null $env)) (param $l0 eqref) (result eqref)
    (local $l1 eqref) (local $l2 eqref) (local $slot i32)
    global.get $prof_375
    i64.const 1
    i64.add
    global.set $prof_375
    call $shadow_enter_call
    local.get $l0
    local.set $l1
    i32.const 7
    i32.const 0
    i64.const 2
    f64.const 0x0p+0 (;=0;)
    ref.null $bytes
    local.get $l1
    global.get $prof_247
    i64.const 1
    i64.add
    global.set $prof_247
    array.new_fixed $values 1
    ref.null $limbs
    ref.null $call
    ref.func $f8
    global.get $prof_376
    i64.const 1
    i64.add
    global.set $prof_376
    struct.new $clo2
    local.set $l2
    i32.const -4
    call $shadow_leave_tail
    ref.null $obj
    local.get $env
    global.get $g142
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 0
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    local.get $l2
    return_call $f7
    unreachable
  )
  (func $f7 (;322;) (type $fn3) (param $clo (ref null $obj)) (param $env (ref null $env)) (param $l0 eqref) (param $l1 eqref) (param $l2 eqref) (result eqref)
    (local $l3 eqref) (local $l4 eqref) (local $l5 eqref) (local $l6 eqref) (local $l7 eqref) (local $l8 eqref) (local $l9 eqref) (local $l10 eqref) (local $slot i32)
    global.get $prof_377
    i64.const 1
    i64.add
    global.set $prof_377
    call $shadow_enter_call
    local.get $l0
    local.set $l3
    local.get $l1
    local.set $l4
    local.get $l2
    local.set $l5
    block $b0
      block $b1
        local.get $l3
        call $value_tag
        i32.const 6
        i32.eq
        i32.eqz
        if ;; label = @3
          br $b1
        else
        end
        local.get $l3
        call $list_is_empty
        i32.eqz
        if ;; label = @3
          br $b1
        else
        end
        call $shadow_leave
        local.get $l4
        return
      end
      block $b2
        local.get $l3
        call $value_tag
        i32.const 6
        i32.eq
        i32.eqz
        if ;; label = @3
          br $b2
        else
        end
        local.get $l3
        call $list_is_empty
        i32.eqz
        i32.eqz
        if ;; label = @3
          br $b2
        else
        end
        local.get $l3
        call $list_head
        local.set $l6
        local.get $l6
        local.set $l7
        local.get $l3
        call $list_tail
        local.set $l8
        local.get $l8
        local.set $l9
        i32.const 3
        call $shadow_site
        call $shadow_top
        local.set $slot
        block $ok0 (result eqref)
          block $cap0 (type 127) (result (ref $cap) exnref)
            try_table (result eqref) (catch_ref $capture $cap0) ;; label = @5
              local.get $l5
    global.get $prof_4
    i64.const 1
    i64.add
    global.set $prof_4
              ref.cast (ref $clo2)
              local.get $env
              local.get $l4
              local.get $l7
              local.get $l5
    global.get $prof_4
    i64.const 1
    i64.add
    global.set $prof_4
              ref.cast (ref $clo2)
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
              struct.get $clo2 8
    global.get $prof_217
    i64.const 1
    i64.add
    global.set $prof_217
              call_ref $fn2
            end
            br $ok0
          end
          ref.func $r0
          local.get $l5
          local.get $l9
    global.get $prof_324
    i64.const 1
    i64.add
    global.set $prof_324
          array.new_fixed $values 2
          local.get $slot
          i32.const 0
          ref.null $values
    global.get $prof_325
    i64.const 1
    i64.add
    global.set $prof_325
          struct.new $frame
          call $capture_rethrow
          unreachable
        end
        local.set $l10
        i32.const -6
        call $shadow_leave_tail
        ref.null $obj
        local.get $env
        local.get $l9
        local.get $l10
        local.get $l5
        return_call $f7
      end
      unreachable
    end
    unreachable
  )
  (func $f8 (;323;) (type $fn2) (param $clo (ref null $obj)) (param $env (ref null $env)) (param $l0 eqref) (param $l1 eqref) (result eqref)
    (local $l2 eqref) (local $l3 eqref) (local $l4 eqref) (local $l5 eqref) (local $l6 eqref) (local $slot i32)
    global.get $prof_378
    i64.const 1
    i64.add
    global.set $prof_378
    call $shadow_enter_call
    local.get $clo
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
    ref.as_non_null
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
    struct.get $obj 5
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
    ref.as_non_null
    i32.const 0
    global.get $prof_12
    i64.const 1
    i64.add
    global.set $prof_12
    array.get $values
    local.set $l2
    local.get $l0
    local.set $l3
    local.get $l1
    local.set $l4
    i32.const 0
    call $shadow_site
    local.get $l4
    local.get $l2
    call $float_mul
    local.set $l5
    i32.const 1
    call $shadow_site
    local.get $l3
    local.get $l5
    call $float_add
    local.set $l6
    call $shadow_leave
    local.get $l6
    return
    unreachable
  )
  (func $f9 (;324;) (type $fn1) (param $clo (ref null $obj)) (param $env (ref null $env)) (param $l0 eqref) (result eqref)
    (local $l1 eqref) (local $l2 eqref) (local $l3 eqref) (local $l4 eqref) (local $slot i32)
    global.get $prof_379
    i64.const 1
    i64.add
    global.set $prof_379
    call $shadow_enter_call
    local.get $l0
    local.set $l1
    i32.const 11
    call $shadow_site
    local.get $l1
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4598175219545276416
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    call $float_add
    local.set $l2
    i32.const 12
    call $shadow_site
    local.get $l1
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4602678819172646912
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    call $float_mul
    local.set $l3
    local.get $l2
    local.get $l3
    global.get $prof_324
    i64.const 1
    i64.add
    global.set $prof_324
    array.new_fixed $values 2
    call $tuple
    local.set $l4
    call $shadow_leave
    local.get $l4
    return
    unreachable
  )
  (func $f10 (;325;) (type $fn2) (param $clo (ref null $obj)) (param $env (ref null $env)) (param $l0 eqref) (param $l1 eqref) (result eqref)
    (local $l2 eqref) (local $l3 eqref) (local $l4 eqref) (local $l5 eqref) (local $l6 eqref) (local $slot i32)
    global.get $prof_380
    i64.const 1
    i64.add
    global.set $prof_380
    call $shadow_enter_call
    local.get $l0
    local.set $l2
    local.get $l1
    local.set $l3
    local.get $l2
    i32.const 0
    global.get $prof_14
    i64.const 1
    i64.add
    global.set $prof_14
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
    global.get $prof_14
    i64.const 1
    i64.add
    global.set $prof_14
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
  (func $f11 (;326;) (type $fn3) (param $clo (ref null $obj)) (param $env (ref null $env)) (param $l0 eqref) (param $l1 eqref) (param $l2 eqref) (result eqref)
    (local $l3 eqref) (local $l4 eqref) (local $l5 eqref) (local $l6 eqref) (local $l7 eqref) (local $l8 eqref) (local $l9 eqref) (local $l10 eqref) (local $l11 eqref) (local $l12 eqref) (local $l13 eqref) (local $l14 eqref) (local $l15 eqref) (local $slot i32)
    global.get $prof_381
    i64.const 1
    i64.add
    global.set $prof_381
    call $shadow_enter_call
    local.get $l0
    local.set $l3
    local.get $l1
    local.set $l4
    local.get $l2
    local.set $l5
    block $b0
      block $b1
        local.get $l3
        call $value_tag
        i32.const 6
        i32.eq
        i32.eqz
        if ;; label = @3
          br $b1
        else
        end
        local.get $l3
        call $list_is_empty
        i32.eqz
        if ;; label = @3
          br $b1
        else
        end
        call $shadow_leave
        local.get $l5
        return
      end
      block $b2
        local.get $l3
        call $value_tag
        i32.const 6
        i32.eq
        i32.eqz
        if ;; label = @3
          br $b2
        else
        end
        local.get $l3
        call $list_is_empty
        i32.eqz
        i32.eqz
        if ;; label = @3
          br $b2
        else
        end
        local.get $l3
        call $list_head
        local.set $l6
        local.get $l6
        local.set $l7
        local.get $l3
        call $list_tail
        local.set $l8
        local.get $l8
        local.set $l9
        block $b3
          block $b4
            local.get $l4
            call $value_tag
            i32.const 6
            i32.eq
            i32.eqz
            if ;; label = @5
              br $b4
            else
            end
            local.get $l4
            call $list_is_empty
            i32.eqz
            if ;; label = @5
              br $b4
            else
            end
            call $shadow_leave
            local.get $l5
            return
          end
          block $b5
            local.get $l4
            call $value_tag
            i32.const 6
            i32.eq
            i32.eqz
            if ;; label = @5
              br $b5
            else
            end
            local.get $l4
            call $list_is_empty
            i32.eqz
            i32.eqz
            if ;; label = @5
              br $b5
            else
            end
            local.get $l4
            call $list_head
            local.set $l10
            local.get $l10
            local.set $l11
            local.get $l4
            call $list_tail
            local.set $l12
            local.get $l12
            local.set $l13
            i32.const 18
            call $shadow_site
            local.get $l7
            local.get $l11
            call $float_mul
            local.set $l14
            i32.const 19
            call $shadow_site
            local.get $l5
            local.get $l14
            call $float_add
            local.set $l15
            i32.const -22
            call $shadow_leave_tail
            ref.null $obj
            local.get $env
            local.get $l9
            local.get $l13
            local.get $l15
            return_call $f11
          end
          unreachable
        end
      end
      unreachable
    end
    unreachable
  )
  (func $f12 (;327;) (type $fn1) (param $clo (ref null $obj)) (param $env (ref null $env)) (param $l0 eqref) (result eqref)
    (local $l1 eqref) (local $l2 eqref) (local $l3 eqref) (local $l4 eqref) (local $slot i32)
    global.get $prof_382
    i64.const 1
    i64.add
    global.set $prof_382
    call $shadow_enter_call
    local.get $l0
    local.set $l1
    i32.const 29
    call $shadow_site
    ref.null $obj
    local.get $env
    local.get $l1
    global.get $value_false
    call $f13
    local.set $l2
    block $b0
      block $b1
        local.get $l2
        i32.const 88
        call $is_ctor
        i32.eqz
        if ;; label = @3
          br $b1
        else
        end
        local.get $l2
        i32.const 0
        call $field
        local.set $l3
        local.get $l3
        local.set $l4
        call $shadow_leave
        local.get $l4
        return
      end
      block $b2
        local.get $l2
        i32.const 89
        call $is_ctor
        i32.eqz
        if ;; label = @3
          br $b2
        else
        end
        call $shadow_leave
        local.get $l1
        return
      end
      unreachable
    end
    unreachable
  )
  (func $f13 (;328;) (type $fn2) (param $clo (ref null $obj)) (param $env (ref null $env)) (param $l0 eqref) (param $l1 eqref) (result eqref)
    (local $l2 eqref) (local $l3 eqref) (local $l4 eqref) (local $l5 eqref) (local $l6 eqref) (local $l7 eqref) (local $l8 eqref) (local $l9 eqref) (local $l10 eqref) (local $slot i32)
    global.get $prof_383
    i64.const 1
    i64.add
    global.set $prof_383
    call $shadow_enter_call
    local.get $l0
    local.set $l2
    local.get $l1
    local.set $l3
    i32.const 30
    call $shadow_site
    ref.null $obj
    local.get $env
    local.get $l2
    call $f14
    local.set $l4
    block $b0
      block $b1
        local.get $l4
        i32.const 89
        call $is_ctor
        i32.eqz
        if ;; label = @3
          br $b1
        else
        end
        call $shadow_leave
        global.get $k89_49
        return
      end
      block $b2
        local.get $l4
        i32.const 88
        call $is_ctor
        i32.eqz
        if ;; label = @3
          br $b2
        else
        end
        local.get $l4
        i32.const 0
        call $field
        local.set $l5
        local.get $l5
        local.set $l6
        i32.const 31
        call $shadow_site
        ref.null $obj
        local.get $env
        local.get $l6
        local.get $l3
        call $f16
        local.set $l7
        local.get $l6
        i32.const 0
        call $field
        local.set $l8
        i32.const 32
        call $shadow_site
        ref.null $obj
        local.get $env
        local.get $l8
        call $f17
        local.set $l9
        i32.const 32
        call $shadow_site
        local.get $l9
        global.get $g14
        call $float_mul
        local.set $l10
        i32.const -35
        call $shadow_leave_tail
        ref.null $obj
        local.get $env
        local.get $l7
        local.get $l10
        return_call $f15
      end
      unreachable
    end
    unreachable
  )
  (func $f14 (;329;) (type $fn1) (param $clo (ref null $obj)) (param $env (ref null $env)) (param $l0 eqref) (result eqref)
    (local $l1 eqref) (local $l2 eqref) (local $l3 eqref) (local $l4 eqref) (local $l5 eqref) (local $l6 eqref) (local $l7 eqref) (local $l8 eqref) (local $l9 eqref) (local $l10 eqref) (local $l11 eqref) (local $l12 eqref) (local $l13 eqref) (local $l14 eqref) (local $l15 eqref) (local $l16 eqref) (local $l17 eqref) (local $l18 eqref) (local $l19 eqref) (local $l20 eqref) (local $l21 eqref) (local $l22 eqref) (local $slot i32)
    global.get $prof_384
    i64.const 1
    i64.add
    global.set $prof_384
    call $shadow_enter_call
    local.get $l0
    local.set $l1
    i32.const 413
    call $shadow_site
    ref.null $obj
    local.get $env
    local.get $l1
    call $f17
    local.set $l2
    local.get $l2
    call $float_get
    global.get $g23
    call $float_get
    f64.gt
    call $bool_new
    local.set $l3
    block $b0
      block $b1
        local.get $l3
        i32.const 1
        call $is_bool
        i32.eqz
        if ;; label = @3
          br $b1
        else
        end
        call $shadow_leave
        global.get $k89_49
        return
      end
      block $b2
        local.get $l3
        i32.const 0
        call $is_bool
        i32.eqz
        if ;; label = @3
          br $b2
        else
        end
        i32.const 414
        call $shadow_site
        local.get $l1
        global.get $g22
        call $float_mul
        local.set $l4
        i32.const 414
        call $shadow_site
        local.get $l4
        global.get $g20
        call $float_add
        local.set $l5
        i32.const 415
        call $shadow_site
        local.get $l5
        call $core_floor
        local.set $l6
        local.get $l6
        local.set $l7
        i32.const 24
        i64.const 27
        local.get $l1
        global.get $g15
    global.get $prof_324
    i64.const 1
    i64.add
    global.set $prof_324
        array.new_fixed $values 2
        call $record
        local.set $l8
        i32.const 24
        i64.const 27
        local.get $l7
        global.get $g15
    global.get $prof_324
    i64.const 1
    i64.add
    global.set $prof_324
        array.new_fixed $values 2
        call $record
        local.set $l9
        i32.const 416
        call $shadow_site
        ref.null $obj
        local.get $env
        local.get $l9
        global.get $g30
        call $f18
        local.set $l10
        i32.const 417
        call $shadow_site
        ref.null $obj
        local.get $env
        local.get $l10
        call $f21
        local.set $l11
        i32.const 418
        call $shadow_site
        ref.null $obj
        local.get $env
        local.get $l8
        local.get $l11
        call $f23
        local.set $l12
        local.get $l12
        local.set $l13
        local.get $l13
        i32.const 0
        call $field
        local.set $l14
        i32.const 419
        call $shadow_site
        ref.null $obj
        local.get $env
        local.get $l14
        call $f17
        local.set $l15
        local.get $l15
        call $float_get
        global.get $g24
        call $float_get
        f64.gt
        call $bool_new
        local.set $l16
        block $b3
          block $b4
            local.get $l16
            i32.const 1
            call $is_bool
            i32.eqz
            if ;; label = @5
              br $b4
            else
            end
            call $shadow_leave
            global.get $k89_49
            return
          end
          block $b5
            local.get $l16
            i32.const 0
            call $is_bool
            i32.eqz
            if ;; label = @5
              br $b5
            else
            end
            i32.const 420
            call $shadow_site
            local.get $l7
            global.get $g18
            call $float_div
            local.set $l17
            i32.const 421
            call $shadow_site
            local.get $l17
            call $core_floor
            local.set $l18
            i32.const 422
            call $shadow_site
            global.get $g18
            local.get $l18
            call $float_mul
            local.set $l19
            i32.const 423
            call $shadow_site
            local.get $l7
            local.get $l19
            call $float_sub
            local.set $l20
            i32.const 25
            i64.const 28
            local.get $l7
            local.get $l20
            local.get $l13
    global.get $prof_385
    i64.const 1
    i64.add
    global.set $prof_385
            array.new_fixed $values 3
            call $record
            local.set $l21
            i32.const 88
            i64.const 49
            local.get $l21
    global.get $prof_247
    i64.const 1
    i64.add
    global.set $prof_247
            array.new_fixed $values 1
            call $record
            local.set $l22
            call $shadow_leave
            local.get $l22
            return
          end
          unreachable
        end
      end
      unreachable
    end
    unreachable
  )
  (func $f15 (;330;) (type $fn2) (param $clo (ref null $obj)) (param $env (ref null $env)) (param $l0 eqref) (param $l1 eqref) (result eqref)
    (local $l2 eqref) (local $l3 eqref) (local $l4 eqref) (local $l5 eqref) (local $l6 eqref) (local $l7 eqref) (local $l8 eqref) (local $l9 eqref) (local $l10 eqref) (local $l11 eqref) (local $l12 eqref) (local $l13 eqref) (local $l14 eqref) (local $l15 eqref) (local $l16 eqref) (local $l17 eqref) (local $l18 eqref) (local $l19 eqref) (local $l20 eqref) (local $l21 eqref) (local $l22 eqref) (local $slot i32)
    global.get $prof_386
    i64.const 1
    i64.add
    global.set $prof_386
    call $shadow_enter_call
    local.get $l0
    local.set $l2
    local.get $l1
    local.set $l3
    local.get $l2
    i32.const 0
    call $field
    local.set $l4
    i32.const 408
    call $shadow_site
    ref.null $obj
    local.get $env
    local.get $l4
    call $f17
    local.set $l5
    i32.const 408
    call $shadow_site
    local.get $l5
    global.get $g13
    call $float_mul
    local.set $l6
    i32.const 408
    call $shadow_site
    local.get $l6
    local.get $l3
    call $float_add
    local.set $l7
    local.get $l7
    local.set $l8
    local.get $l2
    i32.const 0
    call $field
    local.set $l9
    local.get $l2
    i32.const 1
    call $field
    local.set $l10
    i32.const 409
    call $shadow_site
    local.get $l10
    local.get $l8
    call $float_sub
    local.set $l11
    i32.const 410
    call $shadow_site
    local.get $l9
    local.get $l11
    call $float_add
    local.set $l12
    local.get $l12
    local.set $l13
    local.get $l2
    i32.const 0
    call $field
    local.set $l14
    local.get $l2
    i32.const 1
    call $field
    local.set $l15
    i32.const 411
    call $shadow_site
    local.get $l15
    local.get $l8
    call $float_add
    local.set $l16
    i32.const 412
    call $shadow_site
    local.get $l14
    local.get $l16
    call $float_add
    local.set $l17
    local.get $l17
    local.set $l18
    local.get $l13
    local.get $l18
    call $equal
    call $bool_new
    local.set $l19
    local.get $l19
    call $bool_get
    if ;; label = @1
      local.get $l13
      global.get $g15
      call $equal
      i32.eqz
      call $bool_new
      local.set $l21
      local.get $l21
      local.set $l20
    else
      global.get $value_false
      local.set $l20
    end
    block $b0
      block $b1
        local.get $l20
        i32.const 1
        call $is_bool
        i32.eqz
        if ;; label = @3
          br $b1
        else
        end
        i32.const 88
        i64.const 49
        local.get $l13
    global.get $prof_247
    i64.const 1
    i64.add
    global.set $prof_247
        array.new_fixed $values 1
        call $record
        local.set $l22
        call $shadow_leave
        local.get $l22
        return
      end
      block $b2
        local.get $l20
        i32.const 0
        call $is_bool
        i32.eqz
        if ;; label = @3
          br $b2
        else
        end
        call $shadow_leave
        global.get $k89_49
        return
      end
      unreachable
    end
    unreachable
  )
  (func $f16 (;331;) (type $fn2) (param $clo (ref null $obj)) (param $env (ref null $env)) (param $l0 eqref) (param $l1 eqref) (result eqref)
    (local $l2 eqref) (local $l3 eqref) (local $l4 eqref) (local $l5 eqref) (local $l6 eqref) (local $l7 eqref) (local $l8 eqref) (local $l9 eqref) (local $l10 eqref) (local $l11 eqref) (local $l12 eqref) (local $l13 eqref) (local $l14 eqref) (local $l15 eqref) (local $l16 eqref) (local $l17 eqref) (local $l18 eqref) (local $l19 eqref) (local $l20 eqref) (local $l21 eqref) (local $l22 eqref) (local $l23 eqref) (local $l24 eqref) (local $l25 eqref) (local $l26 eqref) (local $l27 eqref) (local $l28 eqref) (local $l29 eqref) (local $l30 eqref) (local $slot i32)
    global.get $prof_387
    i64.const 1
    i64.add
    global.set $prof_387
    call $shadow_enter_call
    local.get $l0
    local.set $l2
    local.get $l1
    local.set $l3
    local.get $l2
    i32.const 2
    call $field
    local.set $l4
    local.get $l4
    local.set $l5
    i32.const 34
    call $shadow_site
    ref.null $obj
    local.get $env
    local.get $l5
    local.get $l5
    call $f18
    local.set $l6
    local.get $l6
    local.set $l7
    local.get $l2
    i32.const 1
    call $field
    local.set $l8
    local.get $l8
    local.set $l9
    block $b0
      block $b1
        local.get $l3
        i32.const 1
        call $is_bool
        i32.eqz
        if ;; label = @3
          br $b1
        else
        end
        local.get $l9
        global.get $g15
        call $equal
        call $bool_new
        local.set $l11
        local.get $l11
        call $bool_get
        if ;; label = @3
          global.get $value_true
          local.set $l12
        else
          local.get $l9
          global.get $g17
          call $equal
          call $bool_new
          local.set $l13
          local.get $l13
          local.set $l12
        end
        local.get $l12
        local.set $l10
        br $b0
      end
      block $b2
        local.get $l3
        i32.const 0
        call $is_bool
        i32.eqz
        if ;; label = @3
          br $b2
        else
        end
        local.get $l9
        global.get $g16
        call $equal
        call $bool_new
        local.set $l14
        local.get $l14
        call $bool_get
        if ;; label = @3
          global.get $value_true
          local.set $l15
        else
          local.get $l9
          global.get $g21
          call $equal
          call $bool_new
          local.set $l16
          local.get $l16
          local.set $l15
        end
        local.get $l15
        local.set $l10
        br $b0
      end
      unreachable
    end
    local.get $l10
    local.set $l17
    block $b3
      block $b4
        local.get $l17
        i32.const 1
        call $is_bool
        i32.eqz
        if ;; label = @3
          br $b4
        else
        end
        i32.const 35
        call $shadow_site
        ref.null $obj
        local.get $env
        local.get $l7
        call $f19
        local.set $l19
        local.get $l19
        local.set $l18
        br $b3
      end
      block $b5
        local.get $l17
        i32.const 0
        call $is_bool
        i32.eqz
        if ;; label = @3
          br $b5
        else
        end
        i32.const 36
        call $shadow_site
        ref.null $obj
        local.get $env
        local.get $l7
        call $f20
        local.set $l20
        i32.const 37
        call $shadow_site
        ref.null $obj
        local.get $env
        local.get $l5
        local.get $l20
        call $f18
        local.set $l21
        local.get $l21
        local.set $l18
        br $b3
      end
      unreachable
    end
    local.get $l18
    local.set $l22
    block $b6
      block $b7
        local.get $l3
        i32.const 1
        call $is_bool
        i32.eqz
        if ;; label = @3
          br $b7
        else
        end
        local.get $l9
        global.get $g16
        call $equal
        call $bool_new
        local.set $l24
        local.get $l24
        call $bool_get
        if ;; label = @3
          global.get $value_true
          local.set $l25
        else
          local.get $l9
          global.get $g17
          call $equal
          call $bool_new
          local.set $l26
          local.get $l26
          local.set $l25
        end
        local.get $l25
        local.set $l23
        br $b6
      end
      block $b8
        local.get $l3
        i32.const 0
        call $is_bool
        i32.eqz
        if ;; label = @3
          br $b8
        else
        end
        local.get $l9
        global.get $g17
        call $equal
        call $bool_new
        local.set $l27
        local.get $l27
        call $bool_get
        if ;; label = @3
          global.get $value_true
          local.set $l28
        else
          local.get $l9
          global.get $g21
          call $equal
          call $bool_new
          local.set $l29
          local.get $l29
          local.set $l28
        end
        local.get $l28
        local.set $l23
        br $b6
      end
      unreachable
    end
    local.get $l23
    local.set $l30
    block $b9
      block $b10
        local.get $l30
        i32.const 1
        call $is_bool
        i32.eqz
        if ;; label = @3
          br $b10
        else
        end
        i32.const -40
        call $shadow_leave_tail
        ref.null $obj
        local.get $env
        local.get $l22
        return_call $f21
      end
      block $b11
        local.get $l30
        i32.const 0
        call $is_bool
        i32.eqz
        if ;; label = @3
          br $b11
        else
        end
        call $shadow_leave
        local.get $l22
        return
      end
      unreachable
    end
    unreachable
  )
  (func $f17 (;332;) (type $fn1) (param $clo (ref null $obj)) (param $env (ref null $env)) (param $l0 eqref) (result eqref)
    (local $l1 eqref) (local $l2 eqref) (local $l3 eqref) (local $slot i32)
    global.get $prof_388
    i64.const 1
    i64.add
    global.set $prof_388
    call $shadow_enter_call
    local.get $l0
    local.set $l1
    local.get $l1
    call $float_get
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 0
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    call $float_get
    f64.lt
    call $bool_new
    local.set $l2
    block $b0
      block $b1
        local.get $l2
        i32.const 1
        call $is_bool
        i32.eqz
        if ;; label = @3
          br $b1
        else
        end
        local.get $l1
        call $float_neg
        local.set $l3
        call $shadow_leave
        local.get $l3
        return
      end
      block $b2
        local.get $l2
        i32.const 0
        call $is_bool
        i32.eqz
        if ;; label = @3
          br $b2
        else
        end
        call $shadow_leave
        local.get $l1
        return
      end
      unreachable
    end
    unreachable
  )
  (func $f18 (;333;) (type $fn2) (param $clo (ref null $obj)) (param $env (ref null $env)) (param $l0 eqref) (param $l1 eqref) (result eqref)
    (local $l2 eqref) (local $l3 eqref) (local $l4 eqref) (local $l5 eqref) (local $l6 eqref) (local $l7 eqref) (local $l8 eqref) (local $l9 eqref) (local $l10 eqref) (local $l11 eqref) (local $l12 eqref) (local $l13 eqref) (local $l14 eqref) (local $l15 eqref) (local $l16 eqref) (local $l17 eqref) (local $l18 eqref) (local $l19 eqref) (local $l20 eqref) (local $l21 eqref) (local $l22 eqref) (local $l23 eqref) (local $l24 eqref) (local $l25 eqref) (local $l26 eqref) (local $l27 eqref) (local $l28 eqref) (local $l29 eqref) (local $l30 eqref) (local $l31 eqref) (local $l32 eqref) (local $l33 eqref) (local $l34 eqref) (local $l35 eqref) (local $l36 eqref) (local $l37 eqref) (local $l38 eqref) (local $l39 eqref) (local $l40 eqref) (local $slot i32)
    global.get $prof_389
    i64.const 1
    i64.add
    global.set $prof_389
    call $shadow_enter_call
    local.get $l0
    local.set $l2
    local.get $l1
    local.set $l3
    local.get $l2
    i32.const 0
    call $field
    local.set $l4
    local.get $l4
    local.set $l5
    local.get $l2
    i32.const 1
    call $field
    local.set $l6
    local.get $l6
    local.set $l7
    local.get $l3
    i32.const 0
    call $field
    local.set $l8
    local.get $l8
    local.set $l9
    local.get $l3
    i32.const 1
    call $field
    local.set $l10
    local.get $l10
    local.set $l11
    i32.const 385
    call $shadow_site
    local.get $l5
    local.get $l9
    call $float_mul
    local.set $l12
    local.get $l12
    local.set $l13
    i32.const 386
    call $shadow_site
    global.get $g12
    local.get $l5
    call $float_mul
    local.set $l14
    local.get $l14
    local.set $l15
    i32.const 387
    call $shadow_site
    local.get $l15
    local.get $l5
    call $float_sub
    local.set $l16
    i32.const 388
    call $shadow_site
    local.get $l15
    local.get $l16
    call $float_sub
    local.set $l17
    local.get $l17
    local.set $l18
    i32.const 389
    call $shadow_site
    local.get $l5
    local.get $l18
    call $float_sub
    local.set $l19
    local.get $l19
    local.set $l20
    i32.const 390
    call $shadow_site
    global.get $g12
    local.get $l9
    call $float_mul
    local.set $l21
    local.get $l21
    local.set $l22
    i32.const 391
    call $shadow_site
    local.get $l22
    local.get $l9
    call $float_sub
    local.set $l23
    i32.const 392
    call $shadow_site
    local.get $l22
    local.get $l23
    call $float_sub
    local.set $l24
    local.get $l24
    local.set $l25
    i32.const 393
    call $shadow_site
    local.get $l9
    local.get $l25
    call $float_sub
    local.set $l26
    local.get $l26
    local.set $l27
    i32.const 394
    call $shadow_site
    local.get $l18
    local.get $l25
    call $float_mul
    local.set $l28
    i32.const 395
    call $shadow_site
    local.get $l28
    local.get $l13
    call $float_sub
    local.set $l29
    i32.const 396
    call $shadow_site
    local.get $l18
    local.get $l27
    call $float_mul
    local.set $l30
    i32.const 397
    call $shadow_site
    local.get $l29
    local.get $l30
    call $float_add
    local.set $l31
    i32.const 398
    call $shadow_site
    local.get $l20
    local.get $l25
    call $float_mul
    local.set $l32
    i32.const 397
    call $shadow_site
    local.get $l31
    local.get $l32
    call $float_add
    local.set $l33
    i32.const 399
    call $shadow_site
    local.get $l20
    local.get $l27
    call $float_mul
    local.set $l34
    i32.const 400
    call $shadow_site
    local.get $l33
    local.get $l34
    call $float_add
    local.set $l35
    local.get $l35
    local.set $l36
    i32.const 401
    call $shadow_site
    local.get $l5
    local.get $l11
    call $float_mul
    local.set $l37
    i32.const 402
    call $shadow_site
    local.get $l36
    local.get $l37
    call $float_add
    local.set $l38
    i32.const 403
    call $shadow_site
    local.get $l7
    local.get $l9
    call $float_mul
    local.set $l39
    i32.const 402
    call $shadow_site
    local.get $l38
    local.get $l39
    call $float_add
    local.set $l40
    i32.const -406
    call $shadow_leave_tail
    ref.null $obj
    local.get $env
    local.get $l13
    local.get $l40
    return_call $f22
    unreachable
  )
  (func $f19 (;334;) (type $fn1) (param $clo (ref null $obj)) (param $env (ref null $env)) (param $l0 eqref) (result eqref)
    (local $l1 eqref) (local $l2 eqref) (local $l3 eqref) (local $l4 eqref) (local $l5 eqref) (local $l6 eqref) (local $l7 eqref) (local $l8 eqref) (local $l9 eqref) (local $l10 eqref) (local $l11 eqref) (local $l12 eqref) (local $l13 eqref) (local $l14 eqref) (local $l15 eqref) (local $l16 eqref) (local $l17 eqref) (local $l18 eqref) (local $l19 eqref) (local $l20 eqref) (local $l21 eqref) (local $l22 eqref) (local $l23 eqref) (local $l24 eqref) (local $l25 eqref) (local $l26 eqref) (local $l27 eqref) (local $l28 eqref) (local $l29 eqref) (local $l30 eqref) (local $l31 eqref) (local $l32 eqref) (local $l33 eqref) (local $l34 eqref) (local $l35 eqref) (local $l36 eqref) (local $l37 eqref) (local $l38 eqref) (local $l39 eqref) (local $l40 eqref) (local $l41 eqref) (local $l42 eqref) (local $l43 eqref) (local $l44 eqref) (local $l45 eqref) (local $l46 eqref) (local $l47 eqref) (local $l48 eqref) (local $l49 eqref) (local $l50 eqref) (local $l51 eqref) (local $l52 eqref) (local $l53 eqref) (local $l54 eqref) (local $l55 eqref) (local $l56 eqref) (local $l57 eqref) (local $l58 eqref) (local $l59 eqref) (local $l60 eqref) (local $l61 eqref) (local $l62 eqref) (local $l63 eqref) (local $l64 eqref) (local $l65 eqref) (local $l66 eqref) (local $l67 eqref) (local $l68 eqref) (local $l69 eqref) (local $l70 eqref) (local $l71 eqref) (local $l72 eqref) (local $l73 eqref) (local $l74 eqref) (local $l75 eqref) (local $l76 eqref) (local $l77 eqref) (local $l78 eqref) (local $l79 eqref) (local $l80 eqref) (local $l81 eqref) (local $l82 eqref) (local $l83 eqref) (local $l84 eqref) (local $l85 eqref) (local $l86 eqref) (local $l87 eqref) (local $l88 eqref) (local $l89 eqref) (local $l90 eqref) (local $l91 eqref) (local $l92 eqref) (local $l93 eqref) (local $l94 eqref) (local $l95 eqref) (local $l96 eqref) (local $l97 eqref) (local $l98 eqref) (local $l99 eqref) (local $l100 eqref) (local $l101 eqref) (local $l102 eqref) (local $l103 eqref) (local $l104 eqref) (local $l105 eqref) (local $l106 eqref) (local $l107 eqref) (local $l108 eqref) (local $l109 eqref) (local $l110 eqref) (local $l111 eqref) (local $l112 eqref) (local $l113 eqref) (local $l114 eqref) (local $l115 eqref) (local $l116 eqref) (local $l117 eqref) (local $l118 eqref) (local $l119 eqref) (local $l120 eqref) (local $l121 eqref) (local $l122 eqref) (local $l123 eqref) (local $l124 eqref) (local $l125 eqref) (local $l126 eqref) (local $l127 eqref) (local $l128 eqref) (local $l129 eqref) (local $l130 eqref) (local $l131 eqref) (local $l132 eqref) (local $l133 eqref) (local $l134 eqref) (local $l135 eqref) (local $l136 eqref) (local $l137 eqref) (local $l138 eqref) (local $l139 eqref) (local $l140 eqref) (local $l141 eqref) (local $l142 eqref) (local $l143 eqref) (local $l144 eqref) (local $l145 eqref) (local $l146 eqref) (local $l147 eqref) (local $l148 eqref) (local $l149 eqref) (local $l150 eqref) (local $l151 eqref) (local $l152 eqref) (local $l153 eqref) (local $l154 eqref) (local $l155 eqref) (local $l156 eqref) (local $l157 eqref) (local $l158 eqref) (local $l159 eqref) (local $l160 eqref) (local $l161 eqref) (local $l162 eqref) (local $l163 eqref) (local $l164 eqref) (local $l165 eqref) (local $l166 eqref) (local $l167 eqref) (local $l168 eqref) (local $l169 eqref) (local $l170 eqref) (local $l171 eqref) (local $l172 eqref) (local $l173 eqref) (local $l174 eqref) (local $l175 eqref) (local $l176 eqref) (local $l177 eqref) (local $l178 eqref) (local $l179 eqref) (local $l180 eqref) (local $l181 eqref) (local $l182 eqref) (local $l183 eqref) (local $l184 eqref) (local $l185 eqref) (local $l186 eqref) (local $l187 eqref) (local $l188 eqref) (local $l189 eqref) (local $l190 eqref) (local $l191 eqref) (local $l192 eqref) (local $l193 eqref) (local $l194 eqref) (local $l195 eqref) (local $l196 eqref) (local $l197 eqref) (local $l198 eqref) (local $l199 eqref) (local $l200 eqref) (local $l201 eqref) (local $l202 eqref) (local $l203 eqref) (local $l204 eqref) (local $l205 eqref) (local $l206 eqref) (local $l207 eqref) (local $l208 eqref) (local $l209 eqref) (local $l210 eqref) (local $l211 eqref) (local $l212 eqref) (local $l213 eqref) (local $l214 eqref) (local $l215 eqref) (local $l216 eqref) (local $l217 eqref) (local $l218 eqref) (local $l219 eqref) (local $l220 eqref) (local $l221 eqref) (local $l222 eqref) (local $l223 eqref) (local $l224 eqref) (local $l225 eqref) (local $l226 eqref) (local $l227 eqref) (local $l228 eqref) (local $l229 eqref) (local $l230 eqref) (local $l231 eqref) (local $l232 eqref) (local $l233 eqref) (local $l234 eqref) (local $l235 eqref) (local $l236 eqref) (local $l237 eqref) (local $l238 eqref) (local $l239 eqref) (local $l240 eqref) (local $l241 eqref) (local $l242 eqref) (local $l243 eqref) (local $l244 eqref) (local $l245 eqref) (local $l246 eqref) (local $l247 eqref) (local $l248 eqref) (local $l249 eqref) (local $l250 eqref) (local $l251 eqref) (local $l252 eqref) (local $l253 eqref) (local $l254 eqref) (local $l255 eqref) (local $l256 eqref) (local $l257 eqref) (local $l258 eqref) (local $l259 eqref) (local $l260 eqref) (local $l261 eqref) (local $l262 eqref) (local $l263 eqref) (local $l264 eqref) (local $l265 eqref) (local $l266 eqref) (local $l267 eqref) (local $l268 eqref) (local $l269 eqref) (local $l270 eqref) (local $l271 eqref) (local $l272 eqref) (local $l273 eqref) (local $l274 eqref) (local $l275 eqref) (local $l276 eqref) (local $l277 eqref) (local $l278 eqref) (local $l279 eqref) (local $l280 eqref) (local $l281 eqref) (local $l282 eqref) (local $l283 eqref) (local $l284 eqref) (local $l285 eqref) (local $l286 eqref) (local $l287 eqref) (local $l288 eqref) (local $l289 eqref) (local $l290 eqref) (local $l291 eqref) (local $l292 eqref) (local $l293 eqref) (local $l294 eqref) (local $l295 eqref) (local $l296 eqref) (local $l297 eqref) (local $l298 eqref) (local $l299 eqref) (local $l300 eqref) (local $l301 eqref) (local $l302 eqref) (local $l303 eqref) (local $l304 eqref) (local $l305 eqref) (local $l306 eqref) (local $l307 eqref) (local $l308 eqref) (local $l309 eqref) (local $l310 eqref) (local $l311 eqref) (local $l312 eqref) (local $l313 eqref) (local $l314 eqref) (local $l315 eqref) (local $l316 eqref) (local $l317 eqref) (local $l318 eqref) (local $l319 eqref) (local $l320 eqref) (local $l321 eqref) (local $slot i32)
    global.get $prof_390
    i64.const 1
    i64.add
    global.set $prof_390
    call $shadow_enter_call
    local.get $l0
    local.set $l1
    local.get $l1
    i32.const 0
    call $field
    local.set $l2
    local.get $l2
    local.set $l3
    local.get $l1
    i32.const 1
    call $field
    local.set $l4
    local.get $l4
    local.set $l5
    i32.const 200
    call $shadow_site
    global.get $g12
    local.get $l3
    call $float_mul
    local.set $l6
    local.get $l6
    local.set $l7
    i32.const 201
    call $shadow_site
    local.get $l7
    local.get $l3
    call $float_sub
    local.set $l8
    i32.const 202
    call $shadow_site
    local.get $l7
    local.get $l8
    call $float_sub
    local.set $l9
    local.get $l9
    local.set $l10
    i32.const 203
    call $shadow_site
    local.get $l3
    local.get $l10
    call $float_sub
    local.set $l11
    local.get $l11
    local.set $l12
    global.get $g128
    local.set $l13
    i32.const 204
    call $shadow_site
    local.get $l13
    local.get $l3
    call $float_mul
    local.set $l14
    i32.const 204
    call $shadow_site
    local.get $l14
    global.get $g127
    call $float_add
    local.set $l15
    local.get $l15
    local.set $l16
    i32.const 205
    call $shadow_site
    local.get $l16
    local.get $l3
    call $float_mul
    local.set $l17
    i32.const 205
    call $shadow_site
    local.get $l17
    global.get $g126
    call $float_add
    local.set $l18
    local.get $l18
    local.set $l19
    i32.const 206
    call $shadow_site
    local.get $l19
    local.get $l3
    call $float_mul
    local.set $l20
    i32.const 206
    call $shadow_site
    local.get $l20
    global.get $g125
    call $float_add
    local.set $l21
    local.get $l21
    local.set $l22
    i32.const 207
    call $shadow_site
    local.get $l22
    local.get $l3
    call $float_mul
    local.set $l23
    i32.const 207
    call $shadow_site
    local.get $l23
    global.get $g124
    call $float_add
    local.set $l24
    local.get $l24
    local.set $l25
    i32.const 208
    call $shadow_site
    local.get $l25
    local.get $l3
    call $float_mul
    local.set $l26
    i32.const 208
    call $shadow_site
    local.get $l26
    global.get $g123
    call $float_add
    local.set $l27
    local.get $l27
    local.set $l28
    i32.const 209
    call $shadow_site
    local.get $l28
    local.get $l3
    call $float_mul
    local.set $l29
    i32.const 209
    call $shadow_site
    local.get $l29
    global.get $g122
    call $float_add
    local.set $l30
    local.get $l30
    local.set $l31
    local.get $l31
    local.set $l32
    global.get $g15
    local.set $l33
    i32.const 210
    call $shadow_site
    local.get $l32
    local.get $l3
    call $float_mul
    local.set $l34
    local.get $l34
    local.set $l35
    i32.const 211
    call $shadow_site
    global.get $g12
    local.get $l32
    call $float_mul
    local.set $l36
    local.get $l36
    local.set $l37
    i32.const 212
    call $shadow_site
    local.get $l37
    local.get $l32
    call $float_sub
    local.set $l38
    i32.const 213
    call $shadow_site
    local.get $l37
    local.get $l38
    call $float_sub
    local.set $l39
    local.get $l39
    local.set $l40
    i32.const 214
    call $shadow_site
    local.get $l32
    local.get $l40
    call $float_sub
    local.set $l41
    local.get $l41
    local.set $l42
    i32.const 215
    call $shadow_site
    local.get $l40
    local.get $l10
    call $float_mul
    local.set $l43
    i32.const 216
    call $shadow_site
    local.get $l43
    local.get $l35
    call $float_sub
    local.set $l44
    i32.const 217
    call $shadow_site
    local.get $l40
    local.get $l12
    call $float_mul
    local.set $l45
    i32.const 218
    call $shadow_site
    local.get $l44
    local.get $l45
    call $float_add
    local.set $l46
    i32.const 219
    call $shadow_site
    local.get $l42
    local.get $l10
    call $float_mul
    local.set $l47
    i32.const 218
    call $shadow_site
    local.get $l46
    local.get $l47
    call $float_add
    local.set $l48
    i32.const 220
    call $shadow_site
    local.get $l42
    local.get $l12
    call $float_mul
    local.set $l49
    i32.const 221
    call $shadow_site
    local.get $l48
    local.get $l49
    call $float_add
    local.set $l50
    local.get $l50
    local.set $l51
    i32.const 222
    call $shadow_site
    local.get $l32
    local.get $l5
    call $float_mul
    local.set $l52
    i32.const 223
    call $shadow_site
    local.get $l51
    local.get $l52
    call $float_add
    local.set $l53
    i32.const 224
    call $shadow_site
    local.get $l33
    local.get $l3
    call $float_mul
    local.set $l54
    i32.const 223
    call $shadow_site
    local.get $l53
    local.get $l54
    call $float_add
    local.set $l55
    local.get $l55
    local.set $l56
    i32.const 225
    call $shadow_site
    local.get $l35
    local.get $l56
    call $float_add
    local.set $l57
    local.get $l57
    local.set $l58
    i32.const 226
    call $shadow_site
    local.get $l58
    local.get $l35
    call $float_sub
    local.set $l59
    i32.const 227
    call $shadow_site
    local.get $l56
    local.get $l59
    call $float_sub
    local.set $l60
    local.get $l60
    local.set $l61
    i32.const 228
    call $shadow_site
    global.get $g120
    local.get $l58
    call $float_add
    local.set $l62
    local.get $l62
    local.set $l63
    i32.const 229
    call $shadow_site
    local.get $l63
    global.get $g120
    call $float_sub
    local.set $l64
    i32.const 230
    call $shadow_site
    local.get $l58
    local.get $l64
    call $float_sub
    local.set $l65
    local.get $l65
    local.set $l66
    i32.const 231
    call $shadow_site
    local.get $l66
    global.get $g121
    call $float_add
    local.set $l67
    i32.const 231
    call $shadow_site
    local.get $l67
    local.get $l61
    call $float_add
    local.set $l68
    local.get $l68
    local.set $l69
    i32.const 232
    call $shadow_site
    local.get $l63
    local.get $l69
    call $float_add
    local.set $l70
    local.get $l70
    local.set $l71
    i32.const 233
    call $shadow_site
    local.get $l71
    local.get $l63
    call $float_sub
    local.set $l72
    i32.const 234
    call $shadow_site
    local.get $l69
    local.get $l72
    call $float_sub
    local.set $l73
    local.get $l73
    local.set $l74
    i32.const 235
    call $shadow_site
    local.get $l71
    local.get $l3
    call $float_mul
    local.set $l75
    local.get $l75
    local.set $l76
    i32.const 236
    call $shadow_site
    global.get $g12
    local.get $l71
    call $float_mul
    local.set $l77
    local.get $l77
    local.set $l78
    i32.const 237
    call $shadow_site
    local.get $l78
    local.get $l71
    call $float_sub
    local.set $l79
    i32.const 238
    call $shadow_site
    local.get $l78
    local.get $l79
    call $float_sub
    local.set $l80
    local.get $l80
    local.set $l81
    i32.const 239
    call $shadow_site
    local.get $l71
    local.get $l81
    call $float_sub
    local.set $l82
    local.get $l82
    local.set $l83
    i32.const 240
    call $shadow_site
    local.get $l81
    local.get $l10
    call $float_mul
    local.set $l84
    i32.const 241
    call $shadow_site
    local.get $l84
    local.get $l76
    call $float_sub
    local.set $l85
    i32.const 242
    call $shadow_site
    local.get $l81
    local.get $l12
    call $float_mul
    local.set $l86
    i32.const 243
    call $shadow_site
    local.get $l85
    local.get $l86
    call $float_add
    local.set $l87
    i32.const 244
    call $shadow_site
    local.get $l83
    local.get $l10
    call $float_mul
    local.set $l88
    i32.const 243
    call $shadow_site
    local.get $l87
    local.get $l88
    call $float_add
    local.set $l89
    i32.const 245
    call $shadow_site
    local.get $l83
    local.get $l12
    call $float_mul
    local.set $l90
    i32.const 246
    call $shadow_site
    local.get $l89
    local.get $l90
    call $float_add
    local.set $l91
    local.get $l91
    local.set $l92
    i32.const 247
    call $shadow_site
    local.get $l71
    local.get $l5
    call $float_mul
    local.set $l93
    i32.const 248
    call $shadow_site
    local.get $l92
    local.get $l93
    call $float_add
    local.set $l94
    i32.const 249
    call $shadow_site
    local.get $l74
    local.get $l3
    call $float_mul
    local.set $l95
    i32.const 248
    call $shadow_site
    local.get $l94
    local.get $l95
    call $float_add
    local.set $l96
    local.get $l96
    local.set $l97
    i32.const 250
    call $shadow_site
    local.get $l76
    local.get $l97
    call $float_add
    local.set $l98
    local.get $l98
    local.set $l99
    i32.const 251
    call $shadow_site
    local.get $l99
    local.get $l76
    call $float_sub
    local.set $l100
    i32.const 252
    call $shadow_site
    local.get $l97
    local.get $l100
    call $float_sub
    local.set $l101
    local.get $l101
    local.set $l102
    i32.const 253
    call $shadow_site
    global.get $g118
    local.get $l99
    call $float_add
    local.set $l103
    local.get $l103
    local.set $l104
    i32.const 254
    call $shadow_site
    local.get $l104
    global.get $g118
    call $float_sub
    local.set $l105
    i32.const 255
    call $shadow_site
    local.get $l99
    local.get $l105
    call $float_sub
    local.set $l106
    local.get $l106
    local.set $l107
    i32.const 256
    call $shadow_site
    local.get $l107
    global.get $g119
    call $float_add
    local.set $l108
    i32.const 256
    call $shadow_site
    local.get $l108
    local.get $l102
    call $float_add
    local.set $l109
    local.get $l109
    local.set $l110
    i32.const 257
    call $shadow_site
    local.get $l104
    local.get $l110
    call $float_add
    local.set $l111
    local.get $l111
    local.set $l112
    i32.const 258
    call $shadow_site
    local.get $l112
    local.get $l104
    call $float_sub
    local.set $l113
    i32.const 259
    call $shadow_site
    local.get $l110
    local.get $l113
    call $float_sub
    local.set $l114
    local.get $l114
    local.set $l115
    i32.const 260
    call $shadow_site
    local.get $l112
    local.get $l3
    call $float_mul
    local.set $l116
    local.get $l116
    local.set $l117
    i32.const 261
    call $shadow_site
    global.get $g12
    local.get $l112
    call $float_mul
    local.set $l118
    local.get $l118
    local.set $l119
    i32.const 262
    call $shadow_site
    local.get $l119
    local.get $l112
    call $float_sub
    local.set $l120
    i32.const 263
    call $shadow_site
    local.get $l119
    local.get $l120
    call $float_sub
    local.set $l121
    local.get $l121
    local.set $l122
    i32.const 264
    call $shadow_site
    local.get $l112
    local.get $l122
    call $float_sub
    local.set $l123
    local.get $l123
    local.set $l124
    i32.const 265
    call $shadow_site
    local.get $l122
    local.get $l10
    call $float_mul
    local.set $l125
    i32.const 266
    call $shadow_site
    local.get $l125
    local.get $l117
    call $float_sub
    local.set $l126
    i32.const 267
    call $shadow_site
    local.get $l122
    local.get $l12
    call $float_mul
    local.set $l127
    i32.const 268
    call $shadow_site
    local.get $l126
    local.get $l127
    call $float_add
    local.set $l128
    i32.const 269
    call $shadow_site
    local.get $l124
    local.get $l10
    call $float_mul
    local.set $l129
    i32.const 268
    call $shadow_site
    local.get $l128
    local.get $l129
    call $float_add
    local.set $l130
    i32.const 270
    call $shadow_site
    local.get $l124
    local.get $l12
    call $float_mul
    local.set $l131
    i32.const 271
    call $shadow_site
    local.get $l130
    local.get $l131
    call $float_add
    local.set $l132
    local.get $l132
    local.set $l133
    i32.const 272
    call $shadow_site
    local.get $l112
    local.get $l5
    call $float_mul
    local.set $l134
    i32.const 273
    call $shadow_site
    local.get $l133
    local.get $l134
    call $float_add
    local.set $l135
    i32.const 274
    call $shadow_site
    local.get $l115
    local.get $l3
    call $float_mul
    local.set $l136
    i32.const 273
    call $shadow_site
    local.get $l135
    local.get $l136
    call $float_add
    local.set $l137
    local.get $l137
    local.set $l138
    i32.const 275
    call $shadow_site
    local.get $l117
    local.get $l138
    call $float_add
    local.set $l139
    local.get $l139
    local.set $l140
    i32.const 276
    call $shadow_site
    local.get $l140
    local.get $l117
    call $float_sub
    local.set $l141
    i32.const 277
    call $shadow_site
    local.get $l138
    local.get $l141
    call $float_sub
    local.set $l142
    local.get $l142
    local.set $l143
    i32.const 278
    call $shadow_site
    global.get $g116
    local.get $l140
    call $float_add
    local.set $l144
    local.get $l144
    local.set $l145
    i32.const 279
    call $shadow_site
    local.get $l145
    global.get $g116
    call $float_sub
    local.set $l146
    i32.const 280
    call $shadow_site
    local.get $l140
    local.get $l146
    call $float_sub
    local.set $l147
    local.get $l147
    local.set $l148
    i32.const 281
    call $shadow_site
    local.get $l148
    global.get $g117
    call $float_add
    local.set $l149
    i32.const 281
    call $shadow_site
    local.get $l149
    local.get $l143
    call $float_add
    local.set $l150
    local.get $l150
    local.set $l151
    i32.const 282
    call $shadow_site
    local.get $l145
    local.get $l151
    call $float_add
    local.set $l152
    local.get $l152
    local.set $l153
    i32.const 283
    call $shadow_site
    local.get $l153
    local.get $l145
    call $float_sub
    local.set $l154
    i32.const 284
    call $shadow_site
    local.get $l151
    local.get $l154
    call $float_sub
    local.set $l155
    local.get $l155
    local.set $l156
    i32.const 285
    call $shadow_site
    local.get $l153
    local.get $l3
    call $float_mul
    local.set $l157
    local.get $l157
    local.set $l158
    i32.const 286
    call $shadow_site
    global.get $g12
    local.get $l153
    call $float_mul
    local.set $l159
    local.get $l159
    local.set $l160
    i32.const 287
    call $shadow_site
    local.get $l160
    local.get $l153
    call $float_sub
    local.set $l161
    i32.const 288
    call $shadow_site
    local.get $l160
    local.get $l161
    call $float_sub
    local.set $l162
    local.get $l162
    local.set $l163
    i32.const 289
    call $shadow_site
    local.get $l153
    local.get $l163
    call $float_sub
    local.set $l164
    local.get $l164
    local.set $l165
    i32.const 290
    call $shadow_site
    local.get $l163
    local.get $l10
    call $float_mul
    local.set $l166
    i32.const 291
    call $shadow_site
    local.get $l166
    local.get $l158
    call $float_sub
    local.set $l167
    i32.const 292
    call $shadow_site
    local.get $l163
    local.get $l12
    call $float_mul
    local.set $l168
    i32.const 293
    call $shadow_site
    local.get $l167
    local.get $l168
    call $float_add
    local.set $l169
    i32.const 294
    call $shadow_site
    local.get $l165
    local.get $l10
    call $float_mul
    local.set $l170
    i32.const 293
    call $shadow_site
    local.get $l169
    local.get $l170
    call $float_add
    local.set $l171
    i32.const 295
    call $shadow_site
    local.get $l165
    local.get $l12
    call $float_mul
    local.set $l172
    i32.const 296
    call $shadow_site
    local.get $l171
    local.get $l172
    call $float_add
    local.set $l173
    local.get $l173
    local.set $l174
    i32.const 297
    call $shadow_site
    local.get $l153
    local.get $l5
    call $float_mul
    local.set $l175
    i32.const 298
    call $shadow_site
    local.get $l174
    local.get $l175
    call $float_add
    local.set $l176
    i32.const 299
    call $shadow_site
    local.get $l156
    local.get $l3
    call $float_mul
    local.set $l177
    i32.const 298
    call $shadow_site
    local.get $l176
    local.get $l177
    call $float_add
    local.set $l178
    local.get $l178
    local.set $l179
    i32.const 300
    call $shadow_site
    local.get $l158
    local.get $l179
    call $float_add
    local.set $l180
    local.get $l180
    local.set $l181
    i32.const 301
    call $shadow_site
    local.get $l181
    local.get $l158
    call $float_sub
    local.set $l182
    i32.const 302
    call $shadow_site
    local.get $l179
    local.get $l182
    call $float_sub
    local.set $l183
    local.get $l183
    local.set $l184
    i32.const 303
    call $shadow_site
    global.get $g114
    local.get $l181
    call $float_add
    local.set $l185
    local.get $l185
    local.set $l186
    i32.const 304
    call $shadow_site
    local.get $l186
    global.get $g114
    call $float_sub
    local.set $l187
    i32.const 305
    call $shadow_site
    local.get $l181
    local.get $l187
    call $float_sub
    local.set $l188
    local.get $l188
    local.set $l189
    i32.const 306
    call $shadow_site
    local.get $l189
    global.get $g115
    call $float_add
    local.set $l190
    i32.const 306
    call $shadow_site
    local.get $l190
    local.get $l184
    call $float_add
    local.set $l191
    local.get $l191
    local.set $l192
    i32.const 307
    call $shadow_site
    local.get $l186
    local.get $l192
    call $float_add
    local.set $l193
    local.get $l193
    local.set $l194
    i32.const 308
    call $shadow_site
    local.get $l194
    local.get $l186
    call $float_sub
    local.set $l195
    i32.const 309
    call $shadow_site
    local.get $l192
    local.get $l195
    call $float_sub
    local.set $l196
    local.get $l196
    local.set $l197
    i32.const 310
    call $shadow_site
    local.get $l194
    local.get $l3
    call $float_mul
    local.set $l198
    local.get $l198
    local.set $l199
    i32.const 311
    call $shadow_site
    global.get $g12
    local.get $l194
    call $float_mul
    local.set $l200
    local.get $l200
    local.set $l201
    i32.const 312
    call $shadow_site
    local.get $l201
    local.get $l194
    call $float_sub
    local.set $l202
    i32.const 313
    call $shadow_site
    local.get $l201
    local.get $l202
    call $float_sub
    local.set $l203
    local.get $l203
    local.set $l204
    i32.const 314
    call $shadow_site
    local.get $l194
    local.get $l204
    call $float_sub
    local.set $l205
    local.get $l205
    local.set $l206
    i32.const 315
    call $shadow_site
    local.get $l204
    local.get $l10
    call $float_mul
    local.set $l207
    i32.const 316
    call $shadow_site
    local.get $l207
    local.get $l199
    call $float_sub
    local.set $l208
    i32.const 317
    call $shadow_site
    local.get $l204
    local.get $l12
    call $float_mul
    local.set $l209
    i32.const 318
    call $shadow_site
    local.get $l208
    local.get $l209
    call $float_add
    local.set $l210
    i32.const 319
    call $shadow_site
    local.get $l206
    local.get $l10
    call $float_mul
    local.set $l211
    i32.const 318
    call $shadow_site
    local.get $l210
    local.get $l211
    call $float_add
    local.set $l212
    i32.const 320
    call $shadow_site
    local.get $l206
    local.get $l12
    call $float_mul
    local.set $l213
    i32.const 321
    call $shadow_site
    local.get $l212
    local.get $l213
    call $float_add
    local.set $l214
    local.get $l214
    local.set $l215
    i32.const 322
    call $shadow_site
    local.get $l194
    local.get $l5
    call $float_mul
    local.set $l216
    i32.const 323
    call $shadow_site
    local.get $l215
    local.get $l216
    call $float_add
    local.set $l217
    i32.const 324
    call $shadow_site
    local.get $l197
    local.get $l3
    call $float_mul
    local.set $l218
    i32.const 323
    call $shadow_site
    local.get $l217
    local.get $l218
    call $float_add
    local.set $l219
    local.get $l219
    local.set $l220
    i32.const 325
    call $shadow_site
    local.get $l199
    local.get $l220
    call $float_add
    local.set $l221
    local.get $l221
    local.set $l222
    i32.const 326
    call $shadow_site
    local.get $l222
    local.get $l199
    call $float_sub
    local.set $l223
    i32.const 327
    call $shadow_site
    local.get $l220
    local.get $l223
    call $float_sub
    local.set $l224
    local.get $l224
    local.set $l225
    i32.const 328
    call $shadow_site
    global.get $g112
    local.get $l222
    call $float_add
    local.set $l226
    local.get $l226
    local.set $l227
    i32.const 329
    call $shadow_site
    local.get $l227
    global.get $g112
    call $float_sub
    local.set $l228
    i32.const 330
    call $shadow_site
    local.get $l222
    local.get $l228
    call $float_sub
    local.set $l229
    local.get $l229
    local.set $l230
    i32.const 331
    call $shadow_site
    local.get $l230
    global.get $g113
    call $float_add
    local.set $l231
    i32.const 331
    call $shadow_site
    local.get $l231
    local.get $l225
    call $float_add
    local.set $l232
    local.get $l232
    local.set $l233
    i32.const 332
    call $shadow_site
    local.get $l227
    local.get $l233
    call $float_add
    local.set $l234
    local.get $l234
    local.set $l235
    i32.const 333
    call $shadow_site
    local.get $l235
    local.get $l227
    call $float_sub
    local.set $l236
    i32.const 334
    call $shadow_site
    local.get $l233
    local.get $l236
    call $float_sub
    local.set $l237
    local.get $l237
    local.set $l238
    i32.const 335
    call $shadow_site
    local.get $l235
    local.get $l3
    call $float_mul
    local.set $l239
    local.get $l239
    local.set $l240
    i32.const 336
    call $shadow_site
    global.get $g12
    local.get $l235
    call $float_mul
    local.set $l241
    local.get $l241
    local.set $l242
    i32.const 337
    call $shadow_site
    local.get $l242
    local.get $l235
    call $float_sub
    local.set $l243
    i32.const 338
    call $shadow_site
    local.get $l242
    local.get $l243
    call $float_sub
    local.set $l244
    local.get $l244
    local.set $l245
    i32.const 339
    call $shadow_site
    local.get $l235
    local.get $l245
    call $float_sub
    local.set $l246
    local.get $l246
    local.set $l247
    i32.const 340
    call $shadow_site
    local.get $l245
    local.get $l10
    call $float_mul
    local.set $l248
    i32.const 341
    call $shadow_site
    local.get $l248
    local.get $l240
    call $float_sub
    local.set $l249
    i32.const 342
    call $shadow_site
    local.get $l245
    local.get $l12
    call $float_mul
    local.set $l250
    i32.const 343
    call $shadow_site
    local.get $l249
    local.get $l250
    call $float_add
    local.set $l251
    i32.const 344
    call $shadow_site
    local.get $l247
    local.get $l10
    call $float_mul
    local.set $l252
    i32.const 343
    call $shadow_site
    local.get $l251
    local.get $l252
    call $float_add
    local.set $l253
    i32.const 345
    call $shadow_site
    local.get $l247
    local.get $l12
    call $float_mul
    local.set $l254
    i32.const 346
    call $shadow_site
    local.get $l253
    local.get $l254
    call $float_add
    local.set $l255
    local.get $l255
    local.set $l256
    i32.const 347
    call $shadow_site
    local.get $l235
    local.get $l5
    call $float_mul
    local.set $l257
    i32.const 348
    call $shadow_site
    local.get $l256
    local.get $l257
    call $float_add
    local.set $l258
    i32.const 349
    call $shadow_site
    local.get $l238
    local.get $l3
    call $float_mul
    local.set $l259
    i32.const 348
    call $shadow_site
    local.get $l258
    local.get $l259
    call $float_add
    local.set $l260
    local.get $l260
    local.set $l261
    i32.const 350
    call $shadow_site
    local.get $l240
    local.get $l261
    call $float_add
    local.set $l262
    local.get $l262
    local.set $l263
    i32.const 351
    call $shadow_site
    local.get $l263
    local.get $l240
    call $float_sub
    local.set $l264
    i32.const 352
    call $shadow_site
    local.get $l261
    local.get $l264
    call $float_sub
    local.set $l265
    local.get $l265
    local.set $l266
    i32.const 353
    call $shadow_site
    global.get $g110
    local.get $l263
    call $float_add
    local.set $l267
    local.get $l267
    local.set $l268
    i32.const 354
    call $shadow_site
    local.get $l268
    global.get $g110
    call $float_sub
    local.set $l269
    i32.const 355
    call $shadow_site
    local.get $l263
    local.get $l269
    call $float_sub
    local.set $l270
    local.get $l270
    local.set $l271
    i32.const 356
    call $shadow_site
    local.get $l271
    global.get $g111
    call $float_add
    local.set $l272
    i32.const 356
    call $shadow_site
    local.get $l272
    local.get $l266
    call $float_add
    local.set $l273
    local.get $l273
    local.set $l274
    i32.const 357
    call $shadow_site
    local.get $l268
    local.get $l274
    call $float_add
    local.set $l275
    local.get $l275
    local.set $l276
    i32.const 358
    call $shadow_site
    local.get $l276
    local.get $l268
    call $float_sub
    local.set $l277
    i32.const 359
    call $shadow_site
    local.get $l274
    local.get $l277
    call $float_sub
    local.set $l278
    local.get $l278
    local.set $l279
    i32.const 360
    call $shadow_site
    local.get $l276
    local.get $l3
    call $float_mul
    local.set $l280
    local.get $l280
    local.set $l281
    i32.const 361
    call $shadow_site
    global.get $g12
    local.get $l276
    call $float_mul
    local.set $l282
    local.get $l282
    local.set $l283
    i32.const 362
    call $shadow_site
    local.get $l283
    local.get $l276
    call $float_sub
    local.set $l284
    i32.const 363
    call $shadow_site
    local.get $l283
    local.get $l284
    call $float_sub
    local.set $l285
    local.get $l285
    local.set $l286
    i32.const 364
    call $shadow_site
    local.get $l276
    local.get $l286
    call $float_sub
    local.set $l287
    local.get $l287
    local.set $l288
    i32.const 365
    call $shadow_site
    local.get $l286
    local.get $l10
    call $float_mul
    local.set $l289
    i32.const 366
    call $shadow_site
    local.get $l289
    local.get $l281
    call $float_sub
    local.set $l290
    i32.const 367
    call $shadow_site
    local.get $l286
    local.get $l12
    call $float_mul
    local.set $l291
    i32.const 368
    call $shadow_site
    local.get $l290
    local.get $l291
    call $float_add
    local.set $l292
    i32.const 369
    call $shadow_site
    local.get $l288
    local.get $l10
    call $float_mul
    local.set $l293
    i32.const 368
    call $shadow_site
    local.get $l292
    local.get $l293
    call $float_add
    local.set $l294
    i32.const 370
    call $shadow_site
    local.get $l288
    local.get $l12
    call $float_mul
    local.set $l295
    i32.const 371
    call $shadow_site
    local.get $l294
    local.get $l295
    call $float_add
    local.set $l296
    local.get $l296
    local.set $l297
    i32.const 372
    call $shadow_site
    local.get $l276
    local.get $l5
    call $float_mul
    local.set $l298
    i32.const 373
    call $shadow_site
    local.get $l297
    local.get $l298
    call $float_add
    local.set $l299
    i32.const 374
    call $shadow_site
    local.get $l279
    local.get $l3
    call $float_mul
    local.set $l300
    i32.const 373
    call $shadow_site
    local.get $l299
    local.get $l300
    call $float_add
    local.set $l301
    local.get $l301
    local.set $l302
    i32.const 375
    call $shadow_site
    local.get $l281
    local.get $l302
    call $float_add
    local.set $l303
    local.get $l303
    local.set $l304
    i32.const 376
    call $shadow_site
    local.get $l304
    local.get $l281
    call $float_sub
    local.set $l305
    i32.const 377
    call $shadow_site
    local.get $l302
    local.get $l305
    call $float_sub
    local.set $l306
    local.get $l306
    local.set $l307
    i32.const 378
    call $shadow_site
    global.get $g108
    local.get $l304
    call $float_add
    local.set $l308
    local.get $l308
    local.set $l309
    i32.const 379
    call $shadow_site
    local.get $l309
    global.get $g108
    call $float_sub
    local.set $l310
    i32.const 380
    call $shadow_site
    local.get $l304
    local.get $l310
    call $float_sub
    local.set $l311
    local.get $l311
    local.set $l312
    i32.const 381
    call $shadow_site
    local.get $l312
    global.get $g109
    call $float_add
    local.set $l313
    i32.const 381
    call $shadow_site
    local.get $l313
    local.get $l307
    call $float_add
    local.set $l314
    local.get $l314
    local.set $l315
    i32.const 382
    call $shadow_site
    local.get $l309
    local.get $l315
    call $float_add
    local.set $l316
    local.get $l316
    local.set $l317
    i32.const 383
    call $shadow_site
    local.get $l317
    local.get $l309
    call $float_sub
    local.set $l318
    i32.const 384
    call $shadow_site
    local.get $l315
    local.get $l318
    call $float_sub
    local.set $l319
    local.get $l319
    local.set $l320
    i32.const 24
    i64.const 27
    local.get $l317
    local.get $l320
    global.get $prof_324
    i64.const 1
    i64.add
    global.set $prof_324
    array.new_fixed $values 2
    call $record
    local.set $l321
    call $shadow_leave
    local.get $l321
    return
    unreachable
  )
  (func $f20 (;335;) (type $fn1) (param $clo (ref null $obj)) (param $env (ref null $env)) (param $l0 eqref) (result eqref)
    (local $l1 eqref) (local $l2 eqref) (local $l3 eqref) (local $l4 eqref) (local $l5 eqref) (local $l6 eqref) (local $l7 eqref) (local $l8 eqref) (local $l9 eqref) (local $l10 eqref) (local $l11 eqref) (local $l12 eqref) (local $l13 eqref) (local $l14 eqref) (local $l15 eqref) (local $l16 eqref) (local $l17 eqref) (local $l18 eqref) (local $l19 eqref) (local $l20 eqref) (local $l21 eqref) (local $l22 eqref) (local $l23 eqref) (local $l24 eqref) (local $l25 eqref) (local $l26 eqref) (local $l27 eqref) (local $l28 eqref) (local $l29 eqref) (local $l30 eqref) (local $l31 eqref) (local $l32 eqref) (local $l33 eqref) (local $l34 eqref) (local $l35 eqref) (local $l36 eqref) (local $l37 eqref) (local $l38 eqref) (local $l39 eqref) (local $l40 eqref) (local $l41 eqref) (local $l42 eqref) (local $l43 eqref) (local $l44 eqref) (local $l45 eqref) (local $l46 eqref) (local $l47 eqref) (local $l48 eqref) (local $l49 eqref) (local $l50 eqref) (local $l51 eqref) (local $l52 eqref) (local $l53 eqref) (local $l54 eqref) (local $l55 eqref) (local $l56 eqref) (local $l57 eqref) (local $l58 eqref) (local $l59 eqref) (local $l60 eqref) (local $l61 eqref) (local $l62 eqref) (local $l63 eqref) (local $l64 eqref) (local $l65 eqref) (local $l66 eqref) (local $l67 eqref) (local $l68 eqref) (local $l69 eqref) (local $l70 eqref) (local $l71 eqref) (local $l72 eqref) (local $l73 eqref) (local $l74 eqref) (local $l75 eqref) (local $l76 eqref) (local $l77 eqref) (local $l78 eqref) (local $l79 eqref) (local $l80 eqref) (local $l81 eqref) (local $l82 eqref) (local $l83 eqref) (local $l84 eqref) (local $l85 eqref) (local $l86 eqref) (local $l87 eqref) (local $l88 eqref) (local $l89 eqref) (local $l90 eqref) (local $l91 eqref) (local $l92 eqref) (local $l93 eqref) (local $l94 eqref) (local $l95 eqref) (local $l96 eqref) (local $l97 eqref) (local $l98 eqref) (local $l99 eqref) (local $l100 eqref) (local $l101 eqref) (local $l102 eqref) (local $l103 eqref) (local $l104 eqref) (local $l105 eqref) (local $l106 eqref) (local $l107 eqref) (local $l108 eqref) (local $l109 eqref) (local $l110 eqref) (local $l111 eqref) (local $l112 eqref) (local $l113 eqref) (local $l114 eqref) (local $l115 eqref) (local $l116 eqref) (local $l117 eqref) (local $l118 eqref) (local $l119 eqref) (local $l120 eqref) (local $l121 eqref) (local $l122 eqref) (local $l123 eqref) (local $l124 eqref) (local $l125 eqref) (local $l126 eqref) (local $l127 eqref) (local $l128 eqref) (local $l129 eqref) (local $l130 eqref) (local $l131 eqref) (local $l132 eqref) (local $l133 eqref) (local $l134 eqref) (local $l135 eqref) (local $l136 eqref) (local $l137 eqref) (local $l138 eqref) (local $l139 eqref) (local $l140 eqref) (local $l141 eqref) (local $l142 eqref) (local $l143 eqref) (local $l144 eqref) (local $l145 eqref) (local $l146 eqref) (local $l147 eqref) (local $l148 eqref) (local $l149 eqref) (local $l150 eqref) (local $l151 eqref) (local $l152 eqref) (local $l153 eqref) (local $l154 eqref) (local $l155 eqref) (local $l156 eqref) (local $l157 eqref) (local $l158 eqref) (local $l159 eqref) (local $l160 eqref) (local $l161 eqref) (local $l162 eqref) (local $l163 eqref) (local $l164 eqref) (local $l165 eqref) (local $l166 eqref) (local $l167 eqref) (local $l168 eqref) (local $l169 eqref) (local $l170 eqref) (local $l171 eqref) (local $l172 eqref) (local $l173 eqref) (local $l174 eqref) (local $l175 eqref) (local $l176 eqref) (local $l177 eqref) (local $l178 eqref) (local $l179 eqref) (local $l180 eqref) (local $l181 eqref) (local $l182 eqref) (local $l183 eqref) (local $l184 eqref) (local $l185 eqref) (local $l186 eqref) (local $l187 eqref) (local $l188 eqref) (local $l189 eqref) (local $l190 eqref) (local $l191 eqref) (local $l192 eqref) (local $l193 eqref) (local $l194 eqref) (local $l195 eqref) (local $l196 eqref) (local $l197 eqref) (local $l198 eqref) (local $l199 eqref) (local $l200 eqref) (local $l201 eqref) (local $l202 eqref) (local $l203 eqref) (local $l204 eqref) (local $l205 eqref) (local $l206 eqref) (local $l207 eqref) (local $l208 eqref) (local $l209 eqref) (local $l210 eqref) (local $l211 eqref) (local $l212 eqref) (local $l213 eqref) (local $l214 eqref) (local $l215 eqref) (local $l216 eqref) (local $l217 eqref) (local $l218 eqref) (local $l219 eqref) (local $l220 eqref) (local $l221 eqref) (local $l222 eqref) (local $l223 eqref) (local $l224 eqref) (local $l225 eqref) (local $l226 eqref) (local $l227 eqref) (local $l228 eqref) (local $l229 eqref) (local $l230 eqref) (local $l231 eqref) (local $l232 eqref) (local $l233 eqref) (local $l234 eqref) (local $l235 eqref) (local $l236 eqref) (local $l237 eqref) (local $l238 eqref) (local $l239 eqref) (local $l240 eqref) (local $l241 eqref) (local $l242 eqref) (local $l243 eqref) (local $l244 eqref) (local $l245 eqref) (local $l246 eqref) (local $l247 eqref) (local $l248 eqref) (local $l249 eqref) (local $l250 eqref) (local $l251 eqref) (local $l252 eqref) (local $l253 eqref) (local $l254 eqref) (local $l255 eqref) (local $l256 eqref) (local $l257 eqref) (local $l258 eqref) (local $l259 eqref) (local $l260 eqref) (local $l261 eqref) (local $l262 eqref) (local $l263 eqref) (local $l264 eqref) (local $l265 eqref) (local $l266 eqref) (local $l267 eqref) (local $l268 eqref) (local $l269 eqref) (local $l270 eqref) (local $l271 eqref) (local $l272 eqref) (local $l273 eqref) (local $l274 eqref) (local $l275 eqref) (local $l276 eqref) (local $l277 eqref) (local $l278 eqref) (local $l279 eqref) (local $l280 eqref) (local $l281 eqref) (local $l282 eqref) (local $l283 eqref) (local $slot i32)
    global.get $prof_391
    i64.const 1
    i64.add
    global.set $prof_391
    call $shadow_enter_call
    local.get $l0
    local.set $l1
    local.get $l1
    i32.const 0
    call $field
    local.set $l2
    local.get $l2
    local.set $l3
    local.get $l1
    i32.const 1
    call $field
    local.set $l4
    local.get $l4
    local.set $l5
    i32.const 39
    call $shadow_site
    global.get $g12
    local.get $l3
    call $float_mul
    local.set $l6
    local.get $l6
    local.set $l7
    i32.const 40
    call $shadow_site
    local.get $l7
    local.get $l3
    call $float_sub
    local.set $l8
    i32.const 41
    call $shadow_site
    local.get $l7
    local.get $l8
    call $float_sub
    local.set $l9
    local.get $l9
    local.set $l10
    i32.const 42
    call $shadow_site
    local.get $l3
    local.get $l10
    call $float_sub
    local.set $l11
    local.get $l11
    local.set $l12
    global.get $g107
    local.set $l13
    i32.const 43
    call $shadow_site
    local.get $l13
    local.get $l3
    call $float_mul
    local.set $l14
    i32.const 43
    call $shadow_site
    local.get $l14
    global.get $g106
    call $float_add
    local.set $l15
    local.get $l15
    local.set $l16
    i32.const 44
    call $shadow_site
    local.get $l16
    local.get $l3
    call $float_mul
    local.set $l17
    i32.const 44
    call $shadow_site
    local.get $l17
    global.get $g105
    call $float_add
    local.set $l18
    local.get $l18
    local.set $l19
    i32.const 45
    call $shadow_site
    local.get $l19
    local.get $l3
    call $float_mul
    local.set $l20
    i32.const 45
    call $shadow_site
    local.get $l20
    global.get $g104
    call $float_add
    local.set $l21
    local.get $l21
    local.set $l22
    i32.const 46
    call $shadow_site
    local.get $l22
    local.get $l3
    call $float_mul
    local.set $l23
    i32.const 46
    call $shadow_site
    local.get $l23
    global.get $g103
    call $float_add
    local.set $l24
    local.get $l24
    local.set $l25
    i32.const 47
    call $shadow_site
    local.get $l25
    local.get $l3
    call $float_mul
    local.set $l26
    i32.const 47
    call $shadow_site
    local.get $l26
    global.get $g102
    call $float_add
    local.set $l27
    local.get $l27
    local.set $l28
    i32.const 48
    call $shadow_site
    local.get $l28
    local.get $l3
    call $float_mul
    local.set $l29
    i32.const 48
    call $shadow_site
    local.get $l29
    global.get $g101
    call $float_add
    local.set $l30
    local.get $l30
    local.set $l31
    i32.const 49
    call $shadow_site
    local.get $l31
    local.get $l3
    call $float_mul
    local.set $l32
    i32.const 49
    call $shadow_site
    local.get $l32
    global.get $g100
    call $float_add
    local.set $l33
    local.get $l33
    local.set $l34
    local.get $l34
    local.set $l35
    global.get $g15
    local.set $l36
    i32.const 50
    call $shadow_site
    local.get $l35
    local.get $l3
    call $float_mul
    local.set $l37
    local.get $l37
    local.set $l38
    i32.const 51
    call $shadow_site
    global.get $g12
    local.get $l35
    call $float_mul
    local.set $l39
    local.get $l39
    local.set $l40
    i32.const 52
    call $shadow_site
    local.get $l40
    local.get $l35
    call $float_sub
    local.set $l41
    i32.const 53
    call $shadow_site
    local.get $l40
    local.get $l41
    call $float_sub
    local.set $l42
    local.get $l42
    local.set $l43
    i32.const 54
    call $shadow_site
    local.get $l35
    local.get $l43
    call $float_sub
    local.set $l44
    local.get $l44
    local.set $l45
    i32.const 55
    call $shadow_site
    local.get $l43
    local.get $l10
    call $float_mul
    local.set $l46
    i32.const 56
    call $shadow_site
    local.get $l46
    local.get $l38
    call $float_sub
    local.set $l47
    i32.const 57
    call $shadow_site
    local.get $l43
    local.get $l12
    call $float_mul
    local.set $l48
    i32.const 58
    call $shadow_site
    local.get $l47
    local.get $l48
    call $float_add
    local.set $l49
    i32.const 59
    call $shadow_site
    local.get $l45
    local.get $l10
    call $float_mul
    local.set $l50
    i32.const 58
    call $shadow_site
    local.get $l49
    local.get $l50
    call $float_add
    local.set $l51
    i32.const 60
    call $shadow_site
    local.get $l45
    local.get $l12
    call $float_mul
    local.set $l52
    i32.const 61
    call $shadow_site
    local.get $l51
    local.get $l52
    call $float_add
    local.set $l53
    local.get $l53
    local.set $l54
    i32.const 62
    call $shadow_site
    local.get $l35
    local.get $l5
    call $float_mul
    local.set $l55
    i32.const 63
    call $shadow_site
    local.get $l54
    local.get $l55
    call $float_add
    local.set $l56
    i32.const 64
    call $shadow_site
    local.get $l36
    local.get $l3
    call $float_mul
    local.set $l57
    i32.const 63
    call $shadow_site
    local.get $l56
    local.get $l57
    call $float_add
    local.set $l58
    local.get $l58
    local.set $l59
    i32.const 65
    call $shadow_site
    local.get $l38
    local.get $l59
    call $float_add
    local.set $l60
    local.get $l60
    local.set $l61
    i32.const 66
    call $shadow_site
    local.get $l61
    local.get $l38
    call $float_sub
    local.set $l62
    i32.const 67
    call $shadow_site
    local.get $l59
    local.get $l62
    call $float_sub
    local.set $l63
    local.get $l63
    local.set $l64
    i32.const 68
    call $shadow_site
    global.get $g98
    local.get $l61
    call $float_add
    local.set $l65
    local.get $l65
    local.set $l66
    i32.const 69
    call $shadow_site
    local.get $l66
    global.get $g98
    call $float_sub
    local.set $l67
    i32.const 70
    call $shadow_site
    local.get $l61
    local.get $l67
    call $float_sub
    local.set $l68
    local.get $l68
    local.set $l69
    i32.const 71
    call $shadow_site
    local.get $l69
    global.get $g99
    call $float_add
    local.set $l70
    i32.const 71
    call $shadow_site
    local.get $l70
    local.get $l64
    call $float_add
    local.set $l71
    local.get $l71
    local.set $l72
    i32.const 72
    call $shadow_site
    local.get $l66
    local.get $l72
    call $float_add
    local.set $l73
    local.get $l73
    local.set $l74
    i32.const 73
    call $shadow_site
    local.get $l74
    local.get $l66
    call $float_sub
    local.set $l75
    i32.const 74
    call $shadow_site
    local.get $l72
    local.get $l75
    call $float_sub
    local.set $l76
    local.get $l76
    local.set $l77
    i32.const 75
    call $shadow_site
    local.get $l74
    local.get $l3
    call $float_mul
    local.set $l78
    local.get $l78
    local.set $l79
    i32.const 76
    call $shadow_site
    global.get $g12
    local.get $l74
    call $float_mul
    local.set $l80
    local.get $l80
    local.set $l81
    i32.const 77
    call $shadow_site
    local.get $l81
    local.get $l74
    call $float_sub
    local.set $l82
    i32.const 78
    call $shadow_site
    local.get $l81
    local.get $l82
    call $float_sub
    local.set $l83
    local.get $l83
    local.set $l84
    i32.const 79
    call $shadow_site
    local.get $l74
    local.get $l84
    call $float_sub
    local.set $l85
    local.get $l85
    local.set $l86
    i32.const 80
    call $shadow_site
    local.get $l84
    local.get $l10
    call $float_mul
    local.set $l87
    i32.const 81
    call $shadow_site
    local.get $l87
    local.get $l79
    call $float_sub
    local.set $l88
    i32.const 82
    call $shadow_site
    local.get $l84
    local.get $l12
    call $float_mul
    local.set $l89
    i32.const 83
    call $shadow_site
    local.get $l88
    local.get $l89
    call $float_add
    local.set $l90
    i32.const 84
    call $shadow_site
    local.get $l86
    local.get $l10
    call $float_mul
    local.set $l91
    i32.const 83
    call $shadow_site
    local.get $l90
    local.get $l91
    call $float_add
    local.set $l92
    i32.const 85
    call $shadow_site
    local.get $l86
    local.get $l12
    call $float_mul
    local.set $l93
    i32.const 86
    call $shadow_site
    local.get $l92
    local.get $l93
    call $float_add
    local.set $l94
    local.get $l94
    local.set $l95
    i32.const 87
    call $shadow_site
    local.get $l74
    local.get $l5
    call $float_mul
    local.set $l96
    i32.const 88
    call $shadow_site
    local.get $l95
    local.get $l96
    call $float_add
    local.set $l97
    i32.const 89
    call $shadow_site
    local.get $l77
    local.get $l3
    call $float_mul
    local.set $l98
    i32.const 88
    call $shadow_site
    local.get $l97
    local.get $l98
    call $float_add
    local.set $l99
    local.get $l99
    local.set $l100
    i32.const 90
    call $shadow_site
    local.get $l79
    local.get $l100
    call $float_add
    local.set $l101
    local.get $l101
    local.set $l102
    i32.const 91
    call $shadow_site
    local.get $l102
    local.get $l79
    call $float_sub
    local.set $l103
    i32.const 92
    call $shadow_site
    local.get $l100
    local.get $l103
    call $float_sub
    local.set $l104
    local.get $l104
    local.set $l105
    i32.const 93
    call $shadow_site
    global.get $g96
    local.get $l102
    call $float_add
    local.set $l106
    local.get $l106
    local.set $l107
    i32.const 94
    call $shadow_site
    local.get $l107
    global.get $g96
    call $float_sub
    local.set $l108
    i32.const 95
    call $shadow_site
    local.get $l102
    local.get $l108
    call $float_sub
    local.set $l109
    local.get $l109
    local.set $l110
    i32.const 96
    call $shadow_site
    local.get $l110
    global.get $g97
    call $float_add
    local.set $l111
    i32.const 96
    call $shadow_site
    local.get $l111
    local.get $l105
    call $float_add
    local.set $l112
    local.get $l112
    local.set $l113
    i32.const 97
    call $shadow_site
    local.get $l107
    local.get $l113
    call $float_add
    local.set $l114
    local.get $l114
    local.set $l115
    i32.const 98
    call $shadow_site
    local.get $l115
    local.get $l107
    call $float_sub
    local.set $l116
    i32.const 99
    call $shadow_site
    local.get $l113
    local.get $l116
    call $float_sub
    local.set $l117
    local.get $l117
    local.set $l118
    i32.const 100
    call $shadow_site
    local.get $l115
    local.get $l3
    call $float_mul
    local.set $l119
    local.get $l119
    local.set $l120
    i32.const 101
    call $shadow_site
    global.get $g12
    local.get $l115
    call $float_mul
    local.set $l121
    local.get $l121
    local.set $l122
    i32.const 102
    call $shadow_site
    local.get $l122
    local.get $l115
    call $float_sub
    local.set $l123
    i32.const 103
    call $shadow_site
    local.get $l122
    local.get $l123
    call $float_sub
    local.set $l124
    local.get $l124
    local.set $l125
    i32.const 104
    call $shadow_site
    local.get $l115
    local.get $l125
    call $float_sub
    local.set $l126
    local.get $l126
    local.set $l127
    i32.const 105
    call $shadow_site
    local.get $l125
    local.get $l10
    call $float_mul
    local.set $l128
    i32.const 106
    call $shadow_site
    local.get $l128
    local.get $l120
    call $float_sub
    local.set $l129
    i32.const 107
    call $shadow_site
    local.get $l125
    local.get $l12
    call $float_mul
    local.set $l130
    i32.const 108
    call $shadow_site
    local.get $l129
    local.get $l130
    call $float_add
    local.set $l131
    i32.const 109
    call $shadow_site
    local.get $l127
    local.get $l10
    call $float_mul
    local.set $l132
    i32.const 108
    call $shadow_site
    local.get $l131
    local.get $l132
    call $float_add
    local.set $l133
    i32.const 110
    call $shadow_site
    local.get $l127
    local.get $l12
    call $float_mul
    local.set $l134
    i32.const 111
    call $shadow_site
    local.get $l133
    local.get $l134
    call $float_add
    local.set $l135
    local.get $l135
    local.set $l136
    i32.const 112
    call $shadow_site
    local.get $l115
    local.get $l5
    call $float_mul
    local.set $l137
    i32.const 113
    call $shadow_site
    local.get $l136
    local.get $l137
    call $float_add
    local.set $l138
    i32.const 114
    call $shadow_site
    local.get $l118
    local.get $l3
    call $float_mul
    local.set $l139
    i32.const 113
    call $shadow_site
    local.get $l138
    local.get $l139
    call $float_add
    local.set $l140
    local.get $l140
    local.set $l141
    i32.const 115
    call $shadow_site
    local.get $l120
    local.get $l141
    call $float_add
    local.set $l142
    local.get $l142
    local.set $l143
    i32.const 116
    call $shadow_site
    local.get $l143
    local.get $l120
    call $float_sub
    local.set $l144
    i32.const 117
    call $shadow_site
    local.get $l141
    local.get $l144
    call $float_sub
    local.set $l145
    local.get $l145
    local.set $l146
    i32.const 118
    call $shadow_site
    global.get $g94
    local.get $l143
    call $float_add
    local.set $l147
    local.get $l147
    local.set $l148
    i32.const 119
    call $shadow_site
    local.get $l148
    global.get $g94
    call $float_sub
    local.set $l149
    i32.const 120
    call $shadow_site
    local.get $l143
    local.get $l149
    call $float_sub
    local.set $l150
    local.get $l150
    local.set $l151
    i32.const 121
    call $shadow_site
    local.get $l151
    global.get $g95
    call $float_add
    local.set $l152
    i32.const 121
    call $shadow_site
    local.get $l152
    local.get $l146
    call $float_add
    local.set $l153
    local.get $l153
    local.set $l154
    i32.const 122
    call $shadow_site
    local.get $l148
    local.get $l154
    call $float_add
    local.set $l155
    local.get $l155
    local.set $l156
    i32.const 123
    call $shadow_site
    local.get $l156
    local.get $l148
    call $float_sub
    local.set $l157
    i32.const 124
    call $shadow_site
    local.get $l154
    local.get $l157
    call $float_sub
    local.set $l158
    local.get $l158
    local.set $l159
    i32.const 125
    call $shadow_site
    local.get $l156
    local.get $l3
    call $float_mul
    local.set $l160
    local.get $l160
    local.set $l161
    i32.const 126
    call $shadow_site
    global.get $g12
    local.get $l156
    call $float_mul
    local.set $l162
    local.get $l162
    local.set $l163
    i32.const 127
    call $shadow_site
    local.get $l163
    local.get $l156
    call $float_sub
    local.set $l164
    i32.const 128
    call $shadow_site
    local.get $l163
    local.get $l164
    call $float_sub
    local.set $l165
    local.get $l165
    local.set $l166
    i32.const 129
    call $shadow_site
    local.get $l156
    local.get $l166
    call $float_sub
    local.set $l167
    local.get $l167
    local.set $l168
    i32.const 130
    call $shadow_site
    local.get $l166
    local.get $l10
    call $float_mul
    local.set $l169
    i32.const 131
    call $shadow_site
    local.get $l169
    local.get $l161
    call $float_sub
    local.set $l170
    i32.const 132
    call $shadow_site
    local.get $l166
    local.get $l12
    call $float_mul
    local.set $l171
    i32.const 133
    call $shadow_site
    local.get $l170
    local.get $l171
    call $float_add
    local.set $l172
    i32.const 134
    call $shadow_site
    local.get $l168
    local.get $l10
    call $float_mul
    local.set $l173
    i32.const 133
    call $shadow_site
    local.get $l172
    local.get $l173
    call $float_add
    local.set $l174
    i32.const 135
    call $shadow_site
    local.get $l168
    local.get $l12
    call $float_mul
    local.set $l175
    i32.const 136
    call $shadow_site
    local.get $l174
    local.get $l175
    call $float_add
    local.set $l176
    local.get $l176
    local.set $l177
    i32.const 137
    call $shadow_site
    local.get $l156
    local.get $l5
    call $float_mul
    local.set $l178
    i32.const 138
    call $shadow_site
    local.get $l177
    local.get $l178
    call $float_add
    local.set $l179
    i32.const 139
    call $shadow_site
    local.get $l159
    local.get $l3
    call $float_mul
    local.set $l180
    i32.const 138
    call $shadow_site
    local.get $l179
    local.get $l180
    call $float_add
    local.set $l181
    local.get $l181
    local.set $l182
    i32.const 140
    call $shadow_site
    local.get $l161
    local.get $l182
    call $float_add
    local.set $l183
    local.get $l183
    local.set $l184
    i32.const 141
    call $shadow_site
    local.get $l184
    local.get $l161
    call $float_sub
    local.set $l185
    i32.const 142
    call $shadow_site
    local.get $l182
    local.get $l185
    call $float_sub
    local.set $l186
    local.get $l186
    local.set $l187
    i32.const 143
    call $shadow_site
    global.get $g92
    local.get $l184
    call $float_add
    local.set $l188
    local.get $l188
    local.set $l189
    i32.const 144
    call $shadow_site
    local.get $l189
    global.get $g92
    call $float_sub
    local.set $l190
    i32.const 145
    call $shadow_site
    local.get $l184
    local.get $l190
    call $float_sub
    local.set $l191
    local.get $l191
    local.set $l192
    i32.const 146
    call $shadow_site
    local.get $l192
    global.get $g93
    call $float_add
    local.set $l193
    i32.const 146
    call $shadow_site
    local.get $l193
    local.get $l187
    call $float_add
    local.set $l194
    local.get $l194
    local.set $l195
    i32.const 147
    call $shadow_site
    local.get $l189
    local.get $l195
    call $float_add
    local.set $l196
    local.get $l196
    local.set $l197
    i32.const 148
    call $shadow_site
    local.get $l197
    local.get $l189
    call $float_sub
    local.set $l198
    i32.const 149
    call $shadow_site
    local.get $l195
    local.get $l198
    call $float_sub
    local.set $l199
    local.get $l199
    local.set $l200
    i32.const 150
    call $shadow_site
    local.get $l197
    local.get $l3
    call $float_mul
    local.set $l201
    local.get $l201
    local.set $l202
    i32.const 151
    call $shadow_site
    global.get $g12
    local.get $l197
    call $float_mul
    local.set $l203
    local.get $l203
    local.set $l204
    i32.const 152
    call $shadow_site
    local.get $l204
    local.get $l197
    call $float_sub
    local.set $l205
    i32.const 153
    call $shadow_site
    local.get $l204
    local.get $l205
    call $float_sub
    local.set $l206
    local.get $l206
    local.set $l207
    i32.const 154
    call $shadow_site
    local.get $l197
    local.get $l207
    call $float_sub
    local.set $l208
    local.get $l208
    local.set $l209
    i32.const 155
    call $shadow_site
    local.get $l207
    local.get $l10
    call $float_mul
    local.set $l210
    i32.const 156
    call $shadow_site
    local.get $l210
    local.get $l202
    call $float_sub
    local.set $l211
    i32.const 157
    call $shadow_site
    local.get $l207
    local.get $l12
    call $float_mul
    local.set $l212
    i32.const 158
    call $shadow_site
    local.get $l211
    local.get $l212
    call $float_add
    local.set $l213
    i32.const 159
    call $shadow_site
    local.get $l209
    local.get $l10
    call $float_mul
    local.set $l214
    i32.const 158
    call $shadow_site
    local.get $l213
    local.get $l214
    call $float_add
    local.set $l215
    i32.const 160
    call $shadow_site
    local.get $l209
    local.get $l12
    call $float_mul
    local.set $l216
    i32.const 161
    call $shadow_site
    local.get $l215
    local.get $l216
    call $float_add
    local.set $l217
    local.get $l217
    local.set $l218
    i32.const 162
    call $shadow_site
    local.get $l197
    local.get $l5
    call $float_mul
    local.set $l219
    i32.const 163
    call $shadow_site
    local.get $l218
    local.get $l219
    call $float_add
    local.set $l220
    i32.const 164
    call $shadow_site
    local.get $l200
    local.get $l3
    call $float_mul
    local.set $l221
    i32.const 163
    call $shadow_site
    local.get $l220
    local.get $l221
    call $float_add
    local.set $l222
    local.get $l222
    local.set $l223
    i32.const 165
    call $shadow_site
    local.get $l202
    local.get $l223
    call $float_add
    local.set $l224
    local.get $l224
    local.set $l225
    i32.const 166
    call $shadow_site
    local.get $l225
    local.get $l202
    call $float_sub
    local.set $l226
    i32.const 167
    call $shadow_site
    local.get $l223
    local.get $l226
    call $float_sub
    local.set $l227
    local.get $l227
    local.set $l228
    i32.const 168
    call $shadow_site
    global.get $g90
    local.get $l225
    call $float_add
    local.set $l229
    local.get $l229
    local.set $l230
    i32.const 169
    call $shadow_site
    local.get $l230
    global.get $g90
    call $float_sub
    local.set $l231
    i32.const 170
    call $shadow_site
    local.get $l225
    local.get $l231
    call $float_sub
    local.set $l232
    local.get $l232
    local.set $l233
    i32.const 171
    call $shadow_site
    local.get $l233
    global.get $g91
    call $float_add
    local.set $l234
    i32.const 171
    call $shadow_site
    local.get $l234
    local.get $l228
    call $float_add
    local.set $l235
    local.get $l235
    local.set $l236
    i32.const 172
    call $shadow_site
    local.get $l230
    local.get $l236
    call $float_add
    local.set $l237
    local.get $l237
    local.set $l238
    i32.const 173
    call $shadow_site
    local.get $l238
    local.get $l230
    call $float_sub
    local.set $l239
    i32.const 174
    call $shadow_site
    local.get $l236
    local.get $l239
    call $float_sub
    local.set $l240
    local.get $l240
    local.set $l241
    i32.const 175
    call $shadow_site
    local.get $l238
    local.get $l3
    call $float_mul
    local.set $l242
    local.get $l242
    local.set $l243
    i32.const 176
    call $shadow_site
    global.get $g12
    local.get $l238
    call $float_mul
    local.set $l244
    local.get $l244
    local.set $l245
    i32.const 177
    call $shadow_site
    local.get $l245
    local.get $l238
    call $float_sub
    local.set $l246
    i32.const 178
    call $shadow_site
    local.get $l245
    local.get $l246
    call $float_sub
    local.set $l247
    local.get $l247
    local.set $l248
    i32.const 179
    call $shadow_site
    local.get $l238
    local.get $l248
    call $float_sub
    local.set $l249
    local.get $l249
    local.set $l250
    i32.const 180
    call $shadow_site
    local.get $l248
    local.get $l10
    call $float_mul
    local.set $l251
    i32.const 181
    call $shadow_site
    local.get $l251
    local.get $l243
    call $float_sub
    local.set $l252
    i32.const 182
    call $shadow_site
    local.get $l248
    local.get $l12
    call $float_mul
    local.set $l253
    i32.const 183
    call $shadow_site
    local.get $l252
    local.get $l253
    call $float_add
    local.set $l254
    i32.const 184
    call $shadow_site
    local.get $l250
    local.get $l10
    call $float_mul
    local.set $l255
    i32.const 183
    call $shadow_site
    local.get $l254
    local.get $l255
    call $float_add
    local.set $l256
    i32.const 185
    call $shadow_site
    local.get $l250
    local.get $l12
    call $float_mul
    local.set $l257
    i32.const 186
    call $shadow_site
    local.get $l256
    local.get $l257
    call $float_add
    local.set $l258
    local.get $l258
    local.set $l259
    i32.const 187
    call $shadow_site
    local.get $l238
    local.get $l5
    call $float_mul
    local.set $l260
    i32.const 188
    call $shadow_site
    local.get $l259
    local.get $l260
    call $float_add
    local.set $l261
    i32.const 189
    call $shadow_site
    local.get $l241
    local.get $l3
    call $float_mul
    local.set $l262
    i32.const 188
    call $shadow_site
    local.get $l261
    local.get $l262
    call $float_add
    local.set $l263
    local.get $l263
    local.set $l264
    i32.const 190
    call $shadow_site
    local.get $l243
    local.get $l264
    call $float_add
    local.set $l265
    local.get $l265
    local.set $l266
    i32.const 191
    call $shadow_site
    local.get $l266
    local.get $l243
    call $float_sub
    local.set $l267
    i32.const 192
    call $shadow_site
    local.get $l264
    local.get $l267
    call $float_sub
    local.set $l268
    local.get $l268
    local.set $l269
    i32.const 193
    call $shadow_site
    global.get $g88
    local.get $l266
    call $float_add
    local.set $l270
    local.get $l270
    local.set $l271
    i32.const 194
    call $shadow_site
    local.get $l271
    global.get $g88
    call $float_sub
    local.set $l272
    i32.const 195
    call $shadow_site
    local.get $l266
    local.get $l272
    call $float_sub
    local.set $l273
    local.get $l273
    local.set $l274
    i32.const 196
    call $shadow_site
    local.get $l274
    global.get $g89
    call $float_add
    local.set $l275
    i32.const 196
    call $shadow_site
    local.get $l275
    local.get $l269
    call $float_add
    local.set $l276
    local.get $l276
    local.set $l277
    i32.const 197
    call $shadow_site
    local.get $l271
    local.get $l277
    call $float_add
    local.set $l278
    local.get $l278
    local.set $l279
    i32.const 198
    call $shadow_site
    local.get $l279
    local.get $l271
    call $float_sub
    local.set $l280
    i32.const 199
    call $shadow_site
    local.get $l277
    local.get $l280
    call $float_sub
    local.set $l281
    local.get $l281
    local.set $l282
    i32.const 24
    i64.const 27
    local.get $l279
    local.get $l282
    global.get $prof_324
    i64.const 1
    i64.add
    global.set $prof_324
    array.new_fixed $values 2
    call $record
    local.set $l283
    call $shadow_leave
    local.get $l283
    return
    unreachable
  )
  (func $f21 (;336;) (type $fn1) (param $clo (ref null $obj)) (param $env (ref null $env)) (param $l0 eqref) (result eqref)
    (local $l1 eqref) (local $l2 eqref) (local $l3 eqref) (local $l4 eqref) (local $l5 eqref) (local $l6 eqref) (local $slot i32)
    global.get $prof_392
    i64.const 1
    i64.add
    global.set $prof_392
    call $shadow_enter_call
    local.get $l0
    local.set $l1
    local.get $l1
    i32.const 0
    call $field
    local.set $l2
    local.get $l2
    call $float_neg
    local.set $l3
    local.get $l1
    i32.const 1
    call $field
    local.set $l4
    local.get $l4
    call $float_neg
    local.set $l5
    i32.const 24
    i64.const 27
    local.get $l3
    local.get $l5
    global.get $prof_324
    i64.const 1
    i64.add
    global.set $prof_324
    array.new_fixed $values 2
    call $record
    local.set $l6
    call $shadow_leave
    local.get $l6
    return
    unreachable
  )
  (func $f22 (;337;) (type $fn2) (param $clo (ref null $obj)) (param $env (ref null $env)) (param $l0 eqref) (param $l1 eqref) (result eqref)
    (local $l2 eqref) (local $l3 eqref) (local $l4 eqref) (local $l5 eqref) (local $l6 eqref) (local $l7 eqref) (local $l8 eqref) (local $slot i32)
    global.get $prof_393
    i64.const 1
    i64.add
    global.set $prof_393
    call $shadow_enter_call
    local.get $l0
    local.set $l2
    local.get $l1
    local.set $l3
    i32.const 405
    call $shadow_site
    local.get $l2
    local.get $l3
    call $float_add
    local.set $l4
    local.get $l4
    local.set $l5
    i32.const 406
    call $shadow_site
    local.get $l5
    local.get $l2
    call $float_sub
    local.set $l6
    i32.const 407
    call $shadow_site
    local.get $l3
    local.get $l6
    call $float_sub
    local.set $l7
    i32.const 24
    i64.const 27
    local.get $l5
    local.get $l7
    global.get $prof_324
    i64.const 1
    i64.add
    global.set $prof_324
    array.new_fixed $values 2
    call $record
    local.set $l8
    call $shadow_leave
    local.get $l8
    return
    unreachable
  )
  (func $f23 (;338;) (type $fn2) (param $clo (ref null $obj)) (param $env (ref null $env)) (param $l0 eqref) (param $l1 eqref) (result eqref)
    (local $l2 eqref) (local $l3 eqref) (local $l4 eqref) (local $l5 eqref) (local $l6 eqref) (local $l7 eqref) (local $l8 eqref) (local $l9 eqref) (local $l10 eqref) (local $l11 eqref) (local $l12 eqref) (local $l13 eqref) (local $slot i32)
    global.get $prof_394
    i64.const 1
    i64.add
    global.set $prof_394
    call $shadow_enter_call
    local.get $l0
    local.set $l2
    local.get $l1
    local.set $l3
    local.get $l2
    i32.const 0
    call $field
    local.set $l4
    local.get $l3
    i32.const 0
    call $field
    local.set $l5
    i32.const 424
    call $shadow_site
    ref.null $obj
    local.get $env
    local.get $l4
    local.get $l5
    call $f24
    local.set $l6
    local.get $l6
    local.set $l7
    local.get $l7
    i32.const 0
    call $field
    local.set $l8
    local.get $l7
    i32.const 1
    call $field
    local.set $l9
    local.get $l2
    i32.const 1
    call $field
    local.set $l10
    i32.const 425
    call $shadow_site
    local.get $l9
    local.get $l10
    call $float_add
    local.set $l11
    local.get $l3
    i32.const 1
    call $field
    local.set $l12
    i32.const 425
    call $shadow_site
    local.get $l11
    local.get $l12
    call $float_add
    local.set $l13
    i32.const -428
    call $shadow_leave_tail
    ref.null $obj
    local.get $env
    local.get $l8
    local.get $l13
    return_call $f24
    unreachable
  )
  (func $f24 (;339;) (type $fn2) (param $clo (ref null $obj)) (param $env (ref null $env)) (param $l0 eqref) (param $l1 eqref) (result eqref)
    (local $l2 eqref) (local $l3 eqref) (local $l4 eqref) (local $l5 eqref) (local $l6 eqref) (local $l7 eqref) (local $l8 eqref) (local $l9 eqref) (local $l10 eqref) (local $l11 eqref) (local $l12 eqref) (local $slot i32)
    global.get $prof_395
    i64.const 1
    i64.add
    global.set $prof_395
    call $shadow_enter_call
    local.get $l0
    local.set $l2
    local.get $l1
    local.set $l3
    i32.const 427
    call $shadow_site
    local.get $l2
    local.get $l3
    call $float_add
    local.set $l4
    local.get $l4
    local.set $l5
    i32.const 428
    call $shadow_site
    local.get $l5
    local.get $l2
    call $float_sub
    local.set $l6
    local.get $l6
    local.set $l7
    i32.const 429
    call $shadow_site
    local.get $l5
    local.get $l7
    call $float_sub
    local.set $l8
    i32.const 430
    call $shadow_site
    local.get $l2
    local.get $l8
    call $float_sub
    local.set $l9
    i32.const 431
    call $shadow_site
    local.get $l3
    local.get $l7
    call $float_sub
    local.set $l10
    i32.const 432
    call $shadow_site
    local.get $l9
    local.get $l10
    call $float_add
    local.set $l11
    i32.const 24
    i64.const 27
    local.get $l5
    local.get $l11
    global.get $prof_324
    i64.const 1
    i64.add
    global.set $prof_324
    array.new_fixed $values 2
    call $record
    local.set $l12
    call $shadow_leave
    local.get $l12
    return
    unreachable
  )
  (func $s0 (;340;) (type 137) (param $env (ref null $env)) (param $a0 eqref) (param $a1 eqref) (param $res eqref) (result eqref)
    (local $clo (ref null $obj)) (local $slot i32) (local $l5 eqref) (local $l9 eqref) (local $l10 eqref)
    global.get $prof_396
    i64.const 1
    i64.add
    global.set $prof_396
    local.get $a0
    local.set $l5
    local.get $a1
    local.set $l9
    local.get $res
    local.set $l10
    i32.const -6
    call $shadow_leave_tail
    ref.null $obj
    local.get $env
    local.get $l9
    local.get $l10
    local.get $l5
    return_call $f7
    unreachable
  )
  (func $r0 (;341;) (type $refn) (param $frame (ref $frame)) (param $env (ref null $env)) (param $v eqref) (param $mode i32) (result eqref)
    (local $saved (ref null $values)) (local $l5 eqref) (local $l9 eqref)
    global.get $prof_397
    i64.const 1
    i64.add
    global.set $prof_397
    local.get $frame
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
    struct.get $frame 1
    local.set $saved
    local.get $saved
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
    ref.as_non_null
    i32.const 0
    global.get $prof_12
    i64.const 1
    i64.add
    global.set $prof_12
    array.get $values
    local.set $l5
    local.get $saved
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
    ref.as_non_null
    i32.const 1
    global.get $prof_12
    i64.const 1
    i64.add
    global.set $prof_12
    array.get $values
    local.set $l9
    local.get $frame
    global.get $prof_6
    i64.const 1
    i64.add
    global.set $prof_6
    struct.get $frame 2
    call $shadow_enter
    local.get $env
    local.get $l5
    local.get $l9
    local.get $v
    return_call $s0
  )
  (func $j0 (;342;) (type 138) (param $env (ref null $env)) (result eqref)
    (local $clo (ref null $obj)) (local $slot i32)
    global.get $prof_398
    i64.const 1
    i64.add
    global.set $prof_398
    unreachable
  )
  (func $j1 (;343;) (type 138) (param $env (ref null $env)) (result eqref)
    (local $clo (ref null $obj)) (local $slot i32)
    global.get $prof_399
    i64.const 1
    i64.add
    global.set $prof_399
    unreachable
    unreachable
  )
  (func $start (;344;) (type $fn0) (param $clo (ref null $obj)) (param $env (ref null $env)) (result eqref)
    (local $l0 eqref) (local $l1 eqref) (local $l2 eqref) (local $l3 eqref) (local $l4 eqref) (local $l5 eqref) (local $l6 eqref) (local $l7 eqref) (local $l8 eqref) (local $l9 eqref) (local $l10 eqref) (local $l11 eqref) (local $l12 eqref) (local $l13 eqref) (local $l14 eqref) (local $l15 eqref) (local $l16 eqref) (local $l17 eqref) (local $l18 eqref) (local $l19 eqref) (local $l20 eqref) (local $l21 eqref) (local $l22 eqref) (local $l23 eqref) (local $l24 eqref) (local $l25 eqref) (local $l26 eqref) (local $l27 eqref) (local $l28 eqref) (local $l29 eqref) (local $l30 eqref) (local $l31 eqref) (local $l32 eqref) (local $l33 eqref) (local $l34 eqref) (local $l35 eqref) (local $l36 eqref) (local $l37 eqref) (local $l38 eqref) (local $l39 eqref) (local $l40 eqref) (local $l41 eqref) (local $l42 eqref) (local $l43 eqref) (local $l44 eqref) (local $l45 eqref) (local $l46 eqref) (local $l47 eqref) (local $l48 eqref) (local $l49 eqref) (local $l50 eqref) (local $l51 eqref) (local $l52 eqref) (local $l53 eqref) (local $l54 eqref) (local $l55 eqref) (local $l56 eqref) (local $l57 eqref) (local $l58 eqref) (local $l59 eqref) (local $l60 eqref) (local $l61 eqref) (local $l62 eqref) (local $l63 eqref) (local $l64 eqref) (local $l65 eqref) (local $l66 eqref) (local $l67 eqref) (local $l68 eqref) (local $l69 eqref) (local $l70 eqref) (local $l71 eqref) (local $l72 eqref) (local $l73 eqref) (local $l74 eqref) (local $l75 eqref) (local $l76 eqref) (local $l77 eqref) (local $l78 eqref) (local $l79 eqref) (local $l80 eqref) (local $l81 eqref) (local $l82 eqref) (local $l83 eqref) (local $l84 eqref) (local $l85 eqref) (local $l86 eqref) (local $l87 eqref) (local $l88 eqref) (local $l89 eqref) (local $l90 eqref) (local $l91 eqref) (local $l92 eqref) (local $l93 eqref) (local $l94 eqref) (local $l95 eqref) (local $l96 eqref) (local $l97 eqref) (local $l98 eqref) (local $l99 eqref) (local $l100 eqref) (local $l101 eqref) (local $l102 eqref) (local $l103 eqref) (local $l104 eqref) (local $l105 eqref) (local $l106 eqref) (local $l107 eqref) (local $l108 eqref) (local $l109 eqref) (local $l110 eqref) (local $l111 eqref) (local $l112 eqref) (local $l113 eqref) (local $l114 eqref) (local $l115 eqref) (local $l116 eqref) (local $l117 eqref) (local $l118 eqref) (local $l119 eqref) (local $l120 eqref) (local $l121 eqref) (local $l122 eqref) (local $l123 eqref) (local $l124 eqref) (local $l125 eqref) (local $l126 eqref) (local $l127 eqref) (local $l128 eqref) (local $l129 eqref) (local $l130 eqref) (local $l131 eqref) (local $l132 eqref) (local $l133 eqref) (local $l134 eqref) (local $l135 eqref) (local $l136 eqref) (local $l137 eqref) (local $l138 eqref) (local $l139 eqref) (local $l140 eqref) (local $l141 eqref) (local $l142 eqref) (local $l143 eqref) (local $l144 eqref) (local $l145 eqref) (local $l146 eqref) (local $l147 eqref) (local $l148 eqref) (local $l149 eqref) (local $l150 eqref) (local $l151 eqref) (local $l152 eqref) (local $l153 eqref) (local $l154 eqref) (local $l155 eqref) (local $l156 eqref) (local $l157 eqref) (local $l158 eqref) (local $l159 eqref) (local $l160 eqref) (local $l161 eqref) (local $l162 eqref) (local $l163 eqref) (local $l164 eqref) (local $l165 eqref) (local $l166 eqref) (local $l167 eqref) (local $l168 eqref) (local $l169 eqref) (local $l170 eqref) (local $l171 eqref) (local $l172 eqref) (local $l173 eqref) (local $l174 eqref) (local $l175 eqref) (local $l176 eqref) (local $l177 eqref) (local $l178 eqref) (local $l179 eqref) (local $l180 eqref) (local $l181 eqref) (local $l182 eqref) (local $l183 eqref) (local $l184 eqref) (local $l185 eqref) (local $l186 eqref) (local $l187 eqref) (local $l188 eqref) (local $l189 eqref) (local $l190 eqref) (local $l191 eqref) (local $l192 eqref) (local $l193 eqref) (local $l194 eqref) (local $l195 eqref) (local $l196 eqref) (local $l197 eqref) (local $l198 eqref) (local $l199 eqref) (local $l200 eqref) (local $l201 eqref) (local $l202 eqref) (local $l203 eqref) (local $l204 eqref) (local $l205 eqref) (local $l206 eqref) (local $l207 eqref) (local $l208 eqref) (local $l209 eqref) (local $l210 eqref) (local $l211 eqref) (local $l212 eqref) (local $l213 eqref) (local $l214 eqref) (local $l215 eqref) (local $l216 eqref) (local $l217 eqref) (local $l218 eqref) (local $l219 eqref) (local $l220 eqref) (local $l221 eqref) (local $l222 eqref) (local $l223 eqref) (local $l224 eqref) (local $l225 eqref) (local $l226 eqref) (local $l227 eqref) (local $l228 eqref) (local $l229 eqref) (local $l230 eqref) (local $l231 eqref) (local $l232 eqref) (local $l233 eqref) (local $l234 eqref) (local $l235 eqref) (local $l236 eqref) (local $l237 eqref) (local $l238 eqref) (local $l239 eqref) (local $l240 eqref) (local $l241 eqref) (local $l242 eqref) (local $l243 eqref) (local $l244 eqref) (local $l245 eqref) (local $l246 eqref) (local $l247 eqref) (local $l248 eqref) (local $l249 eqref) (local $l250 eqref) (local $l251 eqref) (local $l252 eqref) (local $l253 eqref) (local $l254 eqref) (local $l255 eqref) (local $l256 eqref) (local $l257 eqref) (local $l258 eqref) (local $l259 eqref) (local $l260 eqref) (local $l261 eqref) (local $l262 eqref) (local $l263 eqref) (local $l264 eqref) (local $l265 eqref) (local $l266 eqref) (local $l267 eqref) (local $l268 eqref) (local $l269 eqref) (local $l270 eqref) (local $l271 eqref) (local $l272 eqref) (local $l273 eqref) (local $l274 eqref) (local $l275 eqref) (local $l276 eqref) (local $l277 eqref) (local $l278 eqref) (local $l279 eqref) (local $l280 eqref) (local $l281 eqref) (local $l282 eqref) (local $l283 eqref) (local $l284 eqref) (local $l285 eqref) (local $l286 eqref) (local $l287 eqref) (local $l288 eqref) (local $l289 eqref) (local $l290 eqref) (local $l291 eqref) (local $l292 eqref) (local $l293 eqref) (local $l294 eqref) (local $l295 eqref) (local $l296 eqref) (local $l297 eqref) (local $l298 eqref) (local $l299 eqref) (local $slot i32)
    global.get $prof_400
    i64.const 1
    i64.add
    global.set $prof_400
    call $shadow_enter_call
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 9218868437227405311
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    local.set $l0
    local.get $l0
    global.set $g0
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 1
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    local.set $l1
    local.get $l1
    global.set $g1
    i32.const 1
    i32.const 0
    i32.const -2147483648
    global.get $prof_17
    i64.const 1
    i64.add
    global.set $prof_17
    array.new_fixed $limbs 2
    call $int_pack
    local.set $l2
    local.get $l2
    global.set $g2
    i64.const 4503599627370496
    call $int_from_i64
    local.set $l3
    local.get $l3
    global.set $g3
    i64.const 9007199254740992
    call $int_from_i64
    local.set $l4
    local.get $l4
    global.set $g4
    i64.const 4294967296
    call $int_from_i64
    local.set $l5
    local.get $l5
    global.set $g5
    i64.const 4503599627370496
    call $int_from_i64
    local.set $l6
    local.get $l6
    global.set $g6
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4604418534311723008
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    local.set $l7
    local.get $l7
    global.set $g7
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4461442080421002358
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    local.set $l8
    local.get $l8
    global.set $g8
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4609176140021203710
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    local.set $l9
    local.get $l9
    global.set $g9
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4614256656552045848
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    local.set $l10
    local.get $l10
    global.set $g10
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4618760256179416344
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    local.set $l11
    local.get $l11
    global.set $g11
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4728779608772575232
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    local.set $l12
    local.get $l12
    global.set $g12
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4269412446747230208
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    local.set $l13
    local.get $l13
    global.set $g13
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4188347653454561280
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    local.set $l14
    local.get $l14
    global.set $g14
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 0
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    local.set $l15
    local.get $l15
    global.set $g15
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4607182418800017408
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    local.set $l16
    local.get $l16
    global.set $g16
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4611686018427387904
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    local.set $l17
    local.get $l17
    global.set $g17
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4616189618054758400
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    local.set $l18
    local.get $l18
    global.set $g18
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4625196817309499392
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    local.set $l19
    local.get $l19
    global.set $g19
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4602678819172646912
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    local.set $l20
    local.get $l20
    global.set $g20
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4613937818241073152
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    local.set $l21
    local.get $l21
    global.set $g21
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4603909380684499075
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    local.set $l22
    local.get $l22
    global.set $g22
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4697254411347427328
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    local.set $l23
    local.get $l23
    global.set $g23
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4605380978949069210
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    local.set $l24
    local.get $l24
    global.set $g24
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4517110426252607488
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    local.set $l25
    local.get $l25
    global.set $g25
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 5507902344274116608
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    local.set $l26
    local.get $l26
    global.set $g26
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 3706462493325918208
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    local.set $l27
    local.get $l27
    global.set $g27
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 2805742567851819008
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    local.set $l28
    local.get $l28
    global.set $g28
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 6408622269748215808
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    local.set $l29
    local.get $l29
    global.set $g29
    i32.const 24
    i64.const 27
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4609753056924675352
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4364452196894661639
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    global.get $prof_324
    i64.const 1
    i64.add
    global.set $prof_324
    array.new_fixed $values 2
    call $record
    local.set $l30
    local.get $l30
    local.set $l31
    local.get $l31
    global.set $g30
    i32.const 24
    i64.const 27
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4614256656552045848
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4368955796522032135
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    global.get $prof_324
    i64.const 1
    i64.add
    global.set $prof_324
    array.new_fixed $values 2
    call $record
    local.set $l32
    local.get $l32
    local.set $l33
    local.get $l33
    global.set $g31
    i32.const 24
    i64.const 27
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4580687790476533049
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4329579753444856465
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    global.get $prof_324
    i64.const 1
    i64.add
    global.set $prof_324
    array.new_fixed $values 2
    call $record
    local.set $l34
    local.get $l34
    local.set $l35
    local.get $l35
    global.set $g32
    i32.const 24
    i64.const 27
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4633260481411531256
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const -4836329802029595143
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    global.get $prof_324
    i64.const 1
    i64.add
    global.set $prof_324
    array.new_fixed $values 2
    call $record
    local.set $l36
    local.get $l36
    local.set $l37
    local.get $l37
    global.set $g33
    i32.const 24
    i64.const 27
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 0
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const -9223372036854775808
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    global.get $prof_324
    i64.const 1
    i64.add
    global.set $prof_324
    array.new_fixed $values 2
    call $record
    local.set $l38
    local.get $l38
    local.set $l39
    local.get $l39
    global.set $g34
    i32.const 24
    i64.const 27
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4589156319577832938
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const -4882865934792769763
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    global.get $prof_324
    i64.const 1
    i64.add
    global.set $prof_324
    array.new_fixed $values 2
    call $record
    local.set $l40
    local.get $l40
    local.set $l41
    local.get $l41
    global.set $g35
    i32.const 24
    i64.const 27
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4593625142376804206
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const -4878291790333604841
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    global.get $prof_324
    i64.const 1
    i64.add
    global.set $prof_324
    array.new_fixed $values 2
    call $record
    local.set $l42
    local.get $l42
    local.set $l43
    local.get $l43
    global.set $g36
    i32.const 24
    i64.const 27
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4595845883958811394
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4346896889618661834
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    global.get $prof_324
    i64.const 1
    i64.add
    global.set $prof_324
    array.new_fixed $values 2
    call $record
    local.set $l44
    local.get $l44
    local.set $l45
    local.get $l45
    global.set $g37
    i32.const 24
    i64.const 27
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4597994306818310365
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4352917529811201981
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    global.get $prof_324
    i64.const 1
    i64.add
    global.set $prof_324
    array.new_fixed $values 2
    call $record
    local.set $l46
    local.get $l46
    local.set $l47
    local.get $l47
    global.set $g38
    i32.const 24
    i64.const 27
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4599127908639304652
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const -4870251960550919464
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    global.get $prof_324
    i64.const 1
    i64.add
    global.set $prof_324
    array.new_fixed $values 2
    call $record
    local.set $l48
    local.get $l48
    local.set $l49
    local.get $l49
    global.set $g39
    i32.const 24
    i64.const 27
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4600134657745673969
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const -4864903911796446952
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    global.get $prof_324
    i64.const 1
    i64.add
    global.set $prof_324
    array.new_fixed $values 2
    call $record
    local.set $l50
    local.get $l50
    local.set $l51
    local.get $l51
    global.set $g40
    i32.const 24
    i64.const 27
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4601100945962312701
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const -4867742570768189441
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    global.get $prof_324
    i64.const 1
    i64.add
    global.set $prof_324
    array.new_fixed $values 2
    call $record
    local.set $l52
    local.get $l52
    local.set $l53
    local.get $l53
    global.set $g41
    i32.const 24
    i64.const 27
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4602023952714414927
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4357843414468748770
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    global.get $prof_324
    i64.const 1
    i64.add
    global.set $prof_324
    array.new_fixed $values 2
    call $record
    local.set $l54
    local.get $l54
    local.set $l55
    local.get $l55
    global.set $g42
    i32.const 24
    i64.const 27
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4602790413510324432
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const -4864631651799184922
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    global.get $prof_324
    i64.const 1
    i64.add
    global.set $prof_324
    array.new_fixed $values 2
    call $record
    local.set $l56
    local.get $l56
    local.set $l57
    local.get $l57
    global.set $g43
    i32.const 24
    i64.const 27
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4603206634882137780
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const -4874820182729922929
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    global.get $prof_324
    i64.const 1
    i64.add
    global.set $prof_324
    array.new_fixed $values 2
    call $record
    local.set $l58
    local.get $l58
    local.set $l59
    local.get $l59
    global.set $g44
    i32.const 24
    i64.const 27
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4603600141680523195
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4359768187244843266
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    global.get $prof_324
    i64.const 1
    i64.add
    global.set $prof_324
    array.new_fixed $values 2
    call $record
    local.set $l60
    local.get $l60
    local.set $l61
    local.get $l61
    global.set $g45
    i32.const 24
    i64.const 27
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4603971362252824289
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4355615921582285912
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    global.get $prof_324
    i64.const 1
    i64.add
    global.set $prof_324
    array.new_fixed $values 2
    call $record
    local.set $l62
    local.get $l62
    local.set $l63
    local.get $l63
    global.set $g46
    i32.const 24
    i64.const 27
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4604320980709841688
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4350480050219771850
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    global.get $prof_324
    i64.const 1
    i64.add
    global.set $prof_324
    array.new_fixed $values 2
    call $record
    local.set $l64
    local.get $l64
    local.set $l65
    local.get $l65
    global.set $g47
    i32.const 24
    i64.const 27
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4604649864582153780
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const -4865924661261967626
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    global.get $prof_324
    i64.const 1
    i64.add
    global.set $prof_324
    array.new_fixed $values 2
    call $record
    local.set $l66
    local.get $l66
    local.set $l67
    local.get $l67
    global.set $g48
    i32.const 24
    i64.const 27
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4604959003201866317
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const -4865022971463091945
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    global.get $prof_324
    i64.const 1
    i64.add
    global.set $prof_324
    array.new_fixed $values 2
    call $record
    local.set $l68
    local.get $l68
    local.set $l69
    local.get $l69
    global.set $g49
    i32.const 24
    i64.const 27
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4605249457297304856
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4359948597267291143
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    global.get $prof_324
    i64.const 1
    i64.add
    global.set $prof_324
    array.new_fixed $values 2
    call $record
    local.set $l70
    local.get $l70
    local.set $l71
    local.get $l71
    global.set $g50
    i32.const 24
    i64.const 27
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4607182418800017408
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 0
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    global.get $prof_324
    i64.const 1
    i64.add
    global.set $prof_324
    array.new_fixed $values 2
    call $record
    local.set $l72
    local.get $l72
    local.set $l73
    local.get $l73
    global.set $g51
    i32.const 24
    i64.const 27
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const -4628199217061079723
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const -4871393596939086507
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    global.get $prof_324
    i64.const 1
    i64.add
    global.set $prof_324
    array.new_fixed $values 2
    call $record
    local.set $l74
    local.get $l74
    local.set $l75
    local.get $l75
    global.set $g52
    i32.const 24
    i64.const 27
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4575957461383581969
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4323755882250834193
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    global.get $prof_324
    i64.const 1
    i64.add
    global.set $prof_324
    array.new_fixed $values 2
    call $record
    local.set $l76
    local.get $l76
    local.set $l77
    local.get $l77
    global.set $g53
    i32.const 24
    i64.const 27
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const -4671919876300759014
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const -4942135853942988774
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    global.get $prof_324
    i64.const 1
    i64.add
    global.set $prof_324
    array.new_fixed $values 2
    call $record
    local.set $l78
    local.get $l78
    local.set $l79
    local.get $l79
    global.set $g54
    i32.const 24
    i64.const 27
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4523617214285662004
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const -4941551259705644032
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    global.get $prof_324
    i64.const 1
    i64.add
    global.set $prof_324
    array.new_fixed $values 2
    call $record
    local.set $l80
    local.get $l80
    local.set $l81
    local.get $l81
    global.set $g55
    i32.const 24
    i64.const 27
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const -4730215272828025628
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4250278943083590153
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    global.get $prof_324
    i64.const 1
    i64.add
    global.set $prof_324
    array.new_fixed $values 2
    call $record
    local.set $l82
    local.get $l82
    local.set $l83
    local.get $l83
    global.set $g56
    i32.const 24
    i64.const 27
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4460272573143870729
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4219636321837420222
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    global.get $prof_324
    i64.const 1
    i64.add
    global.set $prof_324
    array.new_fixed $values 2
    call $record
    local.set $l84
    local.get $l84
    local.set $l85
    local.get $l85
    global.set $g57
    i32.const 24
    i64.const 27
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const -4797767418267846625
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const -5052519376833550133
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    global.get $prof_324
    i64.const 1
    i64.add
    global.set $prof_324
    array.new_fixed $values 2
    call $record
    local.set $l86
    local.get $l86
    local.set $l87
    local.get $l87
    global.set $g58
    i32.const 24
    i64.const 27
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4389130328383466826
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4146347964016483436
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    global.get $prof_324
    i64.const 1
    i64.add
    global.set $prof_324
    array.new_fixed $values 2
    call $record
    local.set $l88
    local.get $l88
    local.set $l89
    local.get $l89
    global.set $g59
    i32.const 24
    i64.const 27
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const -4872062899121471145
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const -5119918709626118988
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    global.get $prof_324
    i64.const 1
    i64.add
    global.set $prof_324
    array.new_fixed $values 2
    call $record
    local.set $l90
    local.get $l90
    local.set $l91
    local.get $l91
    global.set $g60
    i32.const 24
    i64.const 27
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4311945469071521138
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const -5152957747661355449
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    global.get $prof_324
    i64.const 1
    i64.add
    global.set $prof_324
    array.new_fixed $values 2
    call $record
    local.set $l92
    local.get $l92
    local.set $l93
    local.get $l93
    global.set $g61
    i32.const 24
    i64.const 27
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const -4951881839228340198
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4022631719744127055
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    global.get $prof_324
    i64.const 1
    i64.add
    global.set $prof_324
    array.new_fixed $values 2
    call $record
    local.set $l94
    local.get $l94
    local.set $l95
    local.get $l95
    global.set $g62
    i32.const 24
    i64.const 27
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4229992536239136681
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const -5240626620188572537
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    global.get $prof_324
    i64.const 1
    i64.add
    global.set $prof_324
    array.new_fixed $values 2
    call $record
    local.set $l96
    local.get $l96
    local.set $l97
    local.get $l97
    global.set $g63
    i32.const 24
    i64.const 27
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const -5035839457549562205
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const -5287132706855261676
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    global.get $prof_324
    i64.const 1
    i64.add
    global.set $prof_324
    array.new_fixed $values 2
    call $record
    local.set $l98
    local.get $l98
    local.set $l99
    local.get $l99
    global.set $g64
    i32.const 24
    i64.const 27
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4607182418800017408
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 0
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    global.get $prof_324
    i64.const 1
    i64.add
    global.set $prof_324
    array.new_fixed $values 2
    call $record
    local.set $l100
    local.get $l100
    local.set $l101
    local.get $l101
    global.set $g65
    i32.const 24
    i64.const 27
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const -4620693217682128896
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 0
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    global.get $prof_324
    i64.const 1
    i64.add
    global.set $prof_324
    array.new_fixed $values 2
    call $record
    local.set $l102
    local.get $l102
    local.set $l103
    local.get $l103
    global.set $g66
    i32.const 24
    i64.const 27
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4586165620538955093
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4342971240660948309
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    global.get $prof_324
    i64.const 1
    i64.add
    global.set $prof_324
    array.new_fixed $values 2
    call $record
    local.set $l104
    local.get $l104
    local.set $l105
    local.get $l105
    global.set $g67
    i32.const 24
    i64.const 27
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const -4659324094485795817
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4318751882664866975
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    global.get $prof_324
    i64.const 1
    i64.add
    global.set $prof_324
    array.new_fixed $values 2
    call $record
    local.set $l106
    local.get $l106
    local.set $l107
    local.get $l107
    global.set $g68
    i32.const 24
    i64.const 27
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4537941361671905306
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4267725384029675546
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    global.get $prof_324
    i64.const 1
    i64.add
    global.set $prof_324
    array.new_fixed $values 2
    call $record
    local.set $l108
    local.get $l108
    local.set $l109
    local.get $l109
    global.set $g69
    i32.const 24
    i64.const 27
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const -4714566979057836196
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const -4954879055174325862
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    global.get $prof_324
    i64.const 1
    i64.add
    global.set $prof_324
    array.new_fixed $values 2
    call $record
    local.set $l110
    local.get $l110
    local.set $l111
    local.get $l111
    global.set $g70
    i32.const 24
    i64.const 27
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4477122120089393304
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const -4989233257353491450
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    global.get $prof_324
    i64.const 1
    i64.add
    global.set $prof_324
    array.new_fixed $values 2
    call $record
    local.set $l112
    local.get $l112
    local.set $l113
    local.get $l113
    global.set $g71
    i32.const 24
    i64.const 27
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const -4780226356266894179
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const -5030418050131755745
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    global.get $prof_324
    i64.const 1
    i64.add
    global.set $prof_324
    array.new_fixed $values 2
    call $record
    local.set $l114
    local.get $l114
    local.set $l115
    local.get $l115
    global.set $g72
    i32.const 24
    i64.const 27
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4407590220077447199
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4152838261511743691
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    global.get $prof_324
    i64.const 1
    i64.add
    global.set $prof_324
    array.new_fixed $values 2
    call $record
    local.set $l116
    local.get $l116
    local.set $l117
    local.get $l117
    global.set $g73
    i32.const 24
    i64.const 27
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const -4853048094864343657
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const -5093874638898164981
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    global.get $prof_324
    i64.const 1
    i64.add
    global.set $prof_324
    array.new_fixed $values 2
    call $record
    local.set $l118
    local.get $l118
    local.set $l119
    local.get $l119
    global.set $g74
    i32.const 24
    i64.const 27
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4331992437988917797
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4070874917573114927
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    global.get $prof_324
    i64.const 1
    i64.add
    global.set $prof_324
    array.new_fixed $values 2
    call $record
    local.set $l120
    local.get $l120
    local.set $l121
    local.get $l121
    global.set $g75
    i32.const 24
    i64.const 27
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const -4931214845921462189
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4051810083647009564
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    global.get $prof_324
    i64.const 1
    i64.add
    global.set $prof_324
    array.new_fixed $values 2
    call $record
    local.set $l122
    local.get $l122
    local.set $l123
    local.get $l123
    global.set $g76
    i32.const 24
    i64.const 27
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4251165982991775096
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const -5221451364537619692
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    global.get $prof_324
    i64.const 1
    i64.add
    global.set $prof_324
    array.new_fixed $values 2
    call $record
    local.set $l124
    local.get $l124
    local.set $l125
    local.get $l125
    global.set $g77
    i32.const 24
    i64.const 27
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const -5014601478960427430
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 3965169023428158784
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    global.get $prof_324
    i64.const 1
    i64.add
    global.set $prof_324
    array.new_fixed $values 2
    call $record
    local.set $l126
    local.get $l126
    local.set $l127
    local.get $l127
    global.set $g78
    i32.const 24
    i64.const 27
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4607182418800017408
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 0
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    global.get $prof_324
    i64.const 1
    i64.add
    global.set $prof_324
    array.new_fixed $values 2
    call $record
    local.set $l128
    local.get $l128
    local.set $l129
    local.get $l129
    global.set $g79
    i32.const 24
    i64.const 27
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const -4623695617433709227
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const -4866889997311716011
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    global.get $prof_324
    i64.const 1
    i64.add
    global.set $prof_324
    array.new_fixed $values 2
    call $record
    local.set $l130
    local.get $l130
    local.set $l131
    local.get $l131
    global.set $g80
    i32.const 24
    i64.const 27
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4596373779694328218
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const -4870192637038454374
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    global.get $prof_324
    i64.const 1
    i64.add
    global.set $prof_324
    array.new_fixed $values 2
    call $record
    local.set $l132
    local.get $l132
    local.set $l133
    local.get $l133
    global.set $g81
    i32.const 24
    i64.const 27
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const -4629057045561531246
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const -4872251425439538030
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    global.get $prof_324
    i64.const 1
    i64.add
    global.set $prof_324
    array.new_fixed $values 2
    call $record
    local.set $l134
    local.get $l134
    local.set $l135
    local.get $l135
    global.set $g82
    i32.const 24
    i64.const 27
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4592670820000712476
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4349476440122705692
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    global.get $prof_324
    i64.const 1
    i64.add
    global.set $prof_324
    array.new_fixed $values 2
    call $record
    local.set $l136
    local.get $l136
    local.set $l137
    local.get $l137
    global.set $g83
    i32.const 24
    i64.const 27
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const -4632156925824526522
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4343517131524872006
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    global.get $prof_324
    i64.const 1
    i64.add
    global.set $prof_324
    array.new_fixed $values 2
    call $record
    local.set $l138
    local.get $l138
    local.set $l139
    local.get $l139
    global.set $g84
    i32.const 24
    i64.const 27
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4590207312512236308
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const -4876359104220546284
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    global.get $prof_324
    i64.const 1
    i64.add
    global.set $prof_324
    array.new_fixed $values 2
    call $record
    local.set $l140
    local.get $l140
    local.set $l141
    local.get $l141
    global.set $g85
    i32.const 24
    i64.const 27
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const -4633903776589082351
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const -4886105355721830127
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    global.get $prof_324
    i64.const 1
    i64.add
    global.set $prof_324
    array.new_fixed $values 2
    call $record
    local.set $l142
    local.get $l142
    local.set $l143
    local.get $l143
    global.set $g86
    i32.const 24
    i64.const 27
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4588638185040256542
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4336436605907508766
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    global.get $prof_324
    i64.const 1
    i64.add
    global.set $prof_324
    array.new_fixed $values 2
    call $record
    local.set $l144
    local.get $l144
    local.set $l145
    local.get $l145
    global.set $g87
    global.get $g51
    i32.const 0
    call $field
    local.set $l146
    local.get $l146
    local.set $l147
    local.get $l147
    global.set $g88
    global.get $g51
    i32.const 1
    call $field
    local.set $l148
    local.get $l148
    local.set $l149
    local.get $l149
    global.set $g89
    global.get $g52
    i32.const 0
    call $field
    local.set $l150
    local.get $l150
    local.set $l151
    local.get $l151
    global.set $g90
    global.get $g52
    i32.const 1
    call $field
    local.set $l152
    local.get $l152
    local.set $l153
    local.get $l153
    global.set $g91
    global.get $g53
    i32.const 0
    call $field
    local.set $l154
    local.get $l154
    local.set $l155
    local.get $l155
    global.set $g92
    global.get $g53
    i32.const 1
    call $field
    local.set $l156
    local.get $l156
    local.set $l157
    local.get $l157
    global.set $g93
    global.get $g54
    i32.const 0
    call $field
    local.set $l158
    local.get $l158
    local.set $l159
    local.get $l159
    global.set $g94
    global.get $g54
    i32.const 1
    call $field
    local.set $l160
    local.get $l160
    local.set $l161
    local.get $l161
    global.set $g95
    global.get $g55
    i32.const 0
    call $field
    local.set $l162
    local.get $l162
    local.set $l163
    local.get $l163
    global.set $g96
    global.get $g55
    i32.const 1
    call $field
    local.set $l164
    local.get $l164
    local.set $l165
    local.get $l165
    global.set $g97
    global.get $g56
    i32.const 0
    call $field
    local.set $l166
    local.get $l166
    local.set $l167
    local.get $l167
    global.set $g98
    global.get $g56
    i32.const 1
    call $field
    local.set $l168
    local.get $l168
    local.set $l169
    local.get $l169
    global.set $g99
    global.get $g57
    i32.const 0
    call $field
    local.set $l170
    local.get $l170
    local.set $l171
    local.get $l171
    global.set $g100
    global.get $g58
    i32.const 0
    call $field
    local.set $l172
    local.get $l172
    local.set $l173
    local.get $l173
    global.set $g101
    global.get $g59
    i32.const 0
    call $field
    local.set $l174
    local.get $l174
    local.set $l175
    local.get $l175
    global.set $g102
    global.get $g60
    i32.const 0
    call $field
    local.set $l176
    local.get $l176
    local.set $l177
    local.get $l177
    global.set $g103
    global.get $g61
    i32.const 0
    call $field
    local.set $l178
    local.get $l178
    local.set $l179
    local.get $l179
    global.set $g104
    global.get $g62
    i32.const 0
    call $field
    local.set $l180
    local.get $l180
    local.set $l181
    local.get $l181
    global.set $g105
    global.get $g63
    i32.const 0
    call $field
    local.set $l182
    local.get $l182
    local.set $l183
    local.get $l183
    global.set $g106
    global.get $g64
    i32.const 0
    call $field
    local.set $l184
    local.get $l184
    local.set $l185
    local.get $l185
    global.set $g107
    global.get $g65
    i32.const 0
    call $field
    local.set $l186
    local.get $l186
    local.set $l187
    local.get $l187
    global.set $g108
    global.get $g65
    i32.const 1
    call $field
    local.set $l188
    local.get $l188
    local.set $l189
    local.get $l189
    global.set $g109
    global.get $g66
    i32.const 0
    call $field
    local.set $l190
    local.get $l190
    local.set $l191
    local.get $l191
    global.set $g110
    global.get $g66
    i32.const 1
    call $field
    local.set $l192
    local.get $l192
    local.set $l193
    local.get $l193
    global.set $g111
    global.get $g67
    i32.const 0
    call $field
    local.set $l194
    local.get $l194
    local.set $l195
    local.get $l195
    global.set $g112
    global.get $g67
    i32.const 1
    call $field
    local.set $l196
    local.get $l196
    local.set $l197
    local.get $l197
    global.set $g113
    global.get $g68
    i32.const 0
    call $field
    local.set $l198
    local.get $l198
    local.set $l199
    local.get $l199
    global.set $g114
    global.get $g68
    i32.const 1
    call $field
    local.set $l200
    local.get $l200
    local.set $l201
    local.get $l201
    global.set $g115
    global.get $g69
    i32.const 0
    call $field
    local.set $l202
    local.get $l202
    local.set $l203
    local.get $l203
    global.set $g116
    global.get $g69
    i32.const 1
    call $field
    local.set $l204
    local.get $l204
    local.set $l205
    local.get $l205
    global.set $g117
    global.get $g70
    i32.const 0
    call $field
    local.set $l206
    local.get $l206
    local.set $l207
    local.get $l207
    global.set $g118
    global.get $g70
    i32.const 1
    call $field
    local.set $l208
    local.get $l208
    local.set $l209
    local.get $l209
    global.set $g119
    global.get $g71
    i32.const 0
    call $field
    local.set $l210
    local.get $l210
    local.set $l211
    local.get $l211
    global.set $g120
    global.get $g71
    i32.const 1
    call $field
    local.set $l212
    local.get $l212
    local.set $l213
    local.get $l213
    global.set $g121
    global.get $g72
    i32.const 0
    call $field
    local.set $l214
    local.get $l214
    local.set $l215
    local.get $l215
    global.set $g122
    global.get $g73
    i32.const 0
    call $field
    local.set $l216
    local.get $l216
    local.set $l217
    local.get $l217
    global.set $g123
    global.get $g74
    i32.const 0
    call $field
    local.set $l218
    local.get $l218
    local.set $l219
    local.get $l219
    global.set $g124
    global.get $g75
    i32.const 0
    call $field
    local.set $l220
    local.get $l220
    local.set $l221
    local.get $l221
    global.set $g125
    global.get $g76
    i32.const 0
    call $field
    local.set $l222
    local.get $l222
    local.set $l223
    local.get $l223
    global.set $g126
    global.get $g77
    i32.const 0
    call $field
    local.set $l224
    local.get $l224
    local.set $l225
    local.get $l225
    global.set $g127
    global.get $g78
    i32.const 0
    call $field
    local.set $l226
    local.get $l226
    local.set $l227
    local.get $l227
    global.set $g128
    global.get $g79
    i32.const 0
    call $field
    local.set $l228
    local.get $l228
    local.set $l229
    local.get $l229
    global.set $g129
    global.get $g79
    i32.const 1
    call $field
    local.set $l230
    local.get $l230
    local.set $l231
    local.get $l231
    global.set $g130
    global.get $g80
    i32.const 0
    call $field
    local.set $l232
    local.get $l232
    local.set $l233
    local.get $l233
    global.set $g131
    global.get $g80
    i32.const 1
    call $field
    local.set $l234
    local.get $l234
    local.set $l235
    local.get $l235
    global.set $g132
    global.get $g81
    i32.const 0
    call $field
    local.set $l236
    local.get $l236
    local.set $l237
    local.get $l237
    global.set $g133
    global.get $g81
    i32.const 1
    call $field
    local.set $l238
    local.get $l238
    local.set $l239
    local.get $l239
    global.set $g134
    global.get $g82
    i32.const 0
    call $field
    local.set $l240
    local.get $l240
    local.set $l241
    local.get $l241
    global.set $g135
    global.get $g82
    i32.const 1
    call $field
    local.set $l242
    local.get $l242
    local.set $l243
    local.get $l243
    global.set $g136
    global.get $g83
    i32.const 0
    call $field
    local.set $l244
    local.get $l244
    local.set $l245
    local.get $l245
    global.set $g137
    global.get $g84
    i32.const 0
    call $field
    local.set $l246
    local.get $l246
    local.set $l247
    local.get $l247
    global.set $g138
    global.get $g85
    i32.const 0
    call $field
    local.set $l248
    local.get $l248
    local.set $l249
    local.get $l249
    global.set $g139
    global.get $g86
    i32.const 0
    call $field
    local.set $l250
    local.get $l250
    local.set $l251
    local.get $l251
    global.set $g140
    global.get $g87
    i32.const 0
    call $field
    local.set $l252
    local.get $l252
    local.set $l253
    local.get $l253
    global.set $g141
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4607182418800017408
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    global.get $value_empty_list
    call $list_cons
    local.set $l254
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4606056518893174784
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    local.get $l254
    call $list_cons
    local.set $l255
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4604930618986332160
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    local.get $l255
    call $list_cons
    local.set $l256
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4603804719079489536
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    local.get $l256
    call $list_cons
    local.set $l257
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4602678819172646912
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    local.get $l257
    call $list_cons
    local.set $l258
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4600427019358961664
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    local.get $l258
    call $list_cons
    local.set $l259
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4598175219545276416
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    local.get $l259
    call $list_cons
    local.set $l260
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4593671619917905920
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    local.get $l260
    call $list_cons
    local.set $l261
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4607182418800017408
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    local.get $l261
    call $list_cons
    local.set $l262
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4606056518893174784
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    local.get $l262
    call $list_cons
    local.set $l263
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4604930618986332160
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    local.get $l263
    call $list_cons
    local.set $l264
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4603804719079489536
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    local.get $l264
    call $list_cons
    local.set $l265
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4602678819172646912
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    local.get $l265
    call $list_cons
    local.set $l266
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4600427019358961664
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    local.get $l266
    call $list_cons
    local.set $l267
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4598175219545276416
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    local.get $l267
    call $list_cons
    local.set $l268
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4593671619917905920
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    local.get $l268
    call $list_cons
    local.set $l269
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4607182418800017408
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    local.get $l269
    call $list_cons
    local.set $l270
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4606056518893174784
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    local.get $l270
    call $list_cons
    local.set $l271
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4604930618986332160
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    local.get $l271
    call $list_cons
    local.set $l272
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4603804719079489536
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    local.get $l272
    call $list_cons
    local.set $l273
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4602678819172646912
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    local.get $l273
    call $list_cons
    local.set $l274
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4600427019358961664
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    local.get $l274
    call $list_cons
    local.set $l275
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4598175219545276416
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    local.get $l275
    call $list_cons
    local.set $l276
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4593671619917905920
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    local.get $l276
    call $list_cons
    local.set $l277
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4607182418800017408
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    local.get $l277
    call $list_cons
    local.set $l278
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4606056518893174784
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    local.get $l278
    call $list_cons
    local.set $l279
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4604930618986332160
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    local.get $l279
    call $list_cons
    local.set $l280
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4603804719079489536
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    local.get $l280
    call $list_cons
    local.set $l281
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4602678819172646912
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    local.get $l281
    call $list_cons
    local.set $l282
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4600427019358961664
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    local.get $l282
    call $list_cons
    local.set $l283
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4598175219545276416
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    local.get $l283
    call $list_cons
    local.set $l284
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4593671619917905920
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    local.get $l284
    call $list_cons
    local.set $l285
    local.get $l285
    local.set $l286
    local.get $l286
    global.set $g142
    i32.const 0
    i32.const 16
    global.get $prof_401
    i64.const 1
    i64.add
    global.set $prof_401
    array.new_data $bytes $s0
    call $string_from_bytes
    local.set $l287
    local.get $l287
    global.set $g143
    i32.const 0
    i32.const 64
    global.get $prof_402
    i64.const 1
    i64.add
    global.set $prof_402
    array.new_data $bytes $s1
    call $string_from_bytes
    local.set $l288
    local.get $l288
    global.set $g144
    i32.const 0
    i32.const 64
    global.get $prof_403
    i64.const 1
    i64.add
    global.set $prof_403
    array.new_data $bytes $s2
    call $string_from_bytes
    local.set $l289
    local.get $l289
    global.set $g145
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 9218868437227405311
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    local.set $l290
    local.get $l290
    global.set $g146
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 1
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    local.set $l291
    local.get $l291
    global.set $g147
    i32.const 1
    i32.const 0
    i32.const -2147483648
    global.get $prof_17
    i64.const 1
    i64.add
    global.set $prof_17
    array.new_fixed $limbs 2
    call $int_pack
    local.set $l292
    local.get $l292
    global.set $g148
    i64.const 4503599627370496
    call $int_from_i64
    local.set $l293
    local.get $l293
    global.set $g149
    i64.const 9007199254740992
    call $int_from_i64
    local.set $l294
    local.get $l294
    global.set $g150
    i64.const 4294967296
    call $int_from_i64
    local.set $l295
    local.get $l295
    global.set $g151
    i64.const 4503599627370496
    call $int_from_i64
    local.set $l296
    local.get $l296
    global.set $g152
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4604418534311723008
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    local.set $l297
    local.get $l297
    global.set $g153
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4461442080421002358
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    local.set $l298
    local.get $l298
    global.set $g154
    i32.const 2
    i32.const 0
    i64.const 0
    i64.const 4609176140021203710
    f64.reinterpret_i64
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    global.get $prof_15
    i64.const 1
    i64.add
    global.set $prof_15
    struct.new $obj
    local.set $l299
    local.get $l299
    global.set $g155
    call $shadow_leave
    ref.null eq
    return
    unreachable
  )
  (func $ilang_start (;345;) (type $unpend)
    global.get $prof_404
    i64.const 1
    i64.add
    global.set $prof_404
    i32.const 0
    i32.const 1299
    global.get $prof_405
    i64.const 1
    i64.add
    global.set $prof_405
    array.new_data $sites $site_data
    i32.const 0
    i32.const 11
    global.get $prof_406
    i64.const 1
    i64.add
    global.set $prof_406
    array.new_data $bytes $s3
    call $string_from_bytes
    i32.const 0
    i32.const 8
    global.get $prof_407
    i64.const 1
    i64.add
    global.set $prof_407
    array.new_data $bytes $s4
    call $string_from_bytes
    global.get $prof_324
    i64.const 1
    i64.add
    global.set $prof_324
    array.new_fixed $values 2
    call $sites_install
    i32.const 0
    i32.const 2
    global.get $prof_408
    i64.const 1
    i64.add
    global.set $prof_408
    array.new_data $bytes $s5
    call $string_from_bytes
    i32.const 0
    i32.const 5
    global.get $prof_409
    i64.const 1
    i64.add
    global.set $prof_409
    array.new_data $bytes $s6
    call $string_from_bytes
    i32.const 0
    i32.const 4
    global.get $prof_410
    i64.const 1
    i64.add
    global.set $prof_410
    array.new_data $bytes $s7
    call $string_from_bytes
    i32.const 0
    i32.const 5
    global.get $prof_411
    i64.const 1
    i64.add
    global.set $prof_411
    array.new_data $bytes $s8
    call $string_from_bytes
    i32.const 0
    i32.const 9
    global.get $prof_412
    i64.const 1
    i64.add
    global.set $prof_412
    array.new_data $bytes $s9
    call $string_from_bytes
    i32.const 0
    i32.const 12
    global.get $prof_413
    i64.const 1
    i64.add
    global.set $prof_413
    array.new_data $bytes $s10
    call $string_from_bytes
    i32.const 0
    i32.const 11
    global.get $prof_414
    i64.const 1
    i64.add
    global.set $prof_414
    array.new_data $bytes $s11
    call $string_from_bytes
    i32.const 0
    i32.const 5
    global.get $prof_415
    i64.const 1
    i64.add
    global.set $prof_415
    array.new_data $bytes $s12
    call $string_from_bytes
    i32.const 0
    i32.const 6
    global.get $prof_416
    i64.const 1
    i64.add
    global.set $prof_416
    array.new_data $bytes $s13
    call $string_from_bytes
    i32.const 0
    i32.const 7
    global.get $prof_417
    i64.const 1
    i64.add
    global.set $prof_417
    array.new_data $bytes $s14
    call $string_from_bytes
    i32.const 0
    i32.const 4
    global.get $prof_418
    i64.const 1
    i64.add
    global.set $prof_418
    array.new_data $bytes $s15
    call $string_from_bytes
    i32.const 0
    i32.const 6
    global.get $prof_419
    i64.const 1
    i64.add
    global.set $prof_419
    array.new_data $bytes $s16
    call $string_from_bytes
    i32.const 0
    i32.const 7
    global.get $prof_420
    i64.const 1
    i64.add
    global.set $prof_420
    array.new_data $bytes $s17
    call $string_from_bytes
    i32.const 0
    i32.const 7
    global.get $prof_421
    i64.const 1
    i64.add
    global.set $prof_421
    array.new_data $bytes $s18
    call $string_from_bytes
    i32.const 0
    i32.const 9
    global.get $prof_422
    i64.const 1
    i64.add
    global.set $prof_422
    array.new_data $bytes $s19
    call $string_from_bytes
    i32.const 0
    i32.const 10
    global.get $prof_423
    i64.const 1
    i64.add
    global.set $prof_423
    array.new_data $bytes $s20
    call $string_from_bytes
    i32.const 0
    i32.const 5
    global.get $prof_424
    i64.const 1
    i64.add
    global.set $prof_424
    array.new_data $bytes $s21
    call $string_from_bytes
    i32.const 0
    i32.const 7
    global.get $prof_425
    i64.const 1
    i64.add
    global.set $prof_425
    array.new_data $bytes $s22
    call $string_from_bytes
    i32.const 0
    i32.const 8
    global.get $prof_426
    i64.const 1
    i64.add
    global.set $prof_426
    array.new_data $bytes $s23
    call $string_from_bytes
    i32.const 0
    i32.const 4
    global.get $prof_427
    i64.const 1
    i64.add
    global.set $prof_427
    array.new_data $bytes $s24
    call $string_from_bytes
    i32.const 0
    i32.const 9
    global.get $prof_428
    i64.const 1
    i64.add
    global.set $prof_428
    array.new_data $bytes $s25
    call $string_from_bytes
    i32.const 0
    i32.const 6
    global.get $prof_429
    i64.const 1
    i64.add
    global.set $prof_429
    array.new_data $bytes $s26
    call $string_from_bytes
    i32.const 0
    i32.const 5
    global.get $prof_430
    i64.const 1
    i64.add
    global.set $prof_430
    array.new_data $bytes $s27
    call $string_from_bytes
    i32.const 0
    i32.const 6
    global.get $prof_431
    i64.const 1
    i64.add
    global.set $prof_431
    array.new_data $bytes $s28
    call $string_from_bytes
    i32.const 0
    i32.const 7
    global.get $prof_432
    i64.const 1
    i64.add
    global.set $prof_432
    array.new_data $bytes $s29
    call $string_from_bytes
    i32.const 0
    i32.const 9
    global.get $prof_433
    i64.const 1
    i64.add
    global.set $prof_433
    array.new_data $bytes $s30
    call $string_from_bytes
    i32.const 0
    i32.const 8
    global.get $prof_434
    i64.const 1
    i64.add
    global.set $prof_434
    array.new_data $bytes $s31
    call $string_from_bytes
    i32.const 0
    i32.const 6
    global.get $prof_435
    i64.const 1
    i64.add
    global.set $prof_435
    array.new_data $bytes $s32
    call $string_from_bytes
    i32.const 0
    i32.const 8
    global.get $prof_436
    i64.const 1
    i64.add
    global.set $prof_436
    array.new_data $bytes $s33
    call $string_from_bytes
    i32.const 0
    i32.const 6
    global.get $prof_437
    i64.const 1
    i64.add
    global.set $prof_437
    array.new_data $bytes $s34
    call $string_from_bytes
    i32.const 0
    i32.const 8
    global.get $prof_438
    i64.const 1
    i64.add
    global.set $prof_438
    array.new_data $bytes $s35
    call $string_from_bytes
    i32.const 0
    i32.const 6
    global.get $prof_439
    i64.const 1
    i64.add
    global.set $prof_439
    array.new_data $bytes $s36
    call $string_from_bytes
    i32.const 0
    i32.const 7
    global.get $prof_440
    i64.const 1
    i64.add
    global.set $prof_440
    array.new_data $bytes $s37
    call $string_from_bytes
    i32.const 0
    i32.const 9
    global.get $prof_441
    i64.const 1
    i64.add
    global.set $prof_441
    array.new_data $bytes $s38
    call $string_from_bytes
    i32.const 0
    i32.const 9
    global.get $prof_442
    i64.const 1
    i64.add
    global.set $prof_442
    array.new_data $bytes $s39
    call $string_from_bytes
    i32.const 0
    i32.const 13
    global.get $prof_443
    i64.const 1
    i64.add
    global.set $prof_443
    array.new_data $bytes $s40
    call $string_from_bytes
    i32.const 0
    i32.const 9
    global.get $prof_444
    i64.const 1
    i64.add
    global.set $prof_444
    array.new_data $bytes $s41
    call $string_from_bytes
    i32.const 0
    i32.const 7
    global.get $prof_445
    i64.const 1
    i64.add
    global.set $prof_445
    array.new_data $bytes $s42
    call $string_from_bytes
    i32.const 0
    i32.const 5
    global.get $prof_446
    i64.const 1
    i64.add
    global.set $prof_446
    array.new_data $bytes $s43
    call $string_from_bytes
    i32.const 0
    i32.const 11
    global.get $prof_447
    i64.const 1
    i64.add
    global.set $prof_447
    array.new_data $bytes $s44
    call $string_from_bytes
    i32.const 0
    i32.const 13
    global.get $prof_448
    i64.const 1
    i64.add
    global.set $prof_448
    array.new_data $bytes $s45
    call $string_from_bytes
    i32.const 0
    i32.const 13
    global.get $prof_449
    i64.const 1
    i64.add
    global.set $prof_449
    array.new_data $bytes $s46
    call $string_from_bytes
    i32.const 0
    i32.const 13
    global.get $prof_450
    i64.const 1
    i64.add
    global.set $prof_450
    array.new_data $bytes $s47
    call $string_from_bytes
    i32.const 0
    i32.const 13
    global.get $prof_451
    i64.const 1
    i64.add
    global.set $prof_451
    array.new_data $bytes $s48
    call $string_from_bytes
    i32.const 0
    i32.const 12
    global.get $prof_452
    i64.const 1
    i64.add
    global.set $prof_452
    array.new_data $bytes $s49
    call $string_from_bytes
    i32.const 0
    i32.const 15
    global.get $prof_453
    i64.const 1
    i64.add
    global.set $prof_453
    array.new_data $bytes $s50
    call $string_from_bytes
    i32.const 0
    i32.const 14
    global.get $prof_454
    i64.const 1
    i64.add
    global.set $prof_454
    array.new_data $bytes $s51
    call $string_from_bytes
    i32.const 0
    i32.const 15
    global.get $prof_455
    i64.const 1
    i64.add
    global.set $prof_455
    array.new_data $bytes $s52
    call $string_from_bytes
    i32.const 0
    i32.const 15
    global.get $prof_456
    i64.const 1
    i64.add
    global.set $prof_456
    array.new_data $bytes $s53
    call $string_from_bytes
    i32.const 0
    i32.const 13
    global.get $prof_457
    i64.const 1
    i64.add
    global.set $prof_457
    array.new_data $bytes $s54
    call $string_from_bytes
    i32.const 0
    i32.const 5
    global.get $prof_458
    i64.const 1
    i64.add
    global.set $prof_458
    array.new_data $bytes $s55
    call $string_from_bytes
    i32.const 0
    i32.const 4
    global.get $prof_459
    i64.const 1
    i64.add
    global.set $prof_459
    array.new_data $bytes $s56
    call $string_from_bytes
    i32.const 0
    i32.const 8
    global.get $prof_460
    i64.const 1
    i64.add
    global.set $prof_460
    array.new_data $bytes $s57
    call $string_from_bytes
    i32.const 0
    i32.const 8
    global.get $prof_461
    i64.const 1
    i64.add
    global.set $prof_461
    array.new_data $bytes $s58
    call $string_from_bytes
    i32.const 0
    i32.const 8
    global.get $prof_462
    i64.const 1
    i64.add
    global.set $prof_462
    array.new_data $bytes $s59
    call $string_from_bytes
    i32.const 0
    i32.const 10
    global.get $prof_463
    i64.const 1
    i64.add
    global.set $prof_463
    array.new_data $bytes $s60
    call $string_from_bytes
    i32.const 0
    i32.const 9
    global.get $prof_464
    i64.const 1
    i64.add
    global.set $prof_464
    array.new_data $bytes $s61
    call $string_from_bytes
    i32.const 0
    i32.const 5
    global.get $prof_465
    i64.const 1
    i64.add
    global.set $prof_465
    array.new_data $bytes $s62
    call $string_from_bytes
    i32.const 0
    i32.const 6
    global.get $prof_466
    i64.const 1
    i64.add
    global.set $prof_466
    array.new_data $bytes $s63
    call $string_from_bytes
    i32.const 0
    i32.const 4
    global.get $prof_467
    i64.const 1
    i64.add
    global.set $prof_467
    array.new_data $bytes $s64
    call $string_from_bytes
    i32.const 0
    i32.const 5
    global.get $prof_468
    i64.const 1
    i64.add
    global.set $prof_468
    array.new_data $bytes $s65
    call $string_from_bytes
    i32.const 0
    i32.const 3
    global.get $prof_469
    i64.const 1
    i64.add
    global.set $prof_469
    array.new_data $bytes $s66
    call $string_from_bytes
    i32.const 0
    i32.const 4
    global.get $prof_470
    i64.const 1
    i64.add
    global.set $prof_470
    array.new_data $bytes $s67
    call $string_from_bytes
    i32.const 0
    i32.const 6
    global.get $prof_471
    i64.const 1
    i64.add
    global.set $prof_471
    array.new_data $bytes $s68
    call $string_from_bytes
    i32.const 0
    i32.const 8
    global.get $prof_472
    i64.const 1
    i64.add
    global.set $prof_472
    array.new_data $bytes $s69
    call $string_from_bytes
    i32.const 0
    i32.const 4
    global.get $prof_473
    i64.const 1
    i64.add
    global.set $prof_473
    array.new_data $bytes $s70
    call $string_from_bytes
    i32.const 0
    i32.const 5
    global.get $prof_474
    i64.const 1
    i64.add
    global.set $prof_474
    array.new_data $bytes $s71
    call $string_from_bytes
    i32.const 0
    i32.const 4
    global.get $prof_475
    i64.const 1
    i64.add
    global.set $prof_475
    array.new_data $bytes $s72
    call $string_from_bytes
    i32.const 0
    i32.const 7
    global.get $prof_476
    i64.const 1
    i64.add
    global.set $prof_476
    array.new_data $bytes $s73
    call $string_from_bytes
    i32.const 0
    i32.const 7
    global.get $prof_477
    i64.const 1
    i64.add
    global.set $prof_477
    array.new_data $bytes $s74
    call $string_from_bytes
    i32.const 0
    i32.const 5
    global.get $prof_478
    i64.const 1
    i64.add
    global.set $prof_478
    array.new_data $bytes $s75
    call $string_from_bytes
    i32.const 0
    i32.const 4
    global.get $prof_479
    i64.const 1
    i64.add
    global.set $prof_479
    array.new_data $bytes $s76
    call $string_from_bytes
    i32.const 0
    i32.const 4
    global.get $prof_480
    i64.const 1
    i64.add
    global.set $prof_480
    array.new_data $bytes $s77
    call $string_from_bytes
    i32.const 0
    i32.const 4
    global.get $prof_481
    i64.const 1
    i64.add
    global.set $prof_481
    array.new_data $bytes $s78
    call $string_from_bytes
    i32.const 0
    i32.const 4
    global.get $prof_482
    i64.const 1
    i64.add
    global.set $prof_482
    array.new_data $bytes $s79
    call $string_from_bytes
    i32.const 0
    i32.const 5
    global.get $prof_483
    i64.const 1
    i64.add
    global.set $prof_483
    array.new_data $bytes $s80
    call $string_from_bytes
    i32.const 0
    i32.const 7
    global.get $prof_484
    i64.const 1
    i64.add
    global.set $prof_484
    array.new_data $bytes $s81
    call $string_from_bytes
    i32.const 0
    i32.const 14
    global.get $prof_485
    i64.const 1
    i64.add
    global.set $prof_485
    array.new_data $bytes $s82
    call $string_from_bytes
    i32.const 0
    i32.const 13
    global.get $prof_486
    i64.const 1
    i64.add
    global.set $prof_486
    array.new_data $bytes $s83
    call $string_from_bytes
    i32.const 0
    i32.const 3
    global.get $prof_487
    i64.const 1
    i64.add
    global.set $prof_487
    array.new_data $bytes $s84
    call $string_from_bytes
    i32.const 0
    i32.const 4
    global.get $prof_488
    i64.const 1
    i64.add
    global.set $prof_488
    array.new_data $bytes $s85
    call $string_from_bytes
    i32.const 0
    i32.const 6
    global.get $prof_489
    i64.const 1
    i64.add
    global.set $prof_489
    array.new_data $bytes $s86
    call $string_from_bytes
    i32.const 0
    i32.const 12
    global.get $prof_490
    i64.const 1
    i64.add
    global.set $prof_490
    array.new_data $bytes $s87
    call $string_from_bytes
    i32.const 0
    i32.const 13
    global.get $prof_491
    i64.const 1
    i64.add
    global.set $prof_491
    array.new_data $bytes $s88
    call $string_from_bytes
    i32.const 0
    i32.const 11
    global.get $prof_492
    i64.const 1
    i64.add
    global.set $prof_492
    array.new_data $bytes $s89
    call $string_from_bytes
    i32.const 0
    i32.const 4
    global.get $prof_493
    i64.const 1
    i64.add
    global.set $prof_493
    array.new_data $bytes $s90
    call $string_from_bytes
    i32.const 0
    i32.const 4
    global.get $prof_494
    i64.const 1
    i64.add
    global.set $prof_494
    array.new_data $bytes $s91
    call $string_from_bytes
    i32.const 0
    i32.const 7
    global.get $prof_495
    i64.const 1
    i64.add
    global.set $prof_495
    array.new_data $bytes $s92
    call $string_from_bytes
    i32.const 0
    i32.const 7
    global.get $prof_477
    i64.const 1
    i64.add
    global.set $prof_477
    array.new_data $bytes $s74
    call $string_from_bytes
    i32.const 0
    i32.const 6
    global.get $prof_496
    i64.const 1
    i64.add
    global.set $prof_496
    array.new_data $bytes $s93
    call $string_from_bytes
    i32.const 0
    i32.const 6
    global.get $prof_497
    i64.const 1
    i64.add
    global.set $prof_497
    array.new_data $bytes $s94
    call $string_from_bytes
    i32.const 0
    i32.const 9
    global.get $prof_498
    i64.const 1
    i64.add
    global.set $prof_498
    array.new_data $bytes $s95
    call $string_from_bytes
    i32.const 0
    i32.const 13
    global.get $prof_499
    i64.const 1
    i64.add
    global.set $prof_499
    array.new_data $bytes $s96
    call $string_from_bytes
    i32.const 0
    i32.const 8
    global.get $prof_500
    i64.const 1
    i64.add
    global.set $prof_500
    array.new_data $bytes $s97
    call $string_from_bytes
    i32.const 0
    i32.const 4
    global.get $prof_501
    i64.const 1
    i64.add
    global.set $prof_501
    array.new_data $bytes $s98
    call $string_from_bytes
    i32.const 0
    i32.const 7
    global.get $prof_502
    i64.const 1
    i64.add
    global.set $prof_502
    array.new_data $bytes $s99
    call $string_from_bytes
    i32.const 0
    i32.const 7
    global.get $prof_503
    i64.const 1
    i64.add
    global.set $prof_503
    array.new_data $bytes $s100
    call $string_from_bytes
    i32.const 0
    i32.const 9
    global.get $prof_504
    i64.const 1
    i64.add
    global.set $prof_504
    array.new_data $bytes $s101
    call $string_from_bytes
    i32.const 0
    i32.const 6
    global.get $prof_505
    i64.const 1
    i64.add
    global.set $prof_505
    array.new_data $bytes $s102
    call $string_from_bytes
    i32.const 0
    i32.const 4
    global.get $prof_506
    i64.const 1
    i64.add
    global.set $prof_506
    array.new_data $bytes $s103
    call $string_from_bytes
    i32.const 0
    i32.const 3
    global.get $prof_507
    i64.const 1
    i64.add
    global.set $prof_507
    array.new_data $bytes $s104
    call $string_from_bytes
    i32.const 0
    i32.const 6
    global.get $prof_508
    i64.const 1
    i64.add
    global.set $prof_508
    array.new_data $bytes $s105
    call $string_from_bytes
    i32.const 0
    i32.const 4
    global.get $prof_509
    i64.const 1
    i64.add
    global.set $prof_509
    array.new_data $bytes $s106
    call $string_from_bytes
    i32.const 0
    i32.const 4
    global.get $prof_510
    i64.const 1
    i64.add
    global.set $prof_510
    array.new_data $bytes $s107
    call $string_from_bytes
    i32.const 0
    i32.const 8
    global.get $prof_511
    i64.const 1
    i64.add
    global.set $prof_511
    array.new_data $bytes $s108
    call $string_from_bytes
    i32.const 0
    i32.const 7
    global.get $prof_512
    i64.const 1
    i64.add
    global.set $prof_512
    array.new_data $bytes $s109
    call $string_from_bytes
    i32.const 0
    i32.const 14
    global.get $prof_513
    i64.const 1
    i64.add
    global.set $prof_513
    array.new_data $bytes $s110
    call $string_from_bytes
    i32.const 0
    i32.const 3
    global.get $prof_514
    i64.const 1
    i64.add
    global.set $prof_514
    array.new_data $bytes $s111
    call $string_from_bytes
    i32.const 0
    i32.const 5
    global.get $prof_515
    i64.const 1
    i64.add
    global.set $prof_515
    array.new_data $bytes $s112
    call $string_from_bytes
    i32.const 0
    i32.const 9
    global.get $prof_516
    i64.const 1
    i64.add
    global.set $prof_516
    array.new_data $bytes $s113
    call $string_from_bytes
    i32.const 0
    i32.const 7
    global.get $prof_517
    i64.const 1
    i64.add
    global.set $prof_517
    array.new_data $bytes $s114
    call $string_from_bytes
    i32.const 0
    i32.const 9
    global.get $prof_518
    i64.const 1
    i64.add
    global.set $prof_518
    array.new_data $bytes $s115
    call $string_from_bytes
    i32.const 0
    i32.const 9
    global.get $prof_519
    i64.const 1
    i64.add
    global.set $prof_519
    array.new_data $bytes $s116
    call $string_from_bytes
    i32.const 0
    i32.const 10
    global.get $prof_520
    i64.const 1
    i64.add
    global.set $prof_520
    array.new_data $bytes $s117
    call $string_from_bytes
    i32.const 0
    i32.const 13
    global.get $prof_521
    i64.const 1
    i64.add
    global.set $prof_521
    array.new_data $bytes $s118
    call $string_from_bytes
    i32.const 0
    i32.const 12
    global.get $prof_522
    i64.const 1
    i64.add
    global.set $prof_522
    array.new_data $bytes $s119
    call $string_from_bytes
    i32.const 0
    i32.const 8
    global.get $prof_523
    i64.const 1
    i64.add
    global.set $prof_523
    array.new_data $bytes $s120
    call $string_from_bytes
    i32.const 0
    i32.const 8
    global.get $prof_524
    i64.const 1
    i64.add
    global.set $prof_524
    array.new_data $bytes $s121
    call $string_from_bytes
    i32.const 0
    i32.const 8
    global.get $prof_525
    i64.const 1
    i64.add
    global.set $prof_525
    array.new_data $bytes $s122
    call $string_from_bytes
    i32.const 0
    i32.const 9
    global.get $prof_526
    i64.const 1
    i64.add
    global.set $prof_526
    array.new_data $bytes $s123
    call $string_from_bytes
    i32.const 0
    i32.const 12
    global.get $prof_527
    i64.const 1
    i64.add
    global.set $prof_527
    array.new_data $bytes $s124
    call $string_from_bytes
    i32.const 0
    i32.const 12
    global.get $prof_528
    i64.const 1
    i64.add
    global.set $prof_528
    array.new_data $bytes $s125
    call $string_from_bytes
    i32.const 0
    i32.const 10
    global.get $prof_529
    i64.const 1
    i64.add
    global.set $prof_529
    array.new_data $bytes $s126
    call $string_from_bytes
    i32.const 0
    i32.const 9
    global.get $prof_530
    i64.const 1
    i64.add
    global.set $prof_530
    array.new_data $bytes $s127
    call $string_from_bytes
    i32.const 0
    i32.const 12
    global.get $prof_531
    i64.const 1
    i64.add
    global.set $prof_531
    array.new_data $bytes $s128
    call $string_from_bytes
    i32.const 0
    i32.const 7
    global.get $prof_532
    i64.const 1
    i64.add
    global.set $prof_532
    array.new_data $bytes $s129
    call $string_from_bytes
    i32.const 0
    i32.const 7
    global.get $prof_533
    i64.const 1
    i64.add
    global.set $prof_533
    array.new_data $bytes $s130
    call $string_from_bytes
    i32.const 0
    i32.const 7
    global.get $prof_534
    i64.const 1
    i64.add
    global.set $prof_534
    array.new_data $bytes $s131
    call $string_from_bytes
    i32.const 0
    i32.const 12
    global.get $prof_535
    i64.const 1
    i64.add
    global.set $prof_535
    array.new_data $bytes $s132
    call $string_from_bytes
    i32.const 0
    i32.const 8
    global.get $prof_536
    i64.const 1
    i64.add
    global.set $prof_536
    array.new_data $bytes $s133
    call $string_from_bytes
    i32.const 0
    i32.const 9
    global.get $prof_537
    i64.const 1
    i64.add
    global.set $prof_537
    array.new_data $bytes $s134
    call $string_from_bytes
    i32.const 0
    i32.const 8
    global.get $prof_538
    i64.const 1
    i64.add
    global.set $prof_538
    array.new_data $bytes $s135
    call $string_from_bytes
    i32.const 0
    i32.const 11
    global.get $prof_539
    i64.const 1
    i64.add
    global.set $prof_539
    array.new_data $bytes $s136
    call $string_from_bytes
    i32.const 0
    i32.const 13
    global.get $prof_540
    i64.const 1
    i64.add
    global.set $prof_540
    array.new_data $bytes $s137
    call $string_from_bytes
    global.get $prof_541
    i64.const 1
    i64.add
    global.set $prof_541
    array.new_fixed $values 134
    call $constructors_install
    ref.null eq
    i32.const 2
    global.get $prof_542
    i64.const 1
    i64.add
    global.set $prof_542
    array.new $env
    global.set $root_env
    global.get $root_env
    global.get $prof_10
    i64.const 1
    i64.add
    global.set $prof_10
    ref.as_non_null
    i32.const 0
    i32.const 3
    i64.const 2
    i32.const 7
    i32.const 0
    i64.const 1
    f64.const 0x0p+0 (;=0;)
    ref.null $bytes
    ref.null $values
    ref.null $limbs
    ref.null $call
    ref.func $entry_assert_fail
    global.get $prof_543
    i64.const 1
    i64.add
    global.set $prof_543
    struct.new $clo1
    global.get $prof_247
    i64.const 1
    i64.add
    global.set $prof_247
    array.new_fixed $values 1
    call $record
    array.set $env
    ref.null $obj
    global.get $root_env
    call $start
    drop
  )
  (func (;346;) (type 139) (param f64) (result f64)
    global.get $prof_544
    i64.const 1
    i64.add
    global.set $prof_544
    ref.null $obj
    global.get $root_env
    local.get 0
    call $abi_float_in
    call $f0
    call $abi_float_out
  )
  (func (;347;) (type 139) (param f64) (result f64)
    global.get $prof_545
    i64.const 1
    i64.add
    global.set $prof_545
    ref.null $obj
    global.get $root_env
    local.get 0
    call $abi_float_in
    call $f1
    call $abi_float_out
  )
  (func (;348;) (type 139) (param f64) (result f64)
    global.get $prof_546
    i64.const 1
    i64.add
    global.set $prof_546
    ref.null $obj
    global.get $root_env
    local.get 0
    call $abi_float_in
    call $f2
    call $abi_float_out
  )
  (func (;349;) (type 139) (param f64) (result f64)
    global.get $prof_547
    i64.const 1
    i64.add
    global.set $prof_547
    ref.null $obj
    global.get $root_env
    local.get 0
    call $abi_float_in
    call $f3
    call $abi_float_out
  )
  (func (;350;) (type 139) (param f64) (result f64)
    global.get $prof_548
    i64.const 1
    i64.add
    global.set $prof_548
    ref.null $obj
    global.get $root_env
    local.get 0
    call $abi_float_in
    call $f4
    call $abi_float_out
  )
  (func (;351;) (type 139) (param f64) (result f64)
    global.get $prof_549
    i64.const 1
    i64.add
    global.set $prof_549
    ref.null $obj
    global.get $root_env
    local.get 0
    call $abi_float_in
    call $f5
    call $abi_float_out
  )
  (func (;352;) (type 139) (param f64) (result f64)
    global.get $prof_550
    i64.const 1
    i64.add
    global.set $prof_550
    ref.null $obj
    global.get $root_env
    local.get 0
    call $abi_float_in
    call $f6
    call $abi_float_out
  )
  (data $case_upper_data (;0;) "a\00\00\00z\00\00\00\e0\ff\ff\ff\b5\00\00\00\b5\00\00\00\e7\02\00\00\e0\00\00\00\f6\00\00\00\e0\ff\ff\ff\f8\00\00\00\fe\00\00\00\e0\ff\ff\ff\ff\00\00\00\ff\00\00\00y\00\00\00\01\01\00\00/\01\00\01\ff\ff\ff\ff1\01\00\001\01\00\00\18\ff\ff\ff3\01\00\007\01\00\01\ff\ff\ff\ff:\01\00\00H\01\00\01\ff\ff\ff\ffK\01\00\00w\01\00\01\ff\ff\ff\ffz\01\00\00~\01\00\01\ff\ff\ff\ff\7f\01\00\00\7f\01\00\00\d4\fe\ff\ff\80\01\00\00\80\01\00\00\c3\00\00\00\83\01\00\00\85\01\00\01\ff\ff\ff\ff\88\01\00\00\88\01\00\00\ff\ff\ff\ff\8c\01\00\00\8c\01\00\00\ff\ff\ff\ff\92\01\00\00\92\01\00\00\ff\ff\ff\ff\95\01\00\00\95\01\00\00a\00\00\00\99\01\00\00\99\01\00\00\ff\ff\ff\ff\9a\01\00\00\9a\01\00\00\a3\00\00\00\9b\01\00\00\9b\01\00\00A\a6\00\00\9e\01\00\00\9e\01\00\00\82\00\00\00\a1\01\00\00\a5\01\00\01\ff\ff\ff\ff\a8\01\00\00\a8\01\00\00\ff\ff\ff\ff\ad\01\00\00\ad\01\00\00\ff\ff\ff\ff\b0\01\00\00\b0\01\00\00\ff\ff\ff\ff\b4\01\00\00\b6\01\00\01\ff\ff\ff\ff\b9\01\00\00\b9\01\00\00\ff\ff\ff\ff\bd\01\00\00\bd\01\00\00\ff\ff\ff\ff\bf\01\00\00\bf\01\00\008\00\00\00\c5\01\00\00\c5\01\00\00\ff\ff\ff\ff\c6\01\00\00\c6\01\00\00\fe\ff\ff\ff\c8\01\00\00\c8\01\00\00\ff\ff\ff\ff\c9\01\00\00\c9\01\00\00\fe\ff\ff\ff\cb\01\00\00\cb\01\00\00\ff\ff\ff\ff\cc\01\00\00\cc\01\00\00\fe\ff\ff\ff\ce\01\00\00\dc\01\00\01\ff\ff\ff\ff\dd\01\00\00\dd\01\00\00\b1\ff\ff\ff\df\01\00\00\ef\01\00\01\ff\ff\ff\ff\f2\01\00\00\f2\01\00\00\ff\ff\ff\ff\f3\01\00\00\f3\01\00\00\fe\ff\ff\ff\f5\01\00\00\f5\01\00\00\ff\ff\ff\ff\f9\01\00\00\1f\02\00\01\ff\ff\ff\ff#\02\00\003\02\00\01\ff\ff\ff\ff<\02\00\00<\02\00\00\ff\ff\ff\ff?\02\00\00@\02\00\00?*\00\00B\02\00\00B\02\00\00\ff\ff\ff\ffG\02\00\00O\02\00\01\ff\ff\ff\ffP\02\00\00P\02\00\00\1f*\00\00Q\02\00\00Q\02\00\00\1c*\00\00R\02\00\00R\02\00\00\1e*\00\00S\02\00\00S\02\00\00.\ff\ff\ffT\02\00\00T\02\00\002\ff\ff\ffV\02\00\00W\02\00\003\ff\ff\ffY\02\00\00Y\02\00\006\ff\ff\ff[\02\00\00[\02\00\005\ff\ff\ff\5c\02\00\00\5c\02\00\00O\a5\00\00`\02\00\00`\02\00\003\ff\ff\ffa\02\00\00a\02\00\00K\a5\00\00c\02\00\00c\02\00\001\ff\ff\ffd\02\00\00d\02\00\00g\a5\00\00e\02\00\00e\02\00\00(\a5\00\00f\02\00\00f\02\00\00D\a5\00\00h\02\00\00h\02\00\00/\ff\ff\ffi\02\00\00i\02\00\00-\ff\ff\ffj\02\00\00j\02\00\00D\a5\00\00k\02\00\00k\02\00\00\f7)\00\00l\02\00\00l\02\00\00A\a5\00\00o\02\00\00o\02\00\00-\ff\ff\ffq\02\00\00q\02\00\00\fd)\00\00r\02\00\00r\02\00\00+\ff\ff\ffu\02\00\00u\02\00\00*\ff\ff\ffw\02\00\00w\02\00\00f\a5\00\00|\02\00\00|\02\00\00f\a5\00\00}\02\00\00}\02\00\00\e7)\00\00\80\02\00\00\80\02\00\00&\ff\ff\ff\82\02\00\00\82\02\00\00C\a5\00\00\83\02\00\00\83\02\00\00&\ff\ff\ff\87\02\00\00\87\02\00\00*\a5\00\00\88\02\00\00\88\02\00\00&\ff\ff\ff\89\02\00\00\89\02\00\00\bb\ff\ff\ff\8a\02\00\00\8b\02\00\00'\ff\ff\ff\8c\02\00\00\8c\02\00\00\b9\ff\ff\ff\92\02\00\00\92\02\00\00%\ff\ff\ff\9d\02\00\00\9d\02\00\00\15\a5\00\00\9e\02\00\00\9e\02\00\00\12\a5\00\00E\03\00\00E\03\00\00T\00\00\00q\03\00\00s\03\00\01\ff\ff\ff\ffw\03\00\00w\03\00\00\ff\ff\ff\ff{\03\00\00}\03\00\00\82\00\00\00\ac\03\00\00\ac\03\00\00\da\ff\ff\ff\ad\03\00\00\af\03\00\00\db\ff\ff\ff\b1\03\00\00\c1\03\00\00\e0\ff\ff\ff\c2\03\00\00\c2\03\00\00\e1\ff\ff\ff\c3\03\00\00\cb\03\00\00\e0\ff\ff\ff\cc\03\00\00\cc\03\00\00\c0\ff\ff\ff\cd\03\00\00\ce\03\00\00\c1\ff\ff\ff\d0\03\00\00\d0\03\00\00\c2\ff\ff\ff\d1\03\00\00\d1\03\00\00\c7\ff\ff\ff\d5\03\00\00\d5\03\00\00\d1\ff\ff\ff\d6\03\00\00\d6\03\00\00\ca\ff\ff\ff\d7\03\00\00\d7\03\00\00\f8\ff\ff\ff\d9\03\00\00\ef\03\00\01\ff\ff\ff\ff\f0\03\00\00\f0\03\00\00\aa\ff\ff\ff\f1\03\00\00\f1\03\00\00\b0\ff\ff\ff\f2\03\00\00\f2\03\00\00\07\00\00\00\f3\03\00\00\f3\03\00\00\8c\ff\ff\ff\f5\03\00\00\f5\03\00\00\a0\ff\ff\ff\f8\03\00\00\f8\03\00\00\ff\ff\ff\ff\fb\03\00\00\fb\03\00\00\ff\ff\ff\ff0\04\00\00O\04\00\00\e0\ff\ff\ffP\04\00\00_\04\00\00\b0\ff\ff\ffa\04\00\00\81\04\00\01\ff\ff\ff\ff\8b\04\00\00\bf\04\00\01\ff\ff\ff\ff\c2\04\00\00\ce\04\00\01\ff\ff\ff\ff\cf\04\00\00\cf\04\00\00\f1\ff\ff\ff\d1\04\00\00/\05\00\01\ff\ff\ff\ffa\05\00\00\86\05\00\00\d0\ff\ff\ff\d0\10\00\00\fa\10\00\00\c0\0b\00\00\fd\10\00\00\ff\10\00\00\c0\0b\00\00\f8\13\00\00\fd\13\00\00\f8\ff\ff\ff\80\1c\00\00\80\1c\00\00\92\e7\ff\ff\81\1c\00\00\81\1c\00\00\93\e7\ff\ff\82\1c\00\00\82\1c\00\00\9c\e7\ff\ff\83\1c\00\00\84\1c\00\00\9e\e7\ff\ff\85\1c\00\00\85\1c\00\00\9d\e7\ff\ff\86\1c\00\00\86\1c\00\00\a4\e7\ff\ff\87\1c\00\00\87\1c\00\00\db\e7\ff\ff\88\1c\00\00\88\1c\00\00\c2\89\00\00\8a\1c\00\00\8a\1c\00\00\ff\ff\ff\ffy\1d\00\00y\1d\00\00\04\8a\00\00}\1d\00\00}\1d\00\00\e6\0e\00\00\8e\1d\00\00\8e\1d\00\008\8a\00\00\01\1e\00\00\95\1e\00\01\ff\ff\ff\ff\9b\1e\00\00\9b\1e\00\00\c5\ff\ff\ff\a1\1e\00\00\ff\1e\00\01\ff\ff\ff\ff\00\1f\00\00\07\1f\00\00\08\00\00\00\10\1f\00\00\15\1f\00\00\08\00\00\00 \1f\00\00'\1f\00\00\08\00\00\000\1f\00\007\1f\00\00\08\00\00\00@\1f\00\00E\1f\00\00\08\00\00\00Q\1f\00\00W\1f\00\01\08\00\00\00`\1f\00\00g\1f\00\00\08\00\00\00p\1f\00\00q\1f\00\00J\00\00\00r\1f\00\00u\1f\00\00V\00\00\00v\1f\00\00w\1f\00\00d\00\00\00x\1f\00\00y\1f\00\00\80\00\00\00z\1f\00\00{\1f\00\00p\00\00\00|\1f\00\00}\1f\00\00~\00\00\00\80\1f\00\00\87\1f\00\00\08\00\00\00\90\1f\00\00\97\1f\00\00\08\00\00\00\a0\1f\00\00\a7\1f\00\00\08\00\00\00\b0\1f\00\00\b1\1f\00\00\08\00\00\00\b3\1f\00\00\b3\1f\00\00\09\00\00\00\be\1f\00\00\be\1f\00\00\db\e3\ff\ff\c3\1f\00\00\c3\1f\00\00\09\00\00\00\d0\1f\00\00\d1\1f\00\00\08\00\00\00\e0\1f\00\00\e1\1f\00\00\08\00\00\00\e5\1f\00\00\e5\1f\00\00\07\00\00\00\f3\1f\00\00\f3\1f\00\00\09\00\00\00N!\00\00N!\00\00\e4\ff\ff\ffp!\00\00\7f!\00\00\f0\ff\ff\ff\84!\00\00\84!\00\00\ff\ff\ff\ff\d0$\00\00\e9$\00\00\e6\ff\ff\ff0,\00\00_,\00\00\d0\ff\ff\ffa,\00\00a,\00\00\ff\ff\ff\ffe,\00\00e,\00\00\d5\d5\ff\fff,\00\00f,\00\00\d8\d5\ff\ffh,\00\00l,\00\01\ff\ff\ff\ffs,\00\00s,\00\00\ff\ff\ff\ffv,\00\00v,\00\00\ff\ff\ff\ff\81,\00\00\e3,\00\01\ff\ff\ff\ff\ec,\00\00\ee,\00\01\ff\ff\ff\ff\f3,\00\00\f3,\00\00\ff\ff\ff\ff\00-\00\00%-\00\00\a0\e3\ff\ff'-\00\00'-\00\00\a0\e3\ff\ff--\00\00--\00\00\a0\e3\ff\ffA\a6\00\00m\a6\00\01\ff\ff\ff\ff\81\a6\00\00\9b\a6\00\01\ff\ff\ff\ff#\a7\00\00/\a7\00\01\ff\ff\ff\ff3\a7\00\00o\a7\00\01\ff\ff\ff\ffz\a7\00\00|\a7\00\01\ff\ff\ff\ff\7f\a7\00\00\87\a7\00\01\ff\ff\ff\ff\8c\a7\00\00\8c\a7\00\00\ff\ff\ff\ff\91\a7\00\00\93\a7\00\01\ff\ff\ff\ff\94\a7\00\00\94\a7\00\000\00\00\00\97\a7\00\00\a9\a7\00\01\ff\ff\ff\ff\b5\a7\00\00\c3\a7\00\01\ff\ff\ff\ff\c8\a7\00\00\ca\a7\00\01\ff\ff\ff\ff\cd\a7\00\00\db\a7\00\01\ff\ff\ff\ff\f6\a7\00\00\f6\a7\00\00\ff\ff\ff\ffK\ab\00\00L\ab\00\00!\00\00\00S\ab\00\00S\ab\00\00`\fc\ff\ffp\ab\00\00\bf\ab\00\000h\ff\ffA\ff\00\00Z\ff\00\00\e0\ff\ff\ff(\04\01\00O\04\01\00\d8\ff\ff\ff\d8\04\01\00\fb\04\01\00\d8\ff\ff\ff\97\05\01\00\a1\05\01\00\d9\ff\ff\ff\a3\05\01\00\b1\05\01\00\d9\ff\ff\ff\b3\05\01\00\b9\05\01\00\d9\ff\ff\ff\bb\05\01\00\bc\05\01\00\d9\ff\ff\ff\c0\0c\01\00\f2\0c\01\00\c0\ff\ff\ffp\0d\01\00\85\0d\01\00\e0\ff\ff\ff\c0\18\01\00\df\18\01\00\e0\ff\ff\ff`n\01\00\7fn\01\00\e0\ff\ff\ff\bbn\01\00\d3n\01\00\e5\ff\ff\ffA\df\01\00A\df\01\00\ff\ff\ff\ffI\df\01\00K\df\01\01\ff\ff\ff\ffN\df\01\00N\df\01\00\ff\ff\ff\ffR\df\01\00R\df\01\00\ff\ff\ff\ffi\df\01\00o\df\01\01\ff\ff\ff\ffs\df\01\00\7f\df\01\01\ff\ff\ff\ff\22\e9\01\00C\e9\01\00\de\ff\ff\ff")
  (data $case_lower_data (;1;) "A\00\00\00Z\00\00\00 \00\00\00\c0\00\00\00\d6\00\00\00 \00\00\00\d8\00\00\00\de\00\00\00 \00\00\00\00\01\00\00.\01\00\01\01\00\00\000\01\00\000\01\00\009\ff\ff\ff2\01\00\006\01\00\01\01\00\00\009\01\00\00G\01\00\01\01\00\00\00J\01\00\00v\01\00\01\01\00\00\00x\01\00\00x\01\00\00\87\ff\ff\ffy\01\00\00}\01\00\01\01\00\00\00\81\01\00\00\81\01\00\00\d2\00\00\00\82\01\00\00\84\01\00\01\01\00\00\00\86\01\00\00\86\01\00\00\ce\00\00\00\87\01\00\00\87\01\00\00\01\00\00\00\89\01\00\00\8a\01\00\00\cd\00\00\00\8b\01\00\00\8b\01\00\00\01\00\00\00\8e\01\00\00\8e\01\00\00O\00\00\00\8f\01\00\00\8f\01\00\00\ca\00\00\00\90\01\00\00\90\01\00\00\cb\00\00\00\91\01\00\00\91\01\00\00\01\00\00\00\93\01\00\00\93\01\00\00\cd\00\00\00\94\01\00\00\94\01\00\00\cf\00\00\00\96\01\00\00\96\01\00\00\d3\00\00\00\97\01\00\00\97\01\00\00\d1\00\00\00\98\01\00\00\98\01\00\00\01\00\00\00\9c\01\00\00\9c\01\00\00\d3\00\00\00\9d\01\00\00\9d\01\00\00\d5\00\00\00\9f\01\00\00\9f\01\00\00\d6\00\00\00\a0\01\00\00\a4\01\00\01\01\00\00\00\a6\01\00\00\a6\01\00\00\da\00\00\00\a7\01\00\00\a7\01\00\00\01\00\00\00\a9\01\00\00\a9\01\00\00\da\00\00\00\ac\01\00\00\ac\01\00\00\01\00\00\00\ae\01\00\00\ae\01\00\00\da\00\00\00\af\01\00\00\af\01\00\00\01\00\00\00\b1\01\00\00\b2\01\00\00\d9\00\00\00\b3\01\00\00\b5\01\00\01\01\00\00\00\b7\01\00\00\b7\01\00\00\db\00\00\00\b8\01\00\00\b8\01\00\00\01\00\00\00\bc\01\00\00\bc\01\00\00\01\00\00\00\c4\01\00\00\c4\01\00\00\02\00\00\00\c5\01\00\00\c5\01\00\00\01\00\00\00\c7\01\00\00\c7\01\00\00\02\00\00\00\c8\01\00\00\c8\01\00\00\01\00\00\00\ca\01\00\00\ca\01\00\00\02\00\00\00\cb\01\00\00\db\01\00\01\01\00\00\00\de\01\00\00\ee\01\00\01\01\00\00\00\f1\01\00\00\f1\01\00\00\02\00\00\00\f2\01\00\00\f4\01\00\01\01\00\00\00\f6\01\00\00\f6\01\00\00\9f\ff\ff\ff\f7\01\00\00\f7\01\00\00\c8\ff\ff\ff\f8\01\00\00\1e\02\00\01\01\00\00\00 \02\00\00 \02\00\00~\ff\ff\ff\22\02\00\002\02\00\01\01\00\00\00:\02\00\00:\02\00\00+*\00\00;\02\00\00;\02\00\00\01\00\00\00=\02\00\00=\02\00\00]\ff\ff\ff>\02\00\00>\02\00\00(*\00\00A\02\00\00A\02\00\00\01\00\00\00C\02\00\00C\02\00\00=\ff\ff\ffD\02\00\00D\02\00\00E\00\00\00E\02\00\00E\02\00\00G\00\00\00F\02\00\00N\02\00\01\01\00\00\00p\03\00\00r\03\00\01\01\00\00\00v\03\00\00v\03\00\00\01\00\00\00\7f\03\00\00\7f\03\00\00t\00\00\00\86\03\00\00\86\03\00\00&\00\00\00\88\03\00\00\8a\03\00\00%\00\00\00\8c\03\00\00\8c\03\00\00@\00\00\00\8e\03\00\00\8f\03\00\00?\00\00\00\91\03\00\00\a1\03\00\00 \00\00\00\a3\03\00\00\ab\03\00\00 \00\00\00\cf\03\00\00\cf\03\00\00\08\00\00\00\d8\03\00\00\ee\03\00\01\01\00\00\00\f4\03\00\00\f4\03\00\00\c4\ff\ff\ff\f7\03\00\00\f7\03\00\00\01\00\00\00\f9\03\00\00\f9\03\00\00\f9\ff\ff\ff\fa\03\00\00\fa\03\00\00\01\00\00\00\fd\03\00\00\ff\03\00\00~\ff\ff\ff\00\04\00\00\0f\04\00\00P\00\00\00\10\04\00\00/\04\00\00 \00\00\00`\04\00\00\80\04\00\01\01\00\00\00\8a\04\00\00\be\04\00\01\01\00\00\00\c0\04\00\00\c0\04\00\00\0f\00\00\00\c1\04\00\00\cd\04\00\01\01\00\00\00\d0\04\00\00.\05\00\01\01\00\00\001\05\00\00V\05\00\000\00\00\00\a0\10\00\00\c5\10\00\00`\1c\00\00\c7\10\00\00\c7\10\00\00`\1c\00\00\cd\10\00\00\cd\10\00\00`\1c\00\00\a0\13\00\00\ef\13\00\00\d0\97\00\00\f0\13\00\00\f5\13\00\00\08\00\00\00\89\1c\00\00\89\1c\00\00\01\00\00\00\90\1c\00\00\ba\1c\00\00@\f4\ff\ff\bd\1c\00\00\bf\1c\00\00@\f4\ff\ff\00\1e\00\00\94\1e\00\01\01\00\00\00\9e\1e\00\00\9e\1e\00\00A\e2\ff\ff\a0\1e\00\00\fe\1e\00\01\01\00\00\00\08\1f\00\00\0f\1f\00\00\f8\ff\ff\ff\18\1f\00\00\1d\1f\00\00\f8\ff\ff\ff(\1f\00\00/\1f\00\00\f8\ff\ff\ff8\1f\00\00?\1f\00\00\f8\ff\ff\ffH\1f\00\00M\1f\00\00\f8\ff\ff\ffY\1f\00\00_\1f\00\01\f8\ff\ff\ffh\1f\00\00o\1f\00\00\f8\ff\ff\ff\88\1f\00\00\8f\1f\00\00\f8\ff\ff\ff\98\1f\00\00\9f\1f\00\00\f8\ff\ff\ff\a8\1f\00\00\af\1f\00\00\f8\ff\ff\ff\b8\1f\00\00\b9\1f\00\00\f8\ff\ff\ff\ba\1f\00\00\bb\1f\00\00\b6\ff\ff\ff\bc\1f\00\00\bc\1f\00\00\f7\ff\ff\ff\c8\1f\00\00\cb\1f\00\00\aa\ff\ff\ff\cc\1f\00\00\cc\1f\00\00\f7\ff\ff\ff\d8\1f\00\00\d9\1f\00\00\f8\ff\ff\ff\da\1f\00\00\db\1f\00\00\9c\ff\ff\ff\e8\1f\00\00\e9\1f\00\00\f8\ff\ff\ff\ea\1f\00\00\eb\1f\00\00\90\ff\ff\ff\ec\1f\00\00\ec\1f\00\00\f9\ff\ff\ff\f8\1f\00\00\f9\1f\00\00\80\ff\ff\ff\fa\1f\00\00\fb\1f\00\00\82\ff\ff\ff\fc\1f\00\00\fc\1f\00\00\f7\ff\ff\ff&!\00\00&!\00\00\a3\e2\ff\ff*!\00\00*!\00\00A\df\ff\ff+!\00\00+!\00\00\ba\df\ff\ff2!\00\002!\00\00\1c\00\00\00`!\00\00o!\00\00\10\00\00\00\83!\00\00\83!\00\00\01\00\00\00\b6$\00\00\cf$\00\00\1a\00\00\00\00,\00\00/,\00\000\00\00\00`,\00\00`,\00\00\01\00\00\00b,\00\00b,\00\00\09\d6\ff\ffc,\00\00c,\00\00\1a\f1\ff\ffd,\00\00d,\00\00\19\d6\ff\ffg,\00\00k,\00\01\01\00\00\00m,\00\00m,\00\00\e4\d5\ff\ffn,\00\00n,\00\00\03\d6\ff\ffo,\00\00o,\00\00\e1\d5\ff\ffp,\00\00p,\00\00\e2\d5\ff\ffr,\00\00r,\00\00\01\00\00\00u,\00\00u,\00\00\01\00\00\00~,\00\00\7f,\00\00\c1\d5\ff\ff\80,\00\00\e2,\00\01\01\00\00\00\eb,\00\00\ed,\00\01\01\00\00\00\f2,\00\00\f2,\00\00\01\00\00\00@\a6\00\00l\a6\00\01\01\00\00\00\80\a6\00\00\9a\a6\00\01\01\00\00\00\22\a7\00\00.\a7\00\01\01\00\00\002\a7\00\00n\a7\00\01\01\00\00\00y\a7\00\00{\a7\00\01\01\00\00\00}\a7\00\00}\a7\00\00\fcu\ff\ff~\a7\00\00\86\a7\00\01\01\00\00\00\8b\a7\00\00\8b\a7\00\00\01\00\00\00\8d\a7\00\00\8d\a7\00\00\d8Z\ff\ff\90\a7\00\00\92\a7\00\01\01\00\00\00\96\a7\00\00\a8\a7\00\01\01\00\00\00\aa\a7\00\00\aa\a7\00\00\bcZ\ff\ff\ab\a7\00\00\ab\a7\00\00\b1Z\ff\ff\ac\a7\00\00\ac\a7\00\00\b5Z\ff\ff\ad\a7\00\00\ad\a7\00\00\bfZ\ff\ff\ae\a7\00\00\ae\a7\00\00\bcZ\ff\ff\b0\a7\00\00\b0\a7\00\00\eeZ\ff\ff\b1\a7\00\00\b1\a7\00\00\d6Z\ff\ff\b2\a7\00\00\b2\a7\00\00\ebZ\ff\ff\b3\a7\00\00\b3\a7\00\00\a0\03\00\00\b4\a7\00\00\c2\a7\00\01\01\00\00\00\c4\a7\00\00\c4\a7\00\00\d0\ff\ff\ff\c5\a7\00\00\c5\a7\00\00\bdZ\ff\ff\c6\a7\00\00\c6\a7\00\00\c8u\ff\ff\c7\a7\00\00\c9\a7\00\01\01\00\00\00\cb\a7\00\00\cb\a7\00\00\99Z\ff\ff\cc\a7\00\00\da\a7\00\01\01\00\00\00\dc\a7\00\00\dc\a7\00\00\bfY\ff\ff\dd\a7\00\00\dd\a7\00\00\9aZ\ff\ff\e2\a7\00\00\e2\a7\00\00\9aZ\ff\ff\f5\a7\00\00\f5\a7\00\00\01\00\00\00l\ab\00\00m\ab\00\00\df\ff\ff\ff!\ff\00\00:\ff\00\00 \00\00\00\00\04\01\00'\04\01\00(\00\00\00\b0\04\01\00\d3\04\01\00(\00\00\00p\05\01\00z\05\01\00'\00\00\00|\05\01\00\8a\05\01\00'\00\00\00\8c\05\01\00\92\05\01\00'\00\00\00\94\05\01\00\95\05\01\00'\00\00\00\80\0c\01\00\b2\0c\01\00@\00\00\00P\0d\01\00e\0d\01\00 \00\00\00\a0\18\01\00\bf\18\01\00 \00\00\00@n\01\00_n\01\00 \00\00\00\a0n\01\00\b8n\01\00\1b\00\00\00@\df\01\00@\df\01\00\01\00\00\00H\df\01\00J\df\01\01\01\00\00\00M\df\01\00M\df\01\00\01\00\00\00Q\df\01\00Q\df\01\00\01\00\00\00h\df\01\00n\df\01\01\01\00\00\00r\df\01\00~\df\01\01\01\00\00\00\00\e9\01\00!\e9\01\00\22\00\00\00")
  (data $s0 (;2;) "0123456789ABCDEF")
  (data $s1 (;3;) "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/")
  (data $s2 (;4;) "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_")
  (data $site_data (;5;) "\00\00\00\00+\00\00\009\00\00\00\00\00\00\00+\00\00\002\00\00\00\00\00\00\00+\00\00\00\02\00\00\00\00\00\00\00(\00\00\001\00\00\00\00\00\00\00(\00\00\00&\00\00\00\00\00\00\00\22\00\00\00\0b\00\00\00\00\00\00\00#\00\00\00\10\00\00\00\00\00\00\00#\00\00\00\0b\00\00\00\00\00\00\00$\00\00\00\10\00\00\00\00\00\00\00$\00\00\00\0b\00\00\00\00\00\00\00%\00\00\00\02\00\00\00\00\00\00\00 \00\00\00(\00\00\00\00\00\00\00 \00\00\003\00\00\00\00\00\00\00\1f\00\00\00;\00\00\00\00\00\00\00\1f\00\00\00/\00\00\00\00\00\00\00\1f\00\00\00!\00\00\00\00\00\00\00\1d\00\00\00'\00\00\00\00\00\00\00\1b\00\00\00\1d\00\00\00\00\00\00\00\17\00\00\00'\00\00\00\00\00\00\00\17\00\00\00 \00\00\00\00\00\00\00\17\00\00\00\11\00\00\00\00\00\00\00\06\00\00\00\06\00\00\00\00\00\00\00\07\00\00\00\06\00\00\00\00\00\00\00\08\00\00\00\06\00\00\00\00\00\00\00\09\00\00\00\06\00\00\00\00\00\00\00\0a\00\00\00\06\00\00\00\00\00\00\00\0b\00\00\00\06\00\00\00\00\00\00\00\0c\00\00\00\02\00\00\00\00\00\00\00\04\00\00\00\1e\00\00\00\01\00\00\00\1d\0a\00\00\22\00\00\00\01\00\00\00\03\06\00\00\02\00\00\00\01\00\00\00\06\06\00\00\04\00\00\00\01\00\00\00\07\06\00\00\04\00\00\00\01\00\00\00\05\06\00\00\14\00\00\00\01\00\00\00\ee\05\00\00\0b\00\00\00\01\00\00\00\f5\05\00\00\0c\00\00\00\01\00\00\00\f6\05\00\00\1f\00\00\00\01\00\00\00\f6\05\00\00\0c\00\00\00\01\00\00\00\fd\05\00\00\0c\00\00\00\01\00\00\00s\08\00\00\0c\00\00\00\01\00\00\00t\08\00\00\18\00\00\00\01\00\00\00t\08\00\00\0b\00\00\00\01\00\00\00u\08\00\00\0b\00\00\00\01\00\00\00w\08\00\00\08\00\00\00\01\00\00\00x\08\00\00\08\00\00\00\01\00\00\00y\08\00\00\08\00\00\00\01\00\00\00z\08\00\00\07\00\00\00\01\00\00\00{\08\00\00\07\00\00\00\01\00\00\00|\08\00\00\07\00\00\00\01\00\00\00}\08\00\00\07\00\00\00\01\00\00\00\80\08\00\00\0f\00\00\00\01\00\00\00\81\08\00\00\0d\00\00\00\01\00\00\00\82\08\00\00\1a\00\00\00\01\00\00\00\82\08\00\00\0c\00\00\00\01\00\00\00\83\08\00\00\0b\00\00\00\01\00\00\00\85\08\00\00\07\00\00\00\01\00\00\00\85\08\00\00\05\00\00\00\01\00\00\00\85\08\00\000\00\00\00\01\00\00\00\85\08\00\00\03\00\00\00\01\00\00\00\86\08\00\00\04\00\00\00\01\00\00\00\89\08\00\00\05\00\00\00\01\00\00\00\84\08\00\00\0d\00\00\00\01\00\00\00\8a\08\00\00 \00\00\00\01\00\00\00\8a\08\00\00\12\00\00\00\01\00\00\00\8a\08\00\006\00\00\00\01\00\00\00\8b\08\00\00\0e\00\00\00\01\00\00\00\8c\08\00\00!\00\00\00\01\00\00\00\8c\08\00\00\0e\00\00\00\01\00\00\00\8d\08\00\00\0b\00\00\00\01\00\00\00\8e\08\00\00\1f\00\00\00\01\00\00\00\8e\08\00\00\10\00\00\00\01\00\00\00\8f\08\00\00\0f\00\00\00\01\00\00\00\90\08\00\00\0a\00\00\00\01\00\00\00\91\08\00\00\1a\00\00\00\01\00\00\00\91\08\00\00\0a\00\00\00\01\00\00\00\92\08\00\00\0f\00\00\00\01\00\00\00\93\08\00\00\0d\00\00\00\01\00\00\00\94\08\00\00\1a\00\00\00\01\00\00\00\94\08\00\00\0c\00\00\00\01\00\00\00\95\08\00\00\0b\00\00\00\01\00\00\00\97\08\00\00\07\00\00\00\01\00\00\00\97\08\00\00\05\00\00\00\01\00\00\00\97\08\00\000\00\00\00\01\00\00\00\97\08\00\00\03\00\00\00\01\00\00\00\98\08\00\00\04\00\00\00\01\00\00\00\9b\08\00\00\05\00\00\00\01\00\00\00\96\08\00\00\0d\00\00\00\01\00\00\00\9c\08\00\00 \00\00\00\01\00\00\00\9c\08\00\00\12\00\00\00\01\00\00\00\9c\08\00\006\00\00\00\01\00\00\00\9d\08\00\00\0e\00\00\00\01\00\00\00\9e\08\00\00!\00\00\00\01\00\00\00\9e\08\00\00\0e\00\00\00\01\00\00\00\9f\08\00\00\0b\00\00\00\01\00\00\00\a0\08\00\00\1f\00\00\00\01\00\00\00\a0\08\00\00\10\00\00\00\01\00\00\00\a1\08\00\00\0f\00\00\00\01\00\00\00\a2\08\00\00\0a\00\00\00\01\00\00\00\a3\08\00\00\1a\00\00\00\01\00\00\00\a3\08\00\00\0a\00\00\00\01\00\00\00\a4\08\00\00\0f\00\00\00\01\00\00\00\a5\08\00\00\0d\00\00\00\01\00\00\00\a6\08\00\00\1a\00\00\00\01\00\00\00\a6\08\00\00\0c\00\00\00\01\00\00\00\a7\08\00\00\0b\00\00\00\01\00\00\00\a9\08\00\00\07\00\00\00\01\00\00\00\a9\08\00\00\05\00\00\00\01\00\00\00\a9\08\00\000\00\00\00\01\00\00\00\a9\08\00\00\03\00\00\00\01\00\00\00\aa\08\00\00\04\00\00\00\01\00\00\00\ad\08\00\00\05\00\00\00\01\00\00\00\a8\08\00\00\0d\00\00\00\01\00\00\00\ae\08\00\00 \00\00\00\01\00\00\00\ae\08\00\00\12\00\00\00\01\00\00\00\ae\08\00\006\00\00\00\01\00\00\00\af\08\00\00\0e\00\00\00\01\00\00\00\b0\08\00\00!\00\00\00\01\00\00\00\b0\08\00\00\0e\00\00\00\01\00\00\00\b1\08\00\00\0b\00\00\00\01\00\00\00\b2\08\00\00\1f\00\00\00\01\00\00\00\b2\08\00\00\10\00\00\00\01\00\00\00\b3\08\00\00\0f\00\00\00\01\00\00\00\b4\08\00\00\0a\00\00\00\01\00\00\00\b5\08\00\00\1a\00\00\00\01\00\00\00\b5\08\00\00\0a\00\00\00\01\00\00\00\b6\08\00\00\0f\00\00\00\01\00\00\00\b7\08\00\00\0d\00\00\00\01\00\00\00\b8\08\00\00\1a\00\00\00\01\00\00\00\b8\08\00\00\0c\00\00\00\01\00\00\00\b9\08\00\00\0b\00\00\00\01\00\00\00\bb\08\00\00\07\00\00\00\01\00\00\00\bb\08\00\00\05\00\00\00\01\00\00\00\bb\08\00\000\00\00\00\01\00\00\00\bb\08\00\00\03\00\00\00\01\00\00\00\bc\08\00\00\04\00\00\00\01\00\00\00\bf\08\00\00\05\00\00\00\01\00\00\00\ba\08\00\00\0d\00\00\00\01\00\00\00\c0\08\00\00 \00\00\00\01\00\00\00\c0\08\00\00\12\00\00\00\01\00\00\00\c0\08\00\006\00\00\00\01\00\00\00\c1\08\00\00\0e\00\00\00\01\00\00\00\c2\08\00\00!\00\00\00\01\00\00\00\c2\08\00\00\0e\00\00\00\01\00\00\00\c3\08\00\00\0b\00\00\00\01\00\00\00\c4\08\00\00\1f\00\00\00\01\00\00\00\c4\08\00\00\10\00\00\00\01\00\00\00\c5\08\00\00\0f\00\00\00\01\00\00\00\c6\08\00\00\0a\00\00\00\01\00\00\00\c7\08\00\00\1a\00\00\00\01\00\00\00\c7\08\00\00\0a\00\00\00\01\00\00\00\c8\08\00\00\0f\00\00\00\01\00\00\00\c9\08\00\00\0d\00\00\00\01\00\00\00\ca\08\00\00\1a\00\00\00\01\00\00\00\ca\08\00\00\0c\00\00\00\01\00\00\00\cb\08\00\00\0b\00\00\00\01\00\00\00\cd\08\00\00\07\00\00\00\01\00\00\00\cd\08\00\00\05\00\00\00\01\00\00\00\cd\08\00\000\00\00\00\01\00\00\00\cd\08\00\00\03\00\00\00\01\00\00\00\ce\08\00\00\04\00\00\00\01\00\00\00\d1\08\00\00\05\00\00\00\01\00\00\00\cc\08\00\00\0d\00\00\00\01\00\00\00\d2\08\00\00 \00\00\00\01\00\00\00\d2\08\00\00\12\00\00\00\01\00\00\00\d2\08\00\006\00\00\00\01\00\00\00\d3\08\00\00\0e\00\00\00\01\00\00\00\d4\08\00\00!\00\00\00\01\00\00\00\d4\08\00\00\0e\00\00\00\01\00\00\00\d5\08\00\00\0b\00\00\00\01\00\00\00\d6\08\00\00\1f\00\00\00\01\00\00\00\d6\08\00\00\10\00\00\00\01\00\00\00\d7\08\00\00\0f\00\00\00\01\00\00\00\d8\08\00\00\0a\00\00\00\01\00\00\00\d9\08\00\00\1a\00\00\00\01\00\00\00\d9\08\00\00\0a\00\00\00\01\00\00\00\da\08\00\00\0f\00\00\00\01\00\00\00\db\08\00\00\0d\00\00\00\01\00\00\00\dc\08\00\00\1a\00\00\00\01\00\00\00\dc\08\00\00\0c\00\00\00\01\00\00\00\dd\08\00\00\0b\00\00\00\01\00\00\00\df\08\00\00\07\00\00\00\01\00\00\00\df\08\00\00\05\00\00\00\01\00\00\00\df\08\00\000\00\00\00\01\00\00\00\df\08\00\00\03\00\00\00\01\00\00\00\e0\08\00\00\04\00\00\00\01\00\00\00\e3\08\00\00\05\00\00\00\01\00\00\00\de\08\00\00\0d\00\00\00\01\00\00\00\e4\08\00\00 \00\00\00\01\00\00\00\e4\08\00\00\12\00\00\00\01\00\00\00\e4\08\00\006\00\00\00\01\00\00\00\e5\08\00\00\0e\00\00\00\01\00\00\00\e6\08\00\00!\00\00\00\01\00\00\00\e6\08\00\00\0e\00\00\00\01\00\00\00\e7\08\00\00\0b\00\00\00\01\00\00\00\e8\08\00\00\1f\00\00\00\01\00\00\00\e8\08\00\00\10\00\00\00\01\00\00\00\e9\08\00\00\0f\00\00\00\01\00\00\00\ea\08\00\00\0a\00\00\00\01\00\00\00\eb\08\00\00\1a\00\00\00\01\00\00\00\eb\08\00\00\0a\00\00\00\01\00\00\00\1c\09\00\00\0c\00\00\00\01\00\00\00\1d\09\00\00\18\00\00\00\01\00\00\00\1d\09\00\00\0b\00\00\00\01\00\00\00\1e\09\00\00\0b\00\00\00\01\00\00\00 \09\00\00\08\00\00\00\01\00\00\00!\09\00\00\08\00\00\00\01\00\00\00\22\09\00\00\08\00\00\00\01\00\00\00#\09\00\00\07\00\00\00\01\00\00\00$\09\00\00\07\00\00\00\01\00\00\00%\09\00\00\07\00\00\00\01\00\00\00(\09\00\00\0f\00\00\00\01\00\00\00)\09\00\00\0d\00\00\00\01\00\00\00*\09\00\00\1a\00\00\00\01\00\00\00*\09\00\00\0c\00\00\00\01\00\00\00+\09\00\00\0b\00\00\00\01\00\00\00-\09\00\00\07\00\00\00\01\00\00\00-\09\00\00\05\00\00\00\01\00\00\00-\09\00\000\00\00\00\01\00\00\00-\09\00\00\03\00\00\00\01\00\00\00.\09\00\00\04\00\00\00\01\00\00\001\09\00\00\05\00\00\00\01\00\00\00,\09\00\00\0d\00\00\00\01\00\00\002\09\00\00 \00\00\00\01\00\00\002\09\00\00\12\00\00\00\01\00\00\002\09\00\006\00\00\00\01\00\00\003\09\00\00\0e\00\00\00\01\00\00\004\09\00\00!\00\00\00\01\00\00\004\09\00\00\0e\00\00\00\01\00\00\005\09\00\00\0b\00\00\00\01\00\00\006\09\00\00\1f\00\00\00\01\00\00\006\09\00\00\10\00\00\00\01\00\00\007\09\00\00\0f\00\00\00\01\00\00\008\09\00\00\0a\00\00\00\01\00\00\009\09\00\00\1a\00\00\00\01\00\00\009\09\00\00\0a\00\00\00\01\00\00\00:\09\00\00\0f\00\00\00\01\00\00\00;\09\00\00\0d\00\00\00\01\00\00\00<\09\00\00\1a\00\00\00\01\00\00\00<\09\00\00\0c\00\00\00\01\00\00\00=\09\00\00\0b\00\00\00\01\00\00\00?\09\00\00\07\00\00\00\01\00\00\00?\09\00\00\05\00\00\00\01\00\00\00?\09\00\000\00\00\00\01\00\00\00?\09\00\00\03\00\00\00\01\00\00\00@\09\00\00\04\00\00\00\01\00\00\00C\09\00\00\05\00\00\00\01\00\00\00>\09\00\00\0d\00\00\00\01\00\00\00D\09\00\00 \00\00\00\01\00\00\00D\09\00\00\12\00\00\00\01\00\00\00D\09\00\006\00\00\00\01\00\00\00E\09\00\00\0e\00\00\00\01\00\00\00F\09\00\00!\00\00\00\01\00\00\00F\09\00\00\0e\00\00\00\01\00\00\00G\09\00\00\0b\00\00\00\01\00\00\00H\09\00\00\1f\00\00\00\01\00\00\00H\09\00\00\10\00\00\00\01\00\00\00I\09\00\00\0f\00\00\00\01\00\00\00J\09\00\00\0a\00\00\00\01\00\00\00K\09\00\00\1a\00\00\00\01\00\00\00K\09\00\00\0a\00\00\00\01\00\00\00L\09\00\00\0f\00\00\00\01\00\00\00M\09\00\00\0d\00\00\00\01\00\00\00N\09\00\00\1a\00\00\00\01\00\00\00N\09\00\00\0c\00\00\00\01\00\00\00O\09\00\00\0b\00\00\00\01\00\00\00Q\09\00\00\07\00\00\00\01\00\00\00Q\09\00\00\05\00\00\00\01\00\00\00Q\09\00\000\00\00\00\01\00\00\00Q\09\00\00\03\00\00\00\01\00\00\00R\09\00\00\04\00\00\00\01\00\00\00U\09\00\00\05\00\00\00\01\00\00\00P\09\00\00\0d\00\00\00\01\00\00\00V\09\00\00 \00\00\00\01\00\00\00V\09\00\00\12\00\00\00\01\00\00\00V\09\00\006\00\00\00\01\00\00\00W\09\00\00\0e\00\00\00\01\00\00\00X\09\00\00!\00\00\00\01\00\00\00X\09\00\00\0e\00\00\00\01\00\00\00Y\09\00\00\0b\00\00\00\01\00\00\00Z\09\00\00\1f\00\00\00\01\00\00\00Z\09\00\00\10\00\00\00\01\00\00\00[\09\00\00\0f\00\00\00\01\00\00\00\5c\09\00\00\0a\00\00\00\01\00\00\00]\09\00\00\1a\00\00\00\01\00\00\00]\09\00\00\0a\00\00\00\01\00\00\00^\09\00\00\0f\00\00\00\01\00\00\00_\09\00\00\0d\00\00\00\01\00\00\00`\09\00\00\1a\00\00\00\01\00\00\00`\09\00\00\0c\00\00\00\01\00\00\00a\09\00\00\0b\00\00\00\01\00\00\00c\09\00\00\07\00\00\00\01\00\00\00c\09\00\00\05\00\00\00\01\00\00\00c\09\00\000\00\00\00\01\00\00\00c\09\00\00\03\00\00\00\01\00\00\00d\09\00\00\04\00\00\00\01\00\00\00g\09\00\00\05\00\00\00\01\00\00\00b\09\00\00\0d\00\00\00\01\00\00\00h\09\00\00 \00\00\00\01\00\00\00h\09\00\00\12\00\00\00\01\00\00\00h\09\00\006\00\00\00\01\00\00\00i\09\00\00\0e\00\00\00\01\00\00\00j\09\00\00!\00\00\00\01\00\00\00j\09\00\00\0e\00\00\00\01\00\00\00k\09\00\00\0b\00\00\00\01\00\00\00l\09\00\00\1f\00\00\00\01\00\00\00l\09\00\00\10\00\00\00\01\00\00\00m\09\00\00\0f\00\00\00\01\00\00\00n\09\00\00\0a\00\00\00\01\00\00\00o\09\00\00\1a\00\00\00\01\00\00\00o\09\00\00\0a\00\00\00\01\00\00\00p\09\00\00\0f\00\00\00\01\00\00\00q\09\00\00\0d\00\00\00\01\00\00\00r\09\00\00\1a\00\00\00\01\00\00\00r\09\00\00\0c\00\00\00\01\00\00\00s\09\00\00\0b\00\00\00\01\00\00\00u\09\00\00\07\00\00\00\01\00\00\00u\09\00\00\05\00\00\00\01\00\00\00u\09\00\000\00\00\00\01\00\00\00u\09\00\00\03\00\00\00\01\00\00\00v\09\00\00\04\00\00\00\01\00\00\00y\09\00\00\05\00\00\00\01\00\00\00t\09\00\00\0d\00\00\00\01\00\00\00z\09\00\00 \00\00\00\01\00\00\00z\09\00\00\12\00\00\00\01\00\00\00z\09\00\006\00\00\00\01\00\00\00{\09\00\00\0e\00\00\00\01\00\00\00|\09\00\00!\00\00\00\01\00\00\00|\09\00\00\0e\00\00\00\01\00\00\00}\09\00\00\0b\00\00\00\01\00\00\00~\09\00\00\1f\00\00\00\01\00\00\00~\09\00\00\10\00\00\00\01\00\00\00\7f\09\00\00\0f\00\00\00\01\00\00\00\80\09\00\00\0a\00\00\00\01\00\00\00\81\09\00\00\1a\00\00\00\01\00\00\00\81\09\00\00\0a\00\00\00\01\00\00\00\82\09\00\00\0f\00\00\00\01\00\00\00\83\09\00\00\0d\00\00\00\01\00\00\00\84\09\00\00\1a\00\00\00\01\00\00\00\84\09\00\00\0c\00\00\00\01\00\00\00\85\09\00\00\0b\00\00\00\01\00\00\00\87\09\00\00\07\00\00\00\01\00\00\00\87\09\00\00\05\00\00\00\01\00\00\00\87\09\00\000\00\00\00\01\00\00\00\87\09\00\00\03\00\00\00\01\00\00\00\88\09\00\00\04\00\00\00\01\00\00\00\8b\09\00\00\05\00\00\00\01\00\00\00\86\09\00\00\0d\00\00\00\01\00\00\00\8c\09\00\00 \00\00\00\01\00\00\00\8c\09\00\00\12\00\00\00\01\00\00\00\8c\09\00\006\00\00\00\01\00\00\00\8d\09\00\00\0e\00\00\00\01\00\00\00\8e\09\00\00!\00\00\00\01\00\00\00\8e\09\00\00\0e\00\00\00\01\00\00\00\8f\09\00\00\0b\00\00\00\01\00\00\00\90\09\00\00\1f\00\00\00\01\00\00\00\90\09\00\00\10\00\00\00\01\00\00\00\91\09\00\00\0f\00\00\00\01\00\00\00\92\09\00\00\0a\00\00\00\01\00\00\00\93\09\00\00\1a\00\00\00\01\00\00\00\93\09\00\00\0a\00\00\00\01\00\00\00\94\09\00\00\0f\00\00\00\01\00\00\00\95\09\00\00\0d\00\00\00\01\00\00\00\96\09\00\00\1a\00\00\00\01\00\00\00\96\09\00\00\0c\00\00\00\01\00\00\00\97\09\00\00\0b\00\00\00\01\00\00\00\99\09\00\00\07\00\00\00\01\00\00\00\99\09\00\00\05\00\00\00\01\00\00\00\99\09\00\000\00\00\00\01\00\00\00\99\09\00\00\03\00\00\00\01\00\00\00\9a\09\00\00\04\00\00\00\01\00\00\00\9d\09\00\00\05\00\00\00\01\00\00\00\98\09\00\00\0d\00\00\00\01\00\00\00\9e\09\00\00 \00\00\00\01\00\00\00\9e\09\00\00\12\00\00\00\01\00\00\00\9e\09\00\006\00\00\00\01\00\00\00\9f\09\00\00\0e\00\00\00\01\00\00\00\a0\09\00\00!\00\00\00\01\00\00\00\a0\09\00\00\0e\00\00\00\01\00\00\00\a1\09\00\00\0b\00\00\00\01\00\00\00\a2\09\00\00\1f\00\00\00\01\00\00\00\a2\09\00\00\10\00\00\00\01\00\00\00\a3\09\00\00\0f\00\00\00\01\00\00\00\a4\09\00\00\0a\00\00\00\01\00\00\00\a5\09\00\00\1a\00\00\00\01\00\00\00\a5\09\00\00\0a\00\00\00\01\00\00\00\9e\05\00\00\0c\00\00\00\01\00\00\00\9f\05\00\00\0c\00\00\00\01\00\00\00\a0\05\00\00\14\00\00\00\01\00\00\00\a0\05\00\00\07\00\00\00\01\00\00\00\a1\05\00\00\07\00\00\00\01\00\00\00\a2\05\00\00\0c\00\00\00\01\00\00\00\a3\05\00\00\14\00\00\00\01\00\00\00\a3\05\00\00\07\00\00\00\01\00\00\00\a4\05\00\00\07\00\00\00\01\00\00\00\a5\05\00\00\10\00\00\00\01\00\00\00\a5\05\00\00\0e\00\00\00\01\00\00\00\a5\05\00\00-\00\00\00\01\00\00\00\a5\05\00\00\0c\00\00\00\01\00\00\00\a5\05\00\00=\00\00\00\01\00\00\00\a6\05\00\00\03\00\00\00\01\00\00\00\a5\05\00\00\0a\00\00\00\01\00\00\00\a8\05\00\00 \00\00\00\01\00\00\00\a8\05\00\00\15\00\00\00\01\00\00\00\a8\05\00\007\00\00\00\01\00\00\00\a8\05\00\00\02\00\00\00\01\00\00\00\95\05\00\00\06\00\00\00\01\00\00\00\96\05\00\00\13\00\00\00\01\00\00\00\96\05\00\00\0c\00\00\00\01\00\00\00\c8\05\00\00\0b\00\00\00\01\00\00\00\c9\05\00\00\16\00\00\00\01\00\00\00\c9\05\00\00\08\00\00\00\01\00\00\00\ca\05\00\00\17\00\00\00\01\00\00\00\ca\05\00\00\09\00\00\00\01\00\00\00\d6\05\00\00\04\00\00\00\01\00\00\00\d9\05\00\00\13\00\00\00\01\00\00\00\d9\05\00\00\08\00\00\00\01\00\00\00\dc\05\00\00\10\00\00\00\01\00\00\00\dc\05\00\00\05\00\00\00\01\00\00\00\da\05\00\00\0f\00\00\00\01\00\00\00\de\05\00\00\06\00\00\00\01\00\00\00\e3\05\00\00$\00\00\00\01\00\00\00\e3\05\00\00\19\00\00\00\01\00\00\00\e3\05\00\00\0c\00\00\00\01\00\00\00\e3\05\00\00\07\00\00\00\01\00\00\00[\05\00\00\09\00\00\00\01\00\00\00\5c\05\00\00\13\00\00\00\01\00\00\00\5c\05\00\00\02\00\00\00\01\00\00\00G\05\00\00\06\00\00\00\01\00\00\00H\05\00\00\06\00\00\00\01\00\00\00I\05\00\00\15\00\00\00\01\00\00\00I\05\00\00\0e\00\00\00\01\00\00\00I\05\00\00%\00\00\00\01\00\00\00I\05\00\00\0c\00\00\00")
  (data $s3 (;6;) "num/kernels")
  (data $s4 (;7;) "num/math")
  (data $s5 (;8;) "Io")
  (data $s6 (;9;) "Split")
  (data $s7 (;10;) "Path")
  (data $s8 (;11;) "State")
  (data $s9 (;12;) "Generator")
  (data $s10 (;13;) "InvalidBound")
  (data $s11 (;14;) "InvalidPath")
  (data $s12 (;15;) "Fixed")
  (data $s13 (;16;) "Double")
  (data $s14 (;17;) "Reduced")
  (data $s15 (;18;) "Sine")
  (data $s16 (;19;) "Cosine")
  (data $s17 (;20;) "Tangent")
  (data $s18 (;21;) "Arcsine")
  (data $s19 (;22;) "Arccosine")
  (data $s20 (;23;) "Arctangent")
  (data $s21 (;24;) "Angle")
  (data $s22 (;25;) "UnitsIn")
  (data $s23 (;26;) "UnitsOut")
  (data $s24 (;27;) "Date")
  (data $s25 (;28;) "TimeOfDay")
  (data $s26 (;29;) "Offset")
  (data $s27 (;30;) "Civil")
  (data $s28 (;31;) "Monday")
  (data $s29 (;32;) "Tuesday")
  (data $s30 (;33;) "Wednesday")
  (data $s31 (;34;) "Thursday")
  (data $s32 (;35;) "Friday")
  (data $s33 (;36;) "Saturday")
  (data $s34 (;37;) "Sunday")
  (data $s35 (;38;) "BadMonth")
  (data $s36 (;39;) "BadDay")
  (data $s37 (;40;) "BadHour")
  (data $s38 (;41;) "BadMinute")
  (data $s39 (;42;) "BadSecond")
  (data $s40 (;43;) "BadNanosecond")
  (data $s41 (;44;) "BadOffset")
  (data $s42 (;45;) "BadText")
  (data $s43 (;46;) "Names")
  (data $s44 (;47;) "InvalidName")
  (data $s45 (;48;) "DuplicateName")
  (data $s46 (;49;) "MisplacedRest")
  (data $s47 (;50;) "BadDefinition")
  (data $s48 (;51;) "UnknownOption")
  (data $s49 (;52;) "MissingValue")
  (data $s50 (;53;) "UnexpectedValue")
  (data $s51 (;54;) "RepeatedOption")
  (data $s52 (;55;) "MissingArgument")
  (data $s53 (;56;) "InvalidArgument")
  (data $s54 (;57;) "ExtraArgument")
  (data $s55 (;58;) "Token")
  (data $s56 (;59;) "Flag")
  (data $s57 (;60;) "Required")
  (data $s58 (;61;) "Optional")
  (data $s59 (;62;) "Repeated")
  (data $s60 (;63;) "Positional")
  (data $s61 (;64;) "Remaining")
  (data $s62 (;65;) "Field")
  (data $s63 (;66;) "Parser")
  (data $s64 (;67;) "Dict")
  (data $s65 (;68;) "Empty")
  (data $s66 (;69;) "One")
  (data $s67 (;70;) "Many")
  (data $s68 (;71;) "Branch")
  (data $s69 (;72;) "Continue")
  (data $s70 (;73;) "Stop")
  (data $s71 (;74;) "Debug")
  (data $s72 (;75;) "Info")
  (data $s73 (;76;) "Warning")
  (data $s74 (;77;) "Failure")
  (data $s75 (;78;) "Entry")
  (data $s76 (;79;) "Sink")
  (data $s77 (;80;) "Some")
  (data $s78 (;81;) "None")
  (data $s79 (;82;) "Less")
  (data $s80 (;83;) "Equal")
  (data $s81 (;84;) "Greater")
  (data $s82 (;85;) "DifferentRoots")
  (data $s83 (;86;) "UnknownParent")
  (data $s84 (;87;) "Gen")
  (data $s85 (;88;) "Tree")
  (data $s86 (;89;) "Config")
  (data $s87 (;90;) "InvalidRange")
  (data $s88 (;91;) "InvalidLength")
  (data $s89 (;92;) "EmptyChoice")
  (data $s90 (;93;) "Pass")
  (data $s91 (;94;) "Fail")
  (data $s92 (;95;) "Discard")
  (data $s93 (;96;) "Passed")
  (data $s94 (;97;) "GaveUp")
  (data $s95 (;98;) "Falsified")
  (data $s96 (;99;) "InvalidConfig")
  (data $s97 (;100;) "Sequence")
  (data $s98 (;101;) "Leaf")
  (data $s99 (;102;) "Branch2")
  (data $s100 (;103;) "Branch3")
  (data $s101 (;104;) "EmptyTree")
  (data $s102 (;105;) "Single")
  (data $s103 (;106;) "Deep")
  (data $s104 (;107;) "Set")
  (data $s105 (;108;) "Stream")
  (data $s106 (;109;) "Done")
  (data $s107 (;110;) "Next")
  (data $s108 (;111;) "Duration")
  (data $s109 (;112;) "Instant")
  (data $s110 (;113;) "DivisionByZero")
  (data $s111 (;114;) "Uri")
  (data $s112 (;115;) "Parts")
  (data $s113 (;116;) "Authority")
  (data $s114 (;117;) "RegName")
  (data $s115 (;118;) "IpLiteral")
  (data $s116 (;119;) "WholePart")
  (data $s117 (;120;) "SchemePart")
  (data $s118 (;121;) "AuthorityPart")
  (data $s119 (;122;) "UserInfoPart")
  (data $s120 (;123;) "HostPart")
  (data $s121 (;124;) "PortPart")
  (data $s122 (;125;) "PathPart")
  (data $s123 (;126;) "QueryPart")
  (data $s124 (;127;) "FragmentPart")
  (data $s125 (;128;) "BadCharacter")
  (data $s126 (;129;) "BadPercent")
  (data $s127 (;130;) "BadScheme")
  (data $s128 (;131;) "BadAuthority")
  (data $s129 (;132;) "BadHost")
  (data $s130 (;133;) "BadPort")
  (data $s131 (;134;) "BadPath")
  (data $s132 (;135;) "RelativeBase")
  (data $s133 (;136;) "UriError")
  (data $s134 (;137;) "BadEscape")
  (data $s135 (;138;) "NonAscii")
  (data $s136 (;139;) "InvalidUtf8")
  (data $s137 (;140;) "UnalignedBits")
  (@custom "ilang.abi" (after data) "{\22entry\22:null,\22exports\22:{\22num/kernels.closures\22:{\22async\22:false,\22errors\22:[],\22params\22:[\22float\22],\22result\22:\22float\22},\22num/kernels.dot\22:{\22async\22:false,\22errors\22:[],\22params\22:[\22float\22],\22result\22:\22float\22},\22num/kernels.fast\22:{\22async\22:false,\22errors\22:[],\22params\22:[\22float\22],\22result\22:\22float\22},\22num/kernels.horner\22:{\22async\22:false,\22errors\22:[],\22params\22:[\22float\22],\22result\22:\22float\22},\22num/kernels.identity\22:{\22async\22:false,\22errors\22:[],\22params\22:[\22float\22],\22result\22:\22float\22},\22num/kernels.integer\22:{\22async\22:false,\22errors\22:[],\22params\22:[\22float\22],\22result\22:\22float\22},\22num/kernels.tuples\22:{\22async\22:false,\22errors\22:[],\22params\22:[\22float\22],\22result\22:\22float\22}},\22externals\22:{},\22services\22:{},\22tests\22:[],\22version\22:3}")
  (func (export "counter_0") (result i64) global.get $prof_0)
  (func (export "counter_1") (result i64) global.get $prof_1)
  (func (export "counter_2") (result i64) global.get $prof_2)
  (func (export "counter_3") (result i64) global.get $prof_3)
  (func (export "counter_4") (result i64) global.get $prof_4)
  (func (export "counter_5") (result i64) global.get $prof_5)
  (func (export "counter_6") (result i64) global.get $prof_6)
  (func (export "counter_7") (result i64) global.get $prof_7)
  (func (export "counter_8") (result i64) global.get $prof_8)
  (func (export "counter_9") (result i64) global.get $prof_9)
  (func (export "counter_10") (result i64) global.get $prof_10)
  (func (export "counter_11") (result i64) global.get $prof_11)
  (func (export "counter_12") (result i64) global.get $prof_12)
  (func (export "counter_13") (result i64) global.get $prof_13)
  (func (export "counter_14") (result i64) global.get $prof_14)
  (func (export "counter_15") (result i64) global.get $prof_15)
  (func (export "counter_16") (result i64) global.get $prof_16)
  (func (export "counter_17") (result i64) global.get $prof_17)
  (func (export "counter_18") (result i64) global.get $prof_18)
  (func (export "counter_19") (result i64) global.get $prof_19)
  (func (export "counter_20") (result i64) global.get $prof_20)
  (func (export "counter_21") (result i64) global.get $prof_21)
  (func (export "counter_22") (result i64) global.get $prof_22)
  (func (export "counter_23") (result i64) global.get $prof_23)
  (func (export "counter_24") (result i64) global.get $prof_24)
  (func (export "counter_25") (result i64) global.get $prof_25)
  (func (export "counter_26") (result i64) global.get $prof_26)
  (func (export "counter_27") (result i64) global.get $prof_27)
  (func (export "counter_28") (result i64) global.get $prof_28)
  (func (export "counter_29") (result i64) global.get $prof_29)
  (func (export "counter_30") (result i64) global.get $prof_30)
  (func (export "counter_31") (result i64) global.get $prof_31)
  (func (export "counter_32") (result i64) global.get $prof_32)
  (func (export "counter_33") (result i64) global.get $prof_33)
  (func (export "counter_34") (result i64) global.get $prof_34)
  (func (export "counter_35") (result i64) global.get $prof_35)
  (func (export "counter_36") (result i64) global.get $prof_36)
  (func (export "counter_37") (result i64) global.get $prof_37)
  (func (export "counter_38") (result i64) global.get $prof_38)
  (func (export "counter_39") (result i64) global.get $prof_39)
  (func (export "counter_40") (result i64) global.get $prof_40)
  (func (export "counter_41") (result i64) global.get $prof_41)
  (func (export "counter_42") (result i64) global.get $prof_42)
  (func (export "counter_43") (result i64) global.get $prof_43)
  (func (export "counter_44") (result i64) global.get $prof_44)
  (func (export "counter_45") (result i64) global.get $prof_45)
  (func (export "counter_46") (result i64) global.get $prof_46)
  (func (export "counter_47") (result i64) global.get $prof_47)
  (func (export "counter_48") (result i64) global.get $prof_48)
  (func (export "counter_49") (result i64) global.get $prof_49)
  (func (export "counter_50") (result i64) global.get $prof_50)
  (func (export "counter_51") (result i64) global.get $prof_51)
  (func (export "counter_52") (result i64) global.get $prof_52)
  (func (export "counter_53") (result i64) global.get $prof_53)
  (func (export "counter_54") (result i64) global.get $prof_54)
  (func (export "counter_55") (result i64) global.get $prof_55)
  (func (export "counter_56") (result i64) global.get $prof_56)
  (func (export "counter_57") (result i64) global.get $prof_57)
  (func (export "counter_58") (result i64) global.get $prof_58)
  (func (export "counter_59") (result i64) global.get $prof_59)
  (func (export "counter_60") (result i64) global.get $prof_60)
  (func (export "counter_61") (result i64) global.get $prof_61)
  (func (export "counter_62") (result i64) global.get $prof_62)
  (func (export "counter_63") (result i64) global.get $prof_63)
  (func (export "counter_64") (result i64) global.get $prof_64)
  (func (export "counter_65") (result i64) global.get $prof_65)
  (func (export "counter_66") (result i64) global.get $prof_66)
  (func (export "counter_67") (result i64) global.get $prof_67)
  (func (export "counter_68") (result i64) global.get $prof_68)
  (func (export "counter_69") (result i64) global.get $prof_69)
  (func (export "counter_70") (result i64) global.get $prof_70)
  (func (export "counter_71") (result i64) global.get $prof_71)
  (func (export "counter_72") (result i64) global.get $prof_72)
  (func (export "counter_73") (result i64) global.get $prof_73)
  (func (export "counter_74") (result i64) global.get $prof_74)
  (func (export "counter_75") (result i64) global.get $prof_75)
  (func (export "counter_76") (result i64) global.get $prof_76)
  (func (export "counter_77") (result i64) global.get $prof_77)
  (func (export "counter_78") (result i64) global.get $prof_78)
  (func (export "counter_79") (result i64) global.get $prof_79)
  (func (export "counter_80") (result i64) global.get $prof_80)
  (func (export "counter_81") (result i64) global.get $prof_81)
  (func (export "counter_82") (result i64) global.get $prof_82)
  (func (export "counter_83") (result i64) global.get $prof_83)
  (func (export "counter_84") (result i64) global.get $prof_84)
  (func (export "counter_85") (result i64) global.get $prof_85)
  (func (export "counter_86") (result i64) global.get $prof_86)
  (func (export "counter_87") (result i64) global.get $prof_87)
  (func (export "counter_88") (result i64) global.get $prof_88)
  (func (export "counter_89") (result i64) global.get $prof_89)
  (func (export "counter_90") (result i64) global.get $prof_90)
  (func (export "counter_91") (result i64) global.get $prof_91)
  (func (export "counter_92") (result i64) global.get $prof_92)
  (func (export "counter_93") (result i64) global.get $prof_93)
  (func (export "counter_94") (result i64) global.get $prof_94)
  (func (export "counter_95") (result i64) global.get $prof_95)
  (func (export "counter_96") (result i64) global.get $prof_96)
  (func (export "counter_97") (result i64) global.get $prof_97)
  (func (export "counter_98") (result i64) global.get $prof_98)
  (func (export "counter_99") (result i64) global.get $prof_99)
  (func (export "counter_100") (result i64) global.get $prof_100)
  (func (export "counter_101") (result i64) global.get $prof_101)
  (func (export "counter_102") (result i64) global.get $prof_102)
  (func (export "counter_103") (result i64) global.get $prof_103)
  (func (export "counter_104") (result i64) global.get $prof_104)
  (func (export "counter_105") (result i64) global.get $prof_105)
  (func (export "counter_106") (result i64) global.get $prof_106)
  (func (export "counter_107") (result i64) global.get $prof_107)
  (func (export "counter_108") (result i64) global.get $prof_108)
  (func (export "counter_109") (result i64) global.get $prof_109)
  (func (export "counter_110") (result i64) global.get $prof_110)
  (func (export "counter_111") (result i64) global.get $prof_111)
  (func (export "counter_112") (result i64) global.get $prof_112)
  (func (export "counter_113") (result i64) global.get $prof_113)
  (func (export "counter_114") (result i64) global.get $prof_114)
  (func (export "counter_115") (result i64) global.get $prof_115)
  (func (export "counter_116") (result i64) global.get $prof_116)
  (func (export "counter_117") (result i64) global.get $prof_117)
  (func (export "counter_118") (result i64) global.get $prof_118)
  (func (export "counter_119") (result i64) global.get $prof_119)
  (func (export "counter_120") (result i64) global.get $prof_120)
  (func (export "counter_121") (result i64) global.get $prof_121)
  (func (export "counter_122") (result i64) global.get $prof_122)
  (func (export "counter_123") (result i64) global.get $prof_123)
  (func (export "counter_124") (result i64) global.get $prof_124)
  (func (export "counter_125") (result i64) global.get $prof_125)
  (func (export "counter_126") (result i64) global.get $prof_126)
  (func (export "counter_127") (result i64) global.get $prof_127)
  (func (export "counter_128") (result i64) global.get $prof_128)
  (func (export "counter_129") (result i64) global.get $prof_129)
  (func (export "counter_130") (result i64) global.get $prof_130)
  (func (export "counter_131") (result i64) global.get $prof_131)
  (func (export "counter_132") (result i64) global.get $prof_132)
  (func (export "counter_133") (result i64) global.get $prof_133)
  (func (export "counter_134") (result i64) global.get $prof_134)
  (func (export "counter_135") (result i64) global.get $prof_135)
  (func (export "counter_136") (result i64) global.get $prof_136)
  (func (export "counter_137") (result i64) global.get $prof_137)
  (func (export "counter_138") (result i64) global.get $prof_138)
  (func (export "counter_139") (result i64) global.get $prof_139)
  (func (export "counter_140") (result i64) global.get $prof_140)
  (func (export "counter_141") (result i64) global.get $prof_141)
  (func (export "counter_142") (result i64) global.get $prof_142)
  (func (export "counter_143") (result i64) global.get $prof_143)
  (func (export "counter_144") (result i64) global.get $prof_144)
  (func (export "counter_145") (result i64) global.get $prof_145)
  (func (export "counter_146") (result i64) global.get $prof_146)
  (func (export "counter_147") (result i64) global.get $prof_147)
  (func (export "counter_148") (result i64) global.get $prof_148)
  (func (export "counter_149") (result i64) global.get $prof_149)
  (func (export "counter_150") (result i64) global.get $prof_150)
  (func (export "counter_151") (result i64) global.get $prof_151)
  (func (export "counter_152") (result i64) global.get $prof_152)
  (func (export "counter_153") (result i64) global.get $prof_153)
  (func (export "counter_154") (result i64) global.get $prof_154)
  (func (export "counter_155") (result i64) global.get $prof_155)
  (func (export "counter_156") (result i64) global.get $prof_156)
  (func (export "counter_157") (result i64) global.get $prof_157)
  (func (export "counter_158") (result i64) global.get $prof_158)
  (func (export "counter_159") (result i64) global.get $prof_159)
  (func (export "counter_160") (result i64) global.get $prof_160)
  (func (export "counter_161") (result i64) global.get $prof_161)
  (func (export "counter_162") (result i64) global.get $prof_162)
  (func (export "counter_163") (result i64) global.get $prof_163)
  (func (export "counter_164") (result i64) global.get $prof_164)
  (func (export "counter_165") (result i64) global.get $prof_165)
  (func (export "counter_166") (result i64) global.get $prof_166)
  (func (export "counter_167") (result i64) global.get $prof_167)
  (func (export "counter_168") (result i64) global.get $prof_168)
  (func (export "counter_169") (result i64) global.get $prof_169)
  (func (export "counter_170") (result i64) global.get $prof_170)
  (func (export "counter_171") (result i64) global.get $prof_171)
  (func (export "counter_172") (result i64) global.get $prof_172)
  (func (export "counter_173") (result i64) global.get $prof_173)
  (func (export "counter_174") (result i64) global.get $prof_174)
  (func (export "counter_175") (result i64) global.get $prof_175)
  (func (export "counter_176") (result i64) global.get $prof_176)
  (func (export "counter_177") (result i64) global.get $prof_177)
  (func (export "counter_178") (result i64) global.get $prof_178)
  (func (export "counter_179") (result i64) global.get $prof_179)
  (func (export "counter_180") (result i64) global.get $prof_180)
  (func (export "counter_181") (result i64) global.get $prof_181)
  (func (export "counter_182") (result i64) global.get $prof_182)
  (func (export "counter_183") (result i64) global.get $prof_183)
  (func (export "counter_184") (result i64) global.get $prof_184)
  (func (export "counter_185") (result i64) global.get $prof_185)
  (func (export "counter_186") (result i64) global.get $prof_186)
  (func (export "counter_187") (result i64) global.get $prof_187)
  (func (export "counter_188") (result i64) global.get $prof_188)
  (func (export "counter_189") (result i64) global.get $prof_189)
  (func (export "counter_190") (result i64) global.get $prof_190)
  (func (export "counter_191") (result i64) global.get $prof_191)
  (func (export "counter_192") (result i64) global.get $prof_192)
  (func (export "counter_193") (result i64) global.get $prof_193)
  (func (export "counter_194") (result i64) global.get $prof_194)
  (func (export "counter_195") (result i64) global.get $prof_195)
  (func (export "counter_196") (result i64) global.get $prof_196)
  (func (export "counter_197") (result i64) global.get $prof_197)
  (func (export "counter_198") (result i64) global.get $prof_198)
  (func (export "counter_199") (result i64) global.get $prof_199)
  (func (export "counter_200") (result i64) global.get $prof_200)
  (func (export "counter_201") (result i64) global.get $prof_201)
  (func (export "counter_202") (result i64) global.get $prof_202)
  (func (export "counter_203") (result i64) global.get $prof_203)
  (func (export "counter_204") (result i64) global.get $prof_204)
  (func (export "counter_205") (result i64) global.get $prof_205)
  (func (export "counter_206") (result i64) global.get $prof_206)
  (func (export "counter_207") (result i64) global.get $prof_207)
  (func (export "counter_208") (result i64) global.get $prof_208)
  (func (export "counter_209") (result i64) global.get $prof_209)
  (func (export "counter_210") (result i64) global.get $prof_210)
  (func (export "counter_211") (result i64) global.get $prof_211)
  (func (export "counter_212") (result i64) global.get $prof_212)
  (func (export "counter_213") (result i64) global.get $prof_213)
  (func (export "counter_214") (result i64) global.get $prof_214)
  (func (export "counter_215") (result i64) global.get $prof_215)
  (func (export "counter_216") (result i64) global.get $prof_216)
  (func (export "counter_217") (result i64) global.get $prof_217)
  (func (export "counter_218") (result i64) global.get $prof_218)
  (func (export "counter_219") (result i64) global.get $prof_219)
  (func (export "counter_220") (result i64) global.get $prof_220)
  (func (export "counter_221") (result i64) global.get $prof_221)
  (func (export "counter_222") (result i64) global.get $prof_222)
  (func (export "counter_223") (result i64) global.get $prof_223)
  (func (export "counter_224") (result i64) global.get $prof_224)
  (func (export "counter_225") (result i64) global.get $prof_225)
  (func (export "counter_226") (result i64) global.get $prof_226)
  (func (export "counter_227") (result i64) global.get $prof_227)
  (func (export "counter_228") (result i64) global.get $prof_228)
  (func (export "counter_229") (result i64) global.get $prof_229)
  (func (export "counter_230") (result i64) global.get $prof_230)
  (func (export "counter_231") (result i64) global.get $prof_231)
  (func (export "counter_232") (result i64) global.get $prof_232)
  (func (export "counter_233") (result i64) global.get $prof_233)
  (func (export "counter_234") (result i64) global.get $prof_234)
  (func (export "counter_235") (result i64) global.get $prof_235)
  (func (export "counter_236") (result i64) global.get $prof_236)
  (func (export "counter_237") (result i64) global.get $prof_237)
  (func (export "counter_238") (result i64) global.get $prof_238)
  (func (export "counter_239") (result i64) global.get $prof_239)
  (func (export "counter_240") (result i64) global.get $prof_240)
  (func (export "counter_241") (result i64) global.get $prof_241)
  (func (export "counter_242") (result i64) global.get $prof_242)
  (func (export "counter_243") (result i64) global.get $prof_243)
  (func (export "counter_244") (result i64) global.get $prof_244)
  (func (export "counter_245") (result i64) global.get $prof_245)
  (func (export "counter_246") (result i64) global.get $prof_246)
  (func (export "counter_247") (result i64) global.get $prof_247)
  (func (export "counter_248") (result i64) global.get $prof_248)
  (func (export "counter_249") (result i64) global.get $prof_249)
  (func (export "counter_250") (result i64) global.get $prof_250)
  (func (export "counter_251") (result i64) global.get $prof_251)
  (func (export "counter_252") (result i64) global.get $prof_252)
  (func (export "counter_253") (result i64) global.get $prof_253)
  (func (export "counter_254") (result i64) global.get $prof_254)
  (func (export "counter_255") (result i64) global.get $prof_255)
  (func (export "counter_256") (result i64) global.get $prof_256)
  (func (export "counter_257") (result i64) global.get $prof_257)
  (func (export "counter_258") (result i64) global.get $prof_258)
  (func (export "counter_259") (result i64) global.get $prof_259)
  (func (export "counter_260") (result i64) global.get $prof_260)
  (func (export "counter_261") (result i64) global.get $prof_261)
  (func (export "counter_262") (result i64) global.get $prof_262)
  (func (export "counter_263") (result i64) global.get $prof_263)
  (func (export "counter_264") (result i64) global.get $prof_264)
  (func (export "counter_265") (result i64) global.get $prof_265)
  (func (export "counter_266") (result i64) global.get $prof_266)
  (func (export "counter_267") (result i64) global.get $prof_267)
  (func (export "counter_268") (result i64) global.get $prof_268)
  (func (export "counter_269") (result i64) global.get $prof_269)
  (func (export "counter_270") (result i64) global.get $prof_270)
  (func (export "counter_271") (result i64) global.get $prof_271)
  (func (export "counter_272") (result i64) global.get $prof_272)
  (func (export "counter_273") (result i64) global.get $prof_273)
  (func (export "counter_274") (result i64) global.get $prof_274)
  (func (export "counter_275") (result i64) global.get $prof_275)
  (func (export "counter_276") (result i64) global.get $prof_276)
  (func (export "counter_277") (result i64) global.get $prof_277)
  (func (export "counter_278") (result i64) global.get $prof_278)
  (func (export "counter_279") (result i64) global.get $prof_279)
  (func (export "counter_280") (result i64) global.get $prof_280)
  (func (export "counter_281") (result i64) global.get $prof_281)
  (func (export "counter_282") (result i64) global.get $prof_282)
  (func (export "counter_283") (result i64) global.get $prof_283)
  (func (export "counter_284") (result i64) global.get $prof_284)
  (func (export "counter_285") (result i64) global.get $prof_285)
  (func (export "counter_286") (result i64) global.get $prof_286)
  (func (export "counter_287") (result i64) global.get $prof_287)
  (func (export "counter_288") (result i64) global.get $prof_288)
  (func (export "counter_289") (result i64) global.get $prof_289)
  (func (export "counter_290") (result i64) global.get $prof_290)
  (func (export "counter_291") (result i64) global.get $prof_291)
  (func (export "counter_292") (result i64) global.get $prof_292)
  (func (export "counter_293") (result i64) global.get $prof_293)
  (func (export "counter_294") (result i64) global.get $prof_294)
  (func (export "counter_295") (result i64) global.get $prof_295)
  (func (export "counter_296") (result i64) global.get $prof_296)
  (func (export "counter_297") (result i64) global.get $prof_297)
  (func (export "counter_298") (result i64) global.get $prof_298)
  (func (export "counter_299") (result i64) global.get $prof_299)
  (func (export "counter_300") (result i64) global.get $prof_300)
  (func (export "counter_301") (result i64) global.get $prof_301)
  (func (export "counter_302") (result i64) global.get $prof_302)
  (func (export "counter_303") (result i64) global.get $prof_303)
  (func (export "counter_304") (result i64) global.get $prof_304)
  (func (export "counter_305") (result i64) global.get $prof_305)
  (func (export "counter_306") (result i64) global.get $prof_306)
  (func (export "counter_307") (result i64) global.get $prof_307)
  (func (export "counter_308") (result i64) global.get $prof_308)
  (func (export "counter_309") (result i64) global.get $prof_309)
  (func (export "counter_310") (result i64) global.get $prof_310)
  (func (export "counter_311") (result i64) global.get $prof_311)
  (func (export "counter_312") (result i64) global.get $prof_312)
  (func (export "counter_313") (result i64) global.get $prof_313)
  (func (export "counter_314") (result i64) global.get $prof_314)
  (func (export "counter_315") (result i64) global.get $prof_315)
  (func (export "counter_316") (result i64) global.get $prof_316)
  (func (export "counter_317") (result i64) global.get $prof_317)
  (func (export "counter_318") (result i64) global.get $prof_318)
  (func (export "counter_319") (result i64) global.get $prof_319)
  (func (export "counter_320") (result i64) global.get $prof_320)
  (func (export "counter_321") (result i64) global.get $prof_321)
  (func (export "counter_322") (result i64) global.get $prof_322)
  (func (export "counter_323") (result i64) global.get $prof_323)
  (func (export "counter_324") (result i64) global.get $prof_324)
  (func (export "counter_325") (result i64) global.get $prof_325)
  (func (export "counter_326") (result i64) global.get $prof_326)
  (func (export "counter_327") (result i64) global.get $prof_327)
  (func (export "counter_328") (result i64) global.get $prof_328)
  (func (export "counter_329") (result i64) global.get $prof_329)
  (func (export "counter_330") (result i64) global.get $prof_330)
  (func (export "counter_331") (result i64) global.get $prof_331)
  (func (export "counter_332") (result i64) global.get $prof_332)
  (func (export "counter_333") (result i64) global.get $prof_333)
  (func (export "counter_334") (result i64) global.get $prof_334)
  (func (export "counter_335") (result i64) global.get $prof_335)
  (func (export "counter_336") (result i64) global.get $prof_336)
  (func (export "counter_337") (result i64) global.get $prof_337)
  (func (export "counter_338") (result i64) global.get $prof_338)
  (func (export "counter_339") (result i64) global.get $prof_339)
  (func (export "counter_340") (result i64) global.get $prof_340)
  (func (export "counter_341") (result i64) global.get $prof_341)
  (func (export "counter_342") (result i64) global.get $prof_342)
  (func (export "counter_343") (result i64) global.get $prof_343)
  (func (export "counter_344") (result i64) global.get $prof_344)
  (func (export "counter_345") (result i64) global.get $prof_345)
  (func (export "counter_346") (result i64) global.get $prof_346)
  (func (export "counter_347") (result i64) global.get $prof_347)
  (func (export "counter_348") (result i64) global.get $prof_348)
  (func (export "counter_349") (result i64) global.get $prof_349)
  (func (export "counter_350") (result i64) global.get $prof_350)
  (func (export "counter_351") (result i64) global.get $prof_351)
  (func (export "counter_352") (result i64) global.get $prof_352)
  (func (export "counter_353") (result i64) global.get $prof_353)
  (func (export "counter_354") (result i64) global.get $prof_354)
  (func (export "counter_355") (result i64) global.get $prof_355)
  (func (export "counter_356") (result i64) global.get $prof_356)
  (func (export "counter_357") (result i64) global.get $prof_357)
  (func (export "counter_358") (result i64) global.get $prof_358)
  (func (export "counter_359") (result i64) global.get $prof_359)
  (func (export "counter_360") (result i64) global.get $prof_360)
  (func (export "counter_361") (result i64) global.get $prof_361)
  (func (export "counter_362") (result i64) global.get $prof_362)
  (func (export "counter_363") (result i64) global.get $prof_363)
  (func (export "counter_364") (result i64) global.get $prof_364)
  (func (export "counter_365") (result i64) global.get $prof_365)
  (func (export "counter_366") (result i64) global.get $prof_366)
  (func (export "counter_367") (result i64) global.get $prof_367)
  (func (export "counter_368") (result i64) global.get $prof_368)
  (func (export "counter_369") (result i64) global.get $prof_369)
  (func (export "counter_370") (result i64) global.get $prof_370)
  (func (export "counter_371") (result i64) global.get $prof_371)
  (func (export "counter_372") (result i64) global.get $prof_372)
  (func (export "counter_373") (result i64) global.get $prof_373)
  (func (export "counter_374") (result i64) global.get $prof_374)
  (func (export "counter_375") (result i64) global.get $prof_375)
  (func (export "counter_376") (result i64) global.get $prof_376)
  (func (export "counter_377") (result i64) global.get $prof_377)
  (func (export "counter_378") (result i64) global.get $prof_378)
  (func (export "counter_379") (result i64) global.get $prof_379)
  (func (export "counter_380") (result i64) global.get $prof_380)
  (func (export "counter_381") (result i64) global.get $prof_381)
  (func (export "counter_382") (result i64) global.get $prof_382)
  (func (export "counter_383") (result i64) global.get $prof_383)
  (func (export "counter_384") (result i64) global.get $prof_384)
  (func (export "counter_385") (result i64) global.get $prof_385)
  (func (export "counter_386") (result i64) global.get $prof_386)
  (func (export "counter_387") (result i64) global.get $prof_387)
  (func (export "counter_388") (result i64) global.get $prof_388)
  (func (export "counter_389") (result i64) global.get $prof_389)
  (func (export "counter_390") (result i64) global.get $prof_390)
  (func (export "counter_391") (result i64) global.get $prof_391)
  (func (export "counter_392") (result i64) global.get $prof_392)
  (func (export "counter_393") (result i64) global.get $prof_393)
  (func (export "counter_394") (result i64) global.get $prof_394)
  (func (export "counter_395") (result i64) global.get $prof_395)
  (func (export "counter_396") (result i64) global.get $prof_396)
  (func (export "counter_397") (result i64) global.get $prof_397)
  (func (export "counter_398") (result i64) global.get $prof_398)
  (func (export "counter_399") (result i64) global.get $prof_399)
  (func (export "counter_400") (result i64) global.get $prof_400)
  (func (export "counter_401") (result i64) global.get $prof_401)
  (func (export "counter_402") (result i64) global.get $prof_402)
  (func (export "counter_403") (result i64) global.get $prof_403)
  (func (export "counter_404") (result i64) global.get $prof_404)
  (func (export "counter_405") (result i64) global.get $prof_405)
  (func (export "counter_406") (result i64) global.get $prof_406)
  (func (export "counter_407") (result i64) global.get $prof_407)
  (func (export "counter_408") (result i64) global.get $prof_408)
  (func (export "counter_409") (result i64) global.get $prof_409)
  (func (export "counter_410") (result i64) global.get $prof_410)
  (func (export "counter_411") (result i64) global.get $prof_411)
  (func (export "counter_412") (result i64) global.get $prof_412)
  (func (export "counter_413") (result i64) global.get $prof_413)
  (func (export "counter_414") (result i64) global.get $prof_414)
  (func (export "counter_415") (result i64) global.get $prof_415)
  (func (export "counter_416") (result i64) global.get $prof_416)
  (func (export "counter_417") (result i64) global.get $prof_417)
  (func (export "counter_418") (result i64) global.get $prof_418)
  (func (export "counter_419") (result i64) global.get $prof_419)
  (func (export "counter_420") (result i64) global.get $prof_420)
  (func (export "counter_421") (result i64) global.get $prof_421)
  (func (export "counter_422") (result i64) global.get $prof_422)
  (func (export "counter_423") (result i64) global.get $prof_423)
  (func (export "counter_424") (result i64) global.get $prof_424)
  (func (export "counter_425") (result i64) global.get $prof_425)
  (func (export "counter_426") (result i64) global.get $prof_426)
  (func (export "counter_427") (result i64) global.get $prof_427)
  (func (export "counter_428") (result i64) global.get $prof_428)
  (func (export "counter_429") (result i64) global.get $prof_429)
  (func (export "counter_430") (result i64) global.get $prof_430)
  (func (export "counter_431") (result i64) global.get $prof_431)
  (func (export "counter_432") (result i64) global.get $prof_432)
  (func (export "counter_433") (result i64) global.get $prof_433)
  (func (export "counter_434") (result i64) global.get $prof_434)
  (func (export "counter_435") (result i64) global.get $prof_435)
  (func (export "counter_436") (result i64) global.get $prof_436)
  (func (export "counter_437") (result i64) global.get $prof_437)
  (func (export "counter_438") (result i64) global.get $prof_438)
  (func (export "counter_439") (result i64) global.get $prof_439)
  (func (export "counter_440") (result i64) global.get $prof_440)
  (func (export "counter_441") (result i64) global.get $prof_441)
  (func (export "counter_442") (result i64) global.get $prof_442)
  (func (export "counter_443") (result i64) global.get $prof_443)
  (func (export "counter_444") (result i64) global.get $prof_444)
  (func (export "counter_445") (result i64) global.get $prof_445)
  (func (export "counter_446") (result i64) global.get $prof_446)
  (func (export "counter_447") (result i64) global.get $prof_447)
  (func (export "counter_448") (result i64) global.get $prof_448)
  (func (export "counter_449") (result i64) global.get $prof_449)
  (func (export "counter_450") (result i64) global.get $prof_450)
  (func (export "counter_451") (result i64) global.get $prof_451)
  (func (export "counter_452") (result i64) global.get $prof_452)
  (func (export "counter_453") (result i64) global.get $prof_453)
  (func (export "counter_454") (result i64) global.get $prof_454)
  (func (export "counter_455") (result i64) global.get $prof_455)
  (func (export "counter_456") (result i64) global.get $prof_456)
  (func (export "counter_457") (result i64) global.get $prof_457)
  (func (export "counter_458") (result i64) global.get $prof_458)
  (func (export "counter_459") (result i64) global.get $prof_459)
  (func (export "counter_460") (result i64) global.get $prof_460)
  (func (export "counter_461") (result i64) global.get $prof_461)
  (func (export "counter_462") (result i64) global.get $prof_462)
  (func (export "counter_463") (result i64) global.get $prof_463)
  (func (export "counter_464") (result i64) global.get $prof_464)
  (func (export "counter_465") (result i64) global.get $prof_465)
  (func (export "counter_466") (result i64) global.get $prof_466)
  (func (export "counter_467") (result i64) global.get $prof_467)
  (func (export "counter_468") (result i64) global.get $prof_468)
  (func (export "counter_469") (result i64) global.get $prof_469)
  (func (export "counter_470") (result i64) global.get $prof_470)
  (func (export "counter_471") (result i64) global.get $prof_471)
  (func (export "counter_472") (result i64) global.get $prof_472)
  (func (export "counter_473") (result i64) global.get $prof_473)
  (func (export "counter_474") (result i64) global.get $prof_474)
  (func (export "counter_475") (result i64) global.get $prof_475)
  (func (export "counter_476") (result i64) global.get $prof_476)
  (func (export "counter_477") (result i64) global.get $prof_477)
  (func (export "counter_478") (result i64) global.get $prof_478)
  (func (export "counter_479") (result i64) global.get $prof_479)
  (func (export "counter_480") (result i64) global.get $prof_480)
  (func (export "counter_481") (result i64) global.get $prof_481)
  (func (export "counter_482") (result i64) global.get $prof_482)
  (func (export "counter_483") (result i64) global.get $prof_483)
  (func (export "counter_484") (result i64) global.get $prof_484)
  (func (export "counter_485") (result i64) global.get $prof_485)
  (func (export "counter_486") (result i64) global.get $prof_486)
  (func (export "counter_487") (result i64) global.get $prof_487)
  (func (export "counter_488") (result i64) global.get $prof_488)
  (func (export "counter_489") (result i64) global.get $prof_489)
  (func (export "counter_490") (result i64) global.get $prof_490)
  (func (export "counter_491") (result i64) global.get $prof_491)
  (func (export "counter_492") (result i64) global.get $prof_492)
  (func (export "counter_493") (result i64) global.get $prof_493)
  (func (export "counter_494") (result i64) global.get $prof_494)
  (func (export "counter_495") (result i64) global.get $prof_495)
  (func (export "counter_496") (result i64) global.get $prof_496)
  (func (export "counter_497") (result i64) global.get $prof_497)
  (func (export "counter_498") (result i64) global.get $prof_498)
  (func (export "counter_499") (result i64) global.get $prof_499)
  (func (export "counter_500") (result i64) global.get $prof_500)
  (func (export "counter_501") (result i64) global.get $prof_501)
  (func (export "counter_502") (result i64) global.get $prof_502)
  (func (export "counter_503") (result i64) global.get $prof_503)
  (func (export "counter_504") (result i64) global.get $prof_504)
  (func (export "counter_505") (result i64) global.get $prof_505)
  (func (export "counter_506") (result i64) global.get $prof_506)
  (func (export "counter_507") (result i64) global.get $prof_507)
  (func (export "counter_508") (result i64) global.get $prof_508)
  (func (export "counter_509") (result i64) global.get $prof_509)
  (func (export "counter_510") (result i64) global.get $prof_510)
  (func (export "counter_511") (result i64) global.get $prof_511)
  (func (export "counter_512") (result i64) global.get $prof_512)
  (func (export "counter_513") (result i64) global.get $prof_513)
  (func (export "counter_514") (result i64) global.get $prof_514)
  (func (export "counter_515") (result i64) global.get $prof_515)
  (func (export "counter_516") (result i64) global.get $prof_516)
  (func (export "counter_517") (result i64) global.get $prof_517)
  (func (export "counter_518") (result i64) global.get $prof_518)
  (func (export "counter_519") (result i64) global.get $prof_519)
  (func (export "counter_520") (result i64) global.get $prof_520)
  (func (export "counter_521") (result i64) global.get $prof_521)
  (func (export "counter_522") (result i64) global.get $prof_522)
  (func (export "counter_523") (result i64) global.get $prof_523)
  (func (export "counter_524") (result i64) global.get $prof_524)
  (func (export "counter_525") (result i64) global.get $prof_525)
  (func (export "counter_526") (result i64) global.get $prof_526)
  (func (export "counter_527") (result i64) global.get $prof_527)
  (func (export "counter_528") (result i64) global.get $prof_528)
  (func (export "counter_529") (result i64) global.get $prof_529)
  (func (export "counter_530") (result i64) global.get $prof_530)
  (func (export "counter_531") (result i64) global.get $prof_531)
  (func (export "counter_532") (result i64) global.get $prof_532)
  (func (export "counter_533") (result i64) global.get $prof_533)
  (func (export "counter_534") (result i64) global.get $prof_534)
  (func (export "counter_535") (result i64) global.get $prof_535)
  (func (export "counter_536") (result i64) global.get $prof_536)
  (func (export "counter_537") (result i64) global.get $prof_537)
  (func (export "counter_538") (result i64) global.get $prof_538)
  (func (export "counter_539") (result i64) global.get $prof_539)
  (func (export "counter_540") (result i64) global.get $prof_540)
  (func (export "counter_541") (result i64) global.get $prof_541)
  (func (export "counter_542") (result i64) global.get $prof_542)
  (func (export "counter_543") (result i64) global.get $prof_543)
  (func (export "counter_544") (result i64) global.get $prof_544)
  (func (export "counter_545") (result i64) global.get $prof_545)
  (func (export "counter_546") (result i64) global.get $prof_546)
  (func (export "counter_547") (result i64) global.get $prof_547)
  (func (export "counter_548") (result i64) global.get $prof_548)
  (func (export "counter_549") (result i64) global.get $prof_549)
  (func (export "counter_550") (result i64) global.get $prof_550)
  (func (export "reset")
    i64.const 0
    global.set $prof_0
    i64.const 0
    global.set $prof_1
    i64.const 0
    global.set $prof_2
    i64.const 0
    global.set $prof_3
    i64.const 0
    global.set $prof_4
    i64.const 0
    global.set $prof_5
    i64.const 0
    global.set $prof_6
    i64.const 0
    global.set $prof_7
    i64.const 0
    global.set $prof_8
    i64.const 0
    global.set $prof_9
    i64.const 0
    global.set $prof_10
    i64.const 0
    global.set $prof_11
    i64.const 0
    global.set $prof_12
    i64.const 0
    global.set $prof_13
    i64.const 0
    global.set $prof_14
    i64.const 0
    global.set $prof_15
    i64.const 0
    global.set $prof_16
    i64.const 0
    global.set $prof_17
    i64.const 0
    global.set $prof_18
    i64.const 0
    global.set $prof_19
    i64.const 0
    global.set $prof_20
    i64.const 0
    global.set $prof_21
    i64.const 0
    global.set $prof_22
    i64.const 0
    global.set $prof_23
    i64.const 0
    global.set $prof_24
    i64.const 0
    global.set $prof_25
    i64.const 0
    global.set $prof_26
    i64.const 0
    global.set $prof_27
    i64.const 0
    global.set $prof_28
    i64.const 0
    global.set $prof_29
    i64.const 0
    global.set $prof_30
    i64.const 0
    global.set $prof_31
    i64.const 0
    global.set $prof_32
    i64.const 0
    global.set $prof_33
    i64.const 0
    global.set $prof_34
    i64.const 0
    global.set $prof_35
    i64.const 0
    global.set $prof_36
    i64.const 0
    global.set $prof_37
    i64.const 0
    global.set $prof_38
    i64.const 0
    global.set $prof_39
    i64.const 0
    global.set $prof_40
    i64.const 0
    global.set $prof_41
    i64.const 0
    global.set $prof_42
    i64.const 0
    global.set $prof_43
    i64.const 0
    global.set $prof_44
    i64.const 0
    global.set $prof_45
    i64.const 0
    global.set $prof_46
    i64.const 0
    global.set $prof_47
    i64.const 0
    global.set $prof_48
    i64.const 0
    global.set $prof_49
    i64.const 0
    global.set $prof_50
    i64.const 0
    global.set $prof_51
    i64.const 0
    global.set $prof_52
    i64.const 0
    global.set $prof_53
    i64.const 0
    global.set $prof_54
    i64.const 0
    global.set $prof_55
    i64.const 0
    global.set $prof_56
    i64.const 0
    global.set $prof_57
    i64.const 0
    global.set $prof_58
    i64.const 0
    global.set $prof_59
    i64.const 0
    global.set $prof_60
    i64.const 0
    global.set $prof_61
    i64.const 0
    global.set $prof_62
    i64.const 0
    global.set $prof_63
    i64.const 0
    global.set $prof_64
    i64.const 0
    global.set $prof_65
    i64.const 0
    global.set $prof_66
    i64.const 0
    global.set $prof_67
    i64.const 0
    global.set $prof_68
    i64.const 0
    global.set $prof_69
    i64.const 0
    global.set $prof_70
    i64.const 0
    global.set $prof_71
    i64.const 0
    global.set $prof_72
    i64.const 0
    global.set $prof_73
    i64.const 0
    global.set $prof_74
    i64.const 0
    global.set $prof_75
    i64.const 0
    global.set $prof_76
    i64.const 0
    global.set $prof_77
    i64.const 0
    global.set $prof_78
    i64.const 0
    global.set $prof_79
    i64.const 0
    global.set $prof_80
    i64.const 0
    global.set $prof_81
    i64.const 0
    global.set $prof_82
    i64.const 0
    global.set $prof_83
    i64.const 0
    global.set $prof_84
    i64.const 0
    global.set $prof_85
    i64.const 0
    global.set $prof_86
    i64.const 0
    global.set $prof_87
    i64.const 0
    global.set $prof_88
    i64.const 0
    global.set $prof_89
    i64.const 0
    global.set $prof_90
    i64.const 0
    global.set $prof_91
    i64.const 0
    global.set $prof_92
    i64.const 0
    global.set $prof_93
    i64.const 0
    global.set $prof_94
    i64.const 0
    global.set $prof_95
    i64.const 0
    global.set $prof_96
    i64.const 0
    global.set $prof_97
    i64.const 0
    global.set $prof_98
    i64.const 0
    global.set $prof_99
    i64.const 0
    global.set $prof_100
    i64.const 0
    global.set $prof_101
    i64.const 0
    global.set $prof_102
    i64.const 0
    global.set $prof_103
    i64.const 0
    global.set $prof_104
    i64.const 0
    global.set $prof_105
    i64.const 0
    global.set $prof_106
    i64.const 0
    global.set $prof_107
    i64.const 0
    global.set $prof_108
    i64.const 0
    global.set $prof_109
    i64.const 0
    global.set $prof_110
    i64.const 0
    global.set $prof_111
    i64.const 0
    global.set $prof_112
    i64.const 0
    global.set $prof_113
    i64.const 0
    global.set $prof_114
    i64.const 0
    global.set $prof_115
    i64.const 0
    global.set $prof_116
    i64.const 0
    global.set $prof_117
    i64.const 0
    global.set $prof_118
    i64.const 0
    global.set $prof_119
    i64.const 0
    global.set $prof_120
    i64.const 0
    global.set $prof_121
    i64.const 0
    global.set $prof_122
    i64.const 0
    global.set $prof_123
    i64.const 0
    global.set $prof_124
    i64.const 0
    global.set $prof_125
    i64.const 0
    global.set $prof_126
    i64.const 0
    global.set $prof_127
    i64.const 0
    global.set $prof_128
    i64.const 0
    global.set $prof_129
    i64.const 0
    global.set $prof_130
    i64.const 0
    global.set $prof_131
    i64.const 0
    global.set $prof_132
    i64.const 0
    global.set $prof_133
    i64.const 0
    global.set $prof_134
    i64.const 0
    global.set $prof_135
    i64.const 0
    global.set $prof_136
    i64.const 0
    global.set $prof_137
    i64.const 0
    global.set $prof_138
    i64.const 0
    global.set $prof_139
    i64.const 0
    global.set $prof_140
    i64.const 0
    global.set $prof_141
    i64.const 0
    global.set $prof_142
    i64.const 0
    global.set $prof_143
    i64.const 0
    global.set $prof_144
    i64.const 0
    global.set $prof_145
    i64.const 0
    global.set $prof_146
    i64.const 0
    global.set $prof_147
    i64.const 0
    global.set $prof_148
    i64.const 0
    global.set $prof_149
    i64.const 0
    global.set $prof_150
    i64.const 0
    global.set $prof_151
    i64.const 0
    global.set $prof_152
    i64.const 0
    global.set $prof_153
    i64.const 0
    global.set $prof_154
    i64.const 0
    global.set $prof_155
    i64.const 0
    global.set $prof_156
    i64.const 0
    global.set $prof_157
    i64.const 0
    global.set $prof_158
    i64.const 0
    global.set $prof_159
    i64.const 0
    global.set $prof_160
    i64.const 0
    global.set $prof_161
    i64.const 0
    global.set $prof_162
    i64.const 0
    global.set $prof_163
    i64.const 0
    global.set $prof_164
    i64.const 0
    global.set $prof_165
    i64.const 0
    global.set $prof_166
    i64.const 0
    global.set $prof_167
    i64.const 0
    global.set $prof_168
    i64.const 0
    global.set $prof_169
    i64.const 0
    global.set $prof_170
    i64.const 0
    global.set $prof_171
    i64.const 0
    global.set $prof_172
    i64.const 0
    global.set $prof_173
    i64.const 0
    global.set $prof_174
    i64.const 0
    global.set $prof_175
    i64.const 0
    global.set $prof_176
    i64.const 0
    global.set $prof_177
    i64.const 0
    global.set $prof_178
    i64.const 0
    global.set $prof_179
    i64.const 0
    global.set $prof_180
    i64.const 0
    global.set $prof_181
    i64.const 0
    global.set $prof_182
    i64.const 0
    global.set $prof_183
    i64.const 0
    global.set $prof_184
    i64.const 0
    global.set $prof_185
    i64.const 0
    global.set $prof_186
    i64.const 0
    global.set $prof_187
    i64.const 0
    global.set $prof_188
    i64.const 0
    global.set $prof_189
    i64.const 0
    global.set $prof_190
    i64.const 0
    global.set $prof_191
    i64.const 0
    global.set $prof_192
    i64.const 0
    global.set $prof_193
    i64.const 0
    global.set $prof_194
    i64.const 0
    global.set $prof_195
    i64.const 0
    global.set $prof_196
    i64.const 0
    global.set $prof_197
    i64.const 0
    global.set $prof_198
    i64.const 0
    global.set $prof_199
    i64.const 0
    global.set $prof_200
    i64.const 0
    global.set $prof_201
    i64.const 0
    global.set $prof_202
    i64.const 0
    global.set $prof_203
    i64.const 0
    global.set $prof_204
    i64.const 0
    global.set $prof_205
    i64.const 0
    global.set $prof_206
    i64.const 0
    global.set $prof_207
    i64.const 0
    global.set $prof_208
    i64.const 0
    global.set $prof_209
    i64.const 0
    global.set $prof_210
    i64.const 0
    global.set $prof_211
    i64.const 0
    global.set $prof_212
    i64.const 0
    global.set $prof_213
    i64.const 0
    global.set $prof_214
    i64.const 0
    global.set $prof_215
    i64.const 0
    global.set $prof_216
    i64.const 0
    global.set $prof_217
    i64.const 0
    global.set $prof_218
    i64.const 0
    global.set $prof_219
    i64.const 0
    global.set $prof_220
    i64.const 0
    global.set $prof_221
    i64.const 0
    global.set $prof_222
    i64.const 0
    global.set $prof_223
    i64.const 0
    global.set $prof_224
    i64.const 0
    global.set $prof_225
    i64.const 0
    global.set $prof_226
    i64.const 0
    global.set $prof_227
    i64.const 0
    global.set $prof_228
    i64.const 0
    global.set $prof_229
    i64.const 0
    global.set $prof_230
    i64.const 0
    global.set $prof_231
    i64.const 0
    global.set $prof_232
    i64.const 0
    global.set $prof_233
    i64.const 0
    global.set $prof_234
    i64.const 0
    global.set $prof_235
    i64.const 0
    global.set $prof_236
    i64.const 0
    global.set $prof_237
    i64.const 0
    global.set $prof_238
    i64.const 0
    global.set $prof_239
    i64.const 0
    global.set $prof_240
    i64.const 0
    global.set $prof_241
    i64.const 0
    global.set $prof_242
    i64.const 0
    global.set $prof_243
    i64.const 0
    global.set $prof_244
    i64.const 0
    global.set $prof_245
    i64.const 0
    global.set $prof_246
    i64.const 0
    global.set $prof_247
    i64.const 0
    global.set $prof_248
    i64.const 0
    global.set $prof_249
    i64.const 0
    global.set $prof_250
    i64.const 0
    global.set $prof_251
    i64.const 0
    global.set $prof_252
    i64.const 0
    global.set $prof_253
    i64.const 0
    global.set $prof_254
    i64.const 0
    global.set $prof_255
    i64.const 0
    global.set $prof_256
    i64.const 0
    global.set $prof_257
    i64.const 0
    global.set $prof_258
    i64.const 0
    global.set $prof_259
    i64.const 0
    global.set $prof_260
    i64.const 0
    global.set $prof_261
    i64.const 0
    global.set $prof_262
    i64.const 0
    global.set $prof_263
    i64.const 0
    global.set $prof_264
    i64.const 0
    global.set $prof_265
    i64.const 0
    global.set $prof_266
    i64.const 0
    global.set $prof_267
    i64.const 0
    global.set $prof_268
    i64.const 0
    global.set $prof_269
    i64.const 0
    global.set $prof_270
    i64.const 0
    global.set $prof_271
    i64.const 0
    global.set $prof_272
    i64.const 0
    global.set $prof_273
    i64.const 0
    global.set $prof_274
    i64.const 0
    global.set $prof_275
    i64.const 0
    global.set $prof_276
    i64.const 0
    global.set $prof_277
    i64.const 0
    global.set $prof_278
    i64.const 0
    global.set $prof_279
    i64.const 0
    global.set $prof_280
    i64.const 0
    global.set $prof_281
    i64.const 0
    global.set $prof_282
    i64.const 0
    global.set $prof_283
    i64.const 0
    global.set $prof_284
    i64.const 0
    global.set $prof_285
    i64.const 0
    global.set $prof_286
    i64.const 0
    global.set $prof_287
    i64.const 0
    global.set $prof_288
    i64.const 0
    global.set $prof_289
    i64.const 0
    global.set $prof_290
    i64.const 0
    global.set $prof_291
    i64.const 0
    global.set $prof_292
    i64.const 0
    global.set $prof_293
    i64.const 0
    global.set $prof_294
    i64.const 0
    global.set $prof_295
    i64.const 0
    global.set $prof_296
    i64.const 0
    global.set $prof_297
    i64.const 0
    global.set $prof_298
    i64.const 0
    global.set $prof_299
    i64.const 0
    global.set $prof_300
    i64.const 0
    global.set $prof_301
    i64.const 0
    global.set $prof_302
    i64.const 0
    global.set $prof_303
    i64.const 0
    global.set $prof_304
    i64.const 0
    global.set $prof_305
    i64.const 0
    global.set $prof_306
    i64.const 0
    global.set $prof_307
    i64.const 0
    global.set $prof_308
    i64.const 0
    global.set $prof_309
    i64.const 0
    global.set $prof_310
    i64.const 0
    global.set $prof_311
    i64.const 0
    global.set $prof_312
    i64.const 0
    global.set $prof_313
    i64.const 0
    global.set $prof_314
    i64.const 0
    global.set $prof_315
    i64.const 0
    global.set $prof_316
    i64.const 0
    global.set $prof_317
    i64.const 0
    global.set $prof_318
    i64.const 0
    global.set $prof_319
    i64.const 0
    global.set $prof_320
    i64.const 0
    global.set $prof_321
    i64.const 0
    global.set $prof_322
    i64.const 0
    global.set $prof_323
    i64.const 0
    global.set $prof_324
    i64.const 0
    global.set $prof_325
    i64.const 0
    global.set $prof_326
    i64.const 0
    global.set $prof_327
    i64.const 0
    global.set $prof_328
    i64.const 0
    global.set $prof_329
    i64.const 0
    global.set $prof_330
    i64.const 0
    global.set $prof_331
    i64.const 0
    global.set $prof_332
    i64.const 0
    global.set $prof_333
    i64.const 0
    global.set $prof_334
    i64.const 0
    global.set $prof_335
    i64.const 0
    global.set $prof_336
    i64.const 0
    global.set $prof_337
    i64.const 0
    global.set $prof_338
    i64.const 0
    global.set $prof_339
    i64.const 0
    global.set $prof_340
    i64.const 0
    global.set $prof_341
    i64.const 0
    global.set $prof_342
    i64.const 0
    global.set $prof_343
    i64.const 0
    global.set $prof_344
    i64.const 0
    global.set $prof_345
    i64.const 0
    global.set $prof_346
    i64.const 0
    global.set $prof_347
    i64.const 0
    global.set $prof_348
    i64.const 0
    global.set $prof_349
    i64.const 0
    global.set $prof_350
    i64.const 0
    global.set $prof_351
    i64.const 0
    global.set $prof_352
    i64.const 0
    global.set $prof_353
    i64.const 0
    global.set $prof_354
    i64.const 0
    global.set $prof_355
    i64.const 0
    global.set $prof_356
    i64.const 0
    global.set $prof_357
    i64.const 0
    global.set $prof_358
    i64.const 0
    global.set $prof_359
    i64.const 0
    global.set $prof_360
    i64.const 0
    global.set $prof_361
    i64.const 0
    global.set $prof_362
    i64.const 0
    global.set $prof_363
    i64.const 0
    global.set $prof_364
    i64.const 0
    global.set $prof_365
    i64.const 0
    global.set $prof_366
    i64.const 0
    global.set $prof_367
    i64.const 0
    global.set $prof_368
    i64.const 0
    global.set $prof_369
    i64.const 0
    global.set $prof_370
    i64.const 0
    global.set $prof_371
    i64.const 0
    global.set $prof_372
    i64.const 0
    global.set $prof_373
    i64.const 0
    global.set $prof_374
    i64.const 0
    global.set $prof_375
    i64.const 0
    global.set $prof_376
    i64.const 0
    global.set $prof_377
    i64.const 0
    global.set $prof_378
    i64.const 0
    global.set $prof_379
    i64.const 0
    global.set $prof_380
    i64.const 0
    global.set $prof_381
    i64.const 0
    global.set $prof_382
    i64.const 0
    global.set $prof_383
    i64.const 0
    global.set $prof_384
    i64.const 0
    global.set $prof_385
    i64.const 0
    global.set $prof_386
    i64.const 0
    global.set $prof_387
    i64.const 0
    global.set $prof_388
    i64.const 0
    global.set $prof_389
    i64.const 0
    global.set $prof_390
    i64.const 0
    global.set $prof_391
    i64.const 0
    global.set $prof_392
    i64.const 0
    global.set $prof_393
    i64.const 0
    global.set $prof_394
    i64.const 0
    global.set $prof_395
    i64.const 0
    global.set $prof_396
    i64.const 0
    global.set $prof_397
    i64.const 0
    global.set $prof_398
    i64.const 0
    global.set $prof_399
    i64.const 0
    global.set $prof_400
    i64.const 0
    global.set $prof_401
    i64.const 0
    global.set $prof_402
    i64.const 0
    global.set $prof_403
    i64.const 0
    global.set $prof_404
    i64.const 0
    global.set $prof_405
    i64.const 0
    global.set $prof_406
    i64.const 0
    global.set $prof_407
    i64.const 0
    global.set $prof_408
    i64.const 0
    global.set $prof_409
    i64.const 0
    global.set $prof_410
    i64.const 0
    global.set $prof_411
    i64.const 0
    global.set $prof_412
    i64.const 0
    global.set $prof_413
    i64.const 0
    global.set $prof_414
    i64.const 0
    global.set $prof_415
    i64.const 0
    global.set $prof_416
    i64.const 0
    global.set $prof_417
    i64.const 0
    global.set $prof_418
    i64.const 0
    global.set $prof_419
    i64.const 0
    global.set $prof_420
    i64.const 0
    global.set $prof_421
    i64.const 0
    global.set $prof_422
    i64.const 0
    global.set $prof_423
    i64.const 0
    global.set $prof_424
    i64.const 0
    global.set $prof_425
    i64.const 0
    global.set $prof_426
    i64.const 0
    global.set $prof_427
    i64.const 0
    global.set $prof_428
    i64.const 0
    global.set $prof_429
    i64.const 0
    global.set $prof_430
    i64.const 0
    global.set $prof_431
    i64.const 0
    global.set $prof_432
    i64.const 0
    global.set $prof_433
    i64.const 0
    global.set $prof_434
    i64.const 0
    global.set $prof_435
    i64.const 0
    global.set $prof_436
    i64.const 0
    global.set $prof_437
    i64.const 0
    global.set $prof_438
    i64.const 0
    global.set $prof_439
    i64.const 0
    global.set $prof_440
    i64.const 0
    global.set $prof_441
    i64.const 0
    global.set $prof_442
    i64.const 0
    global.set $prof_443
    i64.const 0
    global.set $prof_444
    i64.const 0
    global.set $prof_445
    i64.const 0
    global.set $prof_446
    i64.const 0
    global.set $prof_447
    i64.const 0
    global.set $prof_448
    i64.const 0
    global.set $prof_449
    i64.const 0
    global.set $prof_450
    i64.const 0
    global.set $prof_451
    i64.const 0
    global.set $prof_452
    i64.const 0
    global.set $prof_453
    i64.const 0
    global.set $prof_454
    i64.const 0
    global.set $prof_455
    i64.const 0
    global.set $prof_456
    i64.const 0
    global.set $prof_457
    i64.const 0
    global.set $prof_458
    i64.const 0
    global.set $prof_459
    i64.const 0
    global.set $prof_460
    i64.const 0
    global.set $prof_461
    i64.const 0
    global.set $prof_462
    i64.const 0
    global.set $prof_463
    i64.const 0
    global.set $prof_464
    i64.const 0
    global.set $prof_465
    i64.const 0
    global.set $prof_466
    i64.const 0
    global.set $prof_467
    i64.const 0
    global.set $prof_468
    i64.const 0
    global.set $prof_469
    i64.const 0
    global.set $prof_470
    i64.const 0
    global.set $prof_471
    i64.const 0
    global.set $prof_472
    i64.const 0
    global.set $prof_473
    i64.const 0
    global.set $prof_474
    i64.const 0
    global.set $prof_475
    i64.const 0
    global.set $prof_476
    i64.const 0
    global.set $prof_477
    i64.const 0
    global.set $prof_478
    i64.const 0
    global.set $prof_479
    i64.const 0
    global.set $prof_480
    i64.const 0
    global.set $prof_481
    i64.const 0
    global.set $prof_482
    i64.const 0
    global.set $prof_483
    i64.const 0
    global.set $prof_484
    i64.const 0
    global.set $prof_485
    i64.const 0
    global.set $prof_486
    i64.const 0
    global.set $prof_487
    i64.const 0
    global.set $prof_488
    i64.const 0
    global.set $prof_489
    i64.const 0
    global.set $prof_490
    i64.const 0
    global.set $prof_491
    i64.const 0
    global.set $prof_492
    i64.const 0
    global.set $prof_493
    i64.const 0
    global.set $prof_494
    i64.const 0
    global.set $prof_495
    i64.const 0
    global.set $prof_496
    i64.const 0
    global.set $prof_497
    i64.const 0
    global.set $prof_498
    i64.const 0
    global.set $prof_499
    i64.const 0
    global.set $prof_500
    i64.const 0
    global.set $prof_501
    i64.const 0
    global.set $prof_502
    i64.const 0
    global.set $prof_503
    i64.const 0
    global.set $prof_504
    i64.const 0
    global.set $prof_505
    i64.const 0
    global.set $prof_506
    i64.const 0
    global.set $prof_507
    i64.const 0
    global.set $prof_508
    i64.const 0
    global.set $prof_509
    i64.const 0
    global.set $prof_510
    i64.const 0
    global.set $prof_511
    i64.const 0
    global.set $prof_512
    i64.const 0
    global.set $prof_513
    i64.const 0
    global.set $prof_514
    i64.const 0
    global.set $prof_515
    i64.const 0
    global.set $prof_516
    i64.const 0
    global.set $prof_517
    i64.const 0
    global.set $prof_518
    i64.const 0
    global.set $prof_519
    i64.const 0
    global.set $prof_520
    i64.const 0
    global.set $prof_521
    i64.const 0
    global.set $prof_522
    i64.const 0
    global.set $prof_523
    i64.const 0
    global.set $prof_524
    i64.const 0
    global.set $prof_525
    i64.const 0
    global.set $prof_526
    i64.const 0
    global.set $prof_527
    i64.const 0
    global.set $prof_528
    i64.const 0
    global.set $prof_529
    i64.const 0
    global.set $prof_530
    i64.const 0
    global.set $prof_531
    i64.const 0
    global.set $prof_532
    i64.const 0
    global.set $prof_533
    i64.const 0
    global.set $prof_534
    i64.const 0
    global.set $prof_535
    i64.const 0
    global.set $prof_536
    i64.const 0
    global.set $prof_537
    i64.const 0
    global.set $prof_538
    i64.const 0
    global.set $prof_539
    i64.const 0
    global.set $prof_540
    i64.const 0
    global.set $prof_541
    i64.const 0
    global.set $prof_542
    i64.const 0
    global.set $prof_543
    i64.const 0
    global.set $prof_544
    i64.const 0
    global.set $prof_545
    i64.const 0
    global.set $prof_546
    i64.const 0
    global.set $prof_547
    i64.const 0
    global.set $prof_548
    i64.const 0
    global.set $prof_549
    i64.const 0
    global.set $prof_550
  )
)
