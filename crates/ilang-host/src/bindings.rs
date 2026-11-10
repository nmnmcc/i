//! Out-of-band declaration permissions. Neither ABI metadata nor a manifest's
//! asserted names establish provenance. Approval is explicit embedding authority.

use crate::{Abi, Signature};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// One resolved declaration, retained before Wasm import coalescing.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BindingDeclaration {
    pub package: String,
    pub module: String,
    pub module_digest: String,
    /// Host type/field or the declared external name; aliases have distinct ids.
    pub declaration: String,
    pub namespace: String,
    pub name: String,
    /// `service` or `external`.
    pub kind: String,
    pub signature: Signature,
}

impl BindingDeclaration {
    pub fn from_json(json: &str) -> Result<Self> {
        Ok(serde_json::from_str(json)?)
    }
}

/// A compiler plan travels beside the exact final artifact, outside ilang.abi.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BindingPlan {
    pub artifact_digest: String,
    pub declarations: Vec<BindingDeclaration>,
}

pub fn artifact_digest(bytes: &[u8]) -> String {
    format!("sha256-{:x}", Sha256::digest(bytes))
}

impl BindingPlan {
    pub fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }
    pub fn new(bytes: &[u8], declarations: Vec<BindingDeclaration>) -> Self {
        Self {
            artifact_digest: artifact_digest(bytes),
            declarations,
        }
    }
    pub(crate) fn validate(&self, digest: &str, abi: &Abi) -> Result<()> {
        ensure!(
            self.artifact_digest == digest,
            "binding plan artifact digest does not match the module"
        );
        for declaration in &self.declarations {
            ensure!(
                !declaration.package.is_empty()
                    && !declaration.module.is_empty()
                    && !declaration.module_digest.is_empty()
                    && !declaration.declaration.is_empty(),
                "binding plan has an empty declaration identity"
            );
            let table = match declaration.kind.as_str() {
                "service" => &abi.services,
                "external" => &abi.externals,
                _ => anyhow::bail!("binding plan has an invalid import kind"),
            };
            ensure!(
                table
                    .get(&declaration.namespace)
                    .and_then(|fields| fields.get(&declaration.name))
                    == Some(&declaration.signature),
                "binding plan declaration {} does not match ABI import {} {}",
                declaration.declaration,
                declaration.namespace,
                declaration.name
            );
        }
        for (kind, table) in [("service", &abi.services), ("external", &abi.externals)] {
            for (namespace, fields) in table {
                for name in fields.keys() {
                    ensure!(
                        self.declarations.iter().any(|d| d.kind == kind
                            && d.namespace == *namespace
                            && d.name == *name),
                        "binding plan is missing import {namespace} {name}"
                    );
                }
            }
        }
        Ok(())
    }
}

#[derive(Clone, Default)]
pub(crate) struct Grants(Vec<(String, BindingDeclaration)>);
impl Grants {
    pub fn grant(&mut self, plan: &BindingPlan, declaration: &BindingDeclaration) {
        self.0
            .push((plan.artifact_digest.clone(), declaration.clone()));
    }
    pub fn allows(&self, plan: &BindingPlan, declaration: &BindingDeclaration) -> bool {
        self.0
            .iter()
            .any(|(digest, granted)| digest == &plan.artifact_digest && granted == declaration)
    }
    pub fn merge(&mut self, other: Self) {
        self.0.extend(other.0);
    }
}
