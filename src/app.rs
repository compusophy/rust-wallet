use leptos::*;
use leptos_meta::*;
use leptos_router::*;

use crate::wallet::WalletView;

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    view! {
        <Title text="Diamond Facet Wallet"/>
        
        <Router>
            <main class="container">
                <Routes>
                    <Route path="" view=WalletView/>
                </Routes>
            </main>
        </Router>
    }
}
