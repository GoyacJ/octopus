use octopus_desktop_host::{
    MemoryRemoteSessionSecretStore, PersistedRemoteSession, RemoteSessionCacheLoadRequest,
    RemoteSessionCacheService,
};

fn fixture_session() -> PersistedRemoteSession {
    PersistedRemoteSession {
        base_url: "http://127.0.0.1:4000".into(),
        workspace_id: "workspace-alpha".into(),
        email: "admin@octopus.local".into(),
        access_token: "remote-token".into(),
        refresh_token: "refresh-token".into(),
        refresh_token_expires_at: "2099-04-05T12:00:00Z".into(),
        session: octopus_desktop_host::PersistedHubSession {
            session_id: "session-1".into(),
            user_id: "user-1".into(),
            email: "admin@octopus.local".into(),
            workspace_id: "workspace-alpha".into(),
            actor_ref: "workspace_admin:bootstrap_admin".into(),
            issued_at: "2026-03-29T10:00:00Z".into(),
            expires_at: "2099-03-29T12:00:00Z".into(),
        },
    }
}

fn matching_request() -> RemoteSessionCacheLoadRequest {
    RemoteSessionCacheLoadRequest {
        base_url: "http://127.0.0.1:4000".into(),
        workspace_id: "workspace-alpha".into(),
        email: "admin@octopus.local".into(),
    }
}

#[test]
fn save_load_clear_round_trip_works() {
    let store = MemoryRemoteSessionSecretStore::default();
    let service = RemoteSessionCacheService::new(store.clone());

    let save = service.save(&fixture_session()).unwrap();
    assert!(save.storage_available);
    assert!(save.warning.is_none());

    let loaded = service.load(&matching_request()).unwrap();
    assert_eq!(loaded.session, Some(fixture_session()));

    let cleared = service.clear().unwrap();
    assert!(cleared.storage_available);
    assert!(cleared.warning.is_none());
    assert_eq!(store.stored_secret(), None);
}

#[test]
fn load_clears_expired_session_cache() {
    let store = MemoryRemoteSessionSecretStore::default();
    let service = RemoteSessionCacheService::new(store.clone());
    let mut expired = fixture_session();
    expired.session.expires_at = "2020-03-29T12:00:00Z".into();

    service.save(&expired).unwrap();

    let loaded = service.load(&matching_request()).unwrap();
    assert_eq!(loaded.session, Some(expired));
    assert_eq!(store.stored_secret().is_some(), true);
}

#[test]
fn load_clears_expired_refresh_session_cache() {
    let store = MemoryRemoteSessionSecretStore::default();
    let service = RemoteSessionCacheService::new(store.clone());
    let mut expired = fixture_session();
    expired.refresh_token_expires_at = "2020-04-05T12:00:00Z".into();

    service.save(&expired).unwrap();

    let loaded = service.load(&matching_request()).unwrap();
    assert_eq!(loaded.session, None);
    assert_eq!(store.stored_secret(), None);
}

#[test]
fn load_clears_profile_binding_mismatches() {
    let store = MemoryRemoteSessionSecretStore::default();
    let service = RemoteSessionCacheService::new(store.clone());

    service.save(&fixture_session()).unwrap();

    let loaded = service
        .load(&RemoteSessionCacheLoadRequest {
            base_url: "http://127.0.0.1:4000".into(),
            workspace_id: "workspace-beta".into(),
            email: "admin@octopus.local".into(),
        })
        .unwrap();

    assert_eq!(loaded.session, None);
    assert_eq!(store.stored_secret(), None);
}

#[test]
fn load_clears_corrupted_cache_payloads() {
    let store = MemoryRemoteSessionSecretStore::default();
    store.set_raw_secret("{not-json");
    let service = RemoteSessionCacheService::new(store.clone());

    let loaded = service.load(&matching_request()).unwrap();

    assert_eq!(loaded.session, None);
    assert_eq!(store.stored_secret(), None);
}
