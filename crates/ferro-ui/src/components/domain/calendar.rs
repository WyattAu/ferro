use leptos::prelude::*;

#[derive(Clone, Debug)]
struct CalendarEvent {
    uid: String,
    title: String,
    start: String,
    end: String,
    all_day: bool,
    color: String,
    description: String,
}

/// Calendar page with month/week/day views.
#[component]
pub fn CalendarPage() -> impl IntoView {
    let (events, set_events) = signal(Vec::<CalendarEvent>::new());
    let (view_mode, set_view_mode) = signal("month".to_string());
    let (current_date, set_current_date) = signal(js_sys::Date::new_0());
    let (loading, set_loading) = signal(true);

    Effect::new(move |_| {
        set_loading.set(true);
        #[cfg(target_arch = "wasm32")]
        {
            let set_e = set_events;
            let set_l = set_loading;
            wasm_bindgen_futures::spawn_local(async move {
                let client = crate::api::ApiClient::from_env();
                match client.get::<serde_json::Value>("/api/v1/calendar/events").await {
                    Ok(val) => {
                        if let Some(arr) = val.as_array() {
                            let items: Vec<CalendarEvent> = arr
                                .iter()
                                .filter_map(|v| {
                                    Some(CalendarEvent {
                                        uid: v["uid"].as_str()?.to_string(),
                                        title: v["title"].as_str().unwrap_or("Untitled").to_string(),
                                        start: v["start"].as_str().unwrap_or("").to_string(),
                                        end: v["end"].as_str().unwrap_or("").to_string(),
                                        all_day: v["all_day"].as_bool().unwrap_or(false),
                                        color: v["color"].as_str().unwrap_or("#3B82F6").to_string(),
                                        description: v["description"].as_str().unwrap_or("").to_string(),
                                    })
                                })
                                .collect();
                            set_e.set(items);
                        }
                        set_l.set(false);
                    }
                    Err(e) => {
                        log::error!("Calendar load failed: {}", e);
                        set_l.set(false);
                    }
                }
            });
        }
    });

    let prev_month = move |_| {
        set_current_date.update(|d| {
            let m = d.get_month();
            if m == 0 {
                d.set_month(11);
                d.set_full_year(d.get_full_year() - 1);
            } else {
                d.set_month(m - 1);
            }
        });
    };

    let next_month = move |_| {
        set_current_date.update(|d| {
            let m = d.get_month();
            if m == 11 {
                d.set_month(0);
                d.set_full_year(d.get_full_year() + 1);
            } else {
                d.set_month(m + 1);
            }
        });
    };

    let today = move |_| {
        set_current_date.set(js_sys::Date::new_0());
    };

    let month_name = move || {
        let d = current_date.get();
        let months = [
            "January",
            "February",
            "March",
            "April",
            "May",
            "June",
            "July",
            "August",
            "September",
            "October",
            "November",
            "December",
        ];
        format!("{} {}", months[d.get_month() as usize], d.get_full_year())
    };

    view! {
        <div class="flex flex-col h-full">
            <div class="flex items-center gap-3 px-4 py-2 border-b border-[var(--color-border)]">
                <h1 class="text-lg font-semibold">"Calendar"</h1>
                <div class="flex items-center gap-2 ml-4">
                    <button class="btn btn-ghost btn-sm" on:click=prev_month>"←"</button>
                    <span class="font-medium">{month_name}</span>
                    <button class="btn btn-ghost btn-sm" on:click=next_month>"→"</button>
                    <button class="btn btn-ghost btn-sm" on:click=today>"Today"</button>
                </div>
                <div class="ml-auto flex gap-2">
                    <button class=move || format!("btn btn-ghost btn-sm {}", if view_mode.get() == "month" { "btn-primary" } else { "" })
                        on:click=move |_| set_view_mode.set("month".to_string())>"Month"</button>
                    <button class=move || format!("btn btn-ghost btn-sm {}", if view_mode.get() == "week" { "btn-primary" } else { "" })
                        on:click=move |_| set_view_mode.set("week".to_string())>"Week"</button>
                    <button class=move || format!("btn btn-ghost btn-sm {}", if view_mode.get() == "day" { "btn-primary" } else { "" })
                        on:click=move |_| set_view_mode.set("day".to_string())>"Day"</button>
                </div>
            </div>
            <div class="flex-1 overflow-auto p-4">
                <div class="grid grid-cols-7 gap-px bg-[var(--color-border)] rounded-lg overflow-hidden">
                    {vec!["Sun","Mon","Tue","Wed","Thu","Fri","Sat"].into_iter().map(|day| {
                        view! { <div class="bg-raised p-2 text-center text-xs font-semibold text-secondary">{day}</div> }
                    }).collect_view()}
                    {move || {
                        let d = current_date.get();
                        let year = d.get_full_year();
                        let month = d.get_month();
                        let first_day = js_sys::Date::new_with_year_month_day(year, month as i32, 1);
                        let start_dow = first_day.get_day();
                        let days_in_month = js_sys::Date::new_with_year_month_day(year, (month + 1) as i32, 0).get_date();

                        let mut cells = Vec::new();
                        for _ in 0..start_dow {
                            cells.push(view! { <div class="bg-raised min-h-20 p-1"></div> }.into_any());
                        }
                        for day in 1..=days_in_month {
                            let day_num = day;
                            let is_today = {
                                let now = js_sys::Date::new_0();
                                now.get_date() == day && now.get_month() == month && now.get_full_year() == year
                            };
                            let day_events: Vec<_> = events.get().into_iter().filter(|e| {
                                e.start.contains(&format!("{:04}-{:02}-{:02}", year, month + 1, day))
                            }).collect();
                            cells.push(view! {
                                <div class="bg-raised min-h-20 p-1">
                                    <div class={if is_today { "w-6 h-6 rounded-full bg-accent text-white text-xs flex items-center justify-center font-bold" } else { "text-xs text-secondary p-1" }}>
                                        {day_num}
                                    </div>
                                    {day_events.into_iter().take(3).map(|ev| {
                                        view! { <div class="text-xs px-1 py-0.5 mt-0.5 rounded truncate" style:background=ev.color style:color="white">{ev.title}</div> }
                                    }).collect_view()}
                                </div>
                            }.into_any());
                        }
                        cells.into_view()
                    }}
                </div>
            </div>
        </div>
    }
}
