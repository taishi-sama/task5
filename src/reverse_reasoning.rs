use std::{
    collections::{HashMap, HashSet},
    mem::swap,
    sync::Arc,
};

use crate::{
    direct_reasoning::{FactState, NodeColoring, RuleState},
    engine::Engine,
    fact::{Fact, Rule},
};
#[derive(Debug, Clone)]
pub struct ReverseReasoning {
    all_facts: Vec<Fact>,
    target_fact: Fact,
    //starting_facts: Vec<Fact>,
    pub all_rules: Vec<Rule>,
    //pub rules: Arc<Engine>,
    pub root: Box<Node>,
    pub starting_facts: HashSet<Fact>,
    pub reversed_rules: HashMap<Fact, Vec<Rule>>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RevStepResult {
    Found,
    Iterated,
    NotProved,
}
impl ReverseReasoning {
    pub fn new(rules: &Engine, target_fact: Fact) -> Self {
        let starting_facts = rules.starting_facts.iter().cloned().collect();
        let mut reversed_rules: HashMap<Fact, Vec<Rule>> = HashMap::new();
        for (res_fact, rule) in rules.rules.iter().map(|x| (x.out.clone(), x.clone())) {
            if reversed_rules.contains_key(&res_fact) {
                reversed_rules
                    .get_mut(&res_fact)
                    .unwrap()
                    .push(rule.clone());
            } else {
                reversed_rules.insert(res_fact.clone(), vec![rule]);
            }
        }
        Self {
            all_facts: rules.all_possible_facts.clone(),
            all_rules: rules.rules.clone(),
            root: Box::new(Node {
                available_rules: Arc::new(rules.rules.iter().cloned().collect()),
                node_info: NodeInfo::FactToProve(target_fact.clone()),
            }),
            starting_facts,
            reversed_rules,
            target_fact,
        }
    }
    pub fn step(&mut self) -> RevStepResult {
        //println!("{:?}", self.root);
        let mut n = Box::new(Node {
            available_rules: Arc::new(HashSet::new()),
            node_info: NodeInfo::Empty,
        });
        swap(&mut n, &mut self.root);
        let t = self.rec_iterate(&mut n);
        swap(&mut n, &mut self.root);
        match t {
            RecResult::Potential => RevStepResult::Iterated,
            RecResult::Found => RevStepResult::Found,
            RecResult::DeadEnd => RevStepResult::NotProved,
        }
    }
    pub fn build_tree(&mut self, coloring: &NodeColoring) -> RevStepResult {
        loop {
            let t = self.step();
            if t != RevStepResult::Iterated {
                self.recolor(coloring);
                return t;
            }
        }
    }
    pub fn recolor(&self, coloring: &NodeColoring) {
        let mut fc = coloring.facts.write().unwrap();
        let mut rc = coloring.rules.write().unwrap();

        for i in &self.all_facts {
            fc.insert(i.clone(), FactState::None);
        }
        for i in &self.all_rules {
            rc.insert(i.clone(), RuleState::None);
        }
        Self::rec_recoloring(&self.root, &mut fc, &mut rc);
        for i in &self.starting_facts {
            fc.insert(i.clone(), FactState::Starting);
        }

        match &self.root.node_info {
            NodeInfo::Or(f, _, s) => {
                if *s == RecResult::Found && f == &self.target_fact {
                    fc.insert(self.target_fact.clone(), FactState::TargetVisited);
                } else {
                    fc.insert(self.target_fact.clone(), FactState::Target);
                }
            }
            NodeInfo::And(f, _, _, s) => {
                if *s == RecResult::Found && f == &self.target_fact {
                    fc.insert(self.target_fact.clone(), FactState::TargetVisited);
                } else {
                    fc.insert(self.target_fact.clone(), FactState::Target);
                }
            }
            NodeInfo::FactToProve(_) => {
                fc.insert(self.target_fact.clone(), FactState::Target);
            }
            NodeInfo::ProvenFact(f) => {
                if f == &self.target_fact {
                    fc.insert(self.target_fact.clone(), FactState::TargetVisited);
                } else {
                    fc.insert(self.target_fact.clone(), FactState::Target);
                }
            }
            NodeInfo::DeadEnd(_) => {
                fc.insert(self.target_fact.clone(), FactState::Target);
            }
            NodeInfo::Empty => {
                fc.insert(self.target_fact.clone(), FactState::Target);
            }
        };
    }
    fn rec_recoloring(
        node: &Node,
        facts: &mut HashMap<Fact, FactState>,
        rules: &mut HashMap<Rule, RuleState>,
    ) {
        match &node.node_info {
            NodeInfo::Or(t, r, q) => {
                for i in r {
                    Self::rec_recoloring(i, facts, rules);
                }
                let fs = match q {
                    RecResult::Potential => FactState::Visited,
                    RecResult::Found => FactState::VisitedPath,
                    RecResult::DeadEnd => FactState::DeadEnd,
                };
                facts.insert(t.clone(), fs);
            }
            NodeInfo::And(f, r, n, q) => {
                for i in n {
                    Self::rec_recoloring(i, facts, rules);
                }
                let rs = match q {
                    RecResult::Potential => RuleState::Visited,
                    RecResult::Found => RuleState::VisitedPath,
                    RecResult::DeadEnd => RuleState::DeadEnd,
                };
                rules.insert(r.clone(), rs);
                let fs = match q {
                    RecResult::Potential => FactState::Visited,
                    RecResult::Found => FactState::VisitedPath,
                    RecResult::DeadEnd => FactState::DeadEnd,
                };
                facts.insert(f.clone(), fs);
            }
            NodeInfo::FactToProve(f) => {
                facts.insert(f.clone(), FactState::Visited);
            }
            NodeInfo::ProvenFact(_) => (),
            NodeInfo::DeadEnd(f) => {
                facts.insert(f.clone(), FactState::DeadEnd);
            }
            NodeInfo::Empty => unreachable!(),
        }
    }
    //At least one branch solved
    fn rec_iterate(&mut self, node: &mut Node) -> RecResult {
        match &mut node.node_info {
            NodeInfo::Or(f, v, status) => {
                if *status != RecResult::Potential {
                    return *status;
                }
                let mut ind_deadend: Vec<usize> = vec![];
                let mut ind_found = None;
                for (i, n) in v.iter_mut().enumerate() {
                    match self.rec_iterate(n) {
                        RecResult::Potential => {}
                        RecResult::Found => {
                            ind_found = Some(i);
                            break;
                        } //Уничтожаем другие деревья
                        RecResult::DeadEnd => ind_deadend.push(i),
                    }
                }
                if let Some(found) = ind_found {
                    let t = v.drain(found..=found).next().unwrap();
                    v.clear();
                    v.push(t);
                    *status = RecResult::Found;
                } else if ind_deadend.len() == v.len() || v.len() == 0 {
                    *status = RecResult::DeadEnd
                }
                *status
            }
            NodeInfo::And(f, r, v, status) => {
                if *status != RecResult::Potential {
                    return *status;
                }
                let mut count_found: usize = 0;
                for n in v.iter_mut() {
                    match self.rec_iterate(n) {
                        RecResult::Potential => {}
                        RecResult::Found => count_found += 1,
                        RecResult::DeadEnd => {
                            *status = RecResult::DeadEnd;
                            return *status;
                        }
                    }
                }
                if count_found == v.len() {
                    *status = RecResult::Found;
                }
                *status
            }
            NodeInfo::FactToProve(f) => {
                //println!("Starting facts: {:?}", self.starting_facts);
                if self.starting_facts.contains(f) {
                    *node = Node {
                        available_rules: node.available_rules.clone(),
                        node_info: NodeInfo::ProvenFact(f.clone()),
                    };
                    RecResult::Found
                } else if let Some(v) = self.reversed_rules.get(f) {
                    let t: Vec<_> = v
                        .iter()
                        .filter(|&x| node.available_rules.contains(x))
                        .collect();
                    if !t.is_empty() {
                        let available_rules: Arc<HashSet<Rule>> = Arc::new(
                            node.available_rules
                                .iter()
                                .filter(|x| !t.contains(x))
                                .cloned()
                                .collect(),
                        );
                        let aval = available_rules.clone();
                        *node = Node {
                            available_rules,
                            node_info: NodeInfo::Or(
                                f.clone(),
                                t.iter()
                                    .map(|x| Node {
                                        available_rules: aval.clone(),
                                        node_info: NodeInfo::And(
                                            f.clone(),
                                            (*x).clone(),
                                            x.reqs
                                                .iter()
                                                .map(|q| Node {
                                                    available_rules: aval.clone(),
                                                    node_info: NodeInfo::FactToProve(q.clone()),
                                                })
                                                .collect(),
                                            RecResult::Potential,
                                        ),
                                    })
                                    .collect(),
                                RecResult::Potential,
                            ),
                        };
                        RecResult::Potential
                    } else {
                        RecResult::DeadEnd
                    }
                } else {
                    RecResult::DeadEnd
                }
            }
            NodeInfo::ProvenFact(_) => RecResult::Found,
            NodeInfo::DeadEnd(_) => RecResult::DeadEnd,
            NodeInfo::Empty => unreachable!(),
        }
    }
    pub fn get_applied_rules(&self) -> impl Iterator<Item = Rule> {
        let t = self.get_applied_rules_unfiltered();
        let mut v = Vec::with_capacity(t.len());
        let mut mentioted_rules: HashSet<Rule> = HashSet::new();
        for r in t {
            if !mentioted_rules.contains(&r) {
                mentioted_rules.insert(r.clone());
                v.push(r);
            }
        }
        v.into_iter()
    }
    fn get_applied_rules_unfiltered(&self) -> Vec<Rule> {
        match &self.root.node_info {
            NodeInfo::Or(f, _, s) => {
                if *s == RecResult::Found && f == &self.target_fact {
                    self.get_applied_rules_rec(&self.root, true)
                } else {
                    self.get_applied_rules_rec(&self.root, false)
                }
            }
            NodeInfo::And(f, _, _, s) => {
                if *s == RecResult::Found && f == &self.target_fact {
                    self.get_applied_rules_rec(&self.root, true)
                } else {
                    self.get_applied_rules_rec(&self.root, false)
                }
            }
            NodeInfo::FactToProve(_) => self.get_applied_rules_rec(&self.root, false),
            NodeInfo::ProvenFact(f) => self.get_applied_rules_rec(&self.root, false),
            NodeInfo::DeadEnd(_) => self.get_applied_rules_rec(&self.root, false),
            NodeInfo::Empty => self.get_applied_rules_rec(&self.root, false),
        }
    }
    fn get_applied_rules_rec(&self, node: &Node, is_final: bool) -> Vec<Rule> {
        let mut t = vec![];
        match &node.node_info {
            NodeInfo::Or(_, r, q) => {
                for i in r {
                    t.append(&mut self.get_applied_rules_rec(i, is_final));
                }
            }
            NodeInfo::And(f, r, n, q) => {
                for i in n {
                    t.append(&mut self.get_applied_rules_rec(i, is_final));
                }
                let rs = match q {
                    RecResult::Potential => !is_final,
                    RecResult::Found => true,
                    RecResult::DeadEnd => false,
                };
                if rs {
                    t.push(r.clone())
                }
            }
            NodeInfo::FactToProve(f) => {}
            NodeInfo::ProvenFact(_) => (),
            NodeInfo::DeadEnd(f) => {}
            NodeInfo::Empty => unreachable!(),
        }
        t
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
    pub node_info: NodeInfo,
}

#[derive(Debug, Clone)]
pub enum NodeInfo {
    Or(Fact, Vec<Node>, RecResult),
    And(Fact, Rule, Vec<Node>, RecResult),
    FactToProve(Fact),
    ProvenFact(Fact),
    DeadEnd(Fact),
    Empty,
}
