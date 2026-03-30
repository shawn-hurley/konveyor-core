//! Konveyor rule definition types.
//!
//! These types represent the YAML rule format that Konveyor/kantra consumes.
//! They are used by the semver-analyzer to generate migration rules and can
//! be consumed by any tool that produces Konveyor-compatible rulesets.

use crate::fix::FixStrategyEntry;
use serde::{Deserialize, Serialize};

/// Ruleset metadata (written to `ruleset.yaml`).
#[derive(Debug, Serialize, Deserialize)]
pub struct KonveyorRuleset {
    pub name: String,
    pub description: String,
    pub labels: Vec<String>,
}

/// A single Konveyor rule.
#[derive(Debug, Serialize, Deserialize)]
pub struct KonveyorRule {
    #[serde(rename = "ruleID")]
    pub rule_id: String,
    pub labels: Vec<String>,
    pub effort: u32,
    pub category: String,
    pub description: String,
    pub message: String,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub links: Vec<KonveyorLink>,
    pub when: KonveyorCondition,
    /// Fix strategy for this rule. Not serialized to kantra YAML -- written
    /// separately to fix-strategies.json after consolidation.
    #[serde(skip)]
    pub fix_strategy: Option<FixStrategyEntry>,
}

/// A hyperlink attached to a rule.
#[derive(Debug, Serialize, Deserialize)]
pub struct KonveyorLink {
    pub url: String,
    pub title: String,
}

/// A Konveyor `when` condition.
///
/// Supports `builtin.filecontent` (regex), `builtin.json` (xpath),
/// `frontend.referenced` (AST-level, requires a frontend-analyzer-provider),
/// and `or`/`and` combinators.
#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum KonveyorCondition {
    FileContent {
        #[serde(rename = "builtin.filecontent")]
        filecontent: FileContentFields,
    },
    Json {
        #[serde(rename = "builtin.json")]
        json: JsonFields,
    },
    FrontendReferenced {
        #[serde(rename = "frontend.referenced")]
        referenced: FrontendReferencedFields,
    },
    FrontendCssClass {
        #[serde(rename = "frontend.cssclass")]
        cssclass: FrontendPatternFields,
    },
    FrontendCssVar {
        #[serde(rename = "frontend.cssvar")]
        cssvar: FrontendPatternFields,
    },
    Or {
        or: Vec<KonveyorCondition>,
    },
    And {
        and: Vec<KonveyorCondition>,
    },
    /// Negated `builtin.filecontent`: matches when the pattern is NOT found.
    FileContentNegated {
        #[serde(rename = "not")]
        negated: bool,
        #[serde(rename = "builtin.filecontent")]
        filecontent: FileContentFields,
    },
}

/// Fields for `frontend.cssclass` and `frontend.cssvar` conditions.
#[derive(Debug, Serialize, Deserialize)]
pub struct FrontendPatternFields {
    pub pattern: String,
}

/// Fields for a `builtin.filecontent` condition.
#[derive(Debug, Serialize, Deserialize)]
pub struct FileContentFields {
    pub pattern: String,
    #[serde(rename = "filePattern")]
    pub file_pattern: String,
}

/// Fields for a `builtin.json` condition.
#[derive(Debug, Serialize, Deserialize)]
pub struct JsonFields {
    pub xpath: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub filepaths: Option<Vec<String>>,
}

/// Fields for a `frontend.referenced` condition.
///
/// This condition requires a frontend-analyzer-provider gRPC server.
/// It performs AST-level symbol matching with location discriminators.
#[derive(Debug, Serialize, Deserialize)]
pub struct FrontendReferencedFields {
    /// Regex pattern for the symbol name.
    pub pattern: String,
    /// Where to look: IMPORT, JSX_COMPONENT, JSX_PROP, FUNCTION_CALL, TYPE_REFERENCE.
    pub location: String,
    /// Filter JSX props to only those on this component (regex).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub component: Option<String>,
    /// Filter JSX components to only those inside this parent (regex).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub parent: Option<String>,
    /// Filter by the parent component's import source (regex).
    #[serde(
        rename = "parentFrom",
        skip_serializing_if = "Option::is_none",
        default
    )]
    pub parent_from: Option<String>,
    /// Filter JSX prop values to only those matching this regex.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub value: Option<String>,
    /// Scope to imports from a specific package (e.g., `@patternfly/react-tokens`).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub from: Option<String>,
}

/// Extract all `FrontendReferencedFields` from a `KonveyorCondition`,
/// recursing into `Or`/`And` combinators.
pub fn extract_frontend_refs(condition: &KonveyorCondition) -> Vec<&FrontendReferencedFields> {
    match condition {
        KonveyorCondition::FrontendReferenced { referenced } => vec![referenced],
        KonveyorCondition::Or { or } => or.iter().flat_map(extract_frontend_refs).collect(),
        KonveyorCondition::And { and } => and.iter().flat_map(extract_frontend_refs).collect(),
        _ => vec![],
    }
}

/// Extract the file pattern from an existing condition (for reuse in consolidated rules).
pub fn extract_file_pattern_from_condition(condition: &KonveyorCondition) -> Option<String> {
    match condition {
        KonveyorCondition::FileContent { filecontent } => Some(filecontent.file_pattern.clone()),
        KonveyorCondition::Or { or } => or.first().and_then(extract_file_pattern_from_condition),
        _ => None,
    }
}

/// Deduplicate conditions by their JSON representation.
pub fn dedup_conditions(conditions: Vec<KonveyorCondition>) -> Vec<KonveyorCondition> {
    use std::collections::BTreeSet;
    let mut seen = BTreeSet::new();
    let mut unique = Vec::new();
    for cond in conditions {
        let key = serde_json::to_string(&cond).unwrap_or_default();
        if seen.insert(key) {
            unique.push(cond);
        }
    }
    unique
}
