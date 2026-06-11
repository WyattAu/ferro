use leptos::prelude::*;

use crate::api::ApiState;
use crate::components::plugin_marketplace::PluginMarketplace;

#[component]
pub fn PluginMarketplacePage(api: RwSignal<ApiState>) -> impl IntoView {
    view! { <PluginMarketplace api=api/> }
}
