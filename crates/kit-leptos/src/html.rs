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

/// An HTML fragment drawn as it is, without a wrapper element and without a placeholder comment: the nodes
/// `innerHTML` would make of `s` (elements, text, the whitespace between them), mounted where the view stands.
/// An app's own HTML builders (Encore's hand-written blocks) go into a component this way, node for node.
/// An empty string is one empty text node, which adds nothing to `outerHTML`.
pub fn raw(s: impl Into<String>) -> RawHtml {
    RawHtml(s.into())
}

/// See [`raw`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RawHtml(pub String);

/// The nodes a [`RawHtml`] put on the page.
pub struct RawHtmlState {
    html: String,
    nodes: Vec<web_sys::Node>,
}

fn parse_nodes(html: &str) -> Vec<web_sys::Node> {
    use wasm_bindgen::JsCast;
    let doc = web_sys::window().and_then(|w| w.document()).expect("a document");
    if html.is_empty() {
        return vec![doc.create_text_node("").into()];
    }
    let t: web_sys::HtmlTemplateElement = doc.create_element("template").expect("template").unchecked_into();
    t.set_inner_html(html);
    let frag = t.content();
    let list = frag.child_nodes();
    let nodes: Vec<web_sys::Node> = (0..list.length()).filter_map(|i| list.get(i)).collect();
    for n in &nodes {
        let _ = frag.remove_child(n);
    }
    // nodes made in a template's document are adopted by the page's when they are inserted
    nodes
}

impl leptos::tachys::view::Render for RawHtml {
    type State = RawHtmlState;

    fn build(self) -> Self::State {
        let nodes = parse_nodes(&self.0);
        RawHtmlState { html: self.0, nodes }
    }

    fn rebuild(self, state: &mut Self::State) {
        use leptos::tachys::view::Mountable;
        if self.0 == state.html {
            return;
        }
        let mut new = <Self as leptos::tachys::view::Render>::build(self);
        state.insert_before_this(&mut new);
        state.unmount();
        *state = new;
    }
}

impl leptos::tachys::view::Mountable for RawHtmlState {
    fn unmount(&mut self) {
        for n in &self.nodes {
            if let Some(p) = n.parent_node() {
                let _ = p.remove_child(n);
            }
        }
    }

    fn mount(&mut self, parent: &web_sys::Element, marker: Option<&web_sys::Node>) {
        for n in &self.nodes {
            let _ = parent.insert_before(n, marker);
        }
    }

    fn insert_before_this(&self, child: &mut dyn leptos::tachys::view::Mountable) -> bool {
        use wasm_bindgen::JsCast;
        let Some(first) = self.nodes.first() else { return false };
        let Some(parent) = first.parent_node().and_then(|p| p.dyn_into::<web_sys::Element>().ok()) else { return false };
        child.mount(&parent, Some(first));
        true
    }

    fn elements(&self) -> Vec<web_sys::Element> {
        use wasm_bindgen::JsCast;
        self.nodes.iter().filter_map(|n| n.clone().dyn_into::<web_sys::Element>().ok()).collect()
    }
}

impl leptos::tachys::view::add_attr::AddAnyAttr for RawHtml {
    type Output<SomeNewAttr: leptos::tachys::html::attribute::Attribute> = RawHtml;

    fn add_any_attr<NewAttr: leptos::tachys::html::attribute::Attribute>(self, _attr: NewAttr) -> Self::Output<NewAttr> {
        self
    }
}

impl leptos::tachys::view::RenderHtml for RawHtml {
    type AsyncOutput = Self;
    type Owned = Self;

    const MIN_LENGTH: usize = 0;

    fn dry_resolve(&mut self) {}

    async fn resolve(self) -> Self::AsyncOutput {
        self
    }

    fn html_len(&self) -> usize {
        self.0.len()
    }

    fn to_html_with_buf(
        self,
        buf: &mut String,
        position: &mut leptos::tachys::view::Position,
        _escape: bool,
        _mark_branches: bool,
        _extra_attrs: Vec<leptos::tachys::html::attribute::any_attribute::AnyAttribute>,
    ) {
        buf.push_str(&self.0);
        *position = leptos::tachys::view::Position::NextChild;
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        _cursor: &leptos::tachys::hydration::Cursor,
        _position: &leptos::tachys::view::PositionState,
    ) -> Self::State {
        // the kit renders on the client only; a hydrating app draws the fragment anew
        <Self as leptos::tachys::view::Render>::build(self)
    }

    fn into_owned(self) -> Self::Owned {
        self
    }
}

/// Views one after another with nothing of Leptos' own between or after them: a `Vec` of views leaves an
/// empty `<!---->` placeholder after its items (it may grow), these nest as tuples instead. For a list that is
/// drawn once; a list that changes is drawn again as a whole.
pub fn seq<V: leptos::prelude::IntoView + 'static>(items: impl IntoIterator<Item = V>) -> leptos::prelude::AnyView {
    use leptos::prelude::IntoAny;
    fn tree(mut v: Vec<leptos::prelude::AnyView>) -> leptos::prelude::AnyView {
        use leptos::prelude::IntoAny;
        match v.len() {
            0 => "".into_any(),
            1 => v.pop().unwrap_or_else(|| "".into_any()),
            n => {
                let right = v.split_off(n / 2);
                (tree(v), tree(right)).into_any()
            }
        }
    }
    tree(items.into_iter().map(|i| i.into_view().into_any()).collect())
}

/// An optional part: the view, or an empty text node (an `Option` that is `None` leaves an empty `<!---->`
/// placeholder, which Encore's HTML does not have).
pub fn opt<V: leptos::prelude::IntoView + 'static>(v: Option<V>) -> leptos::prelude::AnyView {
    use leptos::prelude::IntoAny;
    match v {
        Some(v) => v.into_view().into_any(),
        None => "".into_any(),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn escapes_like_encore() {
        assert_eq!(super::esc(r#"a<b> & "c" 'd'"#), "a&lt;b&gt; &amp; &quot;c&quot; &#39;d&#39;");
    }

    #[test]
    fn raw_seq_and_opt_add_nothing_of_their_own() {
        use leptos::prelude::*;
        let html = view! { <div>{super::raw("<b>a</b>\n  <i>b</i>")}</div> }.to_html();
        assert_eq!(html, "<div><b>a</b>\n  <i>b</i></div>");
        let list = super::seq(["x", "y", "z"].map(|s| view! { <li>{s}</li> }));
        let html = view! { <ul>{list}</ul> }.to_html();
        assert!(html.starts_with("<ul><li>x</li>") && html.contains("<li>z</li>"), "{html}");
        let none: Option<&str> = None;
        let html = view! { <p>{super::opt(none)}</p> }.to_html();
        // no `<!>` placeholder: server-side rendering writes the empty text node as a space
        assert_eq!(html, "<p> </p>");
    }
}
