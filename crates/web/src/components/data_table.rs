use leptos::ev;
use leptos::prelude::*;

use crate::components::icons::{Icon, IconName};

/// Sort direction for a column.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SortDir {
    Asc,
    Desc,
}

/// Describes a sortable column header.
#[derive(Clone, Debug)]
pub struct ColumnDef {
    /// Unique key for the column.
    pub key: String,
    /// Display label.
    pub label: String,
    /// Whether this column is sortable.
    pub sortable: bool,
    /// Optional CSS class for width/styling.
    pub class: String,
}

impl ColumnDef {
    pub fn new(key: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            label: label.into(),
            sortable: false,
            class: String::new(),
        }
    }

    pub fn sortable(mut self) -> Self {
        self.sortable = true;
        self
    }

    pub fn class(mut self, class: impl Into<String>) -> Self {
        self.class = class.into();
        self
    }
}

/// Paginated data table with sortable columns and row selection.
///
/// Features:
/// - Sortable column headers with click-to-sort
/// - Row selection with checkboxes
/// - Pagination controls
/// - Empty state handling
/// - Keyboard accessible
#[component]
pub fn DataTable(
    /// Column definitions.
    columns: Vec<ColumnDef>,
    /// Total number of rows (across all pages).
    total_rows: usize,
    /// Current page (0-indexed).
    page: ReadSignal<usize>,
    /// Page setter.
    set_page: WriteSignal<usize>,
    /// Rows per page.
    #[prop(default = 25)]
    per_page: usize,
    /// Currently selected row keys.
    selected: ReadSignal<std::collections::HashSet<String>>,
    /// Toggle selection of a row.
    _on_toggle_select: Callback<String>,
    /// Toggle selection of all visible rows.
    on_toggle_select_all: Callback<()>,
    /// Current sort column key.
    sort_by: ReadSignal<Option<String>>,
    /// Current sort direction.
    sort_dir: ReadSignal<Option<SortDir>>,
    /// Called when a sortable column header is clicked.
    on_sort: Callback<(String, SortDir)>,
    /// Table body rows (rendered by caller).
    children: Children,
    /// Label for the table (for screen readers).
    #[prop(default = "Data table".to_string())]
    aria_label: String,
    /// Content shown when the table is empty.
    #[prop(default = None)]
    empty_state: Option<Children>,
) -> impl IntoView {
    let total_pages = std::cmp::max(1, total_rows.div_ceil(per_page));

    let on_prev = move |_: ev::MouseEvent| {
        let p = page.get();
        if p > 0 {
            set_page.set(p - 1);
        }
    };

    #[allow(unused_variables)]
    let on_next = move |_: ev::MouseEvent| {
        let p = page.get();
        if p + 1 < total_pages {
            set_page.set(p + 1);
        }
    };

    let on_first = move |_: ev::MouseEvent| set_page.set(0);
    #[allow(unused_variables)]
    let on_last = move |_: ev::MouseEvent| set_page.set(total_pages.saturating_sub(1));

    let all_selected = move || {
        let sel = selected.get();
        total_rows > 0 && sel.len() >= std::cmp::min(per_page, total_rows)
    };

    let page_start = page.get() * per_page + 1;
    let page_end = std::cmp::min((page.get() + 1) * per_page, total_rows);

    let is_empty = total_rows == 0;
    let columns_count = columns.len();
    let aria_label_for_div = aria_label.clone();

    view! {
        <div class="w-full" role="region" aria-label=aria_label_for_div tabindex="0">
            <div class="overflow-x-auto">
                <table class="w-full text-sm text-left" role="grid" aria-label=aria_label>
                    <thead>
                        <tr class="border-b border-[var(--border-default)]">
                            <th class="px-4 py-3 w-10">
                                <input
                                    type="checkbox"
                                    class="rounded border text-[var(--accent)] focus:ring-[var(--border-focus)]"
                                    prop:checked=all_selected
                                    aria-label="Select all rows"
                                    on:click=move |_| on_toggle_select_all.run(())
                                />
                            </th>
                            <For
                                each=move || columns.clone()
                                key=|col| col.key.clone()
                                let:col
                            >
                                {
                                    let col_key = col.key.clone();
                                    let col_label = col.label.clone();
                                    let col_label_for_aria = col.label.clone();
                                    let col_sortable = col.sortable;
                                    let col_class = col.class.clone();
                                    let sort_by_sig = sort_by;
                                    let sort_dir_sig = sort_dir;
                                    let on_sort_cb = on_sort;
                                    let col_key2 = col_key.clone();
                                    view! {
                                        <th class=format!("px-4 py-3 font-bold uppercase text-xs tracking-wider text-[var(--text-secondary)] {}", col_class)>
                                            {if col_sortable {
                                                let current_sort = move || {
                                                    let sb = sort_by_sig.get();
                                                    let sd = sort_dir_sig.get();
                                                    if sb.as_deref() == Some(col_key2.as_str()) {
                                                        sd
                                                    } else {
                                                        None
                                                    }
                                                };
                                                view! {
                                                    <button
                                                        class="flex items-center gap-1 hover:text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] rounded min-h-[44px] px-1"
                                                        aria-label=format!("Sort by {}", col_label_for_aria)
                                                        on:click=move |_| {
                                                            let sb = sort_by_sig.get();
                                                            let sd = sort_dir_sig.get().unwrap_or(SortDir::Asc);
                                                            let new_dir = if sb.as_deref() == Some(col_key.as_str()) {
                                                                match sd {
                                                                    SortDir::Asc => SortDir::Desc,
                                                                    SortDir::Desc => SortDir::Asc,
                                                                }
                                                            } else {
                                                                SortDir::Asc
                                                            };
                                                            on_sort_cb.run((col_key.clone(), new_dir));
                                                        }
                                                    >
                                                        {col_label}
                                                            <span class="text-[var(--text-tertiary)]">
                                                            {move || match current_sort() {
                                                                Some(SortDir::Asc) => view! {
                                                                    <Icon name=IconName::ArrowUp class="w-3 h-3".to_string() />
                                                                }.into_any(),
                                                                Some(SortDir::Desc) => view! {
                                                                    <Icon name=IconName::ArrowDown class="w-3 h-3".to_string() />
                                                                }.into_any(),
                                                                None => view! {
                                                                    <span class="w-3 h-3 inline-block"></span>
                                                                }.into_any(),
                                                            }}
                                                        </span>
                                                    </button>
                                                }.into_any()
                                            } else {
                                                view! { <span>{col_label}</span> }.into_any()
                                            }}
                                        </th>
                                    }
                                }
                            </For>
                            <th class="px-4 py-3 w-10"></th>
                        </tr>
                    </thead>
                    <tbody>
                        {if is_empty {
                            empty_state
                                .map(|es| view! {
                                    <tr>
                                        <td class="px-4 py-16 text-center text-[var(--text-tertiary)]" colspan=columns_count + 2>
                                            {es()}
                                        </td>
                                    </tr>
                                }.into_any())
                                .unwrap_or_else(|| {
                                    view! {
                                        <tr>
                                            <td class="px-4 py-16 text-center text-[var(--text-tertiary)]" colspan=columns_count + 2>
                                                <svg class="w-12 h-12 mx-auto mb-3 text-[var(--text-tertiary)]" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
                                                </svg>
                                                <div class="text-lg font-medium">"No data"</div>
                                                <div class="text-sm">"No records to display."</div>
                                            </td>
                                        </tr>
                                    }.into_any()
                                })
                        } else {
                            children().into_view().into_any()
                        }}
                    </tbody>
                </table>
            </div>

            <div class="flex items-center justify-between px-4 py-3 border-t border-[var(--border-default)]">
                <div class="text-sm text-[var(--text-tertiary)]">
                    {move || {
                        if total_rows == 0 {
                            "No records".to_string()
                        } else {
                            format!("Showing {}-{} of {}", page_start, page_end, total_rows)
                        }
                    }}
                </div>
                <nav class="flex items-center gap-1" aria-label="Pagination">
                    <button
                        class="p-2 rounded text-[var(--text-tertiary)] hover:text-[var(--text-secondary)] hover:bg-[var(--bg-inset)] focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] disabled:opacity-50 disabled:cursor-not-allowed min-w-[44px] min-h-[44px] flex items-center justify-center"
                        disabled=move || page.get() == 0
                        aria-label="First page"
                        on:click=on_first
                    >
                        <Icon name=IconName::ArrowLeft class="w-4 h-4".to_string() />
                        <Icon name=IconName::ArrowLeft class="w-4 h-4 -ml-2".to_string() />
                    </button>
                    <button
                        class="p-2 rounded text-[var(--text-tertiary)] hover:text-[var(--text-secondary)] hover:bg-[var(--bg-inset)] focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] disabled:opacity-50 disabled:cursor-not-allowed min-w-[44px] min-h-[44px] flex items-center justify-center"
                        disabled=move || page.get() == 0
                        aria-label="Previous page"
                        on:click=on_prev
                    >
                        <Icon name=IconName::ArrowLeft class="w-4 h-4".to_string() />
                    </button>

                    <span class="px-3 text-sm font-medium text-[var(--text-secondary)]">
                        {move || format!("{} / {}", page.get() + 1, total_pages)}
                    </span>

                    <button
                        class="p-2 rounded text-[var(--text-tertiary)] hover:text-[var(--text-secondary)] hover:bg-[var(--bg-inset)] focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] disabled:opacity-50 disabled:cursor-not-allowed min-w-[44px] min-h-[44px] flex items-center justify-center"
                        disabled=move || page.get() + 1 >= total_pages
                        aria-label="Next page"
                        on:click=on_next
                    >
                        <Icon name=IconName::ArrowRight class="w-4 h-4".to_string() />
                    </button>
                    <button
                        class="p-2 rounded text-[var(--text-tertiary)] hover:text-[var(--text-secondary)] hover:bg-[var(--bg-inset)] focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] disabled:opacity-50 disabled:cursor-not-allowed min-w-[44px] min-h-[44px] flex items-center justify-center"
                        disabled=move || page.get() + 1 >= total_pages
                        aria-label="Last page"
                        on:click=on_last
                    >
                        <Icon name=IconName::ArrowRight class="w-4 h-4".to_string() />
                        <Icon name=IconName::ArrowRight class="w-4 h-4 -ml-2".to_string() />
                    </button>
                </nav>
            </div>
        </div>
    }
}

/// A single selectable row inside a DataTable.
#[component]
pub fn TableRow(
    /// Unique key for this row.
    key: String,
    /// Whether this row is currently selected.
    is_selected: ReadSignal<bool>,
    /// Toggle selection of this row.
    on_toggle_select: Callback<String>,
    /// Row cell content.
    children: Children,
) -> impl IntoView {
    let key_clone = key.clone();
    let row_key = key.clone();

    view! {
        <tr
            class=move || {
                let base = "border-b border-[var(--border-subtle)] dark:border-[var(--border-strong)] transition-colors";
                if is_selected.get() {
                    format!("{} bg-[var(--accent-subtle)]", base)
                } else {
                    format!("{} hover:bg-[var(--bg-inset)] dark:hover:bg-[var(--interactive-hover)]", base)
                }
            }
            role="row"
            aria-selected=move || is_selected.get()
        >
            <td class="px-4 py-2.5 w-10" role="gridcell">
                <input
                    type="checkbox"
                    class="rounded border text-[var(--accent)] focus:ring-[var(--border-focus)]"
                    prop:checked=move || is_selected.get()
                    attr:aria-label=format!("Select row {}", row_key)
                    on:click=move |_| on_toggle_select.run(key_clone.clone())
                />
            </td>
            {children()}
        </tr>
    }
}
