use std::{default, sync::{Arc, RwLock}, collections::HashMap};

use direct_reasoning::{DirectReasoning, GraphNode, FactState, NodeColoring};
use egui::{Shape, Rect, Vec2, Rounding, Stroke, FontId, FontFamily, epaint::TextShape, Color32, ComboBox};
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
        "My egui App",
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
    engine: Option<Arc<Engine>>,
    state: AppState,
    g:Graph<GraphNode, (), Directed>,
    coloring: NodeColoring,
    dir: Option<DirectReasoning>,
    rev: Option<ReverseReasoning>,
    target_fact: Option<Fact>,
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
        Self { engine: Default::default(), state: Default::default(), coloring: Default::default(), g: (&StableGraph::new()).into(), dir: None, target_fact: None, rev:None }
    }
}
impl MyEguiApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let e = Arc::new(Engine::primitive_engine());
        // Customize egui here with cc.egui_ctx.set_fonts and cc.egui_ctx.set_visuals.
        // Restore app state using cc.storage (requires the "persistence" feature).
        // Use the cc.gl (a glow::Context) to create graphics shaders and buffers that you can use
        // for e.g. egui::PaintCallback.
        let (g, c) = e.to_graph();
        Self {
            engine: Some(e.clone()),
            g, //(&StableGraph::new()).into(),
            coloring: c,
            state: AppState::None,
            dir: None,
            rev: None,
            target_fact: None
        }
    }
    fn update_state(&mut self) {
        match self.state {
            AppState::None => (),
            AppState::DirectReasoning => self.dir = Some(DirectReasoning::new(self.engine.as_ref().unwrap().clone(), self.target_fact.as_ref().unwrap().clone())),
            AppState::ReverseReasoning => self.rev = Some(ReverseReasoning::new(self.engine.as_ref().unwrap().clone(), self.target_fact.as_ref().unwrap().clone())),
        }
        match self.state {
            AppState::None => (),
            AppState::DirectReasoning => match self.dir.as_ref() {
                Some(x) => x.update_hashmap( &self.coloring),
                None => (),
            },
            AppState::ReverseReasoning => match self.dir.as_ref() {
                Some(x) => (), //TODO
                None => (),
            }
        }
    }
}

impl eframe::App for MyEguiApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("Controls").resizable(false).show(ctx, |ui|{
            ui.horizontal(|ui|{
            
            let prev = self.target_fact.clone();
            ComboBox::from_label("").
                selected_text(format!("{}", self.target_fact.as_ref().map(|x|format!("{}",x)).unwrap_or_else(||"None".to_string()))).show_ui(ui, |ui|{
                    for i in &self.engine.clone().unwrap().all_possible_facts {
                        ui.selectable_value(&mut self.target_fact, Some(i.clone()), format!("{:}", i));
                    }
                });
            if prev != self.target_fact {self.update_state()};
            let prev =self.state.clone();
            ComboBox::from_label("l").selected_text(format!("{:?}", self.state)).show_ui(ui, |ui|{
                ui.selectable_value(&mut self.state, AppState::None, "None");
                ui.selectable_value(&mut self.state, AppState::DirectReasoning, "Direct");
                ui.selectable_value(&mut self.state, AppState::ReverseReasoning, "Reverse");
            });
            if self.target_fact.is_none() {self.state = AppState::None;}
            else if prev != self.state {self.update_state()}
            if ui.button("Iterate").clicked(){
                match self.state {
                    AppState::None => (),
                    AppState::DirectReasoning => {self.dir.as_mut().unwrap().step(); self.dir.as_ref().unwrap().update_hashmap(&self.coloring)},
                    AppState::ReverseReasoning => {self.rev.as_mut().unwrap().step(); println!("{:?}", self.rev.as_ref().unwrap().root);},
                }
            }
        })});
        
        egui::SidePanel::right("Rules")
            .resizable(true)
            .show(ctx, |ui| {
                let mut table = TableBuilder::new(ui).resizable(true).column(Column::auto()).column(Column::auto());
                table.header(20.0, |mut header| {
                    header.col(|ui|{ui.strong("Required Facts");});
                    header.col(|ui|{ui.strong("Resulting Facts");});
                }).body(|mut body| {
                    if let Some(e) = &self.engine {
                        body.rows(18.0, e.rules.len(), |row_index, mut row| {
                            row.col(|ui|{ ui.label(
                                e.rules[row_index].reqs.iter().map(|x|format!("{}", x)).reduce(|x, y|x + ", " + &y).unwrap_or_default()
                            ); });
                            row.col(|ui|{ ui.label(format!("{}", e.rules[row_index].out)); });

                        })
                    }
                });
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
                                    direct_reasoning::RuleState::None => Color32::GRAY,
                                    direct_reasoning::RuleState::Visited => Color32::YELLOW,
                                }).unwrap_or(Color32::GRAY)
                            }
                            GraphNode::Fact(f) => {
                                f.state.read().unwrap().get(&f.fact).map(|qt|match qt {
                                       FactState::None => Color32::GRAY,
                                       FactState::Starting => Color32::DARK_GREEN,
                                       FactState::Target => Color32::GREEN,
                                       FactState::Visited => Color32::YELLOW,
                                       FactState::TargetVisited => Color32::LIGHT_GREEN
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
                                    FontId::new(rad, FontFamily::Monospace),
                                    color,
                                )
                            }),
                            GraphNode::Fact(fact) => ctx.fonts(|f| {
                                f.layout_no_wrap(
                                    format!("{:}", fact.fact),
                                    FontId::new(rad, FontFamily::Monospace),
                                    color,
                                )
                            }),
                        }
                        ;

                        // we need to offset label by half its size to place it in the center of the rect
                        let offset = Vec2::new(-galley.size().x / 2., -galley.size().y / 2. - rad * 1.5);
                        // create the shape and add it to the layers
                        let shape_label = TextShape::new(node_center_loc + offset, galley);
                        l.add(shape_label);
                });
            ui.add(&mut gw);

        });
        
    }
}
