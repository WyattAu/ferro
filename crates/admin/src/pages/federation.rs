use leptos::*;

use crate::api::ApiState;
use crate::components::badge::{Badge, BadgeVariant};

#[component]
pub fn FederationPage(api: RwSignal<ApiState>) -> impl IntoView {
    let (followers, set_followers) = create_signal(Vec::<serde_json::Value>::new());
    let (following, set_following) = create_signal(Vec::<serde_json::Value>::new());
    let (inbox, set_inbox) = create_signal(Vec::<serde_json::Value>::new());
    let (outbox, set_outbox) = create_signal(Vec::<serde_json::Value>::new());
    let (nodeinfo, set_nodeinfo) = create_signal(None::<serde_json::Value>);
    let (error, set_error) = create_signal(None::<String>);
    let (loading, set_loading) = create_signal(true);
    let (active_tab, set_active_tab) = create_signal(String::from("followers"));

    create_effect(move |_| {
        let api_clone = api.get_untracked();
        spawn_local(async move {
            set_loading.set(true);
            set_error.set(None);
            let mut errors = Vec::new();
            match api_clone.federation_followers("admin").await {
                Ok(f) => set_followers.set(f),
                Err(e) => errors.push(e),
            }
            match api_clone.federation_following("admin").await {
                Ok(f) => set_following.set(f),
                Err(e) => errors.push(e),
            }
            match api_clone.federation_inbox().await {
                Ok(i) => set_inbox.set(i),
                Err(e) => errors.push(e),
            }
            match api_clone.federation_outbox().await {
                Ok(o) => set_outbox.set(o),
                Err(e) => errors.push(e),
            }
            match api_clone.federation_nodeinfo().await {
                Ok(n) => set_nodeinfo.set(Some(n)),
                Err(e) => errors.push(e),
            }
            if !errors.is_empty() {
                set_error.set(Some(errors.join("; ")));
            }
            set_loading.set(false);
        });
    });

    let set_tab = move |tab: String| set_active_tab.set(tab);

    view! {
        <div class="page">
            {move || loading.get().then(|| view! { <div class="loading">"Loading federation data..."</div> })}
            {move || error.get().map(|e| view! { <div class="error-banner">{e}</div> })}

            <div class="stats-grid">
                <div class="stats-card"><div class="stats-card-header"><span class="stats-card-title">"Followers"</span></div><div class="stats-card-value">{followers.get().len()}</div></div>
                <div class="stats-card"><div class="stats-card-header"><span class="stats-card-title">"Following"</span></div><div class="stats-card-value">{following.get().len()}</div></div>
                <div class="stats-card"><div class="stats-card-header"><span class="stats-card-title">"Inbox"</span></div><div class="stats-card-value">{inbox.get().len()}</div></div>
                <div class="stats-card"><div class="stats-card-header"><span class="stats-card-title">"Outbox"</span></div><div class="stats-card-value">{outbox.get().len()}</div></div>
            </div>

            {move || nodeinfo.get().map(|ni| {
                let software = ni.get("software").and_then(|s| s.get("name")).and_then(|v| v.as_str()).unwrap_or("-").to_string();
                let ver = ni.get("software").and_then(|s| s.get("version")).and_then(|v| v.as_str()).unwrap_or("-").to_string();
                view! {
                    <div class="panel">
                        <h3 class="panel-title">"Node Information"</h3>
                        <div class="detail-row"><span class="detail-label">"Software"</span><span class="detail-value">{software}</span></div>
                        <div class="detail-row"><span class="detail-label">"Version"</span><span class="detail-value">{ver}</span></div>
                    </div>
                }
            })}

            <div class="panel">
                <div class="tab-bar" role="tablist">
                    <button class={format!("tab {}", if active_tab.get() == "followers" { "tab-active" } else { "" })} on:click=move |_| set_tab("followers".to_string()) role="tab" aria-selected={active_tab.get() == "followers"}>"Followers"</button>
                    <button class={format!("tab {}", if active_tab.get() == "following" { "tab-active" } else { "" })} on:click=move |_| set_tab("following".to_string()) role="tab" aria-selected={active_tab.get() == "following"}>"Following"</button>
                    <button class={format!("tab {}", if active_tab.get() == "inbox" { "tab-active" } else { "" })} on:click=move |_| set_tab("inbox".to_string()) role="tab" aria-selected={active_tab.get() == "inbox"}>"Inbox"</button>
                    <button class={format!("tab {}", if active_tab.get() == "outbox" { "tab-active" } else { "" })} on:click=move |_| set_tab("outbox".to_string()) role="tab" aria-selected={active_tab.get() == "outbox"}>"Outbox"</button>
                </div>
                {move || {
                    let tab = active_tab.get();
                    match tab.as_str() {
                        "followers" => {
                            let items = followers.get().iter().map(|item| {
                                let actor = item.get("actor").and_then(|a| a.as_str()).unwrap_or("-").to_string();
                                let act_type = item.get("type").and_then(|t| t.as_str()).unwrap_or("Follow").to_string();
                                let bv = if act_type == "Follow" { BadgeVariant::Success } else { BadgeVariant::Info };
                                view! { <tr><td class="mono">{actor}</td><td><Badge text=act_type variant=bv/></td></tr> }
                            }).collect::<Vec<_>>();
                            if items.is_empty() { view! { <div role="tabpanel"><table class="data-table"><tbody><tr><td class="table-empty">"No followers"</td></tr></tbody></table></div> }.into_any() } else { view! { <div role="tabpanel"><div class="table-wrapper"><table class="data-table"><thead><tr><th>"Actor"</th><th>"Type"</th></tr></thead><tbody>{items}</tbody></table></div></div> }.into_any() }
                        }
                        "following" => {
                            let items = following.get().iter().map(|item| {
                                let actor = item.get("actor").and_then(|a| a.as_str()).unwrap_or("-").to_string();
                                let act_type = item.get("type").and_then(|t| t.as_str()).unwrap_or("Follow").to_string();
                                let bv = BadgeVariant::Info;
                                view! { <tr><td class="mono">{actor}</td><td><Badge text=act_type variant=bv/></td></tr> }
                            }).collect::<Vec<_>>();
                            if items.is_empty() { view! { <div role="tabpanel"><table class="data-table"><tbody><tr><td class="table-empty">"Not following anyone"</td></tr></tbody></table></div> }.into_any() } else { view! { <div role="tabpanel"><div class="table-wrapper"><table class="data-table"><thead><tr><th>"Actor"</th><th>"Type"</th></tr></thead><tbody>{items}</tbody></table></div></div> }.into_any() }
                        }
                        "inbox" => {
                            let items = inbox.get().iter().take(20).map(|item| {
                                let act_type = item.get("type").and_then(|t| t.as_str()).unwrap_or("-").to_string();
                                let actor = item.get("actor").and_then(|a| a.as_str()).unwrap_or("-").to_string();
                                let id = item.get("id").and_then(|i| i.as_str()).unwrap_or("-").to_string();
                                view! { <tr><td><Badge text=act_type variant=BadgeVariant::Neutral/></td><td class="mono">{actor}</td><td class="mono">{id}</td></tr> }
                            }).collect::<Vec<_>>();
                            if items.is_empty() { view! { <div role="tabpanel"><table class="data-table"><tbody><tr><td class="table-empty">"Inbox is empty"</td></tr></tbody></table></div> }.into_any() } else { view! { <div role="tabpanel"><div class="table-wrapper"><table class="data-table"><thead><tr><th>"Type"</th><th>"Actor"</th><th>"ID"</th></tr></thead><tbody>{items}</tbody></table></div></div> }.into_any() }
                        }
                        "outbox" => {
                            let items = outbox.get().iter().take(20).map(|item| {
                                let act_type = item.get("type").and_then(|t| t.as_str()).unwrap_or("-").to_string();
                                let actor = item.get("actor").and_then(|a| a.as_str()).unwrap_or("-").to_string();
                                let id = item.get("id").and_then(|i| i.as_str()).unwrap_or("-").to_string();
                                view! { <tr><td><Badge text=act_type variant=BadgeVariant::Neutral/></td><td class="mono">{actor}</td><td class="mono">{id}</td></tr> }
                            }).collect::<Vec<_>>();
                            if items.is_empty() { view! { <div role="tabpanel"><table class="data-table"><tbody><tr><td class="table-empty">"Outbox is empty"</td></tr></tbody></table></div> }.into_any() } else { view! { <div role="tabpanel"><div class="table-wrapper"><table class="data-table"><thead><tr><th>"Type"</th><th>"Actor"</th><th>"ID"</th></tr></thead><tbody>{items}</tbody></table></div></div> }.into_any() }
                        }
                        _ => view! { <div class="empty-state">"Select a tab"</div> }.into_any(),
                    }
                }}
            </div>
        </div>
    }
}
