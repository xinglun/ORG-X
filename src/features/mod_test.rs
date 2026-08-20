#[test]
fn validation_context_is_exported_from_the_features_root() {
    let _ = std::any::TypeId::of::<super::validation::domain::ValidationHorizon>();
}
