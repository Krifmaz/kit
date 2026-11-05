//! Numbers at a glance: [`StatCard`] (a value with its label, icon and change) and [`ProgressRing`].

use crate::ui::media::unit;
use leptos::prelude::*;

/// A change as people read it: `+12%`, `−3%` (a real minus sign), `0%`; and whether it went up, down or nowhere.
pub fn delta_text(d: f64) -> (String, &'static str) {
    let pct = (d * 100.0).round();
    if pct > 0.0 {
        (format!("+{pct}%"), "up")
    } else if pct < 0.0 {
        (format!("\u{2212}{}%", -pct), "down")
    } else {
        ("0%".to_string(), "flat")
    }
}

/// One number with its label, an icon and how it changed.
#[component]
pub fn StatCard(
    /// What it counts ("Items").
    #[prop(into)]
    label: String,
    /// The value, as shown ("1,284").
    #[prop(into)]
    value: Signal<String>,
    /// A kit icon name.
    #[prop(optional, into)]
    icon: Option<String>,
    /// The change since the last period, as a fraction (0.12 is +12%).
    #[prop(optional, into)]
    delta: Option<f64>,
    /// A line under the value ("this week").
    #[prop(optional, into)]
    hint: Option<String>,
) -> impl IntoView {
    view! {
        <div class="k-stat">
            <div class="k-stat-head">
                <span class="k-stat-label">{label}</span>
                {icon.map(|i| view! { <span class="k-stat-icon" aria-hidden="true" inner_html=crate::icons::icon(&i)></span> })}
            </div>
            <div class="k-stat-value">{move || value.get()}</div>
            <div class="k-stat-foot">
                {delta.map(|d| {
                    let (text, dir) = delta_text(d);
                    let ico = match dir {
                        "up" => crate::icons::icon("trending-up"),
                        "down" => crate::icons::icon("trending-down"),
                        _ => String::new(),
                    };
                    view! { <span class=format!("k-delta {dir}")><span class="k-ico" aria-hidden="true" inner_html=ico></span>{text}</span> }
                })}
                {hint.map(|h| view! { <span class="k-stat-hint">{h}</span> })}
            </div>
        </div>
    }
}

/// A ring that fills with `value` (0–1), the percentage in its middle and an optional label under it.
#[component]
pub fn ProgressRing(
    /// How full (0–1).
    #[prop(into)]
    value: Signal<f64>,
    /// The ring's width and height in px.
    #[prop(optional, default = 72)]
    size: u32,
    /// The ring's thickness in px.
    #[prop(optional, default = 7)]
    stroke: u32,
    /// A label under the ring (and its name for screen readers).
    #[prop(optional, into)]
    label: Option<String>,
    /// The colour: `accent` (default), `ok`, `warn`, `err` or `info`.
    #[prop(optional, into, default = "accent".to_string())]
    tone: String,
) -> impl IntoView {
    let r = (f64::from(size) - f64::from(stroke)) / 2.0;
    let c = std::f64::consts::TAU * r;
    let mid = f64::from(size) / 2.0;
    let pct = move || (unit(value.get()) * 100.0).round();
    let name = label.clone();
    view! {
        <div
            class=format!("k-ring tone-{tone}")
            role="progressbar"
            aria-valuemin="0"
            aria-valuemax="100"
            aria-valuenow=move || pct().to_string()
            aria-label=name
        >
            <div class="k-ring-dial" style=format!("width:{size}px;height:{size}px")>
                <svg viewBox=format!("0 0 {size} {size}") width=size height=size aria-hidden="true">
                    <circle class="k-ring-track" cx=mid cy=mid r=r fill="none" stroke-width=stroke/>
                    <circle
                        class="k-ring-fill"
                        cx=mid
                        cy=mid
                        r=r
                        fill="none"
                        stroke-width=stroke
                        stroke-linecap="round"
                        stroke-dasharray=format!("{c:.2}")
                        stroke-dashoffset=move || format!("{:.2}", c * (1.0 - unit(value.get())))
                        transform=format!("rotate(-90 {mid} {mid})")
                    />
                </svg>
                <span class="k-ring-value">{move || format!("{}%", pct())}</span>
            </div>
            {label.map(|l| view! { <span class="k-ring-label">{l}</span> })}
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn changes_read_as_people_write_them() {
        assert_eq!(delta_text(0.12), ("+12%".to_string(), "up"));
        assert_eq!(delta_text(-0.034), ("\u{2212}3%".to_string(), "down"));
        assert_eq!(delta_text(0.001), ("0%".to_string(), "flat"));
    }

    #[test]
    fn a_ring_says_how_full_it_is() {
        let html = view! { <ProgressRing value=0.25 label="Storage"/> }.to_html();
        for want in ["role=\"progressbar\"", "aria-valuenow=\"25\"", "25%", "Storage", "stroke-dashoffset"] {
            assert!(html.contains(want), "{want} in {html}");
        }
    }

    #[test]
    fn a_stat_card_shows_its_value_and_change() {
        let html = view! { <StatCard label="Items" value="1,284" icon="layers" delta=0.12 hint="this week"/> }.to_html();
        for want in ["Items", "1,284", "+12%", "k-delta up", "this week"] {
            assert!(html.contains(want), "{want} in {html}");
        }
    }
}
