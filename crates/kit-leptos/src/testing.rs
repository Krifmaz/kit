//! Markup tests: HTML compared as the browser builds it, not byte for byte.
//!
//! The expected markup in a test is written by hand, with indentation and in any attribute order; Leptos renders
//! the same elements without the whitespace between them and puts `class` last. Both give the browser the same
//! document, so the tests compare a canonical form: elements with their attributes sorted (a class as its sorted words, an
//! attribute without a value as `""`), text with entities decoded and runs of white space collapsed, white
//! space alone between elements and comments (Leptos' `<!>` markers) left out, and the `<tbody>` a table's parser
//! adds left out.

/// One token of the canonical form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    /// `<name a="1" b="">`.
    Open(String, Vec<(String, String)>),
    /// `</name>` (also added for a void element and for `<x/>`).
    Close(String),
    /// Text between tags, decoded and collapsed.
    Text(String),
}

const VOID: [&str; 9] = ["input", "img", "br", "hr", "meta", "link", "source", "area", "col"];

fn decode(s: &str) -> String {
    s.replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&#39;", "'").replace("&#x27;", "'").replace("&amp;", "&")
}

fn collapse(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The canonical tokens of `html`.
pub fn tokens(html: &str) -> Vec<Token> {
    let mut out = Vec::new();
    let mut text = String::new();
    let b = html.as_bytes();
    let mut i = 0;
    let flush = |text: &mut String, out: &mut Vec<Token>| {
        let t = collapse(&decode(text));
        if !t.is_empty() {
            if let Some(Token::Text(prev)) = out.last_mut() {
                prev.push(' ');
                prev.push_str(&t);
            } else {
                out.push(Token::Text(t));
            }
        }
        text.clear();
    };
    while i < b.len() {
        if b[i] != b'<' {
            let start = i;
            while i < b.len() && b[i] != b'<' {
                i += 1;
            }
            text.push_str(&html[start..i]);
            continue;
        }
        // a comment or Leptos' marker
        if html[i..].starts_with("<!--") {
            i = html[i..].find("-->").map(|j| i + j + 3).unwrap_or(b.len());
            continue;
        }
        if html[i..].starts_with("<!") {
            i = html[i..].find('>').map(|j| i + j + 1).unwrap_or(b.len());
            continue;
        }
        flush(&mut text, &mut out);
        let close = b.get(i + 1) == Some(&b'/');
        let mut j = i + if close { 2 } else { 1 };
        let name_start = j;
        while j < b.len() && !b[j].is_ascii_whitespace() && b[j] != b'>' && b[j] != b'/' {
            j += 1;
        }
        let name = html[name_start..j].to_ascii_lowercase();
        if close {
            j = html[j..].find('>').map(|k| j + k + 1).unwrap_or(b.len());
            out.push(Token::Close(name));
            i = j;
            continue;
        }
        let mut attrs = Vec::new();
        let mut self_closing = false;
        loop {
            while j < b.len() && b[j].is_ascii_whitespace() {
                j += 1;
            }
            if j >= b.len() {
                break;
            }
            if b[j] == b'>' {
                j += 1;
                break;
            }
            if b[j] == b'/' {
                self_closing = true;
                j += 1;
                continue;
            }
            let k0 = j;
            while j < b.len()
                && !b[j].is_ascii_whitespace()
                && b[j] != b'='
                && b[j] != b'>'
                && !(b[j] == b'/' && b.get(j + 1) == Some(&b'>'))
            {
                j += 1;
            }
            let key = html[k0..j].to_ascii_lowercase();
            let mut value = String::new();
            if j < b.len() && b[j] == b'=' {
                j += 1;
                if j < b.len() && (b[j] == b'"' || b[j] == b'\'') {
                    let q = b[j];
                    let v0 = j + 1;
                    j = v0;
                    while j < b.len() && b[j] != q {
                        j += 1;
                    }
                    value = decode(&html[v0..j]);
                    j += 1;
                } else {
                    let v0 = j;
                    while j < b.len() && !b[j].is_ascii_whitespace() && b[j] != b'>' {
                        j += 1;
                    }
                    value = decode(&html[v0..j]);
                }
            }
            if key == "class" {
                let mut words: Vec<&str> = value.split_whitespace().collect();
                words.sort_unstable();
                value = words.join(" ");
            }
            if key == "style" {
                value = value
                    .split(';')
                    .map(|d| d.split_whitespace().collect::<String>())
                    .filter(|d| !d.is_empty())
                    .collect::<Vec<_>>()
                    .join(";");
            }
            attrs.push((key, value));
        }
        attrs.sort();
        out.push(Token::Open(name.clone(), attrs));
        if self_closing || VOID.contains(&name.as_str()) {
            out.push(Token::Close(name));
        }
        i = j;
    }
    flush(&mut text, &mut out);
    // `<x/>` written as `<x></x>` and the other way round: drop a close right after a void element's own close.
    // A plain `<tbody>` is left out: the browser's parser adds it around rows written straight into a `<table>`.
    let mut clean: Vec<Token> = Vec::with_capacity(out.len());
    for t in out {
        if matches!(&t, Token::Open(n, a) if n == "tbody" && a.is_empty()) || matches!(&t, Token::Close(n) if n == "tbody") {
            continue;
        }
        if let Token::Close(n) = &t {
            if VOID.contains(&n.as_str()) && matches!(clean.last(), Some(Token::Close(m)) if m == n) {
                continue;
            }
        }
        clean.push(t);
    }
    clean
}

/// Renders the view `f` makes to HTML, under a reactive owner (as `mount_to` gives one in the browser).
pub fn render<V: leptos::prelude::RenderHtml>(f: impl FnOnce() -> V) -> String {
    let owner = leptos::prelude::Owner::new();
    owner.with(|| f().to_html())
}

/// Whether two HTML strings build the same document (see the module).
pub fn same_html(a: &str, b: &str) -> bool {
    tokens(a) == tokens(b)
}

/// Panics with the first token where `got` differs from `want`.
#[track_caller]
pub fn assert_same_html(got: &str, want: &str) {
    let (g, w) = (tokens(got), tokens(want));
    if g == w {
        return;
    }
    let at = g.iter().zip(&w).position(|(a, b)| a != b).unwrap_or(g.len().min(w.len()));
    let around = |t: &[Token]| format!("{:#?}", &t[at.saturating_sub(2)..(at + 3).min(t.len())]);
    panic!(
        "the markup differs at token {at}:\n--- got\n{}\n--- want\n{}\n--- got html\n{got}\n--- want html\n{want}",
        around(&g),
        around(&w)
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn whitespace_order_and_markers_do_not_count() {
        assert!(same_html(
            "<div class=\"toast \" role=\"alert\">\n  <span>a &amp; b</span><input checked><!></div>",
            "<div role='alert' class='toast'><span>a  &amp;\nb</span><input checked=\"\"/></div>"
        ));
        assert!(!same_html("<b>x</b>", "<b>y</b>"));
        assert!(!same_html("<b class=\"a\">x</b>", "<b>x</b>"));
        assert!(same_html(r#"<p style="margin:0; padding: 0">x</p>"#, r#"<p style="margin:0;padding:0;">x</p>"#));
        assert!(same_html("<table><tr><td>x</td></tr></table>", "<table><tbody><tr><td>x</td></tr></tbody></table>"));
    }
}
