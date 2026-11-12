//! Checked preview-1 capability adapters. Guest values never contain addresses.
use super::*;

fn uses(p: &Program, namespace: &str) -> bool {
    p.imports
        .iter()
        .any(|x| x.kind == ImportKind::WasiAdapter && x.module == namespace)
}
pub(super) fn teardown(p: &Program) -> &'static str {
    if uses(p, "i:fs/v1") {
        "(call $wa_fs_teardown)"
    } else {
        ""
    }
}
pub(super) fn imports(p: &Program) -> String {
    let mut funcs = BTreeSet::new();
    if uses(p, "i:clock/v1") {
        funcs.extend([
            ("clock_time_get", "i32 i64 i32"),
            ("poll_oneoff", "i32 i32 i32 i32"),
        ]);
    }
    if uses(p, "i:stdio/v1") {
        funcs.insert(("fd_read", "i32 i32 i32 i32"));
    }
    if uses(p, "i:entropy/v1") {
        funcs.insert(("random_get", "i32 i32"));
    }
    for (ns, prefix) in [("i:arguments/v1", "args"), ("i:environment/v1", "environ")] {
        if uses(p, ns) {
            funcs.insert((
                if prefix == "args" {
                    "args_sizes_get"
                } else {
                    "environ_sizes_get"
                },
                "i32 i32",
            ));
            funcs.insert((
                if prefix == "args" {
                    "args_get"
                } else {
                    "environ_get"
                },
                "i32 i32",
            ));
        }
    }
    if uses(p, "i:fs/v1") {
        funcs.extend([
            ("fd_read", "i32 i32 i32 i32"),
            ("fd_close", "i32"),
            ("fd_prestat_get", "i32 i32"),
            ("fd_prestat_dir_name", "i32 i32 i32"),
            ("path_open", "i32 i32 i32 i32 i32 i64 i64 i32 i32"),
            ("path_create_directory", "i32 i32 i32"),
            ("path_unlink_file", "i32 i32 i32"),
            ("path_remove_directory", "i32 i32 i32"),
            ("path_rename", "i32 i32 i32 i32 i32 i32"),
            ("path_filestat_get", "i32 i32 i32 i32 i32"),
            ("fd_readdir", "i32 i32 i32 i64 i32"),
        ]);
    }
    funcs.into_iter().map(|(name, params)| format!("(import \"wasi_snapshot_preview1\" \"{name}\" (func $wa_raw_{name} (param {params}) (result i32)))\n")).collect()
}
pub(super) fn call(x: &Import, args: &[String]) -> String {
    let ns = x
        .module
        .strip_prefix("i:")
        .unwrap()
        .strip_suffix("/v1")
        .unwrap();
    let args = args.iter().map(|a| format!(" {a}")).collect::<String>();
    let call = format!("(call $wa_{ns}_{}{args})", x.name);
    if x.request_result {
        format!(
            "(block $wa_done (result eqref) (block $wa_error (result eqref) (br $wa_done (call $result_ok (try_table (result eqref) (catch $error $wa_error) {call})))) (call $result_error))"
        )
    } else {
        call
    }
}
pub(super) fn runtime(p: &Program) -> String {
    if !p.imports.iter().any(|x| x.kind == ImportKind::WasiAdapter) {
        return String::new();
    }
    let mut s = COMMON.to_owned();
    if uses(p, "i:clock/v1") {
        s.push_str(CLOCK);
    }
    if uses(p, "i:stdio/v1") || uses(p, "i:fs/v1") {
        s.push_str(IO);
    }
    if uses(p, "i:stdio/v1") {
        s.push_str(STDIO);
    }
    if uses(p, "i:entropy/v1") {
        s.push_str(ENTROPY);
    }
    for (ns, prefix) in [("i:arguments/v1", "args"), ("i:environment/v1", "environ")] {
        if uses(p, ns) {
            s.push_str(&STRINGS.replace("PREFIX", prefix));
        }
    }
    if uses(p, "i:arguments/v1") {
        s.push_str(ARGS);
    }
    if uses(p, "i:environment/v1") {
        s.push_str(ENV);
    }
    if uses(p, "i:fs/v1") {
        s.push_str(FS);
    }
    runtime::with_literals(
        s,
        &[
            ("invalid", "invalid"),
            ("closed", "closed"),
            ("denied", "denied"),
            ("not_found", "not_found"),
            ("exists", "exists"),
            ("io", "io"),
            ("busy", "busy"),
            ("interrupted", "interrupted"),
            ("empty", ""),
            ("file", "file"),
            ("directory", "directory"),
            ("other", "other"),
            ("read", "read"),
            ("truncate", "truncate"),
            ("append", "append"),
            ("read_write", "read_write"),
            ("dot", "."),
            ("tmp", "/tmp"),
            ("nul", "\0"),
        ],
    )
}
const COMMON: &str = r#"
(func $wa_fail (param $code eqref) (throw $error (local.get $code)))
(func $wa_errno (param $e i32)
 (if (i32.eqz (local.get $e)) (then return))
 (if (i32.eq (local.get $e) (i32.const 44)) (then (call $wa_fail {{not_found}})))
 (if (i32.or (i32.eq (local.get $e) (i32.const 2)) (i32.or (i32.eq (local.get $e) (i32.const 63)) (i32.eq (local.get $e) (i32.const 76)))) (then (call $wa_fail {{denied}})))
 (if (i32.eq (local.get $e) (i32.const 20)) (then (call $wa_fail {{exists}})))
 (if (i32.eq (local.get $e) (i32.const 8)) (then (call $wa_fail {{closed}})))
 (if (i32.eq (local.get $e) (i32.const 28)) (then (call $wa_fail {{invalid}})))
 (if (i32.eq (local.get $e) (i32.const 27)) (then (call $wa_fail {{interrupted}})))
 (call $wa_fail {{io}}))
(func $wa_uint (param $v eqref) (param $max i64) (result i64)
 (if (i32.or (i32.lt_s (call $int_sign (local.get $v)) (i32.const 0))
   (i32.gt_s (call $int_compare (local.get $v) (call $int_from_i64 (local.get $max))) (i32.const 0)))
  (then (call $wa_fail {{invalid}})))
 (call $int_to_i64 (local.get $v)))
(func $wa_u32 (param $v eqref) (result i32)
 (i32.wrap_i64 (call $wa_uint (local.get $v) (i64.const 4294967295))))
(func $wa_u64_load (param $p i32) (result eqref)
 (call $abi_int_in (select (i32.const 1) (i32.const 0) (i64.ne (i64.load (local.get $p)) (i64.const 0)))
  (local.get $p) (select (i32.const 2) (select (i32.const 1) (i32.const 0) (i32.load (local.get $p))) (i32.load offset=4 (local.get $p)))))
(func $wa_bits (param $p i32) (param $n i32) (result eqref)
 (call $bits_new (call $abi_bytes_in (local.get $p) (local.get $n)) (i32.const 0) (i64.shl (i64.extend_i32_u (local.get $n)) (i64.const 3))))
(func $string_write_mem (param $s eqref) (param $dest i32)
 (local $p i32)
 (local.set $p (call $string_to_mem (local.get $s)))
 (memory.copy (local.get $dest) (local.get $p) (call $string_length_bytes (local.get $s)))
 (call $free (local.get $p) (i32.const 0)))
(func $wa_whole (param $b eqref)
 (if (i64.ne (i64.and (call $bits_len64 (local.get $b)) (i64.const 7)) (i64.const 0)) (then (call $wa_fail {{invalid}}))))
"#;
const CLOCK: &str = r#"
(func $wa_time (param $clock i32) (result eqref)
 (local $p i32) (local $e i32) (local $r eqref)
 (local.set $p (call $alloc (i32.const 8)))
 (local.set $e (call $wa_raw_clock_time_get (local.get $clock) (i64.const 1) (local.get $p)))
 (if (i32.eqz (local.get $e)) (then (local.set $r (call $wa_u64_load (local.get $p)))))
 (call $free (local.get $p) (i32.const 8)) (call $wa_errno (local.get $e)) (local.get $r))
(func $wa_clock_monotonic_ns (result eqref) (call $wa_time (i32.const 1)))
(func $wa_clock_unix_ns (result eqref) (call $wa_time (i32.const 0)))
(func $wa_clock_sleep_ns (param $ns eqref) (result eqref)
 (local $duration i64) (local $start i64) (local $elapsed i64) (local $p i32) (local $e i32)
 (local.set $duration (call $wa_uint (local.get $ns) (i64.const 9223372036854775807)))
 ;; subscription[48], event[32], count[4]. Recheck the monotonic deadline:
 ;; preview-1 runners may round relative timeout precision down.
 (local.set $p (call $alloc (i32.const 84))) (memory.fill (local.get $p) (i32.const 0) (i32.const 84))
 (local.set $e (call $wa_raw_clock_time_get (i32.const 1) (i64.const 1) (local.get $p)))
 (if (local.get $e) (then (call $free (local.get $p) (i32.const 84)) (call $wa_errno (local.get $e))))
 (local.set $start (i64.load (local.get $p)))
 (i32.store offset=16 (local.get $p) (i32.const 1))
 (i64.store offset=32 (local.get $p) (i64.const 1))
 (block $done (loop $wait
  (br_if $done (i64.ge_u (local.get $elapsed) (local.get $duration)))
  (i64.store offset=24 (local.get $p) (i64.sub (local.get $duration) (local.get $elapsed)))
  (local.set $e (call $wa_raw_poll_oneoff (local.get $p) (i32.add (local.get $p) (i32.const 48)) (i32.const 1) (i32.add (local.get $p) (i32.const 80))))
  (br_if $done (local.get $e))
  (if (i32.ne (i32.load offset=80 (local.get $p)) (i32.const 1)) (then (call $stop_host_value) unreachable))
  (local.set $e (i32.load16_u offset=56 (local.get $p)))
  (br_if $done (local.get $e))
  (local.set $e (call $wa_raw_clock_time_get (i32.const 1) (i64.const 1) (i32.add (local.get $p) (i32.const 48))))
  (br_if $done (local.get $e))
  (local.set $elapsed (i64.sub (i64.load offset=48 (local.get $p)) (local.get $start)))
  (br $wait)))
 (call $free (local.get $p) (i32.const 84)) (call $wa_errno (local.get $e)) (ref.null eq))
"#;
const IO: &str = r#"
(func $wa_read (param $fd i32) (param $max eqref) (result eqref)
 (local $n i32) (local $p i32) (local $io i32) (local $e i32) (local $got i32) (local $r eqref)
 (if (i32.le_s (call $int_sign (local.get $max)) (i32.const 0)) (then (call $wa_fail {{invalid}})))
 (local.set $n (i32.wrap_i64 (call $int_to_i64 (if (result eqref) (i32.gt_s (call $int_compare (local.get $max) (call $int_from_i64 (i64.const 65536))) (i32.const 0)) (then (call $int_from_i64 (i64.const 65536))) (else (local.get $max))))))
 (local.set $p (call $alloc (local.get $n))) (local.set $io (call $alloc (i32.const 12)))
 (i32.store (local.get $io) (local.get $p)) (i32.store offset=4 (local.get $io) (local.get $n))
 (local.set $e (call $wa_raw_fd_read (local.get $fd) (local.get $io) (i32.const 1) (i32.add (local.get $io) (i32.const 8))))
 (local.set $got (i32.load offset=8 (local.get $io)))
 (if (i32.eqz (local.get $e)) (then
  (if (i32.gt_u (local.get $got) (local.get $n)) (then (call $stop_host_value) unreachable))
  (local.set $r (call $wa_bits (local.get $p) (local.get $got)))))
 (call $free (local.get $io) (i32.const 12)) (call $free (local.get $p) (local.get $n))
 (call $wa_errno (local.get $e)) (local.get $r))
(func $wa_write (param $fd i32) (param $b eqref) (result eqref)
 (local $p i32) (local $n i32) (local $io i32) (local $pos i32) (local $got i32) (local $e i32)
 (call $wa_whole (local.get $b))
 (call $abi_bits_out (local.get $b)) (local.set $n) (local.set $p)
 (local.set $n (i32.shr_u (local.get $n) (i32.const 3)))
 (local.set $io (call $alloc (i32.const 12)))
 (block $done (loop $more
  (br_if $done (i32.eq (local.get $pos) (local.get $n)))
  (i32.store (local.get $io) (i32.add (local.get $p) (local.get $pos)))
  (i32.store offset=4 (local.get $io) (i32.sub (local.get $n) (local.get $pos)))
  (local.set $e (call $wasi_fd_write (local.get $fd) (local.get $io) (i32.const 1) (i32.add (local.get $io) (i32.const 8))))
  (br_if $done (local.get $e))
  (local.set $got (i32.load offset=8 (local.get $io)))
  (if (i32.gt_u (local.get $got) (i32.sub (local.get $n) (local.get $pos))) (then (call $stop_host_value) unreachable))
  (if (i32.eqz (local.get $got)) (then (local.set $e (i32.const 29)) (br $done)))
  (local.set $pos (i32.add (local.get $pos) (local.get $got))) (br $more)))
 (call $free (local.get $io) (i32.const 12)) (call $free (local.get $p) (local.get $n))
 (call $wa_errno (local.get $e)) (ref.null eq))
"#;
const STDIO: &str = r#"
(func $wa_stdio_read (param $n eqref) (result eqref) (call $wa_read (i32.const 0) (local.get $n)))
(func $wa_stdio_write (param $b eqref) (result eqref) (call $wa_write (i32.const 1) (local.get $b)))
(func $wa_stdio_write_error (param $b eqref) (result eqref) (call $wa_write (i32.const 2) (local.get $b)))
(func $wa_stdio_flush (result eqref) (ref.null eq))
(func $wa_stdio_flush_error (result eqref) (ref.null eq))
"#;
const ENTROPY: &str = r#"
(func $wa_entropy_entropy (param $count eqref) (result eqref)
 (local $n i32) (local $p i32) (local $e i32) (local $r eqref)
 (local.set $n (i32.wrap_i64 (call $wa_uint (local.get $count) (i64.const 16777216))))
 (local.set $p (call $alloc (local.get $n)))
 (local.set $e (call $wa_raw_random_get (local.get $p) (local.get $n)))
 (if (i32.eqz (local.get $e)) (then (local.set $r (call $wa_bits (local.get $p) (local.get $n)))))
 (call $free (local.get $p) (local.get $n)) (call $wa_errno (local.get $e)) (local.get $r))
"#;
const STRINGS: &str = r#"
(func $wa_PREFIX_strings (result eqref)
 (local $sizes i32) (local $ptrs i32) (local $buf i32) (local $count i32) (local $size i32)
 (local $e i32) (local $i i32) (local $p i32) (local $n i32) (local $out eqref)
 (local.set $sizes (call $alloc (i32.const 8)))
 (local.set $e (call $wa_raw_PREFIX_sizes_get (local.get $sizes) (i32.add (local.get $sizes) (i32.const 4))))
 (local.set $count (i32.load (local.get $sizes))) (local.set $size (i32.load offset=4 (local.get $sizes)))
 (call $free (local.get $sizes) (i32.const 8)) (call $wa_errno (local.get $e))
 (if (i32.or (i32.gt_u (local.get $count) (local.get $size)) (i32.gt_u (local.get $count) (i32.const 1073741823))) (then (call $stop_host_value) unreachable))
 (local.set $ptrs (call $alloc (i32.mul (local.get $count) (i32.const 4))))
 (local.set $buf (call $alloc (local.get $size)))
 (local.set $e (call $wa_raw_PREFIX_get (local.get $ptrs) (local.get $buf)))
 (local.set $out (call $list_nil))
 (if (i32.eqz (local.get $e)) (then (block $done (loop $each
  (br_if $done (i32.eqz (local.get $count)))
  (local.set $count (i32.sub (local.get $count) (i32.const 1)))
  (local.set $p (i32.load (i32.add (local.get $ptrs) (i32.mul (local.get $count) (i32.const 4)))))
  (if (i32.or (i32.lt_u (local.get $p) (local.get $buf)) (i32.ge_u (local.get $p) (i32.add (local.get $buf) (local.get $size)))) (then (call $stop_host_value) unreachable))
  (local.set $n (i32.const 0))
  (block $end (loop $scan
   (if (i32.ge_u (i32.add (local.get $p) (local.get $n)) (i32.add (local.get $buf) (local.get $size))) (then (call $stop_host_value) unreachable))
   (br_if $end (i32.eqz (i32.load8_u (i32.add (local.get $p) (local.get $n)))))
   (local.set $n (i32.add (local.get $n) (i32.const 1))) (br $scan)))
  (local.set $out (call $list_cons (call $abi_string_in (local.get $p) (local.get $n)) (local.get $out)))
  (br $each)))))
 (call $free (local.get $buf) (local.get $size)) (call $free (local.get $ptrs) (i32.const 0))
 (call $wa_errno (local.get $e)) (local.get $out))
"#;
const ARGS: &str = r#"
(func $wa_arguments_args (result eqref)
 (local $list eqref) (local $item eqref) (local $p i32) (local $n i32) (local $count i32) (local $size i32) (local $pos i32) (local $out eqref)
 (local.set $list (call $wa_args_strings))
 (if (i32.eqz (call $list_is_empty (local.get $list))) (then (local.set $list (call $list_tail (local.get $list)))))
 (local.set $item (local.get $list)) (local.set $size (i32.const 4))
 (block $done (loop $each
  (br_if $done (call $list_is_empty (local.get $item)))
  (local.set $size (i32.add (local.get $size) (i32.add (i32.const 4) (call $string_length_bytes (call $list_head (local.get $item))))))
  (local.set $count (i32.add (local.get $count) (i32.const 1)))
  (local.set $item (call $list_tail (local.get $item))) (br $each)))
 (local.set $p (call $alloc (local.get $size))) (i32.store (local.get $p) (local.get $count)) (local.set $pos (i32.const 4))
 (block $done (loop $each
  (br_if $done (call $list_is_empty (local.get $list)))
  (local.set $item (call $list_head (local.get $list)))
  (local.set $n (call $string_length_bytes (local.get $item)))
  (i32.store (i32.add (local.get $p) (local.get $pos)) (local.get $n))
  (local.set $pos (i32.add (local.get $pos) (i32.const 4)))
  (call $string_write_mem (local.get $item) (i32.add (local.get $p) (local.get $pos)))
  (local.set $pos (i32.add (local.get $pos) (local.get $n)))
  (local.set $list (call $list_tail (local.get $list))) (br $each)))
 (local.set $out (call $wa_bits (local.get $p) (local.get $size)))
 (call $free (local.get $p) (local.get $size)) (local.get $out))
"#;
const ENV: &str = r#"
(func $wa_environment_env (param $name eqref) (result eqref)
 (local $list eqref) (local $s eqref) (local $prefix eqref) (local $n i32) (local $p i32) (local $r eqref)
 (local.set $n (call $string_length_bytes (local.get $name)))
 (if (i32.eqz (local.get $n)) (then (call $wa_fail {{invalid}})))
 (local.set $p (call $string_to_mem (local.get $name)))
 (block $done (loop $scan
  (br_if $done (i32.eqz (local.get $n)))
  (local.set $n (i32.sub (local.get $n) (i32.const 1)))
  (if (i32.or (i32.eqz (i32.load8_u (i32.add (local.get $p) (local.get $n)))) (i32.eq (i32.load8_u (i32.add (local.get $p) (local.get $n))) (i32.const 61)))
   (then (call $free (local.get $p) (i32.const 0)) (call $wa_fail {{invalid}}))) (br $scan)))
 (call $free (local.get $p) (i32.const 0))
 (local.set $list (call $wa_environ_strings))
 (local.set $n (call $string_length_bytes (local.get $name)))
 (block $done (loop $each
  (br_if $done (call $list_is_empty (local.get $list)))
  (local.set $s (call $list_head (local.get $list)))
  (if (i32.gt_u (call $string_length_bytes (local.get $s)) (local.get $n)) (then
   (local.set $p (call $string_to_mem (local.get $s)))
   (local.set $prefix (call $abi_string_in (local.get $p) (local.get $n)))
   (if (i32.and (call $equal (local.get $prefix) (local.get $name)) (i32.eq (i32.load8_u (i32.add (local.get $p) (local.get $n))) (i32.const 61))) (then
    (local.set $r (call $abi_string_in (i32.add (local.get $p) (i32.add (local.get $n) (i32.const 1))) (i32.sub (call $string_length_bytes (local.get $s)) (i32.add (local.get $n) (i32.const 1)))))
    (call $free (local.get $p) (i32.const 0)) (return (local.get $r))))
   (call $free (local.get $p) (i32.const 0))))
  (local.set $list (call $list_tail (local.get $list))) (br $each)))
 (call $wa_fail {{not_found}}) unreachable)
"#;
const FS: &str = include_str!("wasi-fs.wat");
