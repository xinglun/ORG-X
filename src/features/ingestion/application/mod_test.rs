use super::*;

#[test]
fn source_collection_port_is_implementable_without_external_runtime() {
    struct EmptySource;

    impl ObservationSource for EmptySource {
        fn collect(
            &self,
            _request: &IngestionRequest,
        ) -> Result<Vec<Observation>, SourceCollectionError> {
            Ok(Vec::new())
        }
    }

    let source = EmptySource;
    assert!(source.collect(&IngestionRequest::new("research")).is_ok());
}
