use crate::features::validation::application::validation_store::ValidationStore;

#[test]
fn infrastructure_registers_the_in_memory_store() {
    let store = super::in_memory_store::InMemoryValidationStore::new();
    assert!(store.records().is_empty());
}
