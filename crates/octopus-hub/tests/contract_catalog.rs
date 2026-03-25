use octopus_hub::contracts::{contract_catalog, approval_type_values, run_status_values, run_type_values};

#[test]
fn exposes_the_canonical_runtime_enums() {
    let catalog = contract_catalog().expect("contract catalog should load");

    assert_eq!(catalog.enums.run_type, run_type_values());
    assert_eq!(catalog.enums.run_status, run_status_values());
    assert_eq!(catalog.enums.approval_type, approval_type_values());
}

#[test]
fn exposes_the_canonical_core_object_set() {
    let catalog = contract_catalog().expect("contract catalog should load");

    assert!(catalog.core_objects.iter().any(|entry| entry.name == "Run"));
    assert!(catalog.core_objects.iter().any(|entry| entry.name == "ApprovalRequest"));
    assert!(catalog.events.iter().any(|entry| entry.name == "RunStateChanged"));
}

