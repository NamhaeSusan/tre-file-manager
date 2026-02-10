import { createSignal } from 'solid-js'
import * as api from '../lib/api'
import type { AuthStepResponse } from '../lib/types'

const [token, setTokenSignal] = createSignal<string | null>(
  localStorage.getItem('trefm_token')
)

const [username, setUsernameSignal] = createSignal<string | null>(
  localStorage.getItem('trefm_username')
)

if (token()) {
  api.setToken(token()!)
}

export type AuthPhase = 'idle' | 'webauthn' | 'otp'

export function useAuth() {
  const [phase, setPhase] = createSignal<AuthPhase>('idle')
  const [sessionId, setSessionId] = createSignal<string | null>(null)

  const isLoggedIn = () => token() !== null

  function handleAuthResponse(res: AuthStepResponse) {
    if (res.status === 'complete') {
      // Set module-level token BEFORE signal update to prevent race condition:
      // SolidJS effect (loadRoot) fires synchronously on setTokenSignal,
      // so api.setToken must be called first for requests to include the token.
      api.setToken(res.token)
      localStorage.setItem('trefm_token', res.token)
      setTokenSignal(res.token)
      setPhase('idle')
      setSessionId(null)
    } else if (res.status === 'next_step') {
      setSessionId(res.session_id)
      setPhase(res.next_step as AuthPhase)
    }
  }

  async function loginFn(usernameInput: string, password: string) {
    setUsernameSignal(usernameInput)
    localStorage.setItem('trefm_username', usernameInput)
    const res = await api.login(usernameInput, password)
    handleAuthResponse(res)
  }

  async function verifyWebAuthnFn() {
    const sid = sessionId()
    if (!sid) throw new Error('No session')

    const challenge = await api.getWebAuthnChallenge(sid)

    const { startAuthentication } = await import('@simplewebauthn/browser')
    const credential = await startAuthentication(challenge)

    const res = await api.verifyWebAuthn(sid, credential)
    handleAuthResponse(res)
  }

  async function verifyOtpFn(code: string) {
    const sid = sessionId()
    if (!sid) throw new Error('No session')

    const res = await api.verifyOtp(sid, code)
    handleAuthResponse(res)
  }

  function logout() {
    setTokenSignal(null)
    api.setToken(null)
    localStorage.removeItem('trefm_token')
    setUsernameSignal(null)
    localStorage.removeItem('trefm_username')
    setPhase('idle')
    setSessionId(null)
  }

  return {
    token,
    username,
    isLoggedIn,
    phase,
    sessionId,
    login: loginFn,
    verifyWebAuthn: verifyWebAuthnFn,
    verifyOtp: verifyOtpFn,
    logout,
  }
}
