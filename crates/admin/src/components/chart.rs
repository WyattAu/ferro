use leptos::prelude::*;

#[component]
pub fn BarChart(
    data: Vec<(String, f64)>,
    title: String,
    #[prop(default = "#E85D04".to_string())] color: String,
) -> impl IntoView {
    let max_val = data.iter().map(|(_, v)| *v).fold(0.0_f64, f64::max).max(1.0);
    let bar_count = data.len().max(1);
    let bar_width = 100.0 / bar_count as f64;

    let bars: Vec<_> = data
        .iter()
        .enumerate()
        .map(|(i, (label, val))| {
            let height = (*val / max_val) * 80.0;
            let x = i as f64 * bar_width + bar_width * 0.1;
            let w = bar_width * 0.8;
            let y = 80.0 - height;
            let display_val = if *val >= 1_000_000.0 {
                format!("{:.1}M", *val / 1_000_000.0)
            } else if *val >= 1_000.0 {
                format!("{:.1}K", *val / 1_000.0)
            } else {
                format!("{:.0}", val)
            };
            let c = color.clone();
            view! {
                <>
                    <rect x={format!("{}", x)} y={format!("{}", y)} width={format!("{}", w)} height={format!("{}", height)} fill=c rx="2" opacity="0.85" />
                    <text x={format!("{}", x + w / 2.0)} y={format!("{}", y - 2.0)} text-anchor="middle" font-size="5" fill="var(--text-secondary)">{display_val}</text>
                    <text x={format!("{}", x + w / 2.0)} y="96" text-anchor="middle" font-size="5" fill="var(--text-secondary)">{label.clone()}</text>
                </>
            }
        })
        .collect();

    let aria_label = format!("Bar chart: {}", title);

    view! {
        <div class="chart-container">
            <h3 class="chart-title font-display">{title}</h3>
            <svg viewBox="0 0 100 100" preserveAspectRatio="none" class="bar-chart" role="img" aria-label=aria_label aria-hidden="true">
                {bars}
            </svg>
        </div>
    }.into_view()
}

#[component]
pub fn LineChart(
    data: Vec<(String, f64)>,
    title: String,
    #[prop(default = "#E85D04".to_string())] color: String,
) -> impl IntoView {
    if data.is_empty() {
        return view! {
            <div class="chart-container">
                <h3 class="chart-title font-display">{title}</h3>
                <div class="text-sm text-center py-8" style="color: var(--text-secondary)">No data available</div>
            </div>
        }
        .into_any();
    }

    let max_val = data.iter().map(|(_, v)| *v).fold(0.0_f64, f64::max).max(1.0);
    let point_count = data.len().max(2);
    let usable_width = 90.0;
    let usable_height = 70.0;
    let padding_x = 5.0;
    let padding_y = 5.0;

    let points: Vec<(f64, f64)> = data
        .iter()
        .enumerate()
        .map(|(i, (_, val))| {
            let x = padding_x + (i as f64 / (point_count - 1) as f64) * usable_width;
            let y = padding_y + usable_height - (*val / max_val) * usable_height;
            (x, y)
        })
        .collect();

    let polyline: String = points
        .iter()
        .map(|(x, y)| format!("{:.2},{:.2}", x, y))
        .collect::<Vec<_>>()
        .join(" ");

    let area_path = if points.len() > 1 {
        let first_x = points.first().map(|(x, _)| *x).unwrap_or(0.0);
        let last_x = points.last().map(|(x, _)| *x).unwrap_or(0.0);
        let bottom_y = padding_y + usable_height;
        let mut d = format!("M{:.2},{:.2}", first_x, bottom_y);
        for (x, y) in &points {
            d.push_str(&format!(" L{:.2},{:.2}", x, y));
        }
        d.push_str(&format!(" L{:.2},{:.2} Z", last_x, bottom_y));
        Some(d)
    } else {
        None
    };

    let labels: Vec<_> = data
        .iter()
        .enumerate()
        .filter(|(i, _)| *i == 0 || *i == data.len() - 1 || data.len() <= 8 || i % ((data.len() / 6).max(1)) == 0)
        .map(|(i, (label, _))| {
            let x = padding_x + (i as f64 / (point_count - 1) as f64) * usable_width;
            (x, label.clone())
        })
        .collect();

    let aria_label = format!("Line chart: {}", title);

    view! {
        <div class="chart-container">
            <h3 class="chart-title font-display">{title}</h3>
            <svg viewBox="0 0 100 100" preserveAspectRatio="none" class="bar-chart" role="img" aria-label=aria_label aria-hidden="true">
                {area_path.map(|d| view! {
                    <path d=d fill=color.clone() opacity="0.15" />
                })}
                <polyline
                    points=polyline
                    fill="none"
                    stroke=color.clone()
                    stroke-width="1.5"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    opacity="0.9"
                />
                {points.iter().map(|(x, y)| view! {
                    <circle cx=format!("{:.2}", x) cy=format!("{:.2}", y) r="2" fill="white" stroke=color.clone() stroke-width="1" />
                }).collect::<Vec<_>>()}
                {labels.into_iter().map(|(x, label)| view! {
                    <text x=format!("{:.2}", x) y="96" text-anchor="middle" font-size="4.5" fill="var(--text-secondary)">{label}</text>
                }).collect::<Vec<_>>()}
            </svg>
        </div>
    }.into_any()
}

#[component]
pub fn PieChart(data: Vec<(String, f64)>, title: String) -> impl IntoView {
    let total: f64 = data.iter().map(|(_, v)| v).sum();
    let colors = [
        "#E85D04", "#370617", "#CA8A04", "#DC2626", "#8B8178", "#15803D", "#2B2B2B", "#D4520A",
    ];

    let mut cumulative = 0.0;
    let segments: Vec<_> = data
        .iter()
        .enumerate()
        .filter_map(|(i, (_label, val))| {
            let fraction = if total > 0.0 { *val / total } else { 0.0 };
            if fraction < 0.01 {
                return None;
            }
            let start_angle = cumulative * 360.0 - 90.0;
            cumulative += fraction;
            let end_angle = cumulative * 360.0 - 90.0;
            let start_rad = start_angle.to_radians();
            let end_rad = end_angle.to_radians();
            let cx = 50.0;
            let cy = 50.0;
            let r = 35.0;
            let x1 = cx + r * start_rad.cos();
            let y1 = cy + r * start_rad.sin();
            let x2 = cx + r * end_rad.cos();
            let y2 = cy + r * end_rad.sin();
            let large_arc = if fraction > 0.5 { 1 } else { 0 };
            let c = colors[i % colors.len()].to_string();
            let pct = (fraction * 100.0) as u32;
            let mid_angle = (start_angle + end_angle) / 2.0;
            let mid_rad = mid_angle.to_radians();
            let lx = cx + (r + 12.0) * mid_rad.cos();
            let ly = cy + (r + 12.0) * mid_rad.sin();
            Some(view! {
                <>
                    <path d={format!("M {} {} A {} {} 0 {} 1 {} {} L {} {} Z", x1, y1, r, r, large_arc, x2, y2, cx, cy)} fill=c opacity="0.8" />
                    <text x={format!("{}", lx)} y={format!("{}", ly)} text-anchor="middle" dominant-baseline="middle" font-size="4" fill="var(--text-primary)">{format!("{}%", pct)}</text>
                </>
            })
        })
        .collect();

    let legend_items: Vec<_> = data
        .iter()
        .enumerate()
        .map(|(i, (label, val))| {
            let c = colors[i % colors.len()].to_string();
            let pct = if total > 0.0 {
                format!("{:.1}%", (*val / total) * 100.0)
            } else {
                "0%".to_string()
            };
            view! {
                <div class="legend-item" role="listitem">
                    <span class="legend-color" style={format!("background: {}", c)} aria-hidden="true"></span>
                    <span class="legend-label">{label.clone()}</span>
                    <span class="legend-value font-display">{pct}</span>
                </div>
            }
        })
        .collect();

    let aria_label = format!("Pie chart: {}", title);

    view! {
        <div class="chart-container">
            <h3 class="chart-title font-display">{title}</h3>
            <div class="pie-chart-wrapper">
                <svg viewBox="0 0 100 100" class="pie-chart" role="img" aria-label=aria_label aria-hidden="true">{segments}</svg>
                <div class="chart-legend" role="list" aria-label="Chart legend">{legend_items}</div>
            </div>
        </div>
    }
}
