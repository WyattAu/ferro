use leptos::ev;
use leptos::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum BlockType {
    Paragraph, Heading1, Heading2, Heading3, Heading4, Heading5, Heading6,
    BulletList, NumberedList, Todo, Code, Quote, Divider, Image, Table,
}

impl BlockType {
    fn from_shortcut(s: &str) -> Option<Self> {
        match s {
            "# " => Some(Self::Heading1), "## " => Some(Self::Heading2),
            "### " => Some(Self::Heading3), "#### " => Some(Self::Heading4),
            "##### " => Some(Self::Heading5), "###### " => Some(Self::Heading6),
            "- " | "* " => Some(Self::BulletList), "1. " => Some(Self::NumberedList),
            "[] " => Some(Self::Todo), "> " => Some(Self::Quote),
            "---" => Some(Self::Divider), "```" => Some(Self::Code),
            _ => None,
        }
    }
    fn label(&self) -> &str {
        match self {
            Self::Paragraph => "Paragraph", Self::Heading1 => "Heading 1",
            Self::Heading2 => "Heading 2", Self::Heading3 => "Heading 3",
            Self::Heading4 => "Heading 4", Self::Heading5 => "Heading 5",
            Self::Heading6 => "Heading 6", Self::BulletList => "Bullet List",
            Self::NumberedList => "Numbered List", Self::Todo => "Todo / Checkbox",
            Self::Code => "Code Block", Self::Quote => "Quote",
            Self::Divider => "Divider", Self::Image => "Image", Self::Table => "Table",
        }
    }
    fn placeholder(&self) -> &str {
        match self {
            Self::Paragraph => "Type '/' for commands...",
            Self::Heading1 | Self::Heading2 | Self::Heading3
            | Self::Heading4 | Self::Heading5 | Self::Heading6 => "Heading",
            Self::BulletList | Self::NumberedList => "List item",
            Self::Todo => "To-do", Self::Code => "Code",
            Self::Quote => "Quote", Self::Divider => "",
            Self::Image => "Paste image URL or drag & drop", Self::Table => "Table",
        }
    }
    fn icon(&self) -> &str {
        match self {
            Self::Paragraph => "¶", Self::Heading1 => "H1", Self::Heading2 => "H2",
            Self::Heading3 => "H3", Self::Heading4 => "H4", Self::Heading5 => "H5",
            Self::Heading6 => "H6", Self::BulletList => "•", Self::NumberedList => "1.",
            Self::Todo => "☐", Self::Code => "<>", Self::Quote => "❝",
            Self::Divider => "—", Self::Image => "🖼", Self::Table => "⊞",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Block {
    pub id: String,
    pub block_type: BlockType,
    pub content: String,
    pub checked: bool,
    pub meta: serde_json::Value,
}

impl Block {
    fn new(block_type: BlockType) -> Self {
        Self { id: uuid::Uuid::new_v4().to_string(), block_type, content: String::new(), checked: false, meta: serde_json::Value::Null }
    }
}

fn blocks_to_json(blocks: &[Block]) -> String { serde_json::to_string_pretty(blocks).unwrap_or_default() }
fn json_to_blocks(json: &str) -> Vec<Block> { serde_json::from_str(json).unwrap_or_default() }

#[component]
pub fn BlockEditor(#[prop(optional)] initial_content: String, #[prop(optional)] readonly: bool) -> impl IntoView {
    let (blocks, set_blocks) = signal(if initial_content.is_empty() { vec![Block::new(BlockType::Paragraph)] } else { json_to_blocks(&initial_content) });
    let (active_block_id, set_active_block_id) = signal(blocks.get().first().map(|b| b.id.clone()).unwrap_or_default());
    let (show_slash_menu, set_show_slash_menu) = signal(false);
    let (slash_filter, set_slash_filter) = signal(String::new());
    let (dragging_id, set_dragging_id) = signal(None::<String>);
    let (drag_over_id, set_drag_over_id) = signal(None::<String>);

    let all_block_types = vec![
        BlockType::Paragraph, BlockType::Heading1, BlockType::Heading2, BlockType::Heading3,
        BlockType::Heading4, BlockType::Heading5, BlockType::Heading6, BlockType::BulletList,
        BlockType::NumberedList, BlockType::Todo, BlockType::Code, BlockType::Quote,
        BlockType::Divider, BlockType::Image, BlockType::Table,
    ];

    let filtered_types = move || {
        let filter = slash_filter.get().to_lowercase();
        all_block_types.iter().filter(|bt| bt.label().to_lowercase().contains(&filter)).cloned().collect::<Vec<_>>()
    };

    let render_block = move |block: Block| {
        let block_id = block.id.clone();

        let bid1 = block_id.clone();
        let is_active = Memo::new(move |_| active_block_id.get() == bid1);
        let bid2 = block_id.clone();
        let is_dragging = Memo::new(move |_| dragging_id.get() == Some(bid2.clone()));
        let bid3 = block_id.clone();
        let is_drag_over = Memo::new(move |_| drag_over_id.get() == Some(bid3.clone()));
        let bid4 = block_id.clone();
        let content_signal = Memo::new(move |_| blocks.get().iter().find(|b| b.id == bid4).map(|b| b.content.clone()).unwrap_or_default());
        let bid5 = block_id.clone();
        let checked_signal = Memo::new(move |_| blocks.get().iter().find(|b| b.id == bid5).map(|b| b.checked).unwrap_or(false));
        let bid6 = block_id.clone();
        let block_type_signal = Memo::new(move |_| blocks.get().iter().find(|b| b.id == bid6).map(|b| b.block_type.clone()).unwrap_or(BlockType::Paragraph));
        let bid7 = block_id.clone();
        let bid_signal = Memo::new(move |_| bid7.clone());

        view! {
            <div
                class=move || {
                    let mut cls = "group relative flex items-start gap-1 px-2 py-1 rounded transition-colors mb-1".to_string();
                    if is_active.get() { cls.push_str(" bg-[var(--bg-inset)]"); }
                    if is_drag_over.get() { cls.push_str(" border-t-2 border-[var(--accent)]"); }
                    if is_dragging.get() { cls.push_str(" opacity-50"); }
                    cls
                }
                draggable="true"
                on:dragstart=move |ev: ev::DragEvent| {
                    let bid = bid_signal.get();
                    set_dragging_id.set(Some(bid.clone()));
                    if let Some(data) = ev.data_transfer() { let _ = data.set_data("text/plain", &bid); }
                }
                on:dragend=move |_: ev::DragEvent| { set_dragging_id.set(None); set_drag_over_id.set(None); }
                on:dragover=move |ev: ev::DragEvent| { ev.prevent_default(); set_drag_over_id.set(Some(bid_signal.get())); }
                on:dragleave=move |_: ev::DragEvent| { set_drag_over_id.set(None); }
                on:drop=move |ev: ev::DragEvent| {
                    ev.prevent_default();
                    if let Some(data) = ev.data_transfer()
                        && let Ok(from_id) = data.get_data("text/plain")
                    {
                            let bid = bid_signal.get();
                            let mut current = blocks.get();
                            let from_pos = current.iter().position(|b| b.id == from_id);
                            let to_pos = current.iter().position(|b| b.id == bid);
                            if let (Some(from), Some(to)) = (from_pos, to_pos) {
                                let block = current.remove(from);
                                current.insert(to, block);
                                set_blocks.set(current);
                            }
                    }
                    set_dragging_id.set(None); set_drag_over_id.set(None);
                }
                on:click=move |_: ev::MouseEvent| { set_active_block_id.set(bid_signal.get()); }
            >
                <div class="opacity-0 group-hover:opacity-100 transition-opacity cursor-grab text-[var(--text-tertiary)] select-none pt-0.5 w-4 text-center">"⋮⋮"</div>
                <div class="flex-1 min-w-0">
                    {move || match block_type_signal.get() {
                        BlockType::Divider => view! { <hr class="my-2 border-[var(--border-default)]" /> }.into_any(),
                        BlockType::Todo => view! {
                            <div class="flex items-center gap-2">
                                <input type="checkbox" prop:checked=move || checked_signal.get()
                                    on:change=move |_: ev::Event| {
                                        let bid = bid_signal.get();
                                        let mut current = blocks.get();
                                        if let Some(block) = current.iter_mut().find(|b| b.id == bid) { block.checked = !block.checked; set_blocks.set(current); }
                                    }
                                    class="w-4 h-4 rounded border-[var(--border-default)]" disabled=readonly />
                                <input type="text" prop:value=move || content_signal.get()
                                    on:input=move |ev: ev::Event| {
                                        let bid = bid_signal.get();
                                        let mut current = blocks.get();
                                        if let Some(block) = current.iter_mut().find(|b| b.id == bid) { block.content = event_target_value(&ev); set_blocks.set(current); }
                                    }
                                    placeholder="To-do" class="flex-1 bg-transparent text-sm text-[var(--text-primary)] focus:outline-none border-b border-transparent focus:border-[var(--accent)]" disabled=readonly />
                            </div>
                        }.into_any(),
                        BlockType::Code => view! {
                            <textarea prop:value=move || content_signal.get()
                                on:input=move |ev: ev::Event| {
                                    let bid = bid_signal.get();
                                    let mut current = blocks.get();
                                    if let Some(block) = current.iter_mut().find(|b| b.id == bid) { block.content = event_target_value(&ev); set_blocks.set(current); }
                                }
                                placeholder="Code" class="w-full bg-[var(--bg-inset)] font-mono text-xs p-2 rounded border border-[var(--border-default)] text-[var(--text-primary)] resize-y focus:outline-none focus:border-[var(--accent)]" rows="3" disabled=readonly />
                        }.into_any(),
                        BlockType::Image => view! {
                            <div>
                                <input type="text" prop:value=move || content_signal.get()
                                    on:input=move |ev: ev::Event| {
                                        let bid = bid_signal.get();
                                        let mut current = blocks.get();
                                        if let Some(block) = current.iter_mut().find(|b| b.id == bid) { block.content = event_target_value(&ev); set_blocks.set(current); }
                                    }
                                    placeholder="Paste image URL..." class="w-full bg-transparent text-sm text-[var(--text-primary)] focus:outline-none border border-[var(--border-default)] rounded p-1 font-mono" disabled=readonly />
                                {move || {
                                    let url = content_signal.get();
                                    if !url.is_empty() { view! { <img src=url alt="Block image" class="mt-2 max-w-full max-h-64 rounded border border-[var(--border-default)] object-contain" /> }.into_any() }
                                    else { view! { <span class="hidden"></span> }.into_any() }
                                }}
                            </div>
                        }.into_any(),
                        _ => view! {
                            <input type="text" prop:value=move || content_signal.get()
                                on:keydown=move |ev: ev::KeyboardEvent| {
                                    let bid = bid_signal.get();
                                    let key = ev.key();
                                    let val = content_signal.get();
                                    if key == "Enter" && !ev.shift_key() {
                                        ev.prevent_default();
                                        let bt = match block_type_signal.get() {
                                            BlockType::BulletList => BlockType::BulletList,
                                            BlockType::NumberedList => BlockType::NumberedList,
                                            BlockType::Todo => BlockType::Todo,
                                            _ => BlockType::Paragraph,
                                        };
                                        let mut current = blocks.get();
                                        let pos = current.iter().position(|b| b.id == bid);
                                        let new_block = Block::new(bt);
                                        let new_id = new_block.id.clone();
                                        if let Some(pos) = pos { current.insert(pos + 1, new_block); } else { current.push(new_block); }
                                        set_blocks.set(current); set_active_block_id.set(new_id);
                                    }
                                    if key == "Backspace" && val.is_empty() && blocks.get().len() > 1 {
                                        ev.prevent_default();
                                        let mut current = blocks.get();
                                        if let Some(pos) = current.iter().position(|b| b.id == bid) {
                                            current.remove(pos);
                                            let new_active = current.get(pos.min(current.len() - 1)).map(|b| b.id.clone()).unwrap_or_default();
                                            set_blocks.set(current); set_active_block_id.set(new_active);
                                        }
                                    }
                                    if key == "/" && val.is_empty() { ev.prevent_default(); set_show_slash_menu.set(true); set_slash_filter.set(String::new()); }
                                }
                                on:input=move |ev: ev::Event| {
                                    let bid = bid_signal.get();
                                    let val = event_target_value(&ev);
                                    if show_slash_menu.get() {
                                        if let Some(stripped) = val.strip_prefix('/') { set_slash_filter.set(stripped.to_string()); }
                                        else { set_show_slash_menu.set(false); }
                                    }
                                    let mut current = blocks.get();
                                    if let Some(block) = current.iter_mut().find(|b| b.id == bid) {
                                        block.content = val.clone();
                                        if let Some(shortcut) = detect_shortcut(&val) { block.block_type = shortcut; block.content = strip_shortcut_prefix(&val); }
                                        set_blocks.set(current);
                                    }
                                }
                                placeholder=block_type_signal.get().placeholder()
                                class="w-full bg-transparent text-sm text-[var(--text-primary)] focus:outline-none" disabled=readonly />
                        }.into_any(),
                    }}
                </div>
                <div class="opacity-0 group-hover:opacity-100 transition-opacity flex items-center gap-1">
                    <span class="text-[10px] font-mono text-[var(--text-tertiary)] px-1">{block_type_signal.get().icon().to_string()}</span>
                    <button class="text-[10px] text-[var(--text-tertiary)] hover:text-[var(--danger)] px-1" title="Delete block"
                        on:click=move |_: ev::MouseEvent| {
                            let bid = bid_signal.get();
                            let mut current = blocks.get();
                            if current.len() <= 1 { return; }
                            if let Some(pos) = current.iter().position(|b| b.id == bid) {
                                current.remove(pos);
                                let new_active = current.get(pos.min(current.len() - 1)).map(|b| b.id.clone()).unwrap_or_default();
                                set_blocks.set(current); set_active_block_id.set(new_active);
                            }
                        }> "×" </button>
                </div>
            </div>
        }
    };

    view! {
        <div class="flex flex-col border border-[var(--border-default)] rounded-lg overflow-hidden bg-[var(--bg-surface)]">
            <div class="flex items-center gap-1 px-2 py-1 bg-[var(--bg-inset)] border-b border-[var(--border-default)]">
                <span class="text-xs font-mono text-[var(--text-tertiary)] px-2">"Block Editor"</span>
                <div class="flex-1"></div>
                <span class="text-xs font-mono text-[var(--text-tertiary)]">{move || format!("{} blocks", blocks.get().len())}</span>
            </div>
            <div class="min-h-[300px] max-h-[70vh] overflow-y-auto p-2">
                {move || blocks.get().into_iter().map(render_block).collect::<Vec<_>>()}
                <button class="w-full text-left px-2 py-1 text-xs text-[var(--text-tertiary)] hover:text-[var(--text-secondary)] hover:bg-[var(--interactive-hover)] rounded transition-colors mt-1"
                    on:click=move |_: ev::MouseEvent| {
                        let last_id = blocks.get().last().map(|b| b.id.clone()).unwrap_or_default();
                        if !last_id.is_empty() {
                            let mut current = blocks.get();
                            let new_block = Block::new(BlockType::Paragraph);
                            let new_id = new_block.id.clone();
                            current.push(new_block); set_blocks.set(current); set_active_block_id.set(new_id);
                        }
                    } disabled=readonly> "+ Add block" </button>
            </div>
            {move || show_slash_menu.get().then(|| {
                let types = filtered_types();
                let active_id = active_block_id.get();
                view! {
                    <div class="fixed z-50 bg-[var(--bg-surface)] border border-[var(--border-default)] rounded-lg shadow-lg max-h-60 overflow-y-auto w-64">
                        <div class="px-2 py-1 text-xs font-mono text-[var(--text-tertiary)] border-b border-[var(--border-default)]">"Block type"</div>
                        {types.into_iter().map(|bt| {
                            let bt_icon = bt.icon().to_string();
                            let bt_label = bt.label().to_string();
                            let bt_click = bt.clone();
                            let active_id = active_id.clone();
                            view! {
                                <button class="w-full text-left px-3 py-1.5 text-sm hover:bg-[var(--interactive-hover)] flex items-center gap-2 transition-colors"
                                    on:click=move |_: ev::MouseEvent| {
                                        let mut current = blocks.get();
                                        if let Some(block) = current.iter_mut().find(|b| b.id == active_id) { block.block_type = bt_click.clone(); set_blocks.set(current); }
                                        set_show_slash_menu.set(false);
                                    }>
                                    <span class="text-xs font-mono w-6 text-center text-[var(--text-tertiary)]">{bt_icon}</span>
                                    <span class="text-[var(--text-primary)]">{bt_label}</span>
                                </button>
                            }
                        }).collect::<Vec<_>>()}
                    </div>
                }
            })}
        </div>
    }
}

fn detect_shortcut(content: &str) -> Option<BlockType> {
    let shortcuts = ["###### ", "##### ", "#### ", "### ", "## ", "# ", "- ", "* ", "1. ", "[] ", "> ", "```", "---"];
    for s in shortcuts { if content == s || content.starts_with(s) { return BlockType::from_shortcut(s); } }
    None
}

fn strip_shortcut_prefix(content: &str) -> String {
    let prefixes = ["###### ", "##### ", "#### ", "### ", "## ", "# ", "- ", "* ", "1. ", "[] ", "> ", "```", "---"];
    for p in prefixes { if let Some(stripped) = content.strip_prefix(p) { return stripped.to_string(); } }
    content.to_string()
}

pub fn serialize_blocks(blocks: &[Block]) -> String { blocks_to_json(blocks) }
pub fn deserialize_blocks(json: &str) -> Vec<Block> { json_to_blocks(json) }
