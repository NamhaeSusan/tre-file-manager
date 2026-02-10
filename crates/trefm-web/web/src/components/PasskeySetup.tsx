import { createSignal } from 'solid-js'
import * as api from '../lib/api'

export default function PasskeySetup() {
  const [status, setStatus] = createSignal('')
  const [loading, setLoading] = createSignal(false)

  async function handleRegister() {
    setLoading(true)
    setStatus('')
    try {
      const options = await api.startPasskeyRegistration()
      const sessionId = options.session_id

      const { startRegistration } = await import('@simplewebauthn/browser')
      const credential = await startRegistration(options)

      await api.finishPasskeyRegistration(sessionId, credential)
      setStatus('Passkey registered successfully!')
    } catch (err) {
      setStatus(err instanceof Error ? err.message : 'Registration failed')
    } finally {
      setLoading(false)
    }
  }

  return (
    <div class="p-4">
      <h3 class="text-lg font-medium text-gray-200 mb-2">Passkey Setup</h3>
      <p class="text-gray-400 text-sm mb-4">
        Register a passkey for additional security
      </p>
      <button
        onClick={handleRegister}
        disabled={loading()}
        class="px-4 py-2 bg-purple-600 hover:bg-purple-700 disabled:bg-gray-600 text-white rounded text-sm"
      >
        {loading() ? 'Registering...' : 'Register Passkey'}
      </button>
      {status() && (
        <p class="mt-2 text-sm text-gray-300">{status()}</p>
      )}
    </div>
  )
}
