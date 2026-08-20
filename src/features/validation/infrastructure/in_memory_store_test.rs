use crate::features::validation::application::validation_store::ValidationStore;

#[test]
fn new_store_has_no_records() {
    let store = super::InMemoryValidationStore::new();
    assert!(store.records().is_empty());
}
