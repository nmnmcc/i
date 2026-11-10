# K4 corpus integration

Baseline: d655684bdb809c5a38bf86b61fc5ef26f5bc5392. 119 K4 known-failure entries promoted. No K4 known failure remains.

## Flipped entries

### Finite checker stress inputs exceed the corpus observation budget; acceptance has not been established by this run

- review-k4/perf/generic_chain_2048
- review-k4/perf/wide_generic_1024
- review-k4/perf/wide_generic_2048
- review-k4/perf/wide_generic_512
- review-k4c/cases/stress_wide_open_1024
- review-k4c/cases/stress_wide_open_512
- review-k4d/perf/mutual100
- review-k4d/perf/mutual150
- review-k4d/perf/mutual200
- review-k4d/perf/mutual300
- review-k4d/perf/mutual50
- review-k4d/perf/runchain4000
- review-k4d/perf/wideann200
- review-k4d/perf/wideann80
- review-k4s/wide_plain_1024
- review-k4s/wide_plain_512

### K4 review corpus: incompatible service provision or an unprovided generic service must be rejected before entry execution

- review-k4/cases/provision_closure_False
- review-k4/cases/provision_direct_False
- review-k4/cases/provision_helper_False
- review-k4/cases/provision_inferred_False
- review-k4c/cases/closure_statement
- review-k4c/cases/missing_0_0_0_2
- review-k4c/cases/missing_0_1_0_0
- review-k4c/cases/missing_0_1_0_1
- review-k4c/cases/missing_0_1_0_2
- review-k4c/cases/missing_0_2_0_2
- review-k4c/cases/missing_0_7_0_0
- review-k4c/cases/missing_0_7_0_1
- review-k4c/cases/missing_0_7_0_2
- review-k4c/cases/missing_2_0_0_2
- review-k4c/cases/missing_2_1_0_0
- review-k4c/cases/missing_2_1_0_1
- review-k4c/cases/missing_2_1_0_2
- review-k4c/cases/missing_2_2_0_2
- review-k4c/cases/missing_2_7_0_0
- review-k4c/cases/missing_2_7_0_1
- review-k4c/cases/missing_2_7_0_2
- review-k4c/cases/missing_3_0_0_2
- review-k4c/cases/missing_3_1_0_0
- review-k4c/cases/missing_3_1_0_1
- review-k4c/cases/missing_3_1_0_2
- review-k4c/cases/missing_3_2_0_2
- review-k4c/cases/missing_3_7_0_0
- review-k4c/cases/missing_3_7_0_1
- review-k4c/cases/missing_3_7_0_2
- review-k4d/_loose/p/b4
- review-k4d/_loose/p/b7
- review-k4d/_loose/p/r2
- review-k4d/_loose/p/r3
- review-k4d/proj/b4
- review-k4d/proj/b7
- review-k4d/proj/r2
- review-k4d/proj/r3
- review-k4s/min_main_6_0

### K4d F1/F2: annotated handler drops same-service obligation or compares row arguments covariantly

- review-k4d/_loose/p/x1
- review-k4d/_loose/p/x4
- review-k4d/_loose/p/x6
- review-k4d/_loose/p/x7
- review-k4d/_loose/p/x8
- review-k4d/_loose/p/z1
- review-k4d/_loose/p/z4
- review-k4d/proj/x1
- review-k4d/proj/x4
- review-k4d/proj/x6
- review-k4d/proj/x7
- review-k4d/proj/x8
- review-k4d/proj/z1
- review-k4d/proj/z4

### K4d F2: hidden Dep obligation must remain in the entry environment and be rejected when not supplied

- review-k4d/_loose/p/g_ann_SDep
- review-k4d/_loose/p/g_inf_SDep
- review-k4d/_loose/p/m3
- review-k4d/_loose/p/m4
- review-k4d/_loose/p/r6
- review-k4d/proj/g_ann_SDep
- review-k4d/proj/g_inf_SDep
- review-k4d/proj/m3
- review-k4d/proj/m4
- review-k4d/proj/r6

### K4d F3: finite generic service/field-access types must not be rejected as structural recursion

- review-k4d/_loose/p/q21
- review-k4d/_loose/p/q22
- review-k4d/_loose/p/q23
- review-k4d/_loose/p/q25
- review-k4d/_loose/p/q31
- review-k4d/_loose/p/q32
- review-k4d/_loose/p/q33
- review-k4d/_loose/p/q34
- review-k4d/_loose/p/q35
- review-k4d/_loose/p/q36
- review-k4d/_loose/p/q37
- review-k4d/_loose/p/q38
- review-k4d/_loose/p/q39
- review-k4d/_loose/p/q40
- review-k4d/_loose/p/r4
- review-k4d/_loose/p/w14
- review-k4d/_loose/p/w15
- review-k4d/_loose/p/w16
- review-k4d/_loose/p/w6
- review-k4d/_loose/p/w7
- review-k4d/proj/q21
- review-k4d/proj/q22
- review-k4d/proj/q23
- review-k4d/proj/q25
- review-k4d/proj/q31
- review-k4d/proj/q32
- review-k4d/proj/q33
- review-k4d/proj/q34
- review-k4d/proj/q35
- review-k4d/proj/q36
- review-k4d/proj/q37
- review-k4d/proj/q38
- review-k4d/proj/q39
- review-k4d/proj/q40
- review-k4d/proj/r4
- review-k4d/proj/w14
- review-k4d/proj/w15
- review-k4d/proj/w16
- review-k4d/proj/w6
- review-k4d/proj/w7

### K4s P1: annotated local function must not erase an unprovided service from main

- review-k4s/soundness

## Baseline-accepted programs now rejected

| Program | Diagnostic | Reason |
| --- | --- | --- |
| review-k4/cases/conflict_2046229799906290556 | E0515 | §4.5 rule 4: incompatible forms of one service constructor |
| review-k4/cases/conflict_4759503553685934015 | E0515 | §4.5 rule 4: incompatible forms of one service constructor |
| review-k4/cases/hidden_conflict | E0515 | §4.5 rule 4: incompatible forms of one service constructor |
| review-k4/cases/provision_closure_False | E0515 | §4.5 rule 4: incompatible forms of one service constructor |
| review-k4/cases/provision_direct_False | E0515 | §4.5 rule 4: incompatible forms of one service constructor |
| review-k4/cases/provision_helper_False | E0515 | §4.5 rule 4: incompatible forms of one service constructor |
| review-k4/cases/provision_inferred_False | E0515 | §4.5 rule 4: incompatible forms of one service constructor |
| review-k4c/cases/closure_statement | E0522 | §§4.8/9.2/10.2: an entry retains an unprovided service |
| review-k4c/cases/missing_0_0_0_2 | E0522 | §§4.8/9.2/10.2: an entry retains an unprovided service |
| review-k4c/cases/missing_0_1_0_0 | E0522 | §§4.8/9.2/10.2: an entry retains an unprovided service |
| review-k4c/cases/missing_0_1_0_1 | E0522 | §§4.8/9.2/10.2: an entry retains an unprovided service |
| review-k4c/cases/missing_0_1_0_2 | E0522 | §§4.8/9.2/10.2: an entry retains an unprovided service |
| review-k4c/cases/missing_0_1_1_0 | E0522 | §§4.8/9.2/10.2: an entry retains an unprovided service |
| review-k4c/cases/missing_0_1_1_1 | E0522 | §§4.8/9.2/10.2: an entry retains an unprovided service |
| review-k4c/cases/missing_0_1_1_2 | E0522 | §§4.8/9.2/10.2: an entry retains an unprovided service |
| review-k4c/cases/missing_0_2_0_2 | E0522 | §§4.8/9.2/10.2: an entry retains an unprovided service |
| review-k4c/cases/missing_0_4_1_0 | E0522 | §§4.8/9.2/10.2: an entry retains an unprovided service |
| review-k4c/cases/missing_0_4_1_1 | E0522 | §§4.8/9.2/10.2: an entry retains an unprovided service |
| review-k4c/cases/missing_0_4_1_2 | E0522 | §§4.8/9.2/10.2: an entry retains an unprovided service |
| review-k4c/cases/missing_0_7_0_0 | E0522 | §§4.8/9.2/10.2: an entry retains an unprovided service |
| review-k4c/cases/missing_0_7_0_1 | E0522 | §§4.8/9.2/10.2: an entry retains an unprovided service |
| review-k4c/cases/missing_0_7_0_2 | E0522 | §§4.8/9.2/10.2: an entry retains an unprovided service |
| review-k4c/cases/missing_0_7_1_0 | E0522 | §§4.8/9.2/10.2: an entry retains an unprovided service |
| review-k4c/cases/missing_0_7_1_1 | E0522 | §§4.8/9.2/10.2: an entry retains an unprovided service |
| review-k4c/cases/missing_0_7_1_2 | E0522 | §§4.8/9.2/10.2: an entry retains an unprovided service |
| review-k4c/cases/missing_2_0_0_2 | E0522 | §§4.8/9.2/10.2: an entry retains an unprovided service |
| review-k4c/cases/missing_2_1_0_0 | E0522 | §§4.8/9.2/10.2: an entry retains an unprovided service |
| review-k4c/cases/missing_2_1_0_1 | E0522 | §§4.8/9.2/10.2: an entry retains an unprovided service |
| review-k4c/cases/missing_2_1_0_2 | E0522 | §§4.8/9.2/10.2: an entry retains an unprovided service |
| review-k4c/cases/missing_2_1_1_0 | E0522 | §§4.8/9.2/10.2: an entry retains an unprovided service |
| review-k4c/cases/missing_2_1_1_1 | E0522 | §§4.8/9.2/10.2: an entry retains an unprovided service |
| review-k4c/cases/missing_2_1_1_2 | E0522 | §§4.8/9.2/10.2: an entry retains an unprovided service |
| review-k4c/cases/missing_2_2_0_2 | E0522 | §§4.8/9.2/10.2: an entry retains an unprovided service |
| review-k4c/cases/missing_2_4_1_0 | E0522 | §§4.8/9.2/10.2: an entry retains an unprovided service |
| review-k4c/cases/missing_2_4_1_1 | E0522 | §§4.8/9.2/10.2: an entry retains an unprovided service |
| review-k4c/cases/missing_2_4_1_2 | E0522 | §§4.8/9.2/10.2: an entry retains an unprovided service |
| review-k4c/cases/missing_2_7_0_0 | E0522 | §§4.8/9.2/10.2: an entry retains an unprovided service |
| review-k4c/cases/missing_2_7_0_1 | E0522 | §§4.8/9.2/10.2: an entry retains an unprovided service |
| review-k4c/cases/missing_2_7_0_2 | E0522 | §§4.8/9.2/10.2: an entry retains an unprovided service |
| review-k4c/cases/missing_2_7_1_0 | E0522 | §§4.8/9.2/10.2: an entry retains an unprovided service |
| review-k4c/cases/missing_2_7_1_1 | E0522 | §§4.8/9.2/10.2: an entry retains an unprovided service |
| review-k4c/cases/missing_2_7_1_2 | E0522 | §§4.8/9.2/10.2: an entry retains an unprovided service |
| review-k4c/cases/missing_3_0_0_2 | E0522 | §§4.8/9.2/10.2: an entry retains an unprovided service |
| review-k4c/cases/missing_3_1_0_0 | E0522 | §§4.8/9.2/10.2: an entry retains an unprovided service |
| review-k4c/cases/missing_3_1_0_1 | E0522 | §§4.8/9.2/10.2: an entry retains an unprovided service |
| review-k4c/cases/missing_3_1_0_2 | E0522 | §§4.8/9.2/10.2: an entry retains an unprovided service |
| review-k4c/cases/missing_3_1_1_0 | E0522 | §§4.8/9.2/10.2: an entry retains an unprovided service |
| review-k4c/cases/missing_3_1_1_1 | E0522 | §§4.8/9.2/10.2: an entry retains an unprovided service |
| review-k4c/cases/missing_3_1_1_2 | E0522 | §§4.8/9.2/10.2: an entry retains an unprovided service |
| review-k4c/cases/missing_3_2_0_2 | E0522 | §§4.8/9.2/10.2: an entry retains an unprovided service |
| review-k4c/cases/missing_3_7_0_0 | E0522 | §§4.8/9.2/10.2: an entry retains an unprovided service |
| review-k4c/cases/missing_3_7_0_1 | E0522 | §§4.8/9.2/10.2: an entry retains an unprovided service |
| review-k4c/cases/missing_3_7_0_2 | E0522 | §§4.8/9.2/10.2: an entry retains an unprovided service |
| review-k4c/cases/missing_3_7_1_0 | E0522 | §§4.8/9.2/10.2: an entry retains an unprovided service |
| review-k4c/cases/missing_3_7_1_1 | E0522 | §§4.8/9.2/10.2: an entry retains an unprovided service |
| review-k4c/cases/missing_3_7_1_2 | E0522 | §§4.8/9.2/10.2: an entry retains an unprovided service |
| review-k4c/cases/order_hidden_conflict_0 | E0515 | §4.5 rule 4: incompatible forms of one service constructor |
| review-k4c/cases/order_hidden_conflict_1 | E0515 | §4.5 rule 4: incompatible forms of one service constructor |
| review-k4c/cases/order_hidden_conflict_10 | E0515 | §4.5 rule 4: incompatible forms of one service constructor |
| review-k4c/cases/order_hidden_conflict_11 | E0515 | §4.5 rule 4: incompatible forms of one service constructor |
| review-k4c/cases/order_hidden_conflict_12 | E0515 | §4.5 rule 4: incompatible forms of one service constructor |
| review-k4c/cases/order_hidden_conflict_13 | E0515 | §4.5 rule 4: incompatible forms of one service constructor |
| review-k4c/cases/order_hidden_conflict_14 | E0515 | §4.5 rule 4: incompatible forms of one service constructor |
| review-k4c/cases/order_hidden_conflict_15 | E0515 | §4.5 rule 4: incompatible forms of one service constructor |
| review-k4c/cases/order_hidden_conflict_16 | E0515 | §4.5 rule 4: incompatible forms of one service constructor |
| review-k4c/cases/order_hidden_conflict_17 | E0515 | §4.5 rule 4: incompatible forms of one service constructor |
| review-k4c/cases/order_hidden_conflict_18 | E0515 | §4.5 rule 4: incompatible forms of one service constructor |
| review-k4c/cases/order_hidden_conflict_19 | E0515 | §4.5 rule 4: incompatible forms of one service constructor |
| review-k4c/cases/order_hidden_conflict_2 | E0515 | §4.5 rule 4: incompatible forms of one service constructor |
| review-k4c/cases/order_hidden_conflict_20 | E0515 | §4.5 rule 4: incompatible forms of one service constructor |
| review-k4c/cases/order_hidden_conflict_21 | E0515 | §4.5 rule 4: incompatible forms of one service constructor |
| review-k4c/cases/order_hidden_conflict_22 | E0515 | §4.5 rule 4: incompatible forms of one service constructor |
| review-k4c/cases/order_hidden_conflict_23 | E0515 | §4.5 rule 4: incompatible forms of one service constructor |
| review-k4c/cases/order_hidden_conflict_24 | E0515 | §4.5 rule 4: incompatible forms of one service constructor |
| review-k4c/cases/order_hidden_conflict_25 | E0515 | §4.5 rule 4: incompatible forms of one service constructor |
| review-k4c/cases/order_hidden_conflict_26 | E0515 | §4.5 rule 4: incompatible forms of one service constructor |
| review-k4c/cases/order_hidden_conflict_27 | E0515 | §4.5 rule 4: incompatible forms of one service constructor |
| review-k4c/cases/order_hidden_conflict_28 | E0515 | §4.5 rule 4: incompatible forms of one service constructor |
| review-k4c/cases/order_hidden_conflict_29 | E0515 | §4.5 rule 4: incompatible forms of one service constructor |
| review-k4c/cases/order_hidden_conflict_3 | E0515 | §4.5 rule 4: incompatible forms of one service constructor |
| review-k4c/cases/order_hidden_conflict_30 | E0515 | §4.5 rule 4: incompatible forms of one service constructor |
| review-k4c/cases/order_hidden_conflict_31 | E0515 | §4.5 rule 4: incompatible forms of one service constructor |
| review-k4c/cases/order_hidden_conflict_4 | E0515 | §4.5 rule 4: incompatible forms of one service constructor |
| review-k4c/cases/order_hidden_conflict_5 | E0515 | §4.5 rule 4: incompatible forms of one service constructor |
| review-k4c/cases/order_hidden_conflict_6 | E0515 | §4.5 rule 4: incompatible forms of one service constructor |
| review-k4c/cases/order_hidden_conflict_7 | E0515 | §4.5 rule 4: incompatible forms of one service constructor |
| review-k4c/cases/order_hidden_conflict_8 | E0515 | §4.5 rule 4: incompatible forms of one service constructor |
| review-k4c/cases/order_hidden_conflict_9 | E0515 | §4.5 rule 4: incompatible forms of one service constructor |
| review-k4d/_loose/p/b4 | E0522 | §§4.8/9.2/10.2: an entry retains an unprovided service |
| review-k4d/_loose/p/b7 | E0522 | §§4.8/9.2/10.2: an entry retains an unprovided service |
| review-k4d/_loose/p/f2 | E0515 | §4.5 rule 4: incompatible forms of one service constructor |
| review-k4d/_loose/p/f3 | E0515 | §4.5 rule 4: incompatible forms of one service constructor |
| review-k4d/_loose/p/f4 | E0515 | §4.5 rule 4: incompatible forms of one service constructor |
| review-k4d/_loose/p/g_inf_SDep | E0515 | §4.5 rule 4: incompatible forms of one service constructor |
| review-k4d/_loose/p/m3 | E0515 | §4.5 rule 4: incompatible forms of one service constructor |
| review-k4d/_loose/p/m4 | E0515 | §4.5 rule 4: incompatible forms of one service constructor |
| review-k4d/_loose/p/r2 | E0522 | §§4.8/9.2/10.2: an entry retains an unprovided service |
| review-k4d/_loose/p/r3 | E0522 | §§4.8/9.2/10.2: an entry retains an unprovided service |
| review-k4d/_loose/p/r6 | E0515 | §4.5 rule 4: incompatible forms of one service constructor |
| review-k4d/_loose/p/z1 | E0515 | §4.5 rule 4: incompatible forms of one service constructor |
| review-k4d/_loose/p/z4 | E0515 | §4.5 rule 4: incompatible forms of one service constructor |
| review-k4d/proj/b4 | E0522 | §§4.8/9.2/10.2: an entry retains an unprovided service |
| review-k4d/proj/b7 | E0522 | §§4.8/9.2/10.2: an entry retains an unprovided service |
| review-k4d/proj/f2 | E0515 | §4.5 rule 4: incompatible forms of one service constructor |
| review-k4d/proj/f3 | E0515 | §4.5 rule 4: incompatible forms of one service constructor |
| review-k4d/proj/f4 | E0515 | §4.5 rule 4: incompatible forms of one service constructor |
| review-k4d/proj/g_inf_SDep | E0515 | §4.5 rule 4: incompatible forms of one service constructor |
| review-k4d/proj/m3 | E0515 | §4.5 rule 4: incompatible forms of one service constructor |
| review-k4d/proj/m4 | E0515 | §4.5 rule 4: incompatible forms of one service constructor |
| review-k4d/proj/r2 | E0522 | §§4.8/9.2/10.2: an entry retains an unprovided service |
| review-k4d/proj/r3 | E0522 | §§4.8/9.2/10.2: an entry retains an unprovided service |
| review-k4d/proj/r6 | E0515 | §4.5 rule 4: incompatible forms of one service constructor |
| review-k4d/proj/z1 | E0515 | §4.5 rule 4: incompatible forms of one service constructor |
| review-k4d/proj/z4 | E0515 | §4.5 rule 4: incompatible forms of one service constructor |
| review-k4s/hidden_conflict_closure | E0515 | §4.5 rule 4: incompatible forms of one service constructor |
| review-k4s/hidden_conflict_nested | E0515 | §4.5 rule 4: incompatible forms of one service constructor |
| review-k4s/min_main_6_0 | E0522 | §§4.8/9.2/10.2: an entry retains an unprovided service |
| review-k4s/mixed_order_1 | E0515 | §4.5 rule 4: incompatible forms of one service constructor |

## Every changed entry outside K4

| Program | Reason |
| --- | --- |
| review-js4/settings | §4.4: only the order of Await, Fs and Owner changes; all runtime phases are identical |
| review-std2/annotated | Original K4/Std-2 defect: finite generic service inference now accepts or publishes the formerly missing example scheme (§4.7); startup succeeds at both optimization levels |
| review-std2/client_nil_error | Original K4/Std-2 defect: finite generic service inference now accepts or publishes the formerly missing example scheme (§4.7); startup succeeds at both optimization levels |
| review-std2/client_no_marker | Original K4/Std-2 defect: finite generic service inference now accepts or publishes the formerly missing example scheme (§4.7); startup succeeds at both optimization levels |
| review-std2/error | Original K4/Std-2 defect: finite generic service inference now accepts or publishes the formerly missing example scheme (§4.7); startup succeeds at both optimization levels |
| review-std2/full | Original K4/Std-2 defect: finite generic service inference now accepts or publishes the formerly missing example scheme (§4.7); startup succeeds at both optimization levels |
| review-std2/minimal_client | Original K4/Std-2 defect: finite generic service inference now accepts or publishes the formerly missing example scheme (§4.7); startup succeeds at both optimization levels |
| review-std2/minimal_client_final | Original K4/Std-2 defect: finite generic service inference now accepts or publishes the formerly missing example scheme (§4.7); startup succeeds at both optimization levels |
| review-std2/pure_catch | Original K4/Std-2 defect: finite generic service inference now accepts or publishes the formerly missing example scheme (§4.7); startup succeeds at both optimization levels |
| review-std2/pure_wrap | Original K4/Std-2 defect: finite generic service inference now accepts or publishes the formerly missing example scheme (§4.7); startup succeeds at both optimization levels |
| review-std2/replacement | F2: the same E0515 now names the complete S(Int)/S(String) forms; shown types and rejection are preserved |
| review-std2/unsafe | F2: Sink carries a row; Consumer(Int) and Consumer(Never) must be equal, so E0515 replaces E0501; rejection is preserved |
| review-t1/stress-keys-10000 | Only censored runtime phases complete under the longer observation budget; checker types/diagnostics and completed outputs are identical; no K4 speedup claimed |
| review-t1/stress-keys-2000 | Only censored runtime phases complete under the longer observation budget; checker types/diagnostics and completed outputs are identical; no K4 speedup claimed |

## Every benchmark workload

Wall/CPU are seconds; RSS is bytes. A censored baseline has no completed-cost ratio.

| Workload | Wall before → after | CPU before → after | RSS before → after | Censored before/after |
| --- | ---: | ---: | ---: | --- |
| startup | 0.006706 → 0.007898 | 0.000000 → 0.000000 | 12201984 → 12562432 | [False, False] |
| alias-64-ordinary-forward | 0.006932 → 0.008942 | 0.000000 → 0.000000 | 12148736 → 12402688 | [False, False] |
| alias-64-ordinary-reverse | 0.007730 → 0.007868 | 0.000000 → 0.000000 | 12247040 → 12427264 | [False, False] |
| alias-64-value-forward | 0.007773 → 0.008807 | 0.000000 → 0.000000 | 12464128 → 12840960 | [False, False] |
| alias-64-value-reverse | 0.007766 → 0.009945 | 0.000000 → 0.000000 | 12537856 → 12840960 | [False, False] |
| alias-256-ordinary-forward | 0.007770 → 0.009018 | 0.000000 → 0.000000 | 12447744 → 12713984 | [False, False] |
| alias-256-ordinary-reverse | 0.007764 → 0.009951 | 0.000000 → 0.000000 | 12451840 → 12640256 | [False, False] |
| alias-256-value-forward | 0.008863 → 0.009469 | 0.000000 → 0.000000 | 12771328 → 12951552 | [False, False] |
| alias-256-value-reverse | 0.008848 → 0.009981 | 0.000000 → 0.000000 | 12587008 → 12926976 | [False, False] |
| alias-1024-ordinary-forward | 0.011944 → 0.013130 | 0.000000 → 0.000000 | 12926976 → 13053952 | [False, False] |
| alias-1024-ordinary-reverse | 0.012051 → 0.014212 | 0.000000 → 0.000000 | 13045760 → 13090816 | [False, False] |
| alias-1024-value-forward | 0.013050 → 0.013460 | 0.000000 → 0.000000 | 13520896 → 13791232 | [False, False] |
| alias-1024-value-reverse | 0.013172 → 0.013450 | 0.000000 → 0.000000 | 13426688 → 13799424 | [False, False] |
| alias-4096-ordinary-forward | 0.028320 → 0.030413 | 0.010000 → 0.020000 | 15323136 → 15540224 | [False, False] |
| alias-4096-ordinary-reverse | 0.028150 → 0.029811 | 0.020000 → 0.020000 | 15613952 → 15966208 | [False, False] |
| alias-4096-value-forward | 0.028078 → 0.033542 | 0.020000 → 0.020000 | 16760832 → 17645568 | [False, False] |
| alias-4096-value-reverse | 0.028147 → 0.030471 | 0.010000 → 0.020000 | 17059840 → 18022400 | [False, False] |
| literal-union-16 | 0.006748 → 0.007964 | 0.000000 → 0.000000 | 12337152 → 12296192 | [False, False] |
| clauses-16 | 0.006703 → 0.009061 | 0.000000 → 0.000000 | 12283904 → 12300288 | [False, False] |
| wide-union-16 | 0.006740 → 0.009001 | 0.000000 → 0.000000 | 12423168 → 12341248 | [False, False] |
| literal-union-64 | 0.007933 → 0.008909 | 0.000000 → 0.000000 | 12201984 → 12554240 | [False, False] |
| clauses-64 | 0.007764 → 0.009272 | 0.000000 → 0.000000 | 12500992 → 12431360 | [False, False] |
| wide-union-64 | 0.008878 → 0.010232 | 0.000000 → 0.000000 | 12419072 → 12533760 | [False, False] |
| literal-union-256 | 0.007761 → 0.009972 | 0.000000 → 0.000000 | 12447744 → 12546048 | [False, False] |
| clauses-256 | 0.008813 → 0.010705 | 0.000000 → 0.000000 | 13029376 → 13197312 | [False, False] |
| wide-union-256 | 0.028290 → 0.033288 | 0.020000 → 0.020000 | 12677120 → 12730368 | [False, False] |
| literal-union-1024 | 0.010084 → 0.012184 | 0.000000 → 0.000000 | 12767232 → 13074432 | [False, False] |
| clauses-1024 | 0.021004 → 0.024027 | 0.010000 → 0.010000 | 15810560 → 16130048 | [False, False] |
| wide-union-1024 | 0.865795 → 1.105737 | 0.840000 → 1.070000 | 14323712 → 14417920 | [False, False] |
| service-ring-100 | 10.167917 → 0.393885 | 10.010000 → 0.370000 | 4065640448 → 17588224 | [True, False] |
| std | 0.305605 → 0.330411 | 0.290000 → 0.310000 | 54218752 → 54849536 | [False, False] |

Benchmark output hashes differ for: service-ring-100, std.
Exact hashes and all timing/counter differences are in k4-integration.json.

Long-generation result differences from the stored long run: {}.
