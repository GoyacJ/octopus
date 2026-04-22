#[cfg(test)]
mod tests {
    use crate::project_module_for_request;
    use octopus_core::AuthorizationRequest;

    #[test]
    fn project_module_for_request_routes_task_capabilities_and_resources_to_tasks_module() {
        let capability_request = AuthorizationRequest {
            subject_id: "user-owner".into(),
            capability: "task.view".into(),
            project_id: Some("proj-redesign".into()),
            resource_type: None,
            resource_id: None,
            resource_subtype: None,
            tags: Vec::new(),
            classification: None,
            owner_subject_type: None,
            owner_subject_id: None,
        };
        assert_eq!(
            project_module_for_request(&capability_request),
            Some("tasks")
        );

        let resource_request = AuthorizationRequest {
            capability: "project.view".into(),
            resource_type: Some("task".into()),
            resource_id: Some("task-1".into()),
            ..capability_request
        };
        assert_eq!(project_module_for_request(&resource_request), Some("tasks"));
    }
}
