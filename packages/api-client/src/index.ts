export const controlPlaneSpecPath = 'proto/openapi/control-plane.v1.yaml'

export interface ApiClientConfig {
  baseUrl: string
}

export function createApiClient(config: ApiClientConfig) {
  return {
    config,
  }
}
