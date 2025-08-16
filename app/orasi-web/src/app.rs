use leptos::*;
use leptos_meta::*;
use leptos_router::*;

use crate::pages::{HomePage, DashboardPage, NotFoundPage};

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    view! {
        <Stylesheet id="leptos" href="/pkg/orasi-ui.css"/>
        <Title text="Orasi UI"/>
        <Meta name="description" content="Orasi Data Platform UI"/>

        <Router>
            <nav class="bg-gray-800 text-white p-4">
                <div class="container mx-auto flex justify-between items-center">
                    <h1 class="text-xl font-bold">"Orasi"</h1>
                    <div class="space-x-4">
                        <A href="/" class="hover:text-gray-300">"Home"</A>
                        <A href="/dashboard" class="hover:text-gray-300">"Dashboard"</A>
                    </div>
                </div>
            </nav>

            <main class="container mx-auto p-4">
                <Routes>
                    <Route path="/" view=HomePage/>
                    <Route path="/dashboard" view=DashboardPage/>
                    <Route path="/*any" view=NotFoundPage/>
                </Routes>
            </main>
        </Router>
    }
}
