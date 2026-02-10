import { createSignal, Show } from 'solid-js'
import { useAuth } from '../hooks/useAuth'

export default function LoginPage() {
  const auth = useAuth()
  const [username, setUsername] = createSignal(localStorage.getItem('trefm_username') || '')
  const [password, setPassword] = createSignal('')
  const [otpCode, setOtpCode] = createSignal('')
  const [error, setError] = createSignal('')
  const [loading, setLoading] = createSignal(false)

  async function handlePasswordSubmit(e: Event) {
    e.preventDefault()
    setLoading(true)
    setError('')
    try {
      await auth.login(username(), password())
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Login failed')
    } finally {
      setLoading(false)
    }
  }

  async function handleWebAuthn() {
    setLoading(true)
    setError('')
    try {
      await auth.verifyWebAuthn()
    } catch (err) {
      setError(err instanceof Error ? err.message : 'WebAuthn failed')
    } finally {
      setLoading(false)
    }
  }

  async function handleOtpSubmit(e: Event) {
    e.preventDefault()
    setLoading(true)
    setError('')
    try {
      await auth.verifyOtp(otpCode().trim())
    } catch (err) {
      setError(err instanceof Error ? err.message : 'OTP verification failed')
    } finally {
      setLoading(false)
    }
  }

  return (
    <div class="min-h-screen flex items-center justify-center bg-gray-900">
      <div class="bg-gray-800 p-8 rounded-lg shadow-xl w-96">
        <h1 class="text-2xl font-bold text-gray-100 mb-6 text-center">TreFM</h1>

        {error() && (
          <p class="text-red-400 text-sm mb-4">{error()}</p>
        )}

        {/* Step 1: Password */}
        <Show when={auth.phase() === 'idle'}>
          <form onSubmit={handlePasswordSubmit}>
            <div class="mb-4">
              <input
                type="text"
                placeholder="Username"
                value={username()}
                onInput={e => setUsername(e.currentTarget.value)}
                class="w-full px-4 py-2 bg-gray-700 text-gray-100 rounded border border-gray-600 focus:outline-none focus:border-blue-500"
                autofocus
              />
            </div>
            <div class="mb-4">
              <input
                type="password"
                placeholder="Password"
                value={password()}
                onInput={e => setPassword(e.currentTarget.value)}
                class="w-full px-4 py-2 bg-gray-700 text-gray-100 rounded border border-gray-600 focus:outline-none focus:border-blue-500"
              />
            </div>
            <button
              type="submit"
              disabled={loading()}
              class="w-full py-2 bg-blue-600 hover:bg-blue-700 disabled:bg-gray-600 text-white rounded font-medium"
            >
              {loading() ? 'Verifying...' : 'Login'}
            </button>
          </form>
        </Show>

        {/* Step 2: WebAuthn */}
        <Show when={auth.phase() === 'webauthn'}>
          <div class="text-center">
            <p class="text-gray-400 mb-4">Verify your identity with a passkey</p>
            <button
              onClick={handleWebAuthn}
              disabled={loading()}
              class="w-full py-2 bg-purple-600 hover:bg-purple-700 disabled:bg-gray-600 text-white rounded font-medium"
            >
              {loading() ? 'Verifying...' : 'Verify with Passkey'}
            </button>
          </div>
        </Show>

        {/* Step 3: OTP */}
        <Show when={auth.phase() === 'otp'}>
          <form onSubmit={handleOtpSubmit}>
            <p class="text-gray-400 text-sm mb-4 text-center">
              Check Discord for your verification code
            </p>
            <div class="mb-4">
              <input
                type="text"
                placeholder="6-digit code"
                value={otpCode()}
                onInput={e => setOtpCode(e.currentTarget.value)}
                class="w-full px-4 py-2 bg-gray-700 text-gray-100 rounded border border-gray-600 focus:outline-none focus:border-blue-500 text-center text-lg tracking-widest"
                maxLength={6}
                autofocus
              />
            </div>
            <button
              type="submit"
              disabled={loading()}
              class="w-full py-2 bg-green-600 hover:bg-green-700 disabled:bg-gray-600 text-white rounded font-medium"
            >
              {loading() ? 'Verifying...' : 'Verify OTP'}
            </button>
          </form>
        </Show>
      </div>
    </div>
  )
}
