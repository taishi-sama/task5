use std::{sync::Arc, collections::{HashMap, HashSet}, mem::swap};

use crate::{engine::Engine, fact::{Fact, Rule}};
#[derive(Debug, Clone)]
pub struct ReverseReasoning {
    pub rules: Arc<Engine>,
    pub root: Box<Node>,
    pub starting_facts: HashSet<Fact>,
    pub reversed_rules: HashMap<Fact, Vec<Rule>>
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RevStepResult {
    Found,
    Iterated,
    NotProved,
}
impl ReverseReasoning {
    pub fn new(rules: Arc<Engine>, target_fact: Fact) -> Self{
        let starting_facts = rules.starting_facts.iter().cloned().collect();
        let mut reversed_rules: HashMap<Fact, Vec<Rule>> = HashMap::new();
        for (res_fact, rule) in rules.rules.iter().map(|x|(x.out.clone(), x.clone())){
            if reversed_rules.contains_key(&res_fact) {
                reversed_rules.get_mut(&res_fact).unwrap().push(rule.clone());
            }
            else {reversed_rules.insert(res_fact.clone(), vec![rule]);}
        }
        Self { rules: rules.clone(), 
            root: Box::new(Node{available_rules:Arc::new(rules.rules.iter().cloned().collect()), node_info: NodeInfo::FactToProve(target_fact)}), 
            starting_facts,
            reversed_rules}
    }
    pub fn step(&mut self) -> RevStepResult {
        println!("{:?}", self.root);
        let mut n = Box::new(Node{available_rules:Arc::new(HashSet::new()), node_info: NodeInfo::Empty});
        swap(&mut n, &mut self.root);
        let t =self.rec_iterate(&mut n);
        swap(&mut n, &mut self.root);
        match t {
            RecResult::Potential => RevStepResult::Iterated,
            RecResult::Found => RevStepResult::Found,
            RecResult::DeadEnd => RevStepResult::NotProved,
        }
    }
    //At least one branch solved
    fn rec_iterate(&mut self, node: &mut Node) -> RecResult {
        match &mut node.node_info {
            NodeInfo::Or(f, v, status) => {
                if *status != RecResult::Potential { return *status; }
                let mut ind_deadend: Vec<usize> = vec![];
                let mut ind_found = None;
                for (i, n) in v.iter_mut().enumerate() {
                    match self.rec_iterate(n) {
                        RecResult::Potential => {},
                        RecResult::Found => {ind_found = Some(i); break;}, //Уничтожаем другие деревья
                        RecResult::DeadEnd => {ind_deadend.push(i)},
                    }
                }
                if let Some(found) = ind_found {
                    
                    let t = v.drain(found..=found).next().unwrap();
                    v.clear();
                    v.push(t);
                    *status = RecResult::Found;
                }
                else if ind_deadend.len() == v.len() || v.len() == 0 {
                    *status = RecResult::DeadEnd
                }
                *status
            },
            NodeInfo::And(f, r, v, status) => {
                if *status != RecResult::Potential { return *status; }
                let mut count_found:usize = 0;
                for n in v.iter_mut() {
                    match self.rec_iterate(n) {
                        RecResult::Potential => {},
                        RecResult::Found => count_found += 1,
                        RecResult::DeadEnd => {
                            *status = RecResult::DeadEnd;
                            return *status;
                        },
                    }
                }
                if count_found == v.len() {
                    *status = RecResult::Found;
                }
                *status
            },
            NodeInfo::FactToProve(f) => {
                println!("Starting facts: {:?}", self.starting_facts);
                if self.starting_facts.contains(f) {
                    *node = Node{available_rules: node.available_rules.clone(), node_info: NodeInfo::ProvenFact(f.clone())};
                    RecResult::Found
                }
                else if let Some(v) = self.reversed_rules.get(f) {
                    let t: Vec<_> = v.iter().filter(|&x|node.available_rules.contains(x)).collect();
                    if !t.is_empty(){
                        let available_rules: Arc<HashSet<Rule>> = Arc::new(node.available_rules.iter().filter(|x|!t.contains(x)).cloned().collect());
                        let aval = available_rules.clone();
                        *node = Node{available_rules, node_info: NodeInfo::Or(f.clone(),
                            t.iter().map(|x|Node{
                                available_rules: aval.clone(),
                                node_info: NodeInfo::And(f.clone(), (*x).clone(), x.reqs.iter().map(|q|
                                    Node{available_rules: aval.clone(), node_info: NodeInfo::FactToProve(q.clone())}).collect(), RecResult::Potential),
                            }).collect(), RecResult::Potential
                        )};
                        RecResult::Potential
                    }
                    else {
                        RecResult::DeadEnd
                    }
                }
                else {
                    RecResult::DeadEnd
                }
            },
            NodeInfo::ProvenFact(_) => RecResult::Found,
            NodeInfo::DeadEnd(_) => RecResult::DeadEnd,
            NodeInfo::Empty => unreachable!()
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RecResult {
    Potential,
    Found,
    DeadEnd,
}
#[derive(Debug, Clone)]
pub struct Node {
    pub available_rules: Arc<HashSet<Rule>>,
    pub node_info: NodeInfo
}

#[derive(Debug, Clone)]
pub enum NodeInfo {
    Or(Fact, Vec<Node>, RecResult),
    And(Fact, Rule, Vec<Node>, RecResult),
    FactToProve(Fact),
    ProvenFact(Fact),
    DeadEnd(Fact),
    Empty
}