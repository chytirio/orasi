use leptos::*;

#[component]
pub fn DashboardPage() -> impl IntoView {
    let (metrics, _set_metrics) = create_signal(vec![
        ("Total Records", "1,234,567"),
        ("Active Streams", "42"),
        ("Processing Rate", "10.5K/s"),
        ("Error Rate", "0.02%"),
    ]);

    view! {
        <div class="min-h-screen bg-gray-50">
            <div class="max-w-7xl mx-auto py-6 sm:px-6 lg:px-8">
                <div class="px-4 py-6 sm:px-0">
                    <h1 class="text-3xl font-bold text-gray-900 mb-8">"Dashboard"</h1>

                    // Metrics Grid
                    <div class="grid grid-cols-1 gap-5 sm:grid-cols-2 lg:grid-cols-4 mb-8">
                        {move || {
                            metrics.get().into_iter().map(|(name, value)| {
                                view! {
                                    <div class="bg-white overflow-hidden shadow rounded-lg">
                                        <div class="p-5">
                                            <div class="flex items-center">
                                                <div class="flex-shrink-0">
                                                    <div class="w-8 h-8 bg-indigo-500 rounded-md flex items-center justify-center">
                                                        <span class="text-white text-sm font-bold">
                                                            {name.chars().next().unwrap_or('M')}
                                                        </span>
                                                    </div>
                                                </div>
                                                <div class="ml-5 w-0 flex-1">
                                                    <dl>
                                                        <dt class="text-sm font-medium text-gray-500 truncate">
                                                            {name}
                                                        </dt>
                                                        <dd class="text-lg font-medium text-gray-900">
                                                            {value}
                                                        </dd>
                                                    </dl>
                                                </div>
                                            </div>
                                        </div>
                                    </div>
                                }
                            }).collect::<Vec<_>>()
                        }}
                    </div>

                    // Recent Activity
                    <div class="bg-white shadow rounded-lg">
                        <div class="px-4 py-5 sm:p-6">
                            <h3 class="text-lg leading-6 font-medium text-gray-900 mb-4">
                                "Recent Activity"
                            </h3>
                            <div class="flow-root">
                                <ul class="-mb-8">
                                    <li>
                                        <div class="relative pb-8">
                                            <span class="absolute top-4 left-4 -ml-px h-full w-0.5 bg-gray-200" aria-hidden="true"></span>
                                            <div class="relative flex space-x-3">
                                                <div>
                                                    <span class="h-8 w-8 rounded-full bg-green-500 flex items-center justify-center ring-8 ring-white">
                                                        <span class="text-white text-sm font-bold">"✓"</span>
                                                    </span>
                                                </div>
                                                <div class="min-w-0 flex-1 pt-1.5 flex justify-between space-x-4">
                                                    <div>
                                                        <p class="text-sm text-gray-500">
                                                            "New data stream connected: "
                                                            <span class="font-medium text-gray-900">"kafka://events"</span>
                                                        </p>
                                                    </div>
                                                    <div class="text-right text-sm whitespace-nowrap text-gray-500">
                                                        <time>"3m ago"</time>
                                                    </div>
                                                </div>
                                            </div>
                                        </div>
                                    </li>
                                    <li>
                                        <div class="relative pb-8">
                                            <span class="absolute top-4 left-4 -ml-px h-full w-0.5 bg-gray-200" aria-hidden="true"></span>
                                            <div class="relative flex space-x-3">
                                                <div>
                                                    <span class="h-8 w-8 rounded-full bg-blue-500 flex items-center justify-center ring-8 ring-white">
                                                        <span class="text-white text-sm font-bold">"⚡"</span>
                                                    </span>
                                                </div>
                                                <div class="min-w-0 flex-1 pt-1.5 flex justify-between space-x-4">
                                                    <div>
                                                        <p class="text-sm text-gray-500">
                                                            "Processing pipeline started: "
                                                            <span class="font-medium text-gray-900">"user_analytics"</span>
                                                        </p>
                                                    </div>
                                                    <div class="text-right text-sm whitespace-nowrap text-gray-500">
                                                        <time>"10m ago"</time>
                                                    </div>
                                                </div>
                                            </div>
                                        </div>
                                    </li>
                                    <li>
                                        <div class="relative">
                                            <div class="relative flex space-x-3">
                                                <div>
                                                    <span class="h-8 w-8 rounded-full bg-yellow-500 flex items-center justify-center ring-8 ring-white">
                                                        <span class="text-white text-sm font-bold">"⚠"</span>
                                                    </span>
                                                </div>
                                                <div class="min-w-0 flex-1 pt-1.5 flex justify-between space-x-4">
                                                    <div>
                                                        <p class="text-sm text-gray-500">
                                                            "Schema validation warning: "
                                                            <span class="font-medium text-gray-900">"missing_field"</span>
                                                        </p>
                                                    </div>
                                                    <div class="text-right text-sm whitespace-nowrap text-gray-500">
                                                        <time>"1h ago"</time>
                                                    </div>
                                                </div>
                                            </div>
                                        </div>
                                    </li>
                                </ul>
                            </div>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    }
}
