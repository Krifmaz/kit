//! Placeholder art: a cover for an item that has none, drawn from a seed (the item's id) and its title, so the
//! same item always gets the same cover and a grid of covers never shows blank boxes.

/// Two colours of the seed's cover, as hues (0–359).
fn hues(seed: u32) -> (u32, u32) {
    // a multiplicative hash spreads neighbouring ids over the colour wheel
    let h = seed.wrapping_mul(2_654_435_761);
    let a = h % 360;
    (a, (a + 40 + (h >> 9) % 80) % 360)
}

/// Up to two capital letters of `title` (the first letters of its first two words), or `""`.
pub fn initials(title: &str) -> String {
    title.split_whitespace().filter_map(|w| w.chars().find(|c| c.is_alphanumeric())).take(2).flat_map(char::to_uppercase).collect()
}

/// An SVG cover (a 2 : 3 poster box that `object-fit: cover` crops to any shape) as a `data:` URL: a gradient of
/// two colours from `seed`, soft shapes, and the initials of `title`. Safe in an `src` and in `url("…")`.
pub fn placeholder_art(seed: u32, title: &str) -> String {
    let (a, b) = hues(seed);
    let r = 60 + seed % 50;
    let cx = 40 + (seed * 37) % 220;
    let svg = format!(
        "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 300 450'>\
<defs><linearGradient id='g' x1='0' y1='0' x2='1' y2='1'><stop offset='0' stop-color='hsl({a},45%,42%)'/>\
<stop offset='1' stop-color='hsl({b},50%,22%)'/></linearGradient></defs>\
<rect width='300' height='450' fill='url(#g)'/>\
<circle cx='{cx}' cy='140' r='{r}' fill='hsl({b},60%,70%)' fill-opacity='.22'/>\
<circle cx='{}' cy='330' r='{}' fill='hsl({a},70%,80%)' fill-opacity='.14'/>\
<text x='150' y='252' text-anchor='middle' font-family='system-ui,sans-serif' font-size='84' font-weight='700' \
fill='white' fill-opacity='.85'>{}</text></svg>",
        300 - cx,
        r + 30,
        initials(title)
    );
    let mut url = String::from("data:image/svg+xml,");
    for c in svg.chars() {
        match c {
            '#' => url.push_str("%23"),
            '<' => url.push_str("%3C"),
            '>' => url.push_str("%3E"),
            '"' => url.push_str("%22"),
            '%' => url.push_str("%25"),
            '&' => url.push_str("%26"),
            c => url.push(c),
        }
    }
    url
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initials_are_the_first_letters_of_two_words() {
        assert_eq!(initials("northern lights over the bay"), "NL");
        assert_eq!(initials("  (quiet) hours "), "QH");
        assert_eq!(initials("x"), "X");
        assert_eq!(initials(""), "");
    }

    #[test]
    fn the_same_seed_draws_the_same_cover() {
        assert_eq!(placeholder_art(7, "A b"), placeholder_art(7, "A b"));
        assert_ne!(placeholder_art(7, "A b"), placeholder_art(8, "A b"));
    }

    #[test]
    fn the_url_needs_no_escaping_in_html_or_css() {
        let u = placeholder_art(3, "<b> & \"c\"");
        assert!(u.starts_with("data:image/svg+xml,"));
        for bad in ['#', '<', '>', '"', '&'] {
            assert!(!u.contains(bad), "{bad} in {u}");
        }
    }
}
