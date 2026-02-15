import { onMount, onCleanup } from 'solid-js'

interface ToastProps {
  message: string
  type: 'success' | 'error'
  onDismiss: () => void
}

export default function Toast(props: ToastProps) {
  onMount(() => {
    const delay = props.type === 'success' ? 3000 : 5000
    const timer = setTimeout(() => props.onDismiss(), delay)
    onCleanup(() => clearTimeout(timer))
  })

  const bgColor = () => props.type === 'success' ? '#2ea043' : '#d1242f'

  return (
    <div
      style={{
        position: 'fixed',
        bottom: '16px',
        right: '16px',
        'background-color': bgColor(),
        color: '#ffffff',
        padding: '10px 20px',
        'border-radius': '6px',
        'font-size': '13px',
        'font-family': 'system-ui, -apple-system, sans-serif',
        'box-shadow': '0 4px 12px rgba(0, 0, 0, 0.4)',
        'z-index': '2000',
        'max-width': '400px',
        'word-break': 'break-word',
        animation: 'toast-in 0.2s ease-out',
      }}
      onClick={props.onDismiss}
    >
      {props.message}
    </div>
  )
}
