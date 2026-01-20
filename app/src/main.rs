use leptos::*;

mod app;
mod wallet;
mod rpc;
pub mod transactions;

use app::App;

fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(|| view! { <App/> })
}
