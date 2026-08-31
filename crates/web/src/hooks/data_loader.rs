use leptos::prelude::*;
use leptos::task::spawn_local;

/// A reusable data loading hook that manages loading, error, and data state.
///
/// Usage:
/// ```ignore
/// let loader = use_data_loader(|| async { api::fetch_data().await });
/// // Access state:
/// // loader.data.get() -> Option<T>
/// // loader.loading.get() -> bool
/// // loader.error.get() -> Option<String>
/// // loader.reload() -> triggers re-fetch
/// ```
pub struct DataLoader<T: Clone + 'static> {
    pub data: ReadSignal<Option<T>>,
    pub loading: ReadSignal<bool>,
    pub error: ReadSignal<Option<String>>,
    reload: Trigger,
}

impl<T: Clone + 'static> DataLoader<T> {
    pub fn reload(&self) {
        self.reload.0.notify();
    }
}

/// Hook that loads data asynchronously and manages loading/error states.
///
/// The fetcher is called whenever the loader is created or `reload()` is called.
pub fn use_data_loader<F, Fut>(fetcher: F) -> DataLoader<Fut::Output>
where
    F: Fn() -> Fut + 'static,
    Fut: std::future::Future<Output = Result<T, String>> + 'static,
    T: Clone + 'static,
{
    let (data, set_data) = signal(None::<T>);
    let (loading, set_loading) = signal(true);
    let (error, set_error) = signal(None::<String>);
    let (reload, set_reload) = signal(());
    let fetcher = StoredValue::new(fetcher);

    Effect::new(move |_| {
        let _ = reload.track();
        let fetcher = fetcher.clone();
        let set_data = set_data;
        let set_loading = set_loading;
        let set_error = set_error;

        set_loading.set(true);
        set_error.set(None);

        spawn_local(async move {
            match fetcher.get_value()().await {
                Ok(result) => {
                    set_data.set(Some(result));
                    set_loading.set(false);
                }
                Err(e) => {
                    set_error.set(Some(e));
                    set_loading.set(false);
                }
            }
        });
    });

    DataLoader {
        data,
        loading,
        error,
        reload: Trigger::default(),
    }
}
