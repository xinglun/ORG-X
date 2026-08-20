use org_x::features::validation::application::validation_store::{
    ValidationStore, ValidationStoreError,
};
use org_x::features::validation::domain::{ValidationBaseline, ValidationRecord};
use org_x::features::validation::infrastructure::in_memory_store::InMemoryValidationStore;

fn record(company_id: &str) -> ValidationRecord {
    ValidationRecord::new(
        ValidationBaseline::new(
            company_id,
            "STAGE_2",
            Vec::new(),
            vec!["hypothesis"],
            Vec::new(),
            vec!["missing proof"],
            Vec::new(),
        )
        .unwrap(),
    )
    .unwrap()
}

#[test]
fn in_memory_store_preserves_order_and_rejects_overwrite() {
    let mut store = InMemoryValidationStore::new();
    store.save(record("company-2")).unwrap();
    store.save(record("company-1")).unwrap();

    assert_eq!(
        store
            .records()
            .iter()
            .map(|item| item.company_id())
            .collect::<Vec<_>>(),
        ["company-2", "company-1"]
    );
    assert_eq!(store.get("company-1").unwrap().company_id(), "company-1");

    assert_eq!(
        store.save(record("company-2")),
        Err(ValidationStoreError::DuplicateCompany {
            company_id: "company-2".into(),
        })
    );
    assert_eq!(store.records().len(), 2);
}
