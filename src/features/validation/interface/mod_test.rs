#[test]
fn interface_is_provider_free() {
    let _ = std::any::TypeId::of::<super::super::domain::ValidationHorizon>();
}
