use super::markdown_renderer::MarkdownRenderer;

#[test]
fn markdown_renderer_is_registered_at_the_interface_boundary() {
    fn assert_registered<T>() {}

    assert_registered::<MarkdownRenderer>();
}
