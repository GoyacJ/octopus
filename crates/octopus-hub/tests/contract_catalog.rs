use octopus_hub::contracts::{
    approval_type_values, contract_catalog, run_status_values, run_type_values, CoreObjectContract,
    EnumCatalog, EventSkeleton,
};
use serde::Deserialize;

const CORE_OBJECTS_JSON: &str = include_str!("../../../contracts/v1/core-objects.json");
const ENUMS_JSON: &str = include_str!("../../../contracts/v1/enums.json");
const EVENTS_JSON: &str = include_str!("../../../contracts/v1/events.json");

#[derive(Debug, Deserialize)]
struct CoreObjectFile {
    objects: Vec<CoreObjectContract>,
}

#[derive(Debug, Deserialize)]
struct EnumFile {
    enums: EnumCatalog,
}

#[derive(Debug, Deserialize)]
struct EventFile {
    events: Vec<EventSkeleton>,
}

#[test]
fn exposes_the_canonical_runtime_enums() {
    let catalog = contract_catalog().expect("contract catalog should load");
    let enum_file: EnumFile = serde_json::from_str(ENUMS_JSON).expect("enum catalog should parse");

    assert_eq!(catalog.enums.run_type, run_type_values());
    assert_eq!(catalog.enums.run_status, run_status_values());
    assert_eq!(catalog.enums.approval_type, approval_type_values());
    assert_eq!(catalog.enums, enum_file.enums);
}

#[test]
fn exposes_the_canonical_core_object_set() {
    let catalog = contract_catalog().expect("contract catalog should load");
    let core_object_file: CoreObjectFile =
        serde_json::from_str(CORE_OBJECTS_JSON).expect("core object catalog should parse");
    let event_file: EventFile =
        serde_json::from_str(EVENTS_JSON).expect("event catalog should parse");

    assert_eq!(catalog.core_objects, core_object_file.objects);
    assert_eq!(catalog.events, event_file.events);
}
