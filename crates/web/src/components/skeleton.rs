use leptos::*;

use crate::t;

#[component]
pub fn SkeletonList() -> impl IntoView {
    view! {
        <div class="p-3 sm:p-4 space-y-0" role="status" aria-label={t!("skeleton.loading_files")}>
            <div class="hidden md:block bg-gray-50 border-b mb-2 px-4 py-2">
                <div class="flex gap-4">
                    <div class="h-3 w-10 bg-gray-200 rounded animate-pulse"></div>
                    <div class="h-3 w-20 bg-gray-200 rounded animate-pulse"></div>
                    <div class="h-3 w-16 bg-gray-200 rounded animate-pulse"></div>
                    <div class="h-3 w-28 bg-gray-200 rounded animate-pulse"></div>
                    <div class="h-3 w-16 bg-gray-200 rounded animate-pulse ml-auto"></div>
                </div>
            </div>
            {(0..6).map(|_| view! {
                <div class="flex items-center gap-3 px-4 py-3">
                    <div class="w-5 h-5 bg-gray-200 rounded animate-pulse shrink-0"></div>
                    <div class="flex-1 min-w-0 space-y-1.5">
                        <div class="h-4 bg-gray-200 rounded animate-pulse w-3/5"></div>
                        <div class="h-3 bg-gray-200 rounded animate-pulse w-2/5 md:hidden"></div>
                    </div>
                    <div class="h-4 w-16 bg-gray-200 rounded animate-pulse hidden md:block"></div>
                    <div class="h-4 w-28 bg-gray-200 rounded animate-pulse hidden lg:block"></div>
                    <div class="h-8 w-24 bg-gray-200 rounded animate-pulse hidden md:block"></div>
                </div>
            }).collect::<Vec<_>>()
            }
        </div>
    }
}

#[component]
pub fn SkeletonGrid() -> impl IntoView {
    view! {
        <div class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 xl:grid-cols-5 gap-2 sm:gap-3 p-3 sm:p-4" role="status" aria-label={t!("skeleton.loading_files")}>
            {(0..8).map(|_| view! {
                <div class="surface brutal-border rounded-xl p-4">
                    <div class="flex flex-col items-center text-center">
                        <div class="w-10 h-10 bg-gray-200 rounded-lg animate-pulse mb-3"></div>
                        <div class="h-4 w-3/4 bg-gray-200 rounded animate-pulse mb-2"></div>
                        <div class="h-3 w-1/2 bg-gray-200 rounded animate-pulse"></div>
                        <div class="h-3 w-2/5 bg-gray-200 rounded animate-pulse mt-1 hidden sm:block"></div>
                    </div>
                    <div class="flex items-center justify-center gap-1 pt-3 mt-3 border-t border-gray-100">
                        <div class="w-6 h-6 bg-gray-200 rounded animate-pulse"></div>
                        <div class="w-6 h-6 bg-gray-200 rounded animate-pulse"></div>
                        <div class="w-6 h-6 bg-gray-200 rounded animate-pulse"></div>
                    </div>
                </div>
            }).collect::<Vec<_>>()
            }
        </div>
    }
}

#[component]
pub fn SkeletonFavorites() -> impl IntoView {
    view! {
        <div class="p-3 sm:p-4 space-y-0" role="status" aria-label={t!("skeleton.loading_favorites")}>
            <div class="hidden md:block bg-gray-50 border-b mb-2 px-4 py-2">
                <div class="flex gap-4">
                    <div class="h-3 w-10 bg-gray-200 rounded animate-pulse"></div>
                    <div class="h-3 w-20 bg-gray-200 rounded animate-pulse"></div>
                    <div class="h-3 w-16 bg-gray-200 rounded animate-pulse"></div>
                    <div class="h-3 w-28 bg-gray-200 rounded animate-pulse"></div>
                    <div class="h-3 w-16 bg-gray-200 rounded animate-pulse ml-auto"></div>
                </div>
            </div>
            {(0..4).map(|_| view! {
                <div class="flex items-center gap-3 px-4 py-3">
                    <div class="w-5 h-5 bg-gray-200 rounded animate-pulse shrink-0"></div>
                    <div class="flex-1 min-w-0 space-y-1.5">
                        <div class="h-4 bg-gray-200 rounded animate-pulse w-2/3"></div>
                    </div>
                    <div class="h-4 w-16 bg-gray-200 rounded animate-pulse hidden md:block"></div>
                    <div class="h-4 w-28 bg-gray-200 rounded animate-pulse hidden lg:block"></div>
                </div>
            }).collect::<Vec<_>>()
            }
        </div>
    }
}

#[component]
pub fn SkeletonRecent() -> impl IntoView {
    view! {
        <div class="p-3 sm:p-4 space-y-0" role="status" aria-label={t!("skeleton.loading_recent")}>
            <div class="hidden md:block bg-gray-50 border-b mb-2 px-4 py-2">
                <div class="flex gap-4">
                    <div class="h-3 w-10 bg-gray-200 rounded animate-pulse"></div>
                    <div class="h-3 w-20 bg-gray-200 rounded animate-pulse"></div>
                    <div class="h-3 w-16 bg-gray-200 rounded animate-pulse"></div>
                    <div class="h-3 w-28 bg-gray-200 rounded animate-pulse"></div>
                    <div class="h-3 w-16 bg-gray-200 rounded animate-pulse ml-auto"></div>
                </div>
            </div>
            {(0..4).map(|_| view! {
                <div class="flex items-center gap-3 px-4 py-3">
                    <div class="w-5 h-5 bg-gray-200 rounded animate-pulse shrink-0"></div>
                    <div class="flex-1 min-w-0 space-y-1.5">
                        <div class="h-4 bg-gray-200 rounded animate-pulse w-1/2"></div>
                    </div>
                    <div class="h-4 w-16 bg-gray-200 rounded animate-pulse hidden md:block"></div>
                    <div class="h-4 w-28 bg-gray-200 rounded animate-pulse hidden lg:block"></div>
                </div>
            }).collect::<Vec<_>>()
            }
        </div>
    }
}
