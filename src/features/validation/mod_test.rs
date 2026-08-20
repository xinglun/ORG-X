#[test]
fn validation_context_registers_all_required_layers() {
    let _ = std::any::TypeId::of::<super::domain::ValidationRecord>();
    let _ = std::any::TypeId::of::<super::application::validation_evaluator::ValidationEvaluator>();
    let _ =
        std::any::TypeId::of::<super::infrastructure::in_memory_store::InMemoryValidationStore>();
}
