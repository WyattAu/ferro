use ferro_admin::App;

fn main() {
    console_error_panic_hook::set_once();
    leptos::mount_to_body(App);
}
