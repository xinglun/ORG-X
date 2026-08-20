#[test]
fn store_error_is_typed() {
    let error = super::ValidationStoreError::DuplicateCompany {
        company_id: "company-1".into(),
    };
    assert!(error.to_string().contains("company-1"));
}
