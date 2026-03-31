use octopus_core::{default_last_visited_route, default_preferences};

#[test]
fn default_route_uses_workspace_dashboard_and_project_query() {
  let route = default_last_visited_route("ws-local", "proj-redesign");

  assert_eq!(route, "/workspaces/ws-local/dashboard?project=proj-redesign");
}

#[test]
fn default_preferences_preserve_workspace_and_project_context() {
  let preferences = default_preferences("ws-enterprise", "proj-launch");

  assert_eq!(preferences.default_workspace_id, "ws-enterprise");
  assert_eq!(
    preferences.last_visited_route,
    "/workspaces/ws-enterprise/dashboard?project=proj-launch"
  );
  assert_eq!(preferences.locale, "zh-CN");
  assert_eq!(preferences.theme, "system");
}
