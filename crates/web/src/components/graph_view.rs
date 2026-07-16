use leptos::ev;
use leptos::prelude::*;
use std::collections::HashMap;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;

use crate::api;

const MAX_NODES: usize = 1000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeType {
    File,
    Folder,
    Markdown,
    Contact,
    Tag,
}

impl NodeType {
    pub fn color(&self) -> &'static str {
        match self {
            NodeType::File => "#6366f1",
            NodeType::Folder => "#f59e0b",
            NodeType::Markdown => "#10b981",
            NodeType::Contact => "#3b82f6",
            NodeType::Tag => "#ef4444",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            NodeType::File => "File",
            NodeType::Folder => "Folder",
            NodeType::Markdown => "Markdown",
            NodeType::Contact => "Contact",
            NodeType::Tag => "Tag",
        }
    }

    pub fn iter() -> impl Iterator<Item = Self> {
        vec![
            NodeType::File,
            NodeType::Folder,
            NodeType::Markdown,
            NodeType::Contact,
            NodeType::Tag,
        ]
        .into_iter()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EdgeType {
    Link,
    Tag,
    Folder,
    Comment,
}

#[derive(Debug, Clone)]
pub struct GraphNode {
    pub id: String,
    pub label: String,
    pub path: String,
    pub node_type: NodeType,
    pub size: f64,
    pub x: f64,
    pub y: f64,
    pub vx: f64,
    pub vy: f64,
}

#[derive(Debug, Clone)]
pub struct GraphEdge {
    pub source: String,
    pub target: String,
    pub edge_type: EdgeType,
}

#[derive(Debug, Clone)]
pub struct ForceGraph {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
    pub width: f64,
    pub height: f64,
    pub zoom: f64,
    pub pan_x: f64,
    pub pan_y: f64,
    pub dragging: Option<String>,
    pub drag_start_x: f64,
    pub drag_start_y: f64,
    pub is_panning: bool,
    pub pan_start_x: f64,
    pub pan_start_y: f64,
}

impl ForceGraph {
    pub fn new(width: f64, height: f64) -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
            width,
            height,
            zoom: 1.0,
            pan_x: 0.0,
            pan_y: 0.0,
            dragging: None,
            drag_start_x: 0.0,
            drag_start_y: 0.0,
            is_panning: false,
            pan_start_x: 0.0,
            pan_start_y: 0.0,
        }
    }

    pub fn simulate(&mut self, iterations: usize) {
        let repulsion = 5000.0;
        let attraction = 0.01;
        let centering = 0.005;
        let damping = 0.9;

        for _ in 0..iterations {
            let node_count = self.nodes.len();
            if node_count == 0 {
                continue;
            }

            let mut forces: Vec<(f64, f64)> = vec![(0.0, 0.0); node_count];

            for (i, force) in forces.iter_mut().enumerate().take(node_count) {
                // Repulsion from other nodes
                for j in 0..node_count {
                    if i == j {
                        continue;
                    }
                    let dx = self.nodes[i].x - self.nodes[j].x;
                    let dy = self.nodes[i].y - self.nodes[j].y;
                    let dist_sq = dx * dx + dy * dy + 1.0;
                    let f = repulsion / dist_sq;
                    let dist = dist_sq.sqrt();
                    force.0 += (dx / dist) * f;
                    force.1 += (dy / dist) * f;
                }

                // Attraction along edges
                for edge in &self.edges {
                    let other_idx = if edge.source == self.nodes[i].id {
                        self.nodes.iter().position(|n| n.id == edge.target)
                    } else if edge.target == self.nodes[i].id {
                        self.nodes.iter().position(|n| n.id == edge.source)
                    } else {
                        None
                    };

                    if let Some(j) = other_idx {
                        let dx = self.nodes[j].x - self.nodes[i].x;
                        let dy = self.nodes[j].y - self.nodes[i].y;
                        let dist = (dx * dx + dy * dy).sqrt() + 0.001;
                        let f = dist * attraction;
                        force.0 += (dx / dist) * f;
                        force.1 += (dy / dist) * f;
                    }
                }

                // Centering force
                force.0 += (self.width / 2.0 - self.nodes[i].x) * centering;
                force.1 += (self.height / 2.0 - self.nodes[i].y) * centering;
            }

            // Apply forces
            for (i, force) in forces.iter().enumerate().take(node_count) {
                self.nodes[i].vx = (self.nodes[i].vx + force.0) * damping;
                self.nodes[i].vy = (self.nodes[i].vy + force.1) * damping;
                self.nodes[i].x += self.nodes[i].vx;
                self.nodes[i].y += self.nodes[i].vy;
            }
        }
    }

    pub fn node_at(&self, x: f64, y: f64) -> Option<&GraphNode> {
        let graph_x = (x - self.pan_x) / self.zoom;
        let graph_y = (y - self.pan_y) / self.zoom;
        let hit_radius = 12.0;

        self.nodes.iter().find(|node| {
            let dx = node.x - graph_x;
            let dy = node.y - graph_y;
            (dx * dx + dy * dy).sqrt() < hit_radius + node.size
        })
    }
}

#[component]
pub fn GraphView(
    entries: Vec<api::FileEntry>,
    on_open_file: Callback<String>,
) -> impl IntoView {
    let (graph, set_graph) = signal(ForceGraph::new(800.0, 600.0));
    let (search_filter, set_search_filter) = signal(String::new());
    let (type_filter, set_type_filter) = signal(Option::<NodeType>::None);
    let (tooltip_node, set_tooltip_node) = signal(Option::<GraphNode>::None);
    let (tooltip_x, set_tooltip_x) = signal(0.0);
    let (tooltip_y, set_tooltip_y) = signal(0.0);
    let (show_all, set_show_all) = signal(false);
    let (total_entries, set_total_entries) = signal(0usize);

    // Build graph from entries
    Effect::new(move |_| {
        let show_all_nodes = show_all.get();
        let mut g = ForceGraph::new(800.0, 600.0);
        let mut node_map: HashMap<String, usize> = HashMap::new();

        // Cap nodes at MAX_NODES unless show_all is enabled
        let total = entries.len();
        set_total_entries.set(total);
        let cap = if show_all_nodes || total <= MAX_NODES {
            total
        } else {
            MAX_NODES
        };

        // Create nodes for each entry
        for (i, entry) in entries.iter().take(cap).enumerate() {
            let node_type = if entry.is_collection {
                NodeType::Folder
            } else if entry.mime_type.starts_with("text/markdown") {
                NodeType::Markdown
            } else if entry.mime_type.contains("vcard") {
                NodeType::Contact
            } else {
                NodeType::File
            };

            let node = GraphNode {
                id: entry.path.clone(),
                label: entry.name.clone(),
                path: entry.path.clone(),
                node_type,
                size: (entry.size as f64).sqrt().clamp(4.0, 20.0),
                x: 100.0 + ((i as f64) % 10.0) * 80.0,
                y: 100.0 + ((i as f64) / 10.0) * 80.0,
                vx: 0.0,
                vy: 0.0,
            };
            node_map.insert(entry.path.clone(), g.nodes.len());
            g.nodes.push(node);
        }

        // Create folder relationship edges (only for rendered nodes)
        for entry in entries.iter().take(cap) {
            if let Some(parent_pos) = entry.path.rfind('/') {
                let parent = if parent_pos == 0 {
                    "/".to_string()
                } else {
                    entry.path[..parent_pos].to_string()
                };
                if node_map.contains_key(&parent) && parent != entry.path {
                    g.edges.push(GraphEdge {
                        source: parent,
                        target: entry.path.clone(),
                        edge_type: EdgeType::Folder,
                    });
                }
            }
        }

        g.simulate(50);
        set_graph.set(g);
    });

    let handle_mouse_down = move |ev: ev::MouseEvent| {
        #[cfg(target_arch = "wasm32")]
        let rect = ev
            .current_target()
            .unwrap()
            .unchecked_into::<web_sys::HtmlElement>()
            .get_bounding_client_rect();
        #[cfg(not(target_arch = "wasm32"))]
        let rect = web_sys::DomRect::new().unwrap();
        let x = ev.client_x() as f64 - rect.left();
        let y = ev.client_y() as f64 - rect.top();

        let g = graph.get();
        if let Some(node) = g.node_at(x, y) {
            set_graph.update(|g| {
                g.dragging = Some(node.id.clone());
                g.drag_start_x = x;
                g.drag_start_y = y;
            });
        } else {
            set_graph.update(|g| {
                g.is_panning = true;
                g.pan_start_x = x - g.pan_x;
                g.pan_start_y = y - g.pan_y;
            });
        }
    };

    let handle_mouse_move = move |ev: ev::MouseEvent| {
        #[cfg(target_arch = "wasm32")]
        let rect = ev
            .current_target()
            .unwrap()
            .unchecked_into::<web_sys::HtmlElement>()
            .get_bounding_client_rect();
        #[cfg(not(target_arch = "wasm32"))]
        let rect = web_sys::DomRect::new().unwrap();
        let x = ev.client_x() as f64 - rect.left();
        let y = ev.client_y() as f64 - rect.top();

        set_graph.update(|g| {
            if let Some(ref node_id) = g.dragging.clone() {
                let dx = x - g.drag_start_x;
                let dy = y - g.drag_start_y;
                if let Some(node) = g.nodes.iter_mut().find(|n| n.id == *node_id) {
                    node.x += dx / g.zoom;
                    node.y += dy / g.zoom;
                    node.vx = 0.0;
                    node.vy = 0.0;
                }
                g.drag_start_x = x;
                g.drag_start_y = y;
            } else if g.is_panning {
                g.pan_x = x - g.pan_start_x;
                g.pan_y = y - g.pan_start_y;
            }
        });

        // Update tooltip
        let g = graph.get();
        if let Some(node) = g.node_at(x, y) {
            set_tooltip_node.set(Some(node.clone()));
            set_tooltip_x.set(x + 10.0);
            set_tooltip_y.set(y - 30.0);
        } else {
            set_tooltip_node.set(None);
        }
    };

    let handle_mouse_up = move |_: ev::MouseEvent| {
        set_graph.update(|g| {
            g.dragging = None;
            g.is_panning = false;
        });
    };

    let handle_wheel = move |ev: ev::WheelEvent| {
        ev.prevent_default();
        let delta = -ev.delta_y() / 1000.0;
        set_graph.update(|g| {
            g.zoom = (g.zoom + delta).clamp(0.1, 5.0);
        });
    };

    let handle_click = move |ev: ev::MouseEvent| {
        #[cfg(target_arch = "wasm32")]
        let rect = ev
            .current_target()
            .unwrap()
            .unchecked_into::<web_sys::HtmlElement>()
            .get_bounding_client_rect();
        #[cfg(not(target_arch = "wasm32"))]
        let rect = web_sys::DomRect::new().unwrap();
        let x = ev.client_x() as f64 - rect.left();
        let y = ev.client_y() as f64 - rect.top();

        let g = graph.get();
        if let Some(node) = g.node_at(x, y) {
            on_open_file.run(node.path.clone());
        }
    };

    let toggle_type_filter = move |nt: NodeType| {
        set_type_filter.update(|f| {
            if *f == Some(nt.clone()) {
                *f = None;
            } else {
                *f = Some(nt);
            }
        });
    };

    let reset_view = move |_: ev::MouseEvent| {
        set_graph.update(|g| {
            g.zoom = 1.0;
            g.pan_x = 0.0;
            g.pan_y = 0.0;
        });
    };

    view! {
        <div class="flex flex-col h-full">
            <div class="flex items-center gap-3 px-4 py-2 bg-[var(--bg-surface)] border-b border-[var(--border-default)]">
                <input
                    type="text"
                    placeholder="Search nodes..."
                    prop:value=move || search_filter.get()
                    on:input=move |ev| set_search_filter.set(event_target_value(&ev))
                    class="flex-1 px-3 py-1.5 text-sm border border-[var(--border-default)] rounded bg-[var(--bg-base)] text-[var(--text-primary)] placeholder-[var(--text-tertiary)]"
                />
                <div class="flex gap-1">
                    {NodeType::iter().map(|nt| {
                        let nt_for_click = nt.clone();
                        view! {
                            <button
                                class=move || {
                                    let nt_clone = nt.clone();
                                    format!(
                                        "px-2 py-1 text-xs rounded border transition-colors {}",
                                        if type_filter.get() == Some(nt_clone) {
                                            "bg-[var(--accent)] text-white border-[var(--accent)]"
                                        } else {
                                            "bg-[var(--bg-base)] text-[var(--text-secondary)] border-[var(--border-default)] hover:bg-[var(--interactive-hover)]"
                                        }
                                    )
                                }
                                on:click=move |_| toggle_type_filter(nt_for_click.clone())
                            >
                                {nt.label()}
                            </button>
                        }
                    }).collect::<Vec<_>>()}
                </div>
                <button
                    on:click=reset_view
                    class="px-2 py-1 text-xs rounded border border-[var(--border-default)] bg-[var(--bg-base)] text-[var(--text-secondary)] hover:bg-[var(--interactive-hover)]"
                >
                    "Reset View"
                </button>
                {move || {
                    let total = total_entries.get();
                    if total > MAX_NODES && !show_all.get() {
                        view! {
                            <span class="text-xs text-[var(--text-warning)]">
                                {format!("Showing {} of {} nodes", MAX_NODES, total)}
                            </span>
                            <button
                                on:click=move |_| set_show_all.set(true)
                                class="px-2 py-1 text-xs rounded border border-[var(--accent)] text-[var(--accent)] hover:bg-[var(--accent)]/10"
                            >
                                "Show all (may be slow)"
                            </button>
                        }.into_any()
                    } else {
                        ().into_any()
                    }
                }}
                <div class="text-xs text-[var(--text-tertiary)]">
                    {move || format!("Nodes: {} | Edges: {}", graph.get().nodes.len(), graph.get().edges.len())}
                </div>
            </div>

            <div
                class="flex-1 relative overflow-hidden bg-[var(--bg-base)] cursor-crosshair touch-none"
                on:mousedown=handle_mouse_down
                on:mousemove=handle_mouse_move
                on:mouseup=handle_mouse_up
                on:mouseleave=handle_mouse_up
                on:wheel=handle_wheel
                on:click=handle_click
            >
                <svg
                    width="100%"
                    height="100%"
                    viewBox=move || {
                        let g = graph.get();
                        format!("{} {} {} {}",
                            -g.pan_x / g.zoom,
                            -g.pan_y / g.zoom,
                            g.width / g.zoom,
                            g.height / g.zoom
                        )
                    }
                    class="select-none"
                >
                    {move || {
                        let g = graph.get();
                        g.edges.iter().map(|edge| {
                            let source = g.nodes.iter().find(|n| n.id == edge.source);
                            let target = g.nodes.iter().find(|n| n.id == edge.target);
                            if let (Some(s), Some(t)) = (source, target) {
                                view! {
                                    <line
                                        x1=s.x.to_string()
                                        y1=s.y.to_string()
                                        x2=t.x.to_string()
                                        y2=t.y.to_string()
                                        stroke=match edge.edge_type {
                                            EdgeType::Link => "#6366f1",
                                            EdgeType::Tag => "#ef4444",
                                            EdgeType::Folder => "#f59e0b",
                                            EdgeType::Comment => "#8b5cf6",
                                        }
                                        stroke-width="1.5"
                                        stroke-opacity="0.6"
                                    />
                                }.into_any()
                            } else {
                                ().into_any()
                            }
                        }).collect::<Vec<_>>()
                    }}

                    {move || {
                        let g = graph.get();
                        let q = search_filter.get().to_lowercase();
                        let type_f = type_filter.get();
                        g.nodes.iter().filter(|node| {
                            if !q.is_empty() && !node.label.to_lowercase().contains(&q) {
                                return false;
                            }
                            if let Some(ref tf) = type_f
                                && node.node_type != *tf {
                                    return false;
                                }
                            true
                        }).map(|node| {
                            let id = node.id.clone();
                            let is_dragging = graph.get().dragging.as_ref() == Some(&id);
                            view! {
                                <g
                                    transform=format!("translate({},{})", node.x, node.y)
                                    class=if is_dragging { "cursor-grabbing" } else { "cursor-grab" }
                                >
                                    <circle
                                        r=node.size.to_string()
                                        fill=node.node_type.color()
                                        stroke=if is_dragging { "#fff" } else { "transparent" }
                                        stroke-width="2"
                                        opacity=if is_dragging { "1.0" } else { "0.8" }
                                    />
                                    <text
                                        dy=(node.size + 12.0).to_string()
                                        text-anchor="middle"
                                        class="fill-[var(--text-secondary)] text-[10px] pointer-events-none"
                                    >
                                        {node.label.clone()}
                                    </text>
                                </g>
                            }
                        }).collect::<Vec<_>>()
                    }}
                </svg>

                {move || {
                    tooltip_node.get().map(|node| view! {
                        <div
                            class="fixed z-50 px-3 py-2 bg-[var(--bg-surface)] border border-[var(--border-default)] rounded shadow-lg text-sm pointer-events-none"
                            style=format!("left:{}px;top:{}px", tooltip_x.get(), tooltip_y.get())
                        >
                            <div class="font-bold text-[var(--text-primary)]">{node.label}</div>
                            <div class="text-xs text-[var(--text-tertiary)]">{node.path}</div>
                            <div class="text-xs text-[var(--text-tertiary)]">{node.node_type.label()}</div>
                        </div>
                    })
                }}
            </div>

            <div class="flex items-center gap-4 px-4 py-2 bg-[var(--bg-surface)] border-t border-[var(--border-default)] text-xs text-[var(--text-tertiary)]">
                <span class="font-bold">Legend:</span>
                {NodeType::iter().map(|nt| view! {
                    <span class="flex items-center gap-1">
                        <span class="w-3 h-3 rounded-full" style=format!("background-color:{}", nt.color())></span>
                        {nt.label()}
                    </span>
                }).collect::<Vec<_>>()}
                <span class="flex items-center gap-1">
                    <span class="w-3 h-0.5 bg-[#6366f1]"></span>
                    "Link"
                </span>
                <span class="flex items-center gap-1">
                    <span class="w-3 h-0.5 bg-[#ef4444]"></span>
                    "Tag"
                </span>
                <span class="flex items-center gap-1">
                    <span class="w-3 h-0.5 bg-[#f59e0b]"></span>
                    "Folder"
                </span>
            </div>
        </div>
    }
}
