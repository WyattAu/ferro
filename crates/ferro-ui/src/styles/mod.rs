//! CSS modules — imported via include_str! for Trunk bundling.
//!
//! Tokens: design system custom properties
//! Utilities: complete utility class set
//! Components: semantic component styles

/// Inject all CSS into the document head.
pub fn inject_styles() {
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::JsCast;

        let css = [
            include_str!("tokens.css"),
            include_str!("utilities.css"),
            include_str!("components.css"),
        ]
        .join("\n");

        if let Some(window) = web_sys::window() {
            if let Some(doc) = window.document() {
                if doc.query_selector("#ferro-styles").ok().flatten().is_some() {
                    return;
                }
                if let Some(style) = doc
                    .create_element("style")
                    .ok()
                    .and_then(|e| e.dyn_into::<web_sys::HtmlStyleElement>().ok())
                {
                    style.set_id("ferro-styles");
                    let _ = style.set_text_content(Some(&css));
                    if let Some(head) = doc.head() {
                        let _ = head.append_child(&style);
                    }
                }
            }
        }
    }
}
