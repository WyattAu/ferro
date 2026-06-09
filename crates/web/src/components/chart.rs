use leptos::prelude::*;

/// A bar chart rendered entirely with SVG.
///
/// Each bar is a `<rect>` element with proper ARIA labels for screen readers.
/// The chart is responsive and scales to fill its container width.
#[component]
pub fn BarChart(
    /// Data points for the chart.
    data: Vec<ChartDataPoint>,
    /// Chart height in pixels.
    #[prop(default = 200)]
    height: u32,
    /// Bar color (CSS color string).
    #[prop(default = "#3b82f6".to_string())]
    bar_color: String,
    /// Color for bars that exceed the threshold.
    #[prop(default = "#ef4444".to_string())]
    threshold_color: String,
    /// Optional threshold value (bars above this use threshold_color).
    #[prop(default = None)]
    threshold: Option<f64>,
    /// Accessible label for the chart.
    #[prop(default = "Bar chart".to_string())]
    aria_label: String,
    /// Whether to show value labels on bars.
    #[prop(default = true)]
    show_values: bool,
    /// Maximum number of bars to display.
    #[prop(default = 20)]
    max_bars: usize,
) -> impl IntoView {
    let display_data: Vec<ChartDataPoint> = data.into_iter().take(max_bars).collect();
    let max_value = display_data
        .iter()
        .map(|d| d.value)
        .fold(0.0_f64, f64::max)
        .max(1.0);

    let chart_width = 100.0; // percentage-based
    let bar_count = display_data.len() as f64;
    let bar_gap = if bar_count > 0.0 { 2.0 / bar_count } else { 0.0 };
    let bar_width = if bar_count > 0.0 {
        (chart_width - bar_gap * (bar_count + 1.0)) / bar_count
    } else {
        0.0
    };

    let aria_label_for_chart = aria_label.clone();
    let aria_label_for_chart_caption = aria_label.clone();

    view! {
        <div
            class="w-full overflow-hidden"
            role="img"
            aria-label=aria_label_for_chart
        >
            <svg
                viewBox=format!("0 0 {} {}", chart_width, height)
                class="w-full"
                preserveAspectRatio="none"
                aria-hidden="true"
            >
                {display_data.iter().enumerate().map(|(i, point)| {
                    let bar_h = (point.value / max_value) * (height as f64 - 20.0);
                    let x = bar_gap + i as f64 * (bar_width + bar_gap);
                    let y = height as f64 - bar_h - 10.0;
                    let color = match threshold {
                        Some(t) if point.value > t => threshold_color.clone(),
                        _ => bar_color.clone(),
                    };
                    let aria_text = format!("{}: {}", point.label, point.value);
                    view! {
                        <g>
                            <rect
                                x=format!("{:.2}", x)
                                y=format!("{:.2}", y)
                                width=format!("{:.2}", bar_width)
                                height=format!("{:.2}", bar_h)
                                rx="2"
                                fill=color
                                role="img"
                                aria-label=aria_text
                            />
                            {if show_values {
                                let text_x = x + bar_width / 2.0;
                                view! {
                                    <text
                                        x=format!("{:.2}", text_x)
                                        y=format!("{:.2}", y - 4.0)
                                        text-anchor="middle"
                                        class="text-xs fill-current text-gray-600 dark:text-gray-400"
                                        font-size="10"
                                    >
                                        {point.value}
                                    </text>
                                }.into_any()
                            } else {
                                view! {}.into_any()
                            }}
                        </g>
                    }
                }).collect_view()}
            </svg>
            // Screen reader accessible data table
            <table class="sr-only">
                <caption>{aria_label_for_chart_caption}</caption>
                <thead>
                    <tr>
                        <th>"Label"</th>
                        <th>"Value"</th>
                    </tr>
                </thead>
                <tbody>
                    {display_data.iter().map(|point| view! {
                        <tr>
                            <td>{point.label.clone()}</td>
                            <td>{point.value}</td>
                        </tr>
                    }).collect_view()}
                </tbody>
            </table>
        </div>
    }
}

/// A line chart rendered entirely with SVG.
///
/// Draws a polyline connecting data points with optional area fill.
/// Includes proper ARIA labels for each data point.
#[component]
pub fn LineChart(
    /// Data points for the chart.
    data: Vec<ChartDataPoint>,
    /// Chart height in pixels.
    #[prop(default = 200)]
    height: u32,
    /// Line color (CSS color string).
    #[prop(default = "#3b82f6".to_string())]
    line_color: String,
    /// Whether to fill the area under the line.
    #[prop(default = false)]
    fill_area: bool,
    /// Fill color (CSS color string). Defaults to line_color with opacity.
    #[prop(default = None)]
    fill_color: Option<String>,
    /// Accessible label for the chart.
    #[prop(default = "Line chart".to_string())]
    aria_label: String,
    /// Whether to show data point markers.
    #[prop(default = true)]
    show_dots: bool,
    /// Maximum number of data points to display.
    #[prop(default = 50)]
    max_points: usize,
) -> impl IntoView {
    let display_data: Vec<ChartDataPoint> = data.into_iter().take(max_points).collect();
    let max_value = display_data
        .iter()
        .map(|d| d.value)
        .fold(0.0_f64, f64::max)
        .max(1.0);

    let chart_width = 100.0;
    let padding = 5.0;
    let usable_width = chart_width - padding * 2.0;
    let usable_height = height as f64 - 20.0 - padding;

    let points_svg: String = display_data
        .iter()
        .enumerate()
        .map(|(i, point)| {
            let x = padding + (i as f64 / (display_data.len().max(1) - 1) as f64) * usable_width;
            let y = padding + usable_height - (point.value / max_value) * usable_height;
            format!("{:.2},{:.2}", x, y)
        })
        .collect::<Vec<_>>()
        .join(" ");

    let area_path = if fill_area && !display_data.is_empty() {
        let first_x = padding;
        let last_x = padding + usable_width;
        Some(format!(
            "M{:.2},{:.2} L{} L{:.2},{:.2} Z",
            first_x,
            padding + usable_height,
            points_svg
                .split(' ')
                .map(|p| format!("L{}", p))
                .collect::<Vec<_>>()
                .join(" "),
            last_x,
            padding + usable_height,
        ))
    } else {
        None
    };

    let resolved_fill_color = fill_color.unwrap_or_else(|| {
        let hex = line_color.trim_start_matches('#');
        if hex.len() == 6 {
            format!("#{}33", hex) // 20% opacity
        } else {
            format!("{}33", line_color)
        }
    });

    let aria_label_for_line = aria_label.clone();
    let aria_label_for_line_caption = aria_label.clone();

    view! {
        <div
            class="w-full overflow-hidden"
            role="img"
            aria-label=aria_label_for_line
        >
            <svg
                viewBox=format!("0 0 {} {}", chart_width, height)
                class="w-full"
                preserveAspectRatio="none"
                aria-hidden="true"
            >
                {area_path.map(|d| view! {
                    <path d=d fill=resolved_fill_color.clone() opacity="0.3" />
                })}
                {if !points_svg.is_empty() {
                    view! {
                        <polyline
                            points=points_svg
                            fill="none"
                            stroke=line_color.clone()
                            stroke-width="2"
                            stroke-linecap="round"
                            stroke-linejoin="round"
                        />
                    }.into_any()
                } else {
                    view! {}.into_any()
                }}
                {if show_dots {
                    display_data.iter().enumerate().map(|(i, point)| {
                        let x = padding + (i as f64 / (display_data.len().max(1) - 1) as f64) * usable_width;
                        let y = padding + usable_height - (point.value / max_value) * usable_height;
                        let aria_text = format!("{}: {}", point.label, point.value);
                        view! {
                            <circle
                                cx=format!("{:.2}", x)
                                cy=format!("{:.2}", y)
                                r="3"
                                fill="white"
                                stroke=line_color.clone()
                                stroke-width="2"
                                role="img"
                                aria-label=aria_text
                            />
                        }
                    }).collect_view().into_any()
                } else {
                    view! {}.into_any()
                }}
            </svg>
            // Screen reader accessible data table
            <table class="sr-only">
                <caption>{aria_label_for_line_caption}</caption>
                <thead>
                    <tr>
                        <th>"Label"</th>
                        <th>"Value"</th>
                    </tr>
                </thead>
                <tbody>
                    {display_data.iter().map(|point| view! {
                        <tr>
                            <td>{point.label.clone()}</td>
                            <td>{point.value}</td>
                        </tr>
                    }).collect_view()}
                </tbody>
            </table>
        </div>
    }
}

/// A data point for charts.
#[derive(Clone, Debug)]
pub struct ChartDataPoint {
    /// Label for the data point.
    pub label: String,
    /// Numeric value.
    pub value: f64,
}

impl ChartDataPoint {
    pub fn new(label: impl Into<String>, value: f64) -> Self {
        Self {
            label: label.into(),
            value,
        }
    }
}
