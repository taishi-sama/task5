use std::{collections::{HashSet, HashMap}, sync::{Arc, Mutex, RwLock}};

use egui::Vec2;
use egui_graphs::Graph;
use petgraph::{Directed, stable_graph::StableGraph};
use regex::Regex;

use crate::{
    fact::{Fact, Rule, CoreFact, CoreRule},
    ruletree::RuleTree, direct_reasoning::{GraphNode, FactState, StatedFact, StatedRule, self, NodeColoring},
};
#[derive(Debug, Clone)]
pub struct Engine {
    pub starting_facts: HashSet<Fact>,
    pub all_possible_facts: Vec<Fact>,
    pub rules: Vec<Rule>,
}

impl Engine {
    pub fn primitive_engine() -> Self{
        let planks = CoreFact::new("oak_planks");
        let wood = CoreFact::new("oak_wood");
        let stick = CoreFact::new("stick");
        let pickaxe = CoreFact::new("wooden_pickaxe");
        let starting:HashSet<_> = vec![wood.clone()].into_iter().collect();
        let mut possible = vec![planks.clone(), wood.clone(), stick.clone(), pickaxe.clone() ];
        //possible.extend(starting.iter().cloned());
        let rules = vec![
            CoreRule::new(vec![wood].into_iter(), planks.clone()),
            CoreRule::new(vec![planks.clone()].into_iter(), stick.clone()),
            CoreRule::new(vec![planks, stick].into_iter(), pickaxe)
            ];
        Self::new(possible, starting, rules)
    }
    pub fn from_string(s: &str) -> Self {
        let mut facts_vec = vec![];
        let mut facts: HashMap<&str, Fact> = HashMap::new();
        let mut rules = vec![];
        let comment = Regex::new(r#"//.*"#).unwrap();
        //let rule = Regex::new(r#"\{(?:(\w+)\s*,\s*)*(?:(\w+)\s*)\}\s*->\s*(\w+)"#).unwrap();
        let rule = Regex::new(r#"\{(.*)\} \s*->\s*(\w+)"#).unwrap();
        let fact = Regex::new(r#"(\w+)"#).unwrap();
        for i in s.lines(){
            if comment.is_match(i) {continue;}
            if i.trim().is_empty() {continue;}
            if let Some(capt) = rule.captures(i) {
                let l = capt.len();
                let res = l - 1;
                let last_req = res - 1;
                let res_str = capt.get(res).unwrap().as_str().trim();

                let res_fact = if facts.contains_key(res_str) {
                    facts.get(res_str).unwrap().clone()
                } else { facts_vec.push(res_str); let f = CoreFact::new(res_str); facts.insert(res_str, f.clone()); f};
                let mut v = vec![];
                let reqs = capt.get(last_req).unwrap().as_str();
                for t in fact.captures_iter(reqs){
                    for req in t.get(0){
                        let req_str = req.as_str().trim();
                        let req_fact = if facts.contains_key(req_str) {
                            facts.get(req_str).unwrap().clone()
                        } else { facts_vec.push(req_str); let f = CoreFact::new(req_str); facts.insert(req_str, f.clone()); f};

                        v.push(req_fact);
                    } 
                }
                let rule = CoreRule::new(v.into_iter(), res_fact);
                rules.push(rule);
            } else if let Some(capt) = fact.captures(i) {
                let fact_str = capt.get(1).unwrap().as_str().trim();
                if !facts.contains_key(fact_str) {
                    facts_vec.push(fact_str); let f = CoreFact::new(fact_str); facts.insert(fact_str, f.clone());
                };
            } 
            else {continue;}
        }
        //facts_vec.dedup();
        let f = facts_vec.iter().map(|x|facts.get(x).unwrap()).cloned().collect();
        Engine { starting_facts: HashSet::new(), all_possible_facts: f, rules }
    }
    pub fn new(all_facts: Vec<Fact>, starting_facts: HashSet<Fact>, rules: Vec<Rule>) -> Self{
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
        let t = (g.node_count() as f32).sqrt().round() as usize;
        let mut gr : Graph<_, _, _> = (&g).into();
        let mut v: Vec<_> = nodes.values().collect();
        v.sort();
        for (i, ind) in g.node_indices().enumerate() {
            let node = gr.node_mut(ind).unwrap();
            node.set_location(Vec2{x:((i%t) * 80) as _,y: ((i/t) * 50) as _})
        }
        (gr, NodeColoring{ facts: coloring_facts, rules: coloring_rules})
    }
    pub fn recolor_node(&self, target: Option<Fact>, coloring: &NodeColoring) {
        for f in &self.all_possible_facts {
            let f = f.clone();
            if self.starting_facts.contains(&f) {
                coloring.facts.write().unwrap().insert(f.clone(), FactState::Starting);
            } else {
                coloring.facts.write().unwrap().insert(f.clone(), FactState::None);
            };
        }
        for r in &self.rules {
            coloring.rules.write().unwrap().insert(r.clone(), direct_reasoning::RuleState::None);
        };
        if let Some(fact) = target {
            coloring.facts.write().unwrap().insert(fact, FactState::Target);
        }
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
