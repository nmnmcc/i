Original model programs, copied byte for byte from the round2 archive.

The `.txt` suffix keeps these intentionally invalid programs out of source
formatting and snippet discovery. The CLI regression test supplies `src/main.i`
and a manifest pointing at this worktree’s std; the program text is unchanged.

After merging E4, `t5-a1-r3.stderr` also retains W0505 at 162:7 and W0501 at
166:8. The original program binds `Ok(source) = bit_array.to_string(bytes)`
and later matches nested `Ok` values. The E4 table in
`docs/design/effects-e4.md` changes `bit_array.to_string` to return `String`
through `Fail(Nil)`, and `json.parse` to return `Json` through `Fail(JsonError)`.
Its controlling paragraph says: “An operation that may fail reports through
`Fail`; `Result` is data used after `catch`.” The input and both grouping
errors are preserved; these extra warnings pin the current API's behavior.

- `t1-a1-r3.txt`: `/home/agent/Programming/i/.rd/cache/ai-eval/round2/runs/subjects/gpt-6-luna/condition-2/T1/i/attempt-1/execution-00/round-3/project/src/main.i`
  SHA-256: `7a1c3daca5fe6657b0e004297b4ff47397a058a3cea8cf6918748bc577242b31`
- `t1-a2-r3.txt`: `/home/agent/Programming/i/.rd/cache/ai-eval/round2/runs/subjects/gpt-6-luna/condition-2/T1/i/attempt-2/execution-00/round-3/project/src/main.i`
  SHA-256: `d69857ff856de94865a37f1059b5d833fb441f51148d70ce35b5645e47e0163f`
- `t2-a1-r3.txt`: `/home/agent/Programming/i/.rd/cache/ai-eval/round2/runs/subjects/gpt-6-luna/condition-2/T2/i/attempt-1/execution-00/round-3/project/src/main.i`
  SHA-256: `32eae626ef9d24673bf6f5f53d6eeb8c6140403ecf5eeab421d9b6dd4246e4ea`
- `t2-a2-r3.txt`: `/home/agent/Programming/i/.rd/cache/ai-eval/round2/runs/subjects/gpt-6-luna/condition-2/T2/i/attempt-2/execution-00/round-3/project/src/main.i`
  SHA-256: `e199be9c11fa0acafd70f9c168fdd507d7d83eab087d88b41d90da5eb277d222`
- `t5-a1-r3.txt`: `/home/agent/Programming/i/.rd/cache/ai-eval/round2/runs/subjects/gpt-6-luna/condition-2/T5/i/attempt-1/execution-00/round-3/project/src/main.i`
  SHA-256: `3ff3e7b105dc0ce205b2f182f8ea395ba89ddefa2b197c7dbac9c7a8a64a611c`
- `t5-a2-r3.txt`: `/home/agent/Programming/i/.rd/cache/ai-eval/round2/runs/subjects/gpt-6-luna/condition-2/T5/i/attempt-2/execution-00/round-3/project/src/main.i`
  SHA-256: `b7608a43c70378b4874c00b6f2eba7b8f3cfe831913e8a5f89405b4059e1fbeb`
- `t5-a1-r1.txt`: `/home/agent/Programming/i/.rd/cache/ai-eval/round2/runs/subjects/gpt-6-luna/condition-2/T5/i/attempt-1/execution-00/round-1/project/src/main.i`
  SHA-256: `38cdf2229db4579e9707795046b2c705b8afec2f1d855ea86b304444518ad932`
- `t6-a2-r3.txt`: `/home/agent/Programming/i/.rd/cache/ai-eval/round2/runs/subjects/gpt-6-luna/condition-2/T6/i/attempt-2/execution-00/round-3/project/src/main.i`
  SHA-256: `9e7fa79954161a8acf89bd07c517c7c086bc55bd7d569dca1d3c9d33a5e9b081`
