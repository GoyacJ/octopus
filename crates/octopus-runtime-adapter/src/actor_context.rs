use super::*;

#[allow(dead_code)]
pub(super) fn resolve_actor_label(
    paths: &WorkspacePaths,
    actor_kind: Option<&str>,
    actor_id: Option<&str>,
) -> Option<String> {
    let actor_id = actor_id?.trim();
    if actor_id.is_empty() {
        return None;
    }
    let connection = Connection::open(&paths.db_path).ok()?;
    match actor_kind.unwrap_or_default() {
        "team" => connection
            .query_row(
                "SELECT name FROM teams WHERE id = ?1",
                params![actor_id],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .ok()
            .flatten()
            .map(|name| format!("{} · Team", name)),
        "agent" => connection
            .query_row(
                "SELECT name FROM agents WHERE id = ?1",
                params![actor_id],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .ok()
            .flatten()
            .map(|name| format!("{} · Agent", name)),
        _ => Some(actor_id.to_string()),
    }
}

pub(super) fn build_actor_system_prompt(
    sections: impl IntoIterator<Item = Option<String>>,
) -> Option<String> {
    let sections = sections
        .into_iter()
        .flatten()
        .map(|section| section.trim().to_string())
        .filter(|section| !section.is_empty())
        .collect::<Vec<_>>();
    if sections.is_empty() {
        None
    } else {
        Some(sections.join("\n\n"))
    }
}

#[allow(dead_code)]
pub(super) fn resolve_actor_system_prompt(
    paths: &WorkspacePaths,
    actor_kind: Option<&str>,
    actor_id: Option<&str>,
) -> Option<String> {
    let actor_id = actor_id?.trim();
    if actor_id.is_empty() {
        return None;
    }
    let connection = Connection::open(&paths.db_path).ok()?;
    match actor_kind.unwrap_or_default() {
        "agent" => connection
            .query_row(
                "SELECT name, personality, prompt FROM agents WHERE id = ?1",
                params![actor_id],
                |row| {
                    let name: String = row.get(0)?;
                    let personality: String = row.get(1)?;
                    let prompt: String = row.get(2)?;
                    Ok(build_actor_system_prompt(vec![
                        Some(format!("You are the agent `{name}`.")),
                        (!personality.trim().is_empty())
                            .then(|| format!("Personality: {personality}")),
                        (!prompt.trim().is_empty()).then(|| format!("Instructions: {prompt}")),
                    ]))
                },
            )
            .optional()
            .ok()
            .flatten()
            .flatten(),
        "team" => connection
            .query_row(
                "SELECT name, personality, prompt, leader_ref, member_refs FROM teams WHERE id = ?1",
                params![actor_id],
                |row| {
                    let name: String = row.get(0)?;
                    let personality: String = row.get(1)?;
                    let prompt: String = row.get(2)?;
                    let leader_ref: String = row.get(3)?;
                    let member_refs_raw: String = row.get(4)?;
                    let member_refs =
                        serde_json::from_str::<Vec<String>>(&member_refs_raw).unwrap_or_default();
                    Ok(build_actor_system_prompt(vec![
                        Some(format!(
                            "You are the team `{name}` operating as a single execution actor."
                        )),
                        (!personality.trim().is_empty())
                            .then(|| format!("Team personality: {personality}")),
                        (!prompt.trim().is_empty())
                            .then(|| format!("Team instructions: {prompt}")),
                        (!leader_ref.trim().is_empty())
                            .then(|| format!("Leader ref: {leader_ref}")),
                        (!member_refs.is_empty())
                            .then(|| format!("Member refs: {}", member_refs.join(", "))),
                    ]))
                },
            )
            .optional()
            .ok()
            .flatten()
            .flatten(),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::build_actor_system_prompt;

    #[test]
    fn builds_prompt_sections() {
        let prompt = build_actor_system_prompt([
            Some(" Alpha ".to_string()),
            None,
            Some(String::new()),
            Some("Beta".to_string()),
        ]);

        assert_eq!(prompt.as_deref(), Some("Alpha\n\nBeta"));
    }
}
