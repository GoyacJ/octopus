import {
  buildGithubReleasesUrl,
  normalizeGithubReleases,
  type GithubReleaseApiResponse,
  type NormalizedRelease,
} from '../utils/github-releases'

const cacheTtlMs = 5 * 60 * 1000
const cacheKeyPrefix = 'octopus.website.github-releases'

interface GithubReleaseCachePayload {
  fetchedAt: number
  releases: NormalizedRelease[]
}

function readCachedReleases(cacheKey: string) {
  if (!import.meta.client) {
    return null
  }

  const cachedValue = sessionStorage.getItem(cacheKey)
  if (!cachedValue) {
    return null
  }

  try {
    const parsed = JSON.parse(cachedValue) as GithubReleaseCachePayload
    if (Date.now() - parsed.fetchedAt > cacheTtlMs) {
      sessionStorage.removeItem(cacheKey)
      return null
    }

    return parsed.releases
  } catch {
    sessionStorage.removeItem(cacheKey)
    return null
  }
}

function writeCachedReleases(cacheKey: string, releases: NormalizedRelease[]) {
  if (!import.meta.client) {
    return
  }

  const payload: GithubReleaseCachePayload = {
    fetchedAt: Date.now(),
    releases,
  }

  sessionStorage.setItem(cacheKey, JSON.stringify(payload))
}

export function useGithubReleases() {
  const runtimeConfig = useRuntimeConfig()
  const releases = useState<NormalizedRelease[]>('website.github-releases.data', () => [])
  const pending = useState<boolean>('website.github-releases.pending', () => true)
  const error = useState<string | null>('website.github-releases.error', () => null)
  const loadedAt = useState<number | null>('website.github-releases.loaded-at', () => null)

  const owner = computed(() => runtimeConfig.public.githubRepoOwner as string)
  const repo = computed(() => runtimeConfig.public.githubRepoName as string)
  const apiBase = computed(() => (runtimeConfig.public.githubApiBase as string).replace(/\/+$/, ''))
  const releasesUrl = computed(() => buildGithubReleasesUrl(owner.value, repo.value))
  const apiUrl = computed(() => `${apiBase.value}/repos/${owner.value}/${repo.value}/releases?per_page=12`)
  const cacheKey = computed(() => `${cacheKeyPrefix}:${owner.value}:${repo.value}`)

  async function load(force = false) {
    if (import.meta.server) {
      pending.value = false
      return
    }

    if (!force) {
      const cachedReleases = readCachedReleases(cacheKey.value)
      if (cachedReleases) {
        releases.value = cachedReleases
        pending.value = false
        error.value = null
        loadedAt.value = Date.now()
        return
      }
    }

    pending.value = true

    try {
      const payload = await $fetch<GithubReleaseApiResponse[]>(apiUrl.value, {
        headers: {
          accept: 'application/vnd.github+json',
        },
      })

      const normalizedReleases = normalizeGithubReleases(payload)
      releases.value = normalizedReleases
      error.value = null
      loadedAt.value = Date.now()
      writeCachedReleases(cacheKey.value, normalizedReleases)
    } catch (cause) {
      error.value = cause instanceof Error ? cause.message : 'Failed to fetch GitHub releases'
      if (releases.value.length === 0) {
        releases.value = []
      }
    } finally {
      pending.value = false
    }
  }

  onMounted(() => {
    if (loadedAt.value && Date.now() - loadedAt.value < cacheTtlMs && releases.value.length > 0) {
      pending.value = false
      return
    }

    void load()
  })

  return {
    releases,
    pending,
    error,
    releasesUrl,
    load,
  }
}
