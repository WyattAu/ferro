use leptos::*;

#[component]
pub fn BarChart(
    data: Vec<(String, f64)>,
    title: String,
    #[prop(default = "#E85D04".to_string())] color: String,
) -> impl IntoView {
    let max_val = data
        .iter()
        .map(|(_, v)| *v)
        .fold(0.0_f64, f64::max)
        .max(1.0);
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
                    <text x={format!("{}", x + w / 2.0)} y="96" text-anchor="middle" font-size="5" fill="var(--text-secondary)">{label}</text>
                </>
            }
        })
        .collect();

    let aria_label = format!("Bar chart: {}", title);

    view! {
        <div class="chart-container">
            <h4 class="chart-title">{title}</h4>
            <svg viewBox="0 0 100 100" preserveAspectRatio="none" class="bar-chart" role="img" aria-label=aria_label>
                {bars}
            </svg>
        </div>
    }
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
                <div class="legend-item">
                    <span class="legend-color" style={format!("background: {}", c)}></span>
                    <span class="legend-label">{label}</span>
                    <span class="legend-value">{pct}</span>
                </div>
            }
        })
        .collect();

    let aria_label = format!("Pie chart: {}", title);

    view! {
        <div class="chart-container">
            <h4 class="chart-title">{title}</h4>
            <div class="pie-chart-wrapper">
                <svg viewBox="0 0 100 100" class="pie-chart" role="img" aria-label=aria_label>{segments}</svg>
                <div class="chart-legend">{legend_items}</div>
            </div>
        </div>
    }
}
