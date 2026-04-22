    fn runtime() -> tokio::runtime::Runtime {
        tokio::runtime::Runtime::new().expect("runtime")
    }

    fn encoded_file(
        relative_path: &str,
        content_type: &str,
        content: &str,
    ) -> octopus_core::WorkspaceResourceFolderUploadEntry {
        octopus_core::WorkspaceResourceFolderUploadEntry {
            relative_path: relative_path.into(),
            file_name: Path::new(relative_path)
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or(relative_path)
                .into(),
            content_type: content_type.into(),
            data_base64: BASE64_STANDARD.encode(content.as_bytes()),
            byte_size: content.len() as u64,
        }
    }

    fn avatar_payload() -> octopus_core::AvatarUploadPayload {
        octopus_core::AvatarUploadPayload {
            content_type: "image/png".into(),
            data_base64: "iVBORw0KGgo=".into(),
            file_name: "avatar.png".into(),
            byte_size: 8,
        }
    }

    fn bootstrap_admin_session(bundle: &crate::InfraBundle) -> SessionRecord {
        runtime()
            .block_on(
                bundle
                    .auth
                    .register_bootstrap_admin(RegisterBootstrapAdminRequest {
                        client_app_id: "octopus-desktop".into(),
                        username: "owner".into(),
                        display_name: "Owner".into(),
                        password: "password123".into(),
                        confirm_password: "password123".into(),
                        avatar: avatar_payload(),
                        workspace_id: Some("ws-local".into()),
                        mapped_directory: None,
                    }),
            )
            .expect("bootstrap admin")
            .session
    }

    fn create_user_session(
        bundle: &crate::InfraBundle,
        username: &str,
        display_name: &str,
    ) -> SessionRecord {
        runtime().block_on(async {
            bundle
                .access_control
                .create_user(AccessUserUpsertRequest {
                    username: username.into(),
                    display_name: display_name.into(),
                    status: "active".into(),
                    password: Some("password123".into()),
                    confirm_password: Some("password123".into()),
                    reset_password: Some(false),
                })
                .await
                .expect("create user");

            bundle
                .auth
                .login(LoginRequest {
                    client_app_id: "octopus-desktop".into(),
                    username: username.into(),
                    password: "password123".into(),
                    workspace_id: Some("ws-local".into()),
                })
                .await
                .expect("login user")
                .session
        })
    }

