//! HTML escaping, for the components that keep Encore's HTML strings (a modal's title and body, the markup
//! of hand-written pages) and for the markup tests.
//!
//! From Encore's `web-rs/src/ui.rs` `esc`.

/// HTML-escape `s` the way Encore's `esc` does (`&`, `<`, `>`, `"` and `'`).
pub fn esc(s: &str) -> String {
    let mut o = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => o.push_str("&amp;"),
            '<' => o.push_str("&lt;"),
            '>' => o.push_str("&gt;"),
            '"' => o.push_str("&quot;"),
            '\'' => o.push_str("&#39;"),
            _ => o.push(c),
        }
    }
    o
}

/// Inner HTML for an element that also gets an empty text child (`{""}`): `<b {..html(s)}>{""}</b>`. Server-side
/// rendering writes `s` in place of the children; in the browser the HTML is set first and the empty text node
/// adds nothing to `outerHTML`. `inner_html=` instead leaves an empty `<!---->` placeholder after the HTML.
pub fn html<T: leptos::tachys::html::element::InnerHtmlValue>(s: T) -> leptos::tachys::html::element::InnerHtml<T> {
    leptos::tachys::html::element::inner_html(s)
}

#[cfg(test)]
mod tests {
    #[test]
    fn escapes_like_encore() {
        assert_eq!(super::esc(r#"a<b> & "c" 'd'"#), "a&lt;b&gt; &amp; &quot;c&quot; &#39;d&#39;");
    }
}
