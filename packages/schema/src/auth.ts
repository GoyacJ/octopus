import type {
  AvatarUploadPayload as OpenApiAvatarUploadPayload,
  LoginRequest as OpenApiLoginRequest,
  LoginResponse as OpenApiLoginResponse,
  RegisterWorkspaceOwnerRequest as OpenApiRegisterWorkspaceOwnerRequest,
  RegisterWorkspaceOwnerResponse as OpenApiRegisterWorkspaceOwnerResponse,
  SessionRecord as OpenApiSessionRecord,
} from './generated'

export type LoginRequest = OpenApiLoginRequest
export type AvatarUploadPayload = OpenApiAvatarUploadPayload
export type RegisterWorkspaceOwnerRequest = OpenApiRegisterWorkspaceOwnerRequest
export type SessionRecord = OpenApiSessionRecord
export type LoginResponse = OpenApiLoginResponse
export type RegisterWorkspaceOwnerResponse = OpenApiRegisterWorkspaceOwnerResponse
