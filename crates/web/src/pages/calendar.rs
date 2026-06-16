use leptos::ev;
use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::api;
use crate::components::header::{Header, provide_header_state};
use crate::components::theme_toggle::provide_theme_state;
use crate::t;
use crate::utils::percent_encode;

use chrono::Datelike;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CalendarEvent {
    pub uid: String,
    pub calendar_id: String,
    pub ical_data: String,
    pub etag: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq)]
enum ViewMode {
    Month,
    Week,
    Day,
}

fn parse_event_summary(ical: &str) -> String {
    for line in ical.lines() {
        if let Some(rest) = line.strip_prefix("SUMMARY:") {
            return rest.trim().to_string();
        }
    }
    "Untitled".to_string()
}

fn parse_event_dtstart(ical: &str) -> Option<String> {
    for line in ical.lines() {
        if let Some(rest) = line.strip_prefix("DTSTART") {
            let value = if let Some(idx) = rest.find(':') {
                &rest[idx + 1..]
            } else {
                rest
            };
            return Some(value.trim().to_string());
        }
    }
    None
}

fn parse_event_dtend(ical: &str) -> Option<String> {
    for line in ical.lines() {
        if let Some(rest) = line.strip_prefix("DTEND") {
            let value = if let Some(idx) = rest.find(':') {
                &rest[idx + 1..]
            } else {
                rest
            };
            return Some(value.trim().to_string());
        }
    }
    None
}

fn parse_event_allday(ical: &str) -> bool {
    for line in ical.lines() {
        if line.starts_with("DTSTART") && line.contains("VALUE=DATE") {
            return true;
        }
    }
    false
}

fn parse_event_color(ical: &str) -> String {
    for line in ical.lines() {
        if let Some(rest) = line.strip_prefix("COLOR:") {
            return rest.trim().to_string();
        }
    }
    "#3b82f6".to_string()
}

fn format_ical_datetime(dt: &str) -> String {
    if dt.len() >= 15 {
        let year = &dt[..4];
        let month = &dt[4..6];
        let day = &dt[6..8];
        let hour = &dt[9..11];
        let min = &dt[11..13];
        format!("{}-{}-{} {}:{}", year, month, day, hour, min)
    } else if dt.len() >= 8 {
        let year = &dt[..4];
        let month = &dt[4..6];
        let day = &dt[6..8];
        format!("{}-{}-{}", year, month, day)
    } else {
        dt.to_string()
    }
}

fn days_in_month(year: i32, month: u32) -> u32 {
    let (y, m) = if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    };
    (chrono::NaiveDate::from_ymd_opt(y, m, 1).unwrap()
        - chrono::TimeDelta::days(1))
    .day()
}

fn first_day_of_month(year: i32, month: u32) -> u32 {
    chrono::NaiveDate::from_ymd_opt(year, month, 1)
        .unwrap()
        .weekday()
        .num_days_from_sunday()
}

#[component]
pub fn CalendarPage() -> impl IntoView {
    provide_theme_state();
    provide_header_state();

    let (loading, set_loading) = signal(true);
    let (events, set_events) = signal(Vec::<CalendarEvent>::new());
    let (error_msg, set_error) = signal(String::new());
    let (view_mode, set_view_mode) = signal(ViewMode::Month);
    let (current_date, set_current_date) = signal(chrono::Utc::now().date_naive());
    let (show_dialog, set_show_dialog) = signal(false);
    let (editing_event, set_editing_event) = signal(None::<CalendarEvent>);

    let (dialog_title, set_dialog_title) = signal(String::new());
    let (dialog_start, set_dialog_start) = signal(String::new());
    let (dialog_end, set_dialog_end) = signal(String::new());
    let (dialog_allday, set_dialog_allday) = signal(false);
    let (dialog_description, set_dialog_description) = signal(String::new());
    let (dialog_recurrence, set_dialog_recurrence) = signal(String::new());
    let (dialog_color, set_dialog_color) = signal("#3b82f6".to_string());

    let fetch_events = move || {
        set_loading.set(true);
        set_error.set(String::new());
        let start = current_date.get().and_hms_opt(0, 0, 0).unwrap().and_utc();
        let end = {
            let d = current_date.get();
            let (y, m) = if d.month() == 12 {
                (d.year() + 1, 1)
            } else {
                (d.year(), d.month() + 1)
            };
            chrono::NaiveDate::from_ymd_opt(y, m, 1)
                .unwrap_or(d)
                .and_hms_opt(23, 59, 59)
                .unwrap()
                .and_utc()
        };

        spawn_local(async move {
            let start_str = start.to_rfc3339();
            let end_str = end.to_rfc3339();
            let url = format!(
                "/api/calendar/events?start={}&end={}",
                percent_encode(&start_str),
                percent_encode(&end_str)
            );
            match api::fetch_json(&url).await {
                Ok(val) => {
                    let evts = val
                        .get("events")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| {
                                    Some(CalendarEvent {
                                        uid: v.get("uid")?.as_str()?.to_string(),
                                        calendar_id: v
                                            .get("calendar_id")
                                            .and_then(|c| c.as_str())
                                            .unwrap_or("")
                                            .to_string(),
                                        ical_data: v
                                            .get("ical_data")
                                            .and_then(|i| i.as_str())
                                            .unwrap_or("")
                                            .to_string(),
                                        etag: v
                                            .get("etag")
                                            .and_then(|e| e.as_str())
                                            .unwrap_or("")
                                            .to_string(),
                                        created_at: v
                                            .get("created_at")
                                            .and_then(|c| c.as_str())
                                            .unwrap_or("")
                                            .to_string(),
                                        updated_at: v
                                            .get("updated_at")
                                            .and_then(|u| u.as_str())
                                            .unwrap_or("")
                                            .to_string(),
                                    })
                                })
                                .collect()
                        })
                        .unwrap_or_default();
                    set_events.set(evts);
                    set_loading.set(false);
                }
                Err(e) => {
                    set_error.set(e);
                    set_loading.set(false);
                }
            }
        });
    };

    Effect::new(move |_| {
        let _ = current_date.get();
        let _ = view_mode.get();
        fetch_events();
    });

    let open_create_dialog = move |_: ev::MouseEvent| {
        set_editing_event.set(None);
        set_dialog_title.set(String::new());
        set_dialog_start.set(String::new());
        set_dialog_end.set(String::new());
        set_dialog_allday.set(false);
        set_dialog_description.set(String::new());
        set_dialog_recurrence.set(String::new());
        set_dialog_color.set("#3b82f6".to_string());
        set_show_dialog.set(true);
    };

    let open_edit_dialog = move |event: CalendarEvent| {
        set_editing_event.set(Some(event.clone()));
        set_dialog_title.set(parse_event_summary(&event.ical_data));
        set_dialog_start.set(
            parse_event_dtstart(&event.ical_data)
                .map(|d| format_ical_datetime(&d))
                .unwrap_or_default(),
        );
        set_dialog_end.set(
            parse_event_dtend(&event.ical_data)
                .map(|d| format_ical_datetime(&d))
                .unwrap_or_default(),
        );
        set_dialog_allday.set(parse_event_allday(&event.ical_data));
        set_dialog_color.set(parse_event_color(&event.ical_data));
        set_dialog_description.set(String::new());
        set_dialog_recurrence.set(String::new());
        set_show_dialog.set(true);
    };

    let save_event = move |_: ev::MouseEvent| {
        let title = dialog_title.get();
        let start = dialog_start.get();
        let end = dialog_end.get();
        let allday = dialog_allday.get();
        let color = dialog_color.get();
        let editing = editing_event.get();

        let dtstart = if allday {
            format!("DTSTART;VALUE=DATE:{}", start.replace('-', ""))
        } else {
            format!("DTSTART:{}", start.replace('-', "").replace(':', "") + "00Z")
        };
        let dtend = if allday {
            format!("DTEND;VALUE=DATE:{}", end.replace('-', ""))
        } else {
            format!("DTEND:{}", end.replace('-', "").replace(':', "") + "00Z")
        };

        let ical = format!(
            "BEGIN:VCALENDAR\r\n\
             BEGIN:VEVENT\r\n\
             SUMMARY:{}\r\n\
             {}\r\n\
             {}\r\n\
             COLOR:{}\r\n\
             END:VEVENT\r\n\
             END:VCALENDAR\r\n",
            title, dtstart, dtend, color
        );

        set_show_dialog.set(false);

        spawn_local(async move {
            if let Some(ref evt) = editing {
                let body = serde_json::json!({ "ical_data": ical });
                let _ = api::fetch_json_with_method(
                    &format!("/api/calendar/events/{}", evt.uid),
                    "PUT",
                    Some(&body.to_string()),
                )
                .await;
            } else {
                let body = serde_json::json!({
                    "calendar_id": "",
                    "ical_data": ical
                });
                let _ = api::fetch_json_with_method(
                    "/api/calendar/events",
                    "POST",
                    Some(&body.to_string()),
                )
                .await;
            }
            fetch_events();
        });
    };

    let delete_event = move |uid: String| {
        spawn_local(async move {
            let _ = api::fetch_json_with_method(
                &format!("/api/calendar/events/{}", uid),
                "DELETE",
                None,
            )
            .await;
            fetch_events();
        });
    };

    let navigate_prev = move |_: ev::MouseEvent| {
        set_current_date.update(|d| {
            let mode = view_mode.get();
            match mode {
                ViewMode::Month => {
                    *d = (*d - chrono::TimeDelta::days(1)).with_day(1).unwrap_or(*d);
                }
                ViewMode::Week => {
                    *d = *d - chrono::TimeDelta::days(7);
                }
                ViewMode::Day => {
                    *d = *d - chrono::TimeDelta::days(1);
                }
            }
        });
    };

    let navigate_next = move |_: ev::MouseEvent| {
        set_current_date.update(|d| {
            let mode = view_mode.get();
            match mode {
                ViewMode::Month => {
                    let (next_year, next_month) = if d.month() == 12 {
                        (d.year() + 1, 1)
                    } else {
                        (d.year(), d.month() + 1)
                    };
                    *d = chrono::NaiveDate::from_ymd_opt(next_year, next_month, 1).unwrap_or(*d);
                }
                ViewMode::Week => {
                    *d = *d + chrono::TimeDelta::days(7);
                }
                ViewMode::Day => {
                    *d = *d + chrono::TimeDelta::days(1);
                }
            }
        });
    };

    let navigate_today = move |_: ev::MouseEvent| {
        set_current_date.set(chrono::Utc::now().date_naive());
    };

    let month_name = move || {
        let d = current_date.get();
        match d.month() {
            1 => "January",
            2 => "February",
            3 => "March",
            4 => "April",
            5 => "May",
            6 => "June",
            7 => "July",
            8 => "August",
            9 => "September",
            10 => "October",
            11 => "November",
            12 => "December",
            _ => "",
        }
        .to_string()
    };

    let header_labels = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];

    view! {
        <div class="h-screen flex flex-col bg-gray-100 dark:bg-gray-900">
            <a href="#main-content" class="sr-only focus:not-sr-only focus:absolute focus:top-2 focus:left-2 focus:z-50 focus:px-4 focus:py-2 focus:bg-blue-600 focus:text-white focus:rounded">{t!("nav.skip_to_content")}</a>
            <Header />
            <div class="flex-1 overflow-auto px-2 sm:px-4 pt-16">
                <main id="main-content" class="max-w-7xl w-full mx-auto p-6">
                    <div class="flex items-center justify-between mb-6">
                        <h1 class="text-2xl font-bold font-mono text-gray-900 dark:text-white">{t!("calendar.title")}</h1>
                        <button
                            on:click=open_create_dialog
                            class="inline-flex items-center gap-2 px-4 py-2 bg-blue-600 text-white text-sm font-bold rounded-lg hover:bg-blue-700 transition-colors"
                        >
                            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" /></svg>
                            {t!("calendar.new_event")}
                        </button>
                    </div>

                    // Navigation controls
                    <div class="flex items-center gap-4 mb-6">
                        <div class="flex items-center gap-2">
                            <button on:click=navigate_prev class="p-2 rounded-lg hover:bg-gray-200 dark:hover:bg-gray-700 transition-colors" aria-label="Previous">
                                <svg class="w-5 h-5 text-gray-600 dark:text-gray-300" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7" /></svg>
                            </button>
                            <button on:click=navigate_today class="px-3 py-1.5 text-sm font-medium text-gray-700 dark:text-gray-300 bg-white dark:bg-gray-800 border border-gray-300 dark:border-gray-600 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors">
                                {t!("calendar.today")}
                            </button>
                            <button on:click=navigate_next class="p-2 rounded-lg hover:bg-gray-200 dark:hover:bg-gray-700 transition-colors" aria-label="Next">
                                <svg class="w-5 h-5 text-gray-600 dark:text-gray-300" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7" /></svg>
                            </button>
                        </div>
                        <h2 class="text-lg font-bold font-mono text-gray-900 dark:text-white">
                            {move || {
                                let mode = view_mode.get();
                                let d = current_date.get();
                                match mode {
                                    ViewMode::Month => format!("{} {}", month_name(), d.year()),
                                    ViewMode::Week => {
                                        let start = d - chrono::TimeDelta::days(d.weekday().num_days_from_sunday() as i64);
                                        let end = start + chrono::TimeDelta::days(6);
                                        format!("{} {} - {} {}", month_name(), start.day(), month_name(), end.day())
                                    }
                                    ViewMode::Day => format!("{} {}, {}", month_name(), d.day(), d.year()),
                                }
                            }}
                        </h2>
                        <div class="flex items-center gap-1 ml-auto">
                            <button
                                on:click=move |_| set_view_mode.set(ViewMode::Month)
                                class=move || format!("px-3 py-1.5 text-sm font-medium rounded-lg transition-colors {}",
                                    if view_mode.get() == ViewMode::Month {
                                        "bg-blue-600 text-white"
                                    } else {
                                        "text-gray-600 dark:text-gray-300 hover:bg-gray-200 dark:hover:bg-gray-700"
                                    }
                                )
                            >
                                {t!("calendar.month")}
                            </button>
                            <button
                                on:click=move |_| set_view_mode.set(ViewMode::Week)
                                class=move || format!("px-3 py-1.5 text-sm font-medium rounded-lg transition-colors {}",
                                    if view_mode.get() == ViewMode::Week {
                                        "bg-blue-600 text-white"
                                    } else {
                                        "text-gray-600 dark:text-gray-300 hover:bg-gray-200 dark:hover:bg-gray-700"
                                    }
                                )
                            >
                                {t!("calendar.week")}
                            </button>
                            <button
                                on:click=move |_| set_view_mode.set(ViewMode::Day)
                                class=move || format!("px-3 py-1.5 text-sm font-medium rounded-lg transition-colors {}",
                                    if view_mode.get() == ViewMode::Day {
                                        "bg-blue-600 text-white"
                                    } else {
                                        "text-gray-600 dark:text-gray-300 hover:bg-gray-200 dark:hover:bg-gray-700"
                                    }
                                )
                            >
                                {t!("calendar.day")}
                            </button>
                        </div>
                    </div>

                    {move || loading.get().then(|| view! {
                        <div class="flex items-center justify-center py-12" role="status" aria-busy="true">
                            <div class="text-sm text-gray-500 font-mono">{t!("common.loading")}</div>
                        </div>
                    })}

                    {move || (!error_msg.get().is_empty() && !loading.get()).then(|| view! {
                        <div class="p-4 bg-red-50 border-l-4 border-l-red-500 rounded text-sm text-red-700" role="alert">
                            <span class="font-bold">{t!("error.prefix")}</span> {error_msg}
                        </div>
                    })}

                    // Month view
                    {move || (view_mode.get() == ViewMode::Month && !loading.get()).then(|| {
                        let d = current_date.get();
                        let year = d.year();
                        let month = d.month();
                        let days = days_in_month(year, month);
                        let first_day = first_day_of_month(year, month);
                        let evts = events.get();

                        let mut cells: Vec<_> = Vec::new();
                        for _ in 0..first_day {
                            cells.push(view! { <div class="min-h-[80px] bg-gray-50 dark:bg-gray-800/50 border border-gray-200 dark:border-gray-700 p-1"></div> }.into_any());
                        }
                        for day in 1..=days {
                            let day_num = day;
                            let is_today = d.day() == day;
                            let day_evts: Vec<_> = evts.iter().filter(|e| {
                                parse_event_dtstart(&e.ical_data)
                                    .map(|dt| dt.starts_with(&format!("{:04}{:02}{:02}", year, month, day_num)))
                                    .unwrap_or(false)
                            }).cloned().collect();

                            let cell_class = format!("min-h-[80px] border border-gray-200 dark:border-gray-700 p-1 {}",
                                if is_today { "bg-blue-50 dark:bg-blue-900/20" } else { "bg-white dark:bg-gray-800" }
                            );
                            let day_class = format!("text-xs font-mono mb-1 {}",
                                if is_today { "font-bold text-blue-600 dark:text-blue-400" } else { "text-gray-500" }
                            );

                            cells.push(view! {
                                <div class=cell_class>
                                    <div class=day_class>{day_num}</div>
                                    <For
                                        each=move || day_evts.clone()
                                        key=|e| e.uid.clone()
                                        let:event
                                    >
                                        {
                                            let summary = parse_event_summary(&event.ical_data);
                                            let color = parse_event_color(&event.ical_data);
                                            let summary_clone = summary.clone();
                                            let evt_clone = event.clone();
                                            view! {
                                                <div
                                                    class="text-xs px-1 py-0.5 mb-0.5 rounded truncate cursor-pointer hover:opacity-80 transition-opacity text-white"
                                                    style=format!("background-color: {}", color)
                                                    on:click=move |_: ev::MouseEvent| open_edit_dialog(evt_clone.clone())
                                                    title=summary_clone
                                                >
                                                    {summary}
                                                </div>
                                            }
                                        }
                                    </For>
                                </div>
                            }.into_any());
                        }
                        let total_cells = cells.len();
                        let remainder = total_cells % 7;
                        if remainder != 0 {
                            for _ in 0..(7 - remainder) {
                                cells.push(view! { <div class="min-h-[80px] bg-gray-50 dark:bg-gray-800/50 border border-gray-200 dark:border-gray-700 p-1"></div> }.into_any());
                            }
                        }

                        view! {
                            <div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm brutal-border overflow-hidden">
                                <div class="grid grid-cols-7">
                                    {header_labels.iter().map(|label| {
                                        let label = (*label).to_string();
                                        view! {
                                            <div class="px-2 py-2 text-xs font-bold uppercase font-mono text-gray-500 text-center border-b border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-800/50">{label}</div>
                                        }
                                    }).collect::<Vec<_>>()}
                                </div>
                                <div class="grid grid-cols-7">
                                    {cells}
                                </div>
                            </div>
                        }
                    })}

                    // Week view
                    {move || (view_mode.get() == ViewMode::Week && !loading.get()).then(|| {
                        let d = current_date.get();
                        let start = d - chrono::TimeDelta::days(d.weekday().num_days_from_sunday() as i64);
                        let evts = events.get();

                        let mut day_cols = Vec::new();
                        for i in 0..7i32 {
                            let day = start + chrono::TimeDelta::days(i as i64);
                            let day_str = format!("{:04}{:02}{:02}", day.year(), day.month(), day.day());
                            let is_today = day == d;
                            let day_evts: Vec<_> = evts.iter().filter(|e| {
                                parse_event_dtstart(&e.ical_data)
                                    .map(|dt| dt.starts_with(&day_str))
                                    .unwrap_or(false)
                            }).cloned().collect();

                            day_cols.push(view! {
                                <div class="flex-1 min-w-[120px] border-r border-gray-200 dark:border-gray-700">
                                    <div class=format!("px-2 py-2 text-center border-b border-gray-200 dark:border-gray-700 {}",
                                        if is_today { "bg-blue-50 dark:bg-blue-900/20" } else { "bg-gray-50 dark:bg-gray-800/50" }
                                    )>
                                        <div class="text-xs font-mono text-gray-500">{header_labels[i as usize]}</div>
                                        <div class=format!("text-lg font-bold font-mono {}",
                                            if is_today { "text-blue-600 dark:text-blue-400" } else { "text-gray-900 dark:text-white" }
                                        )>{day.day()}</div>
                                    </div>
                                    <div class="p-1">
                                        <For
                                            each=move || day_evts.clone()
                                            key=|e| e.uid.clone()
                                            let:event
                                        >
                                            {
                                                let summary = parse_event_summary(&event.ical_data);
                                                let color = parse_event_color(&event.ical_data);
                                                let evt_clone = event.clone();
                                                view! {
                                                    <div
                                                        class="text-xs px-2 py-1 mb-1 rounded cursor-pointer hover:opacity-80 transition-opacity text-white"
                                                        style=format!("background-color: {}", color)
                                                        on:click=move |_: ev::MouseEvent| open_edit_dialog(evt_clone.clone())
                                                    >
                                                        {summary}
                                                    </div>
                                                }
                                            }
                                        </For>
                                    </div>
                                </div>
                            });
                        }

                        view! {
                            <div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm brutal-border overflow-hidden flex">
                                {day_cols}
                            </div>
                        }
                    })}

                    // Day view
                    {move || (view_mode.get() == ViewMode::Day && !loading.get()).then(|| {
                        let d = current_date.get();
                        let day_str = format!("{:04}{:02}{:02}", d.year(), d.month(), d.day());
                        let evts = events.get();
                        let day_evts: Vec<_> = evts.iter().filter(|e| {
                            parse_event_dtstart(&e.ical_data)
                                .map(|dt| dt.starts_with(&day_str))
                                .unwrap_or(false)
                        }).cloned().collect();

                        let mut hours = Vec::new();
                        for h in 0..24i32 {
                            let hour_str = format!("{:02}:00", h);
                            let hour_evts: Vec<_> = day_evts.iter().filter(|e| {
                                parse_event_dtstart(&e.ical_data)
                                    .map(|dt| {
                                        if dt.len() >= 16 {
                                            &dt[9..11] == format!("{:02}", h).as_str()
                                        } else {
                                            false
                                        }
                                    })
                                    .unwrap_or(false)
                            }).cloned().collect();

                            hours.push(view! {
                                <div class="flex border-b border-gray-200 dark:border-gray-700">
                                    <div class="w-16 px-2 py-3 text-xs font-mono text-gray-500 text-right border-r border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-800/50">{hour_str}</div>
                                    <div class="flex-1 p-1 min-h-[48px]">
                                        <For
                                            each=move || hour_evts.clone()
                                            key=|e| e.uid.clone()
                                            let:event
                                        >
                                            {
                                                let summary = parse_event_summary(&event.ical_data);
                                                let color = parse_event_color(&event.ical_data);
                                                let start_str = parse_event_dtstart(&event.ical_data).map(|d| format_ical_datetime(&d)).unwrap_or_default();
                                                let end_str = parse_event_dtend(&event.ical_data).map(|d| format_ical_datetime(&d)).unwrap_or_default();
                                                let evt_clone = event.clone();
                                                view! {
                                                    <div
                                                        class="flex items-center gap-2 px-2 py-1 mb-1 rounded cursor-pointer hover:opacity-80 transition-opacity text-white"
                                                        style=format!("background-color: {}", color)
                                                        on:click=move |_: ev::MouseEvent| open_edit_dialog(evt_clone.clone())
                                                    >
                                                        <span class="text-xs font-bold">{summary}</span>
                                                        <span class="text-xs opacity-75">{start_str} " - " {end_str}</span>
                                                    </div>
                                                }
                                            }
                                        </For>
                                    </div>
                                </div>
                            });
                        }

                        view! {
                            <div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm brutal-border overflow-hidden">
                                <div class="px-4 py-3 border-b border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-800/50">
                                    <div class="text-lg font-bold font-mono text-gray-900 dark:text-white">
                                        {format!("{} {}, {}", month_name(), d.day(), d.year())}
                                    </div>
                                </div>
                                <div class="max-h-[600px] overflow-y-auto">
                                    {hours}
                                </div>
                            </div>
                        }
                    })}

                    // Event creation/editing dialog
                    {move || show_dialog.get().then(|| view! {
                        <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/50" on:click=move |_: ev::MouseEvent| set_show_dialog.set(false)>
                            <div class="bg-white dark:bg-gray-800 rounded-xl shadow-xl max-w-lg w-full mx-4 p-6" on:click=move |e: ev::MouseEvent| e.stop_propagation()>
                                <h3 class="text-lg font-bold font-mono text-gray-900 dark:text-white mb-4">
                                    {move || if editing_event.get().is_some() { t!("calendar.edit_event") } else { t!("calendar.new_event") }}
                                </h3>
                                <div class="space-y-4">
                                    <div>
                                        <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">{t!("calendar.title")}</label>
                                        <input
                                            type="text"
                                            prop:value=move || dialog_title.get()
                                            on:input=move |ev| set_dialog_title.set(event_target_value(&ev))
                                            class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                                        />
                                    </div>
                                    <div class="grid grid-cols-2 gap-4">
                                        <div>
                                            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">{t!("calendar.start")}</label>
                                            <input
                                                type="datetime-local"
                                                prop:value=move || dialog_start.get()
                                                on:input=move |ev| set_dialog_start.set(event_target_value(&ev))
                                                class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                                            />
                                        </div>
                                        <div>
                                            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">{t!("calendar.end")}</label>
                                            <input
                                                type="datetime-local"
                                                prop:value=move || dialog_end.get()
                                                on:input=move |ev| set_dialog_end.set(event_target_value(&ev))
                                                class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                                            />
                                        </div>
                                    </div>
                                    <div class="flex items-center gap-4">
                                        <label class="flex items-center gap-2 cursor-pointer">
                                            <input
                                                type="checkbox"
                                                prop:checked=move || dialog_allday.get()
                                                on:change=move |ev| set_dialog_allday.set(event_target_checked(&ev))
                                                class="w-4 h-4 text-blue-600 border-gray-300 rounded focus:ring-blue-500"
                                            />
                                            <span class="text-sm text-gray-700 dark:text-gray-300">{t!("calendar.all_day")}</span>
                                        </label>
                                        <div>
                                            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">{t!("calendar.color")}</label>
                                            <input
                                                type="color"
                                                prop:value=move || dialog_color.get()
                                                on:input=move |ev| set_dialog_color.set(event_target_value(&ev))
                                                class="w-10 h-8 border border-gray-300 dark:border-gray-600 rounded cursor-pointer"
                                            />
                                        </div>
                                    </div>
                                    <div>
                                        <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">{t!("calendar.description")}</label>
                                        <textarea
                                            prop:value=move || dialog_description.get()
                                            on:input=move |ev| set_dialog_description.set(event_target_value(&ev))
                                            rows="3"
                                            class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                                        ></textarea>
                                    </div>
                                    <div>
                                        <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">{t!("calendar.recurrence")}</label>
                                        <select
                                            prop:value=move || dialog_recurrence.get()
                                            on:change=move |ev| set_dialog_recurrence.set(event_target_value(&ev))
                                            class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                                        >
                                            <option value="">{t!("calendar.none")}</option>
                                            <option value="DAILY">{t!("calendar.daily")}</option>
                                            <option value="WEEKLY">{t!("calendar.weekly")}</option>
                                            <option value="MONTHLY">{t!("calendar.monthly")}</option>
                                            <option value="YEARLY">{t!("calendar.yearly")}</option>
                                        </select>
                                    </div>
                                </div>
                                <div class="flex items-center justify-between mt-6">
                                    <div>
                                        {move || editing_event.get().is_some().then(|| {
                                            let uid = editing_event.get().map(|e| e.uid.clone()).unwrap_or_default();
                                            view! {
                                                <button
                                                    on:click=move |_: ev::MouseEvent| {
                                                        set_show_dialog.set(false);
                                                        delete_event(uid.clone());
                                                    }
                                                    class="px-4 py-2 text-sm font-medium text-red-600 hover:text-red-700 hover:bg-red-50 dark:hover:bg-red-900/20 rounded-lg transition-colors"
                                                >
                                                    {t!("calendar.delete")}
                                                </button>
                                            }
                                        })}
                                    </div>
                                    <div class="flex items-center gap-3">
                                        <button
                                            on:click=move |_: ev::MouseEvent| set_show_dialog.set(false)
                                            class="px-4 py-2 text-sm font-medium text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg transition-colors"
                                        >
                                            {t!("common.cancel")}
                                        </button>
                                        <button
                                            on:click=save_event
                                            class="px-4 py-2 text-sm font-medium text-white bg-blue-600 hover:bg-blue-700 rounded-lg transition-colors"
                                        >
                                            {t!("common.save")}
                                        </button>
                                    </div>
                                </div>
                            </div>
                        </div>
                    })}
                </main>
            </div>
        </div>
    }
}
