export interface LoginResponse {
  token: string
  expires_at: number
}

export interface ErrorResponse {
  success: false
  error: string
}

export interface FileEntry {
  name: string
  path: string
  is_dir: boolean
  is_hidden: boolean
  is_symlink: boolean
  size: number | null
}

export interface ListDirResponse {
  entries: FileEntry[]
  current_path: string
}

export interface AuthStepNextStep {
  status: 'next_step'
  session_id: string
  next_step: string
}

export interface AuthStepComplete {
  status: 'complete'
  token: string
  expires_at: number
}

export type AuthStepResponse = AuthStepNextStep | AuthStepComplete
