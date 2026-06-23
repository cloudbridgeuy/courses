/// URL path of the always-included base stylesheet.
pub const GUIDE_CSS_PATH: &str = "/static/guide.css";
/// URL path of the solution-toggle script, injected when a course uses solutions.
pub const TOGGLE_JS_PATH: &str = "/static/toggle.js";
/// URL path of the reveal.js script used by slideshow pages.
pub const REVEAL_JS_PATH: &str = "/static/reveal.min.js";
/// URL path of the reveal.js base stylesheet.
pub const REVEAL_CSS_PATH: &str = "/static/reveal.min.css";
/// URL path of the CloudBridge slideshow theme stylesheet.
pub const SLIDES_CSS_PATH: &str = "/static/slides.css";
/// URL path of the mermaid.js bundle, injected when a page renders diagrams.
pub const MERMAID_JS_PATH: &str = "/static/mermaid.min.js";
/// URL path of the mermaid initializer, injected after the bundle on guide pages.
pub const MERMAID_INIT_JS_PATH: &str = "/static/mermaid-init.js";
/// URL path of the live-toast client (SSE notifications), loaded on all pages.
pub const NOTIFICATIONS_JS_PATH: &str = "/static/notifications.js";

/// The stylesheets, and scripts, a rendered page must reference, in order,
/// without duplicates.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PageAssets {
    pub styles: Vec<String>,
    pub scripts: Vec<String>,
}

impl PageAssets {
    /// The platform baseline: the guide stylesheet, no scripts.
    pub fn base() -> Self {
        Self {
            styles: vec![GUIDE_CSS_PATH.to_owned()],
            scripts: Vec::new(),
        }
    }

    /// Appends a stylesheet href, skipping duplicates, preserving order.
    pub fn push_style(&mut self, href: &str) {
        push_unique(&mut self.styles, href);
    }

    /// Appends a script src, skipping duplicates, preserving order.
    pub fn push_script(&mut self, src: &str) {
        push_unique(&mut self.scripts, src);
    }
}

fn push_unique(list: &mut Vec<String>, value: &str) {
    if !list.iter().any(|existing| existing == value) {
        list.push(value.to_owned());
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn base_holds_only_guide_css_and_no_scripts() {
        let assets = PageAssets::base();
        assert_eq!(assets.styles, vec![GUIDE_CSS_PATH]);
        assert!(assets.scripts.is_empty());
    }

    #[test]
    fn pushing_duplicate_style_leaves_one_entry() {
        let mut assets = PageAssets::base();
        assets.push_style(GUIDE_CSS_PATH);
        assert_eq!(assets.styles.len(), 1);
    }

    #[test]
    fn pushing_two_distinct_scripts_preserves_insertion_order() {
        let mut assets = PageAssets::base();
        assets.push_script("/a.js");
        assets.push_script("/b.js");
        assert_eq!(assets.scripts, vec!["/a.js", "/b.js"]);
    }

    #[test]
    fn push_unique_on_empty_list_appends() {
        let mut list: Vec<String> = Vec::new();
        push_unique(&mut list, "/x.css");
        assert_eq!(list, vec!["/x.css"]);
    }
}
