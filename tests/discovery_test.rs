use org_x::features::weekly_radar::runtime::discovery::document_metadata;

#[test]
fn document_metadata_keeps_substantive_paragraph_after_navigation_cleanup() {
    let (_, _, body) = document_metadata(
        r#"<nav>Skip to content</nav>
        <div class="social-share"><p>Share this update.</p></div>
        <main><p>Acme moved its engineering workflow to production scheduling.</p></main>
        <footer>Privacy policy</footer>"#,
        "fallback",
    );

    assert_eq!(
        body,
        "Acme moved its engineering workflow to production scheduling."
    );
}
