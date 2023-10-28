use std::{default, sync::{Arc, RwLock}, collections::HashMap, fmt::format};

use direct_reasoning::{DirectReasoning, GraphNode, FactState, NodeColoring, RuleState};
use egui::{Shape, Rect, Vec2, Rounding, Stroke, FontId, FontFamily, epaint::TextShape, Color32, ComboBox, ScrollArea, RichText};
use egui_extras::{TableBuilder, Column};
use egui_graphs::{Graph, GraphView, SettingsStyle, SettingsInteraction};
use engine::Engine;
use fact::{Fact, Rule};
use petgraph::{Directed, stable_graph::StableGraph, visit::EdgeRef};
use reverse_reasoning::ReverseReasoning;

pub mod direct_reasoning;
pub mod engine;
pub mod fact;
pub mod ruletree;
pub mod reverse_reasoning;

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "Production system",
        native_options,
        Box::new(|cc| Box::new(MyEguiApp::new(cc))),
    )
    .unwrap();
}
#[cfg(target_arch = "wasm32")]
fn main() {
    // Redirect `log` message to `console.log` and friends:
    eframe::WebLogger::init(log::LevelFilter::Debug).ok();

    let web_options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async {
        eframe::WebRunner::new()
            .start(
                "the_canvas_id", // hardcode it
                web_options,
                Box::new(|cc| Box::new(MyEguiApp::new(cc))),
            )
            .await
            .expect("failed to start eframe");
    });
}
struct MyEguiApp {
    engine: Option<Engine>,
    state: AppState,
    g:Graph<GraphNode, (), Directed>,
    coloring: NodeColoring,
    dir: Option<DirectReasoning>,
    rev: Option<ReverseReasoning>,
    target_fact: Option<Fact>,
    all_rules: bool,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
enum AppState {
    #[default]
    None,
    DirectReasoning,
    ReverseReasoning,
}
impl Default for MyEguiApp {
    fn default() -> Self {
        Self { engine: Default::default(), state: Default::default(), coloring: Default::default(), g: (&StableGraph::new()).into(), dir: None, target_fact: None, rev:None, all_rules: false }
    }
}
impl MyEguiApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let e = Engine::from_string(include_str!("crafts.txt"));
        //println!("{:?}", e);
        // Customize egui here with cc.egui_ctx.set_fonts and cc.egui_ctx.set_visuals.
        // Restore app state using cc.storage (requires the "persistence" feature).
        // Use the cc.gl (a glow::Context) to create graphics shaders and buffers that you can use
        // for e.g. egui::PaintCallback.
        let (g, c) = e.to_graph();
        Self {
            engine: Some(e),
            g, //(&StableGraph::new()).into(),
            coloring: c,
            state: AppState::None,
            dir: None,
            rev: None,
            target_fact: None,
            all_rules: false
        }
    }
    fn update_state(&mut self) {
        match self.state {
            AppState::None => (),
            AppState::DirectReasoning => self.dir = Some(DirectReasoning::new(self.engine.as_ref().unwrap(), self.target_fact.as_ref().unwrap().clone())),
            AppState::ReverseReasoning => self.rev = Some(ReverseReasoning::new(self.engine.as_ref().unwrap(), self.target_fact.as_ref().unwrap().clone())),
        }
        match self.state {
            AppState::None => match self.engine.as_ref() {
                Some(x) => x.recolor_node(self.target_fact.clone(), &self.coloring),
                None => (),
            },
            AppState::DirectReasoning => match self.dir.as_ref() {
                Some(x) => x.update_hashmap( &self.coloring),
                None => (),
            },
            AppState::ReverseReasoning => match self.rev.as_mut() {
                Some(x) => _ = x.recolor(&self.coloring), //TODO
                None => (),
            }
        }
    }
}

impl eframe::App for MyEguiApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("Controls").resizable(false).show(ctx, |ui|{
            ui.horizontal(|ui|{
            
            // ComboBox::from_label("").
            //     selected_text(format!("{}", self.target_fact.as_ref().map(|x|format!("{}",x)).unwrap_or_else(||"None".to_string()))).show_ui(ui, |ui|{
            //         for i in &self.engine.clone().unwrap().all_possible_facts {
            //             ui.selectable_value(&mut self.target_fact, Some(i.clone()), format!("{:}", i));
            //         }
            //     });
            let prev =self.state.clone();
            ComboBox::from_label("Type of production system").selected_text(format!("{:?}", self.state)).show_ui(ui, |ui|{
                ui.selectable_value(&mut self.state, AppState::None, "None");
                ui.selectable_value(&mut self.state, AppState::DirectReasoning, "Direct");
                ui.selectable_value(&mut self.state, AppState::ReverseReasoning, "Reverse");
            });
            if self.target_fact.is_none() {self.state = AppState::None;}
            else if prev != self.state {self.update_state()}
            if ui.button("Iterate to find").clicked(){
                match self.state {
                    AppState::None => (),
                    AppState::DirectReasoning => {self.dir.as_mut().unwrap().step(); self.dir.as_ref().unwrap().update_hashmap(&self.coloring)},
                    AppState::ReverseReasoning => {self.rev.as_mut().unwrap().step(); self.rev.as_mut().unwrap().recolor(&self.coloring)},
                }
            }
            if ui.button("Find").clicked(){
                match self.state {
                    AppState::None => (),
                    AppState::DirectReasoning => {self.dir.as_mut().unwrap().try_find(); self.dir.as_ref().unwrap().update_hashmap(&self.coloring)},
                    AppState::ReverseReasoning => {self.rev.as_mut().unwrap().build_tree(&self.coloring);},
                }
            }
        })});
        let mut update_state = false;
        egui::SidePanel::right("Facts")
            .resizable(false)
            .show(ctx, |ui| {
                let mut table = TableBuilder::new(ui).resizable(false).column(Column::exact(15.0)).column(Column::auto().at_least(15.0)).column(Column::exact(15.0));
                table.header(20.0, |mut header| {
                    header.col(|ui|{ui.strong("Starting");});
                    header.col(|ui|{ui.strong("Facts");});
                    header.col(|ui|{ui.strong("Target");});
                }).body(|mut body| {
                    if let Some(e) = &mut self.engine {
                        body.rows(24.0, e.all_possible_facts.len(), |row_index, mut row| {
                            let f = e.all_possible_facts[row_index].clone();
                            let mut start = e.starting_facts.contains(&f);
                            let old_start = start;
                            // row.col(|ui|{ ui.label(
                            //     e.rules[row_index].reqs.iter().map(|x|format!("{}", x)).reduce(|x, y|x + ", " + &y).unwrap_or_default()
                            // ); });
                            // row.col(|ui|{ ui.label(format!("{}", e.rules[row_index].out)); });
                            row.col(|ui|{
                                ui.checkbox(&mut start, "");
                            });
                            if old_start != start {
                                if start {
                                    e.starting_facts.insert(f.clone());
                                }
                                else {e.starting_facts.remove(&f);}
                                update_state = true;
                            }
                            row.col(|ui|{
                                ui.label(format!("{}", f));
                            });
                            let t = self.target_fact.clone();
                            row.col(|ui|{
                                ui.radio_value(&mut self.target_fact, Some(f.clone()), "");
                            });
                            if t != self.target_fact {
                                update_state = true;
                            }
                        })
                    }
                });
            });
        if update_state{
            self.update_state();
        }
        egui::TopBottomPanel::bottom("Rules").resizable(true).show(ctx, |ui|{
            ui.vertical(|ui|{
            
            ScrollArea::vertical().drag_to_scroll(true).show(ui, |ui|{
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new("Select starting facts and target fact. After this select type of production system. \nScroll down to list of rules.\n"));
                    ui.label(RichText::new("Use \"Iterate to find\" to make one iteration of search, \"Find\" to apply iteration until result.\n"));
                    ui.label(RichText::new("Rectangles are rules, circles are facts. Color scheme:\n"));
                    ui.label(RichText::new("Starting facts\n").color(Color32::DARK_GREEN));
                    ui.label(RichText::new("Visited facts and rules\n").color(Color32::YELLOW));
                    ui.label(RichText::new("Target fact(while searching)\n").color(Color32::GREEN));
                    ui.label(RichText::new("Target fact(got after finding)\n").color(Color32::LIGHT_GREEN));
                    ui.label(RichText::new("Target fact(not possible with rules and these starting facts)\n").color(Color32::LIGHT_RED));
                    ui.label(RichText::new("Optimal rules and facts for getting target fact(only in reversive production system)\n").color(Color32::BLUE).background_color(Color32::LIGHT_GRAY));
                    ui.label(RichText::new("Dead end while searching path to target fact(only in reversive production system)\n").color(Color32::DARK_RED));
                    

                });
                ui.checkbox(&mut self.all_rules, "Show all rules:");
                if self.all_rules {
                    for i in &self.engine.as_ref().unwrap().rules {
                        ui.label(format!("{}", i));
                    }
                }
                else {
                    match self.state {
                        AppState::None => (),
                        AppState::DirectReasoning => {
                            for i in self.dir.as_ref().unwrap().all_rules.iter().filter(|&x|self.dir.as_ref().unwrap().unused_rules.contains(x)) {
                                ui.label(format!("{}", i));
                            }
                        },
                        AppState::ReverseReasoning => {
                            for i in self.rev.as_ref().unwrap().get_applied_rules() {
                                ui.label(format!("{}", i));
                            }
                        },
                    }
                }
            })});
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            let style_settings = &SettingsStyle::new().with_labels_always(true);
            let interaction_settings = &SettingsInteraction::new()
                .with_dragging_enabled(true);
            let mut gw = GraphView::new(&mut self.g)
                .with_interactions(interaction_settings)
                .with_styles(style_settings)
                .with_custom_node_draw(|ctx: &egui::Context, n, state, l| {
                    let node_center_loc = n.screen_location(state.meta).to_pos2();
                    let rad = n.screen_radius(state.meta, state.style);
                    let size = Vec2::new(rad * 1.5, rad * 1.5);
                    
                        let rect = Rect::from_center_size(node_center_loc, size);
                        
                        
                        
                        //let node_color =  if self.engine.unwrap().starting_facts.connect(n.data().unwrap()) {Color32::GREEN} else {Color32::GRAY};
                        //let node_color = Color32::GRAY;
                        let shape_color = match n.data().unwrap() {
                            GraphNode::Rule(r) => {
                                r.state.read().unwrap().get(&r.rule).map(|qt| match qt {
                                    RuleState::None => Color32::GRAY,
                                    RuleState::Visited => Color32::YELLOW,
                                    RuleState::VisitedPath => Color32::BLUE,
                                    RuleState::DeadEnd => Color32::DARK_RED,
                                }).unwrap_or(Color32::GRAY)
                            }
                            GraphNode::Fact(f) => {
                                f.state.read().unwrap().get(&f.fact).map(|qt|match qt {
                                       FactState::None => Color32::GRAY,
                                       FactState::Starting => Color32::DARK_GREEN,
                                       FactState::Target => Color32::GREEN,
                                       FactState::Visited => Color32::YELLOW,
                                       FactState::TargetVisited => Color32::LIGHT_GREEN,
                                       FactState::VisitedPath => Color32::BLUE,
                                       FactState::DeadEnd => Color32::DARK_RED,
                                       FactState::TargetNotPossible => Color32::LIGHT_RED,
                                   }).unwrap_or(Color32::GRAY)
                                }
                            };
                        let shape_rect = Shape::rect_filled(
                                rect,
                                Rounding::default(),
                                shape_color);
                        let shape_circle = Shape::circle_filled(node_center_loc, rad, shape_color);
                        match n.data().unwrap() {
                            GraphNode::Rule(_) => l.add(shape_rect),
                            GraphNode::Fact(_) => l.add(shape_circle),
                        }
                        
                        let color = ctx.style().visuals.text_color();
                        let galley = match n.data().unwrap() {
                            GraphNode::Rule(r) => ctx.fonts(|f| {
                                f.layout_no_wrap(
                                    format!("{:}", r.rule),
                                    FontId::new(rad*1.5, FontFamily::Monospace),
                                    color,
                                )
                            }),
                            GraphNode::Fact(fact) => ctx.fonts(|f| {
                                f.layout_no_wrap(
                                    format!("{:}", fact.fact),
                                    FontId::new(rad*1.5, FontFamily::Monospace),
                                    color,
                                )
                            }),
                        }
                        ;

                        // we need to offset label by half its size to place it in the center of the rect
                        let offset = Vec2::new(-galley.size().x / 2., -galley.size().y / 2. - rad * 1.5*1.5);
                        // create the shape and add it to the layers
                        let shape_label = TextShape::new(node_center_loc + offset, galley);
                        l.add(shape_label);
                });
            ui.add(&mut gw);

        });
        
    }
}
