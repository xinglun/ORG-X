#[test]
fn application_registers_evaluator_and_store_ports() {
    let _ = std::any::TypeId::of::<super::validation_evaluator::ValidationEvaluator>();
    let _ = std::any::TypeId::of::<super::validation_store::ValidationStoreError>();
}
