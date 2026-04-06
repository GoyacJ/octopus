export interface ClientAppRecord {
  id: string
  name: string
  platform: 'desktop' | 'web' | 'mobile'
  status: 'active' | 'disabled'
  firstParty: boolean
  allowedOrigins: string[]
  allowedHosts: string[]
  sessionPolicy: string
  defaultScopes: string[]
}
