use std::{collections::{HashSet, HashMap}, sync::{Arc, RwLock}};

// #[derive(Debug, Clone)]
// pub struct ConcreteRule {
//     input: Vec<Arc<CoreFact>>,
//     output: Arc<CoreFact>
// }
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CoreRule {
    pub reqs: Vec<Fact>,
    pub out: Fact, //Правило вывода одного конкретного факта из множества.
}
pub type Fact = Arc<CoreFact>;
impl CoreRule {
    pub fn match_requirement(&self, facts: &HashSet<Fact>) -> bool {
        self.reqs.iter().all(|y| facts.contains(y))
    }
    pub fn new(reqs: impl Iterator<Item = Fact>, out: Fact) -> Arc<Self>{
        Arc::new(CoreRule { reqs: reqs.collect(), out})
    }   
}
pub type Rule = Arc<CoreRule>;
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CoreFact {
    Symbol(String),
    Symbols(Vec<CoreFact>),
}
impl CoreFact {
    pub fn new(symbol: impl Into<String>) -> Arc<Self>{
        Arc::new(CoreFact::Symbol(symbol.into()))
    }    
}
#[derive(Debug, Clone)]
pub struct StatedFact {
    pub fact: Fact,
    pub state: Arc<RwLock<HashMap<Fact, FactState>>>
}
#[derive(Debug, Clone)]
pub enum GraphNode {
    Rule(Rule),
    Fact(StatedFact)    
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FactState {
    None,
    Starting,
    Target,
    TargetVisited,
    Visited,

}