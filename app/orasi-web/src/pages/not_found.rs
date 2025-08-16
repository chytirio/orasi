use leptos::*;

#[component]
pub fn NotFoundPage() -> impl IntoView {
    view! {
        <div class="min-h-screen bg-gray-50 flex items-center justify-center">
            <div class="max-w-md w-full text-center">
                <div class="text-6xl font-bold text-gray-300 mb-4">"404"</div>
                <h1 class="text-2xl font-bold text-gray-900 mb-4">"Page Not Found"</h1>
                <p class="text-gray-600 mb-8">
                    "The page you're looking for doesn't exist or has been moved."
                </p>
                <a href="/" class="inline-flex items-center px-4 py-2 border border-transparent text-sm font-medium rounded-md text-white bg-indigo-600 hover:bg-indigo-700">
                    "Go Home"
                </a>
            </div>
        </div>
    }
}
