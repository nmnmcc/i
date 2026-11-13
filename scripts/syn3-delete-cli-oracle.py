"""Pin the exact five old-engine answers before removing its production hook."""
from pathlib import Path
import subprocess, json, importlib.util
ROOT=Path.cwd()

def apply():
    head=subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip()
    proof=json.loads((ROOT/'target/fa-night/step3-green.json').read_text())
    assert proof['commit']==head
    oracle_path=ROOT/'target/fa-r3/engine-boundary-oracle.json'
    oracle=json.loads(oracle_path.read_text())
    assert oracle['source_hash']==head and len(oracle['cases'])==5
    spec=importlib.util.spec_from_file_location('resolver_delete', ROOT/'target/fa-night/delete-resolver.py')
    helper=importlib.util.module_from_spec(spec);spec.loader.exec_module(helper)
    path=ROOT/'crates/ilang-cli/tests/macro_run.rs';text=path.read_text()
    start=text.index('fn type_macro_keeps_its_standalone_need_channel()')
    end=helper.item_end(text,start)
    body=text[start:end]
    cut=body.index('        let mut answers = Vec::new();')
    loop=body.index('        for native in [false, true] {',cut)
    init_begin=body.index('\n',loop)+1
    branch=body.index('            let lowered = if native {',init_begin)
    init=body[init_begin:branch]
    native_begin=body.index('{',branch)+1
    native_end=helper.item_end(body,branch)-1
    native=body[native_begin:native_end]
    native=native.replace('ilang_syntax::syn3::lower::', 'migration_syntax::syn3::lower::')
    native=native.replace('&module.lowering,', '&migration_syntax::syn3::lower::Transition::from_native(&module.lowering),')
    native=native.replace('Some(engine.scoped.transition()),', 'Some(&migration_syntax::syn3::lower::Transition::from_native(engine.scoped.transition())),')
    prefix=body[:cut]
    prefix=prefix.replace('    for text in [', '    for (case, text) in [',1)
    prefix=prefix.replace('    ] {','    ].into_iter().enumerate() {',1)
    prefix=prefix.replace('    use std::sync::Arc;','''    use std::sync::Arc;
    // The old and native engines agreed byte for byte before deletion.
    // Keep the complete five answers as the independent historical oracle.
    // §4 of syn3-final-ast.md removes the old production engine/tree; its
    // standalone function-need and grouping boundaries remain asserted.
    let oracle: serde_json::Value = serde_json::from_str(include_str!("fixtures/type_macro_engine_boundary.json")).unwrap();
    let cases = oracle["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 5);''',1)
    replacement=prefix+init+'        let lowered = {'+native+'''        };
        let actual = migration_syntax::print::print_module(&lowered);
        assert_eq!(cases[case]["source"].as_str().unwrap(), text);
        assert_eq!(
            actual, cases[case]["expected_old"].as_str().unwrap(),
            "expansion must retain its lowering boundary: {text}"
        );
    }
}'''
    assert 'for native in' not in replacement
    path.write_text(text[:start]+replacement+text[end:])
    fixture=path.parent/'fixtures/type_macro_engine_boundary.json'
    fixture.parent.mkdir(parents=True,exist_ok=True)
    fixture.write_text(json.dumps(oracle,ensure_ascii=False,indent=2)+'\n')
    manifest=ROOT/'crates/ilang-cli/Cargo.toml'
    contents=manifest.read_text();assert '[dev-dependencies]' in contents
    manifest.write_text(contents.replace('[dev-dependencies]\n', '[dev-dependencies]\nmigration-syntax = { package = "ilang-syntax-migration", path = "../ilang-syntax/examples/legacy" }\n', 1))

if __name__=='__main__':
    import argparse
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--apply',action='store_true')
    if p.parse_args().apply:apply()
    else:print('Requires the original matching engine answers captured on the exact step-three/S12 head.')
