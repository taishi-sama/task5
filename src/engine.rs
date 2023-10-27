use std::{collections::{HashSet, HashMap}, sync::{Arc, Mutex, RwLock}};

use egui_graphs::Graph;
use petgraph::{Directed, stable_graph::StableGraph};

use crate::{
    fact::{Fact, Rule, CoreFact, CoreRule},
    ruletree::RuleTree, direct_reasoning::{GraphNode, FactState, StatedFact, StatedRule, self, NodeColoring},
};
#[derive(Debug, Clone)]
pub struct Engine {
    pub starting_facts: Vec<Fact>,
    pub all_possible_facts: Vec<Fact>,
    pub rules: Vec<Rule>,
}

impl Engine {
    pub fn primitive_engine() -> Self{
        let planks = CoreFact::new("oak_planks");
        let wood = CoreFact::new("oak_wood");
        let stick = CoreFact::new("stick");
        let pickaxe = CoreFact::new("wooden_pickaxe");
        let starting = vec![wood.clone()];
        let mut possible = vec![planks.clone(), wood.clone(), stick.clone(), pickaxe.clone() ];
        //possible.extend(starting.iter().cloned());
        let rules = vec![
            CoreRule::new(vec![wood].into_iter(), planks.clone()),
            CoreRule::new(vec![planks.clone()].into_iter(), stick.clone()),
            CoreRule::new(vec![planks, stick].into_iter(), pickaxe)
            ];
        Self::new(possible, starting, rules)
    }
    pub fn new(all_facts: Vec<Fact>, starting_facts: Vec<Fact>, rules: Vec<Rule>) -> Self{
        Self { starting_facts, all_possible_facts: all_facts, rules }
    }
    pub fn to_graph(&self) -> (Graph<GraphNode, (), Directed>, NodeColoring) {
        let mut coloring_facts = Arc::new(RwLock::new(HashMap::new()));
        let mut coloring_rules = Arc::new(RwLock::new(HashMap::new()));

        let mut g = StableGraph::new();
        //StableGraph::add_node(&mut self, weight)
        let mut nodes = HashMap::new();
        for f in &self.all_possible_facts {
            let f = f.clone();
            let stated = if self.starting_facts.contains(&f) {
                coloring_facts.write().unwrap().insert(f.clone(), FactState::Starting);
                StatedFact{fact: f.clone(), state: coloring_facts.clone()}
            } else {
                coloring_facts.write().unwrap().insert(f.clone(), FactState::None);
                 StatedFact{fact: f.clone(), state: coloring_facts.clone()}};
            
            let node = g.add_node(GraphNode::Fact(stated));
            nodes.insert(f.clone(), node);
        }
        for r in &self.rules {
            let rule = GraphNode::Rule(StatedRule{rule: r.clone(), state:coloring_rules.clone()});
            coloring_rules.write().unwrap().insert(r.clone(), direct_reasoning::RuleState::None);
            let rule_ind = g.add_node(rule);
            let outind = nodes[&r.out];
            g.add_edge(rule_ind, outind, ());
            
            for reqs in &r.reqs {
                let inind = nodes[reqs];
                g.add_edge(inind, rule_ind, ());
            }
        };
        ((&g).into(), NodeColoring{ facts: coloring_facts, rules: coloring_rules})
    }
    fn try_direct_output(&self, target_fact: Fact) -> Option<RuleTree> {
        let mut appliedRules: HashSet<Rule> = HashSet::new();
        let current_facts: HashSet<Fact> = self.starting_facts.iter().cloned().collect();
        while appliedRules.len() < self.rules.len() {
            if current_facts.contains(&target_fact) {
                todo!()
            }
        }
        None
    }
    fn try_reverse_output(&self, fact: Fact) -> Option<RuleTree> {
        todo!()
    }
}
