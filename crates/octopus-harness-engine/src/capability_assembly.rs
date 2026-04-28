use std::sync::Arc;

use harness_contracts::{BlobStore, CapabilityRegistry, ToolCapability};

pub(crate) fn assemble_capability_registry(
    base: Option<&Arc<CapabilityRegistry>>,
    blob_store: Option<&Arc<dyn BlobStore>>,
    overrides: &CapabilityRegistry,
) -> Arc<CapabilityRegistry> {
    let mut registry = base.map_or_else(CapabilityRegistry::default, |base| base.as_ref().clone());

    if let Some(blob_store) = blob_store {
        registry.install::<dyn BlobStore>(ToolCapability::BlobReader, Arc::clone(blob_store));
    }

    registry.overlay_from(overrides);
    Arc::new(registry)
}
