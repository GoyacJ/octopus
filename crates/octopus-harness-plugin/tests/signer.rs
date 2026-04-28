use std::sync::Arc;

use async_trait::async_trait;
use chrono::{TimeZone, Utc};
use harness_contracts::TrustLevel;
use harness_plugin::{
    DiscoverySource, ManifestLoaderError, ManifestOrigin, ManifestRecord, ManifestSignature,
    ManifestSigner, PluginCapabilities, PluginError, PluginManifest, PluginManifestLoader,
    PluginName, PluginRegistry, SignatureAlgorithm, SignerId, SignerProvenance,
    StaticTrustedSignerStore, TrustedSigner,
};
use ring::signature::{Ed25519KeyPair, KeyPair};

#[tokio::test]
async fn admin_trusted_manifest_without_signature_is_rejected() {
    let registry = PluginRegistry::builder()
        .with_manifest_loader(Arc::new(StaticManifestLoader::new(vec![record(manifest(
            "unsigned-admin",
            TrustLevel::AdminTrusted,
        ))])))
        .build()
        .unwrap();

    let error = registry.discover().await.unwrap_err();

    assert!(matches!(error, PluginError::SignatureInvalid { .. }));
}

#[tokio::test]
async fn admin_trusted_manifest_signed_by_unknown_signer_is_rejected() {
    let keypair = keypair();
    let signed_at = Utc.with_ymd_and_hms(2026, 4, 27, 0, 0, 0).unwrap();
    let manifest = signed_manifest(
        manifest("unknown-signer", TrustLevel::AdminTrusted),
        &keypair,
        "acme-prod-r1",
        signed_at,
        SignatureAlgorithm::Ed25519,
    );
    let registry = PluginRegistry::builder()
        .with_manifest_loader(Arc::new(StaticManifestLoader::new(vec![record(manifest)])))
        .build()
        .unwrap();

    let error = registry.discover().await.unwrap_err();

    assert_eq!(error, PluginError::UnknownSigner("acme-prod-r1".to_owned()));
}

#[tokio::test]
async fn admin_trusted_manifest_rejects_algorithm_mismatch_and_bad_windows() {
    let keypair = keypair();
    let signer = trusted_signer(
        "acme-prod-r1",
        &keypair,
        SignatureAlgorithm::RsaPkcs1Sha256,
        None,
    );
    let store = Arc::new(StaticTrustedSignerStore::new(vec![signer]).unwrap());
    let signed_at = Utc.with_ymd_and_hms(2026, 4, 27, 0, 0, 0).unwrap();
    let registry = PluginRegistry::builder()
        .with_signer_store(store)
        .with_manifest_loader(Arc::new(StaticManifestLoader::new(vec![record(
            signed_manifest(
                manifest("algorithm-mismatch", TrustLevel::AdminTrusted),
                &keypair,
                "acme-prod-r1",
                signed_at,
                SignatureAlgorithm::Ed25519,
            ),
        )])))
        .build()
        .unwrap();

    let error = registry.discover().await.unwrap_err();
    assert!(matches!(error, PluginError::SignatureInvalid { .. }));

    let signer = trusted_signer("acme-prod-r2", &keypair, SignatureAlgorithm::Ed25519, None)
        .with_window(
            Utc.with_ymd_and_hms(2026, 5, 1, 0, 0, 0).unwrap(),
            Some(Utc.with_ymd_and_hms(2026, 6, 1, 0, 0, 0).unwrap()),
        );
    let store = Arc::new(StaticTrustedSignerStore::new(vec![signer]).unwrap());
    let registry = PluginRegistry::builder()
        .with_signer_store(store)
        .with_manifest_loader(Arc::new(StaticManifestLoader::new(vec![record(
            signed_manifest(
                manifest("too-early", TrustLevel::AdminTrusted),
                &keypair,
                "acme-prod-r2",
                signed_at,
                SignatureAlgorithm::Ed25519,
            ),
        )])))
        .build()
        .unwrap();

    let error = registry.discover().await.unwrap_err();
    assert!(matches!(error, PluginError::SignatureInvalid { .. }));
}

#[tokio::test]
async fn revoked_signer_rejects_manifest_even_when_signature_is_valid() {
    let keypair = keypair();
    let revoked_at = Utc.with_ymd_and_hms(2026, 4, 27, 1, 0, 0).unwrap();
    let signer = trusted_signer(
        "acme-revoked-r1",
        &keypair,
        SignatureAlgorithm::Ed25519,
        Some(revoked_at),
    );
    let store = Arc::new(StaticTrustedSignerStore::new(vec![signer]).unwrap());
    let registry = PluginRegistry::builder()
        .with_signer_store(store)
        .with_manifest_loader(Arc::new(StaticManifestLoader::new(vec![record(
            signed_manifest(
                manifest("revoked", TrustLevel::AdminTrusted),
                &keypair,
                "acme-revoked-r1",
                Utc.with_ymd_and_hms(2026, 4, 27, 0, 0, 0).unwrap(),
                SignatureAlgorithm::Ed25519,
            ),
        )])))
        .build()
        .unwrap();

    let error = registry.discover().await.unwrap_err();

    assert_eq!(
        error,
        PluginError::SignerRevoked {
            signer: "acme-revoked-r1".to_owned(),
            revoked_at,
        }
    );
}

#[tokio::test]
async fn valid_admin_trusted_signature_passes_discovery() {
    let keypair = keypair();
    let signer = trusted_signer("acme-prod-r1", &keypair, SignatureAlgorithm::Ed25519, None);
    let store = Arc::new(StaticTrustedSignerStore::new(vec![signer]).unwrap());
    let registry = PluginRegistry::builder()
        .with_signer_store(store)
        .with_manifest_loader(Arc::new(StaticManifestLoader::new(vec![record(
            signed_manifest(
                manifest("signed-admin", TrustLevel::AdminTrusted),
                &keypair,
                "acme-prod-r1",
                Utc.with_ymd_and_hms(2026, 4, 27, 0, 0, 0).unwrap(),
                SignatureAlgorithm::Ed25519,
            ),
        )])))
        .build()
        .unwrap();

    let discovered = registry.discover().await.unwrap();

    assert_eq!(discovered.len(), 1);
    assert_eq!(
        discovered[0].record.manifest.plugin_id().0,
        "signed-admin@0.1.0"
    );
}

#[tokio::test]
async fn user_controlled_signature_does_not_upgrade_trust_level() {
    let keypair = keypair();
    let signer = trusted_signer("acme-prod-r1", &keypair, SignatureAlgorithm::Ed25519, None);
    let store = Arc::new(StaticTrustedSignerStore::new(vec![signer]).unwrap());
    let registry = PluginRegistry::builder()
        .with_signer_store(store)
        .with_manifest_loader(Arc::new(StaticManifestLoader::new(vec![record(
            signed_manifest(
                manifest("signed-user", TrustLevel::UserControlled),
                &keypair,
                "acme-prod-r1",
                Utc.with_ymd_and_hms(2026, 4, 27, 0, 0, 0).unwrap(),
                SignatureAlgorithm::Ed25519,
            ),
        )])))
        .build()
        .unwrap();

    let discovered = registry.discover().await.unwrap();

    assert_eq!(
        discovered[0].record.manifest.trust_level,
        TrustLevel::UserControlled
    );
}

#[tokio::test]
async fn static_signer_store_tracks_active_and_revoked_signers() {
    let keypair = keypair();
    let revoked_at = Utc.with_ymd_and_hms(2026, 4, 27, 0, 0, 0).unwrap();
    let store = StaticTrustedSignerStore::new(vec![
        trusted_signer(
            "acme-active-r1",
            &keypair,
            SignatureAlgorithm::Ed25519,
            None,
        ),
        trusted_signer(
            "acme-revoked-r1",
            &keypair,
            SignatureAlgorithm::Ed25519,
            Some(revoked_at),
        ),
    ])
    .unwrap();

    let active = store.list_active().await.unwrap();

    assert_eq!(active.len(), 1);
    assert_eq!(active[0].id.as_str(), "acme-active-r1");
    assert!(store
        .is_revoked(&SignerId::new("acme-revoked-r1").unwrap(), Utc::now())
        .await
        .unwrap());
    assert!(store
        .get(&SignerId::new("acme-active-r1").unwrap())
        .await
        .unwrap()
        .is_some());
}

#[test]
fn signer_store_and_trusted_signer_builder_modes_are_mutually_exclusive() {
    let keypair = keypair();
    let store = Arc::new(
        StaticTrustedSignerStore::new(vec![trusted_signer(
            "acme-prod-r1",
            &keypair,
            SignatureAlgorithm::Ed25519,
            None,
        )])
        .unwrap(),
    );

    let error = PluginRegistry::builder()
        .with_signer_store(store)
        .with_trusted_signer(keypair.public_key().as_ref().to_vec())
        .build()
        .unwrap_err();

    assert!(matches!(error, PluginError::Builder(_)));
}

struct StaticManifestLoader {
    records: Vec<ManifestRecord>,
}

impl StaticManifestLoader {
    fn new(records: Vec<ManifestRecord>) -> Self {
        Self { records }
    }
}

#[async_trait]
impl PluginManifestLoader for StaticManifestLoader {
    async fn enumerate(
        &self,
        _source: &DiscoverySource,
    ) -> Result<Vec<ManifestRecord>, ManifestLoaderError> {
        Ok(self.records.clone())
    }
}

fn keypair() -> Ed25519KeyPair {
    let pkcs8 = Ed25519KeyPair::generate_pkcs8(&ring::rand::SystemRandom::new()).unwrap();
    Ed25519KeyPair::from_pkcs8(pkcs8.as_ref()).unwrap()
}

fn trusted_signer(
    id: &str,
    keypair: &Ed25519KeyPair,
    algorithm: SignatureAlgorithm,
    revoked_at: Option<chrono::DateTime<Utc>>,
) -> TrustedSigner {
    TrustedSigner {
        id: SignerId::new(id).unwrap(),
        algorithm,
        public_key: keypair.public_key().as_ref().to_vec(),
        activated_at: Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
        retired_at: None,
        revoked_at,
        provenance: SignerProvenance::BuilderInjected,
    }
}

trait TrustedSignerTestExt {
    fn with_window(
        self,
        activated_at: chrono::DateTime<Utc>,
        retired_at: Option<chrono::DateTime<Utc>>,
    ) -> Self;
}

impl TrustedSignerTestExt for TrustedSigner {
    fn with_window(
        mut self,
        activated_at: chrono::DateTime<Utc>,
        retired_at: Option<chrono::DateTime<Utc>>,
    ) -> Self {
        self.activated_at = activated_at;
        self.retired_at = retired_at;
        self
    }
}

fn signed_manifest(
    mut manifest: PluginManifest,
    keypair: &Ed25519KeyPair,
    signer: &str,
    signed_at: chrono::DateTime<Utc>,
    algorithm: SignatureAlgorithm,
) -> PluginManifest {
    let payload = ManifestSigner::canonical_payload(&manifest).unwrap();
    let signature = keypair.sign(&payload);
    manifest.signature = Some(ManifestSignature {
        algorithm,
        signer: signer.to_owned(),
        signature: signature.as_ref().to_vec(),
        timestamp: signed_at.to_rfc3339(),
    });
    manifest
}

fn record(manifest: PluginManifest) -> ManifestRecord {
    ManifestRecord::new(
        manifest,
        ManifestOrigin::File {
            path: "/plugins/plugin.json".into(),
        },
        [9; 32],
    )
    .unwrap()
}

fn manifest(name: &str, trust_level: TrustLevel) -> PluginManifest {
    PluginManifest {
        manifest_schema_version: 1,
        name: PluginName::new(name).unwrap(),
        version: "0.1.0".to_owned(),
        trust_level,
        description: None,
        authors: Vec::new(),
        repository: None,
        signature: None,
        capabilities: PluginCapabilities::default(),
        dependencies: Vec::new(),
        min_harness_version: ">=0.0.0".to_owned(),
    }
}
