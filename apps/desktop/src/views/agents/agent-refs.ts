export function canonicalAgentRef(value?: string | null) {
  const trimmed = value?.trim() ?? ''
  if (!trimmed) {
    return ''
  }
  return trimmed.includes(':') ? trimmed : `agent:${trimmed}`
}

export function agentIdFromRef(value?: string | null) {
  const canonical = canonicalAgentRef(value)
  if (!canonical) {
    return ''
  }
  return canonical.startsWith('agent:') ? canonical.slice('agent:'.length) : canonical
}

export function matchesAgentRef(agentId: string, actorRef?: string | null) {
  return canonicalAgentRef(agentId) === canonicalAgentRef(actorRef)
}
