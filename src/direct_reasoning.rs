use std::{collections::{HashSet, HashMap}, sync::{Arc, RwLock}};

use egui_graphs::Graph;
use petgraph::{Directed, stable_graph::StableGraph};

use crate::{
    engine::Engine,
    fact::{Fact, Rule},
};
#[derive(Debug, Clone)]
pub struct DirectReasoning {
    rules: Arc<Engine>,
    current_facts: HashSet<Fact>,
    target_fact: Fact,
    used_rules: Vec<Rule>,
    unused_rules: HashSet<Rule>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StepResult {
    FoundAfter(Rule, Fact),
    Applied(Rule),
    NotProved,
}
impl DirectReasoning {
    pub fn new(rules: Arc<Engine>, target_fact: Fact) -> Self {
        DirectReasoning {
            rules: rules.clone(),
            current_facts: rules.starting_facts.iter().cloned().collect(),
            target_fact,
            used_rules: vec![],
            unused_rules: rules.rules.iter().cloned().collect(),
        }
    }
    pub fn update_hashmap(&self, color: &NodeColoring){
        let mut c = color.facts.write().unwrap();
        let mut r = color.rules.write().unwrap();

        for i in &self.rules.all_possible_facts {
            c.insert(i.clone(), FactState::None);
        }
        for i in &self.current_facts {
            c.insert(i.clone(), FactState::Visited);
        }
        for i in &self.rules.starting_facts {
            c.insert(i.clone(), FactState::Starting);
        }
        if self.current_facts.contains(&self.target_fact) {
            c.insert(self.target_fact.clone(), FactState::TargetVisited);
        }
        else {
            c.insert(self.target_fact.clone(), FactState::Target);

        }
        for i in &self.unused_rules {
            r.insert(i.clone(), RuleState::None);
        }
        for i in &self.used_rules {
            r.insert(i.clone(), RuleState::Visited);
        }
        
        
    }
    pub fn step(&mut self) -> StepResult {
        if let Some(r) = self
            .unused_rules
            .iter().find(|x| x.match_requirement(&self.current_facts))
        {
            let r = r.clone();
            let f = r.out.clone();
            self.current_facts.insert(f.clone());
            self.unused_rules.remove(&r);
            self.used_rules.push(r.clone());
            if f == self.target_fact {
                return StepResult::FoundAfter(r, f);
            } else {
                return StepResult::Applied(r);
            }
        }
        StepResult::NotProved
    }
}
#[derive(Debug, Clone)]
pub struct StatedFact {
    pub fact: Fact,
    pub state: Arc<RwLock<HashMap<Fact, FactState>>>
}
#[derive(Debug, Clone)]
pub struct StatedRule {
    pub rule: Rule,
    pub state: Arc<RwLock<HashMap<Rule, RuleState>>>
}
#[derive(Debug, Clone)]
pub enum GraphNode {
    Rule(StatedRule),
    Fact(StatedFact)    
}
#[derive(Debug, Clone, Default)]
pub struct NodeColoring {
    pub facts: Arc<RwLock<HashMap<Fact, FactState>>>,
    pub rules: Arc<RwLock<HashMap<Rule, RuleState>>>
}
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FactState {
    None,
    Starting,
    Target,
    TargetVisited,
    Visited,
}
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RuleState {
    None,
    Visited,
}
