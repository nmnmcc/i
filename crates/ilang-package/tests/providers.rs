use ilang_package::manifest::{parse_manifest, parse_dependency_manifest};
use ilang_package::providers::{Argument, Cell, Role, Target};

fn parse(body: &str) -> (Option<ilang_package::manifest::Manifest>, Vec<ilang_package::Diagnostic>) { parse_manifest(&format!("name = \"app\"\n{body}")) }
fn ok(body: &str) { let (_, d) = parse(body); assert!(d.is_empty(), "{d:?}"); }
fn bad(body: &str) { let (_, d) = parse(body); assert!(!d.is_empty(), "accepted {body}"); }
#[test] fn provider_empty() { ok("[providers]\n"); }
#[test] fn provider_shorthand() { ok("[providers.\"app/main.Service\"]\njs = \"app/main.make\"\n"); }
#[test] fn provider_host() { ok("[providers.\"app/main.Service\"]\nhost = {host=true}\n"); }
#[test] fn provider_factory() { ok("[providers.\"app/main.Service\"]\nnative = {factory=\"app/main.make\", arguments=[\"seed\",\"entry-root-key\"], seed_required=true}\n"); }
#[test] fn provider_adapter() { ok("[providers.\"app/main.Service\"]\nrole=\"runner\"\nadapter=\"app/main.run\"\n[provider_adapters.\"app/main.run\"]\nsupplies=[\"app/main.Service\"]\nrequires=[]\n"); }
#[test] fn provider_discriminant() { ok("[providers.\"app/main.Service\"]\ndiscriminant={js=\"app/main.A\",nodejs=\"app/main.A\",native=\"app/main.B\",wasi=\"app/main.B\",host=\"app/main.B\"}\n"); }
#[test] fn provider_synchronous_subset() { ok("[wasm]\nexports=[\"app/main.a\"]\nsynchronous=[\"app/main.a\"]\n"); }
#[test] fn provider_root_shape() { bad("providers=2\n"); }
#[test] fn provider_adapters_shape() { bad("provider_adapters=2\n"); }
#[test] fn provider_bad_service() { bad("[providers.\"bad\"]\n"); }
#[test] fn provider_row_shape() { bad("[providers]\n\"app/main.S\"=1\n"); }
#[test] fn provider_unknown_target() { bad("[providers.\"app/main.S\"]\nilang=\"app/main.f\"\n"); }
#[test] fn provider_bad_role() { bad("[providers.\"app/main.S\"]\nrole=\"other\"\n"); }
#[test] fn provider_role_type() { bad("[providers.\"app/main.S\"]\nrole=1\n"); }
#[test] fn provider_missing_adapter() { bad("[providers.\"app/main.S\"]\nrole=\"scheduler\"\n"); }
#[test] fn provider_unexpected_adapter() { bad("[providers.\"app/main.S\"]\nadapter=\"app/main.f\"\n"); }
#[test] fn provider_bad_adapter_path() { bad("[provider_adapters.\"bad\"]\n"); }
#[test] fn provider_adapter_shape() { bad("[provider_adapters]\n\"app/main.f\"=1\n"); }
#[test] fn provider_adapter_unknown() { bad("[provider_adapters.\"app/main.f\"]\nextra=[]\n"); }
#[test] fn provider_supplies_shape() { bad("[provider_adapters.\"app/main.f\"]\nsupplies=1\n"); }
#[test] fn provider_requires_path() { bad("[provider_adapters.\"app/main.f\"]\nrequires=[\"bad\"]\n"); }
#[test] fn provider_cell_shape() { bad("[providers.\"app/main.S\"]\njs=2\n"); }
#[test] fn provider_cell_unknown() { bad("[providers.\"app/main.S\"]\njs={factory=\"app/main.f\", extra=true}\n"); }
#[test] fn provider_factory_path() { bad("[providers.\"app/main.S\"]\njs=\"bad\"\n"); }
#[test] fn provider_cell_empty() { bad("[providers.\"app/main.S\"]\njs={}\n"); }
#[test] fn provider_host_false() { bad("[providers.\"app/main.S\"]\njs={host=false}\n"); }
#[test] fn provider_host_combined() { bad("[providers.\"app/main.S\"]\njs={host=true,factory=\"app/main.f\"}\n"); }
#[test] fn provider_arguments_shape() { bad("[providers.\"app/main.S\"]\njs={factory=\"app/main.f\",arguments=1}\n"); }
#[test] fn provider_argument_unknown() { bad("[providers.\"app/main.S\"]\njs={factory=\"app/main.f\",arguments=[\"other\"]}\n"); }
#[test] fn provider_argument_duplicate() { bad("[providers.\"app/main.S\"]\njs={factory=\"app/main.f\",arguments=[\"seed\",\"seed\"]}\n"); }
#[test] fn provider_seed_without_argument() { bad("[providers.\"app/main.S\"]\njs={factory=\"app/main.f\",seed_required=true}\n"); }
#[test] fn provider_seed_type() { bad("[providers.\"app/main.S\"]\njs={factory=\"app/main.f\",arguments=[\"seed\"],seed_required=1}\n"); }
#[test] fn provider_discriminant_shape() { bad("[providers.\"app/main.S\"]\ndiscriminant=1\n"); }
#[test] fn provider_discriminant_missing() { bad("[providers.\"app/main.S\"]\ndiscriminant={js=\"app/main.A\"}\n"); }
#[test] fn provider_discriminant_target() { bad("[providers.\"app/main.S\"]\ndiscriminant={ilang=\"app/main.A\"}\n"); }
#[test] fn provider_synchronous_not_exported() { bad("[wasm]\nexports=[]\nsynchronous=[\"app/main.a\"]\n"); }
#[test] fn provider_duplicate_row() { bad("[providers.\"app/main.S\"]\n[providers.\"app/main.S\"]\n"); }
#[test] fn provider_duplicate_cell() { bad("[providers.\"app/main.S\"]\njs=\"app/main.f\"\njs=\"app/main.g\"\n"); }
#[test] fn provider_typed_plan() { let (m,d)=parse("[providers.\"app/main.S\"]\nrole=\"runner\"\nadapter=\"app/main.run\"\njs={factory=\"app/main.f\",arguments=[\"seed\"],seed_required=true}\n"); assert!(d.is_empty(),"{d:?}"); let m=m.unwrap(); let r=&m.provider_plan.rows[0]; assert_eq!(r.role,Role::Runner); assert_eq!(r.cells[&Target::Js],Cell::Factory {factory:"app/main.f".into(),arguments:vec![Argument::Seed],seed_required:true}); }
#[test] fn provider_dependency_parsed() { let (m,d)=parse_dependency_manifest("name=\"dep\"\n[providers.\"dep/main.S\"]\nhost=\"dep/main.f\"\n"); assert!(d.is_empty(),"{d:?}"); assert_eq!(m.unwrap().provider_plan.rows.len(),1); }
