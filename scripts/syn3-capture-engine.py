from pathlib import Path
import subprocess, json, sys
root=Path.cwd();path=root/'crates/ilang-cli/tests/macro_run.rs'
marker='''        assert_eq!(
            answers[0], answers[1],
            "expansion must retain its lowering boundary: {text}"
        );'''
probe='''        eprintln!("FA_ENGINE_ORACLE {}", serde_json::to_string(&(text, &answers)).unwrap());
'''
if sys.argv[1]=='instrument':
    assert subprocess.check_output(['git','branch','--show-current'],text=True).strip().endswith('-delete')
    assert not subprocess.check_output(['git','status','--porcelain'],text=True).strip()
    text=path.read_text();assert text.count(marker)==1;path.write_text(text.replace(marker,probe+marker))
elif sys.argv[1]=='collect':
    log=(root/'target/fa-r3/capture-engine.log').read_text();cases=[]
    for line in log.splitlines():
        if line.startswith('FA_ENGINE_ORACLE '):
            source,answers=json.loads(line.removeprefix('FA_ENGINE_ORACLE '))
            assert len(answers)==2 and answers[0]==answers[1]
            cases.append({'source':source,'expected_old':answers[0]})
    assert len(cases)==5 and 'test result: ok. 1 passed;' in log
    output={'source_hash':subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip(),'cases':cases}
    (root/'target/fa-r3/engine-boundary-oracle.json').write_text(json.dumps(output,ensure_ascii=False,indent=2)+'\n')
    text=path.read_text();assert text.count(probe)==1;path.write_text(text.replace(probe,''))
    assert not subprocess.check_output(['git','diff','--',str(path)],text=True).strip()
    print(json.dumps(output,ensure_ascii=False,indent=2))
else:raise SystemExit('instrument or collect')
