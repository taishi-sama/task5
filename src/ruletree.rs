use std::sync::Arc;

use crate::fact::{Fact, Rule};
#[derive(Debug, Clone, PartialEq, Eq, Hash)]

pub struct RuleTree {
    sources: Vec<Source>,
    rule: Rule,
}
#[derive(Debug, Clone, PartialEq, Eq, Hash)]

pub enum Source {
    Rule(Arc<RuleTree>),
    BasicFact(Fact),
}
