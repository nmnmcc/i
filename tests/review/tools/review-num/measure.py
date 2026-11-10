from pathlib import Path
import subprocess, json, os
ROOT=Path(__file__).resolve().parents[2]
WORK=Path(__file__).resolve().parent
def instance(path):
    import wasmtime
    config = wasmtime.Config()
    config.wasm_gc = True
    config.wasm_function_references = True
    config.wasm_tail_call = True
    config.wasm_exceptions = True
    engine = wasmtime.Engine(config)
    module = wasmtime.Module.from_file(engine, str(path))
    store = wasmtime.Store(engine)
    imports = []
    def panic(*args):
        raise RuntimeError('unexpected guest panic: ' + repr(args))
    for item in module.imports:
        assert item.module == 'ilang:host/v1' and item.name in ['panic','reference_id']
        imports.append(wasmtime.Func(store, item.type, panic if item.name=='panic' else lambda *args: 0))
    inst = wasmtime.Instance(store, module, imports)
    exports = inst.exports(store)
    if 'ilang_start' in exports:
        exports['ilang_start'](store)
    return store, exports


def timing(path, output):
    import ctypes as c
    import statistics
    from wasmtime import _ffi as ffi
    from wasmtime import _bindings as binding
    assert c.sizeof(ffi.wasmtime_val_raw_t) == 16
    assert c.sizeof(ffi.wasmtime_func_t) == 16
    library = WORK / 'timer.so'
    subprocess.run(['cc', '-O2', '-shared', '-fPIC', str(WORK / 'float_profile_native.c'),
                    '-o', str(library)], check=True)
    timer = c.CDLL(str(library)).measure
    timer.argtypes = [c.c_void_p]*4 + [c.c_double, c.c_size_t, c.POINTER(c.c_int64), c.POINTER(c.c_int64)]
    timer.restype = c.c_int
    store, exports = instance(path)
    results = {}
    names = json.loads((WORK / 'exports.json').read_text())
    prefix = os.environ.get('NUM_PREFIX', 'num/kernels.')
    for name in names:
        count = 1001
        target = (c.c_int64 * count)()
        empty = (c.c_int64 * count)()
        if prefix + name not in exports:
            continue
        func = exports[prefix + name]
        baseline = exports[prefix + 'identity']
        assert [str(v) for v in func.type(store).params] == ['f64']
        assert [str(v) for v in func.type(store).results] == ['f64']
        status = timer(c.cast(binding._wasmtime_func_call_unchecked, c.c_void_p),
                       c.cast(store._context(), c.c_void_p), c.byref(func._func),
                       c.byref(baseline._func), .5, count, target, empty)
        assert status == 0, name
        differences = sorted(t-e for t,e in zip(target, empty))
        results[name] = dict(median_ns=statistics.median(differences),
                             p99_ns=differences[int(count*.99)], worst_ns=max(differences),
                             raw_median_ns=statistics.median(target),
                             empty_median_ns=statistics.median(empty), samples=count,
                             target_ns=list(target), empty_ns=list(empty))
        print(name, {k:v for k,v in results[name].items() if k not in ['target_ns','empty_ns']}, flush=True)
    (WORK / output).write_text(json.dumps(results, indent=2))

if __name__ == "__main__":
 import sys
 timing(Path(sys.argv[1]),sys.argv[2])
