use leptos::*;

use crate::api::ApiState;
use crate::components::badge::{Badge, BadgeVariant};
use crate::components::modal::Modal;
use crate::state::format_timestamp;

/// User row type for leptos-struct-table integration.
///
/// When migrating to leptos-struct-table, derive `TableRow` on this struct
/// and use `<Table rows=users />` instead of manual `<table>` rendering.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct UserRow {
    pub username: String,
    pub role: String,
    pub created_at: String,
    pub last_login: String,
}

#[component]
pub fn UsersPage(api: RwSignal<ApiState>) -> impl IntoView {
    let (users, set_users) = create_signal(Vec::<serde_json::Value>::new());
    let (error, set_error) = create_signal(None::<String>);
    let (loading, set_loading) = create_signal(true);
    let (search, set_search) = create_signal(String::new());
    let (show_create, set_show_create) = create_signal(false);
    let (show_delete_confirm, set_show_delete_confirm) = create_signal(false);
    let (delete_target, set_delete_target) = create_signal(String::new());
    let (form_error, set_form_error) = create_signal(None::<String>);
    let (msg, set_msg) = create_signal(None::<String>);
    let (new_username, set_new_username) = create_signal(String::new());
    let (new_password, set_new_password) = create_signal(String::new());
    let (new_role, set_new_role) = create_signal(String::from("viewer"));

    let load_users = move || {
        let api_clone = api.get_untracked();
        spawn_local(async move {
            set_loading.set(true);
            set_error.set(None);
            match api_clone.list_users().await {
                Ok(u) => set_users.set(u),
                Err(e) => set_error.set(Some(e)),
            }
            set_loading.set(false);
        });
    };

    create_effect(move |_| load_users());

    let filtered_users = move || {
        let query = search.get().to_lowercase();
        let all = users.get();
        if query.is_empty() {
            all
        } else {
            all.into_iter()
                .filter(|u| {
                    u.get("username")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_lowercase()
                        .contains(&query)
                })
                .collect::<Vec<_>>()
        }
    };

    let _do_create = move |_: leptos::ev::MouseEvent| {
        let username = new_username.get();
        let password = new_password.get();
        let role = new_role.get();
        if username.trim().is_empty() {
            set_form_error.set(Some("Username is required".to_string()));
            return;
        }
        if password.trim().is_empty() {
            set_form_error.set(Some("Password is required".to_string()));
            return;
        }
        set_form_error.set(None);
        let api_clone = api.get_untracked();
        let u = username.trim().to_string();
        let p = password.trim().to_string();
        let r = role.trim().to_string();
        spawn_local(async move {
            match api_clone.create_user(&u, &p, &r).await {
                Ok(_) => {
                    set_msg.set(Some(format!("User '{}' created successfully", u)));
                    set_new_username.set(String::new());
                    set_new_password.set(String::new());
                    set_show_create.set(false);
                    load_users();
                }
                Err(e) => set_form_error.set(Some(e)),
            }
        });
    };

    let do_delete = move |_: leptos::ev::MouseEvent| {
        let target = delete_target.get();
        let api_clone = api.get_untracked();
        spawn_local(async move {
            match api_clone.delete_user(&target).await {
                Ok(_) => {
                    set_msg.set(Some(format!("User '{}' deleted", target)));
                    set_show_delete_confirm.set(false);
                    load_users();
                }
                Err(e) => set_msg.set(Some(format!("Delete failed: {}", e))),
            }
        });
    };

    view! {
        <div class="page">
            <div class="page-header surface brutal-border">
                <div class="page-header-left">
                    <input type="text" class="search-input" placeholder="Search users..." prop:value=search on:input=move |ev| set_search.set(event_target_value(&ev)) aria-label="Search users" />
                </div>
                <button class="btn btn-primary" on:click=move |_| set_show_create.set(true) aria-label="Create new user">"Create User"</button>
            </div>

            <div aria-live="polite">
                {move || msg.get().map(|m| view! { <div class="success-banner" role="status">{m}</div> })}
            </div>
            <div aria-live="assertive">
                {move || error.get().map(|e| view! { <div class="error-banner" role="alert">{e}</div> })}
            </div>
            <div aria-live="polite">
                {move || loading.get().then(|| view! { <div class="loading" role="status">"Loading users..."</div> })}
            </div>

            // NOTE: When leptos-struct-table is wired up, replace the manual
            // <table> below with: `<Table rows=filtered_users columns=columns />`
            // using leptos_struct_table::Table and derive(TableRow) on UserRow.
            <div class="table-wrapper">
                <table class="data-table" aria-label="User management table">
                    <thead><tr><th scope="col">"Username"</th><th scope="col">"Role"</th><th scope="col">"Created"</th><th scope="col">"Last Login"</th><th scope="col">"Actions"</th></tr></thead>
                    <tbody>
                        {move || {
                            let filtered = filtered_users();
                            if filtered.is_empty() {
                                vec![view! { <tr><td colspan="5" class="table-empty">"No users found"</td></tr> }]
                            } else {
                                filtered.iter().map(|user| {
                                    let username = user.get("username").and_then(|v| v.as_str()).unwrap_or("-").to_string();
                                    let role = user.get("role").and_then(|v| v.as_str()).unwrap_or("viewer").to_string();
                                    let created = user.get("created_at").and_then(|v| v.as_str()).unwrap_or("-").to_string();
                                    let last_login = user.get("last_login").and_then(|v| v.as_str()).unwrap_or("Never").to_string();
                                    let bv = match role.as_str() { "admin" => BadgeVariant::Danger, "editor" => BadgeVariant::Warning, _ => BadgeVariant::Success };
                                    let uname = username.clone();
                                    view! {
                                        <tr>
                                            <td><strong>{username.clone()}</strong></td>
                                            <td><Badge text=role variant=bv/></td>
                                            <td>{format_timestamp(&created)}</td>
                                            <td>{format_timestamp(&last_login)}</td>
                                            <td class="actions-cell">
                                                <button class="btn btn-danger btn-sm" on:click=move |_| { set_delete_target.set(uname.clone()); set_show_delete_confirm.set(true); }>"Delete"</button>
                                            </td>
                                        </tr>
                                    }
                                }).collect::<Vec<_>>()
                            }
                        }}
                    </tbody>
                </table>
            </div>

            <Modal title="Create User".to_string() show=show_create.get() on_close=Callback::new(move |()| set_show_create.set(false))>
                <form class="modal-form" on:submit=move |ev| ev.prevent_default() aria-label="Create new user form">
                    <div class="form-group">
                        <label class="form-label" for="new-username">"Username"</label>
                        <input id="new-username" type="text" class="form-input" placeholder="Enter username" prop:value=new_username on:input=move |ev| set_new_username.set(event_target_value(&ev)) aria-required="true" />
                    </div>
                    <div class="form-group">
                        <label class="form-label" for="new-password">"Password"</label>
                        <input id="new-password" type="password" class="form-input" placeholder="Enter password" prop:value=new_password on:input=move |ev| set_new_password.set(event_target_value(&ev)) aria-required="true" />
                    </div>
                    <div class="form-group">
                        <label class="form-label" for="new-role">"Role"</label>
                        <select id="new-role" class="form-input" prop:value=new_role on:change=move |ev| set_new_role.set(event_target_value(&ev)) aria-required="true">
                            <option value="viewer">"Viewer"</option>
                            <option value="editor">"Editor"</option>
                            <option value="admin">"Admin"</option>
                        </select>
                    </div>
                    <div aria-live="assertive">
                        {move || form_error.get().map(|e| view! { <div class="form-error" role="alert">{e}</div> })}
                    </div>
                    <div class="modal-actions">
                        <button type="button" class="btn btn-secondary" on:click=move |_| set_show_create.set(false)>"Cancel"</button>
                        <button type="submit" class="btn btn-primary">"Create User"</button>
                    </div>
                </form>
            </Modal>

            <Modal title="Delete User".to_string() show=show_delete_confirm.get() on_close=Callback::new(move |()| set_show_delete_confirm.set(false))>
                <div class="modal-form">
                    <p>"Are you sure you want to delete user "</p>
                    <strong>{move || delete_target.get()}</strong>
                    <p>"? This action cannot be undone."</p>
                    <div class="modal-actions">
                        <button type="button" class="btn btn-secondary" on:click=move |_| set_show_delete_confirm.set(false) aria-label="Cancel deletion">"Cancel"</button>
                        <button type="button" class="btn btn-danger" on:click=do_delete aria-label="Confirm user deletion">"Delete"</button>
                    </div>
                </div>
            </Modal>
        </div>
    }
}
