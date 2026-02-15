import type { AuthStepResponse, ListDirResponse } from './types'

let authToken: string | null = null

export function setToken(token: string | null) {
  authToken = token
}

export function getToken(): string | null {
  return authToken
}

async function request<T>(path: string, options: RequestInit = {}): Promise<T> {
  const headers: Record<string, string> = {
    'Content-Type': 'application/json',
    ...(options.headers as Record<string, string> || {}),
  }

  if (authToken) {
    headers['Authorization'] = `Bearer ${authToken}`
  }

  const res = await fetch(path, { ...options, headers })

  if (res.status === 401) {
    const body = await res.json().catch(() => ({ error: 'Unauthorized' }))
    // Only clear stored token if we had one (not during auth flow)
    if (authToken) {
      authToken = null
      sessionStorage.removeItem('trefm_token')
    }
    throw new Error(body.error || 'Unauthorized')
  }

  if (!res.ok) {
    const body = await res.json().catch(() => ({ error: res.statusText }))
    throw new Error(body.error || res.statusText)
  }

  return res.json()
}

export async function login(username: string, password: string): Promise<AuthStepResponse> {
  return request('/api/auth/login', {
    method: 'POST',
    body: JSON.stringify({ username, password }),
  })
}

export async function listDirectory(path?: string): Promise<ListDirResponse> {
  const params = path ? `?path=${encodeURIComponent(path)}` : ''
  return request(`/api/files${params}`)
}

export async function getWebAuthnChallenge(sessionId: string): Promise<any> {
  return request('/api/auth/webauthn/challenge', {
    method: 'POST',
    body: JSON.stringify({ session_id: sessionId }),
  })
}

export async function verifyWebAuthn(sessionId: string, credential: any): Promise<AuthStepResponse> {
  return request('/api/auth/webauthn/verify', {
    method: 'POST',
    body: JSON.stringify({ session_id: sessionId, credential }),
  })
}

export async function verifyOtp(sessionId: string, code: string): Promise<AuthStepResponse> {
  return request('/api/auth/otp/verify', {
    method: 'POST',
    body: JSON.stringify({ session_id: sessionId, code }),
  })
}

export async function startPasskeyRegistration(): Promise<any> {
  return request('/api/auth/webauthn/register/start', { method: 'POST' })
}

export async function finishPasskeyRegistration(sessionId: string, credential: any): Promise<any> {
  return request('/api/auth/webauthn/register/finish', {
    method: 'POST',
    body: JSON.stringify({ session_id: sessionId, credential }),
  })
}

export async function createWsTicket(): Promise<{ ticket: string }> {
  return request('/api/ws/ticket', { method: 'POST' })
}

export async function logout(): Promise<void> {
  try {
    await request('/api/auth/logout', { method: 'POST' })
  } catch {
    // Ignore errors — token may already be invalid
  }
}

export function sendBeaconLogout(): void {
  if (!authToken) return
  const blob = new Blob(
    [JSON.stringify({ token: authToken })],
    { type: 'application/json' },
  )
  navigator.sendBeacon('/api/auth/logout', blob)
}
