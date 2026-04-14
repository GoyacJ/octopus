use super::*;

pub(crate) fn summarize_auth_state(
    provider_state_summary: &[RuntimeCapabilityProviderState],
    pending_auth_challenge: Option<&RuntimeAuthChallengeSummary>,
) -> RuntimeAuthStateSummary {
    let mut summary = RuntimeAuthStateSummary::default();
    for provider in provider_state_summary {
        match provider.state.as_str() {
            "auth_required" => {
                if !summary.challenged_provider_keys.contains(&provider.provider_key) {
                    summary
                        .challenged_provider_keys
                        .push(provider.provider_key.clone());
                }
            }
            "degraded" | "unavailable" => {
                if !summary.failed_provider_keys.contains(&provider.provider_key) {
                    summary
                        .failed_provider_keys
                        .push(provider.provider_key.clone());
                }
            }
            _ => {}
        }
    }

    if let Some(challenge) = pending_auth_challenge {
        if let Some(provider_key) = challenge.provider_key.as_ref() {
            if !summary.challenged_provider_keys.contains(provider_key) {
                summary.challenged_provider_keys.push(provider_key.clone());
            }
        }
        summary.last_challenge_at = Some(challenge.created_at);
    }

    summary.pending_challenge_count = summary.challenged_provider_keys.len() as u64;
    summary
}
