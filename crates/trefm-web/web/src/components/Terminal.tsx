import { onMount, onCleanup } from 'solid-js'
import { useTerminal } from '../hooks/useTerminal'

export interface TerminalHandle {
  sendCommand: (command: string) => void
}

interface TerminalProps {
  token: () => string | null
  currentPath: () => string
  onFocus?: () => void
  onBlur?: () => void
  onRef?: (handle: TerminalHandle) => void
}

export default function Terminal(props: TerminalProps) {
  let containerRef: HTMLDivElement | undefined

  const term = useTerminal({
    token: props.token,
    currentPath: props.currentPath,
  })

  onMount(() => {
    if (containerRef) {
      term.connect(containerRef)

      // Expose sendCommand handle to parent
      props.onRef?.({ sendCommand: term.sendCommand })

      // Focus/blur handlers
      containerRef.addEventListener('focusin', () => props.onFocus?.())
      containerRef.addEventListener('focusout', () => props.onBlur?.())

      // Auto-focus terminal
      term.focus()
    }
  })

  onCleanup(() => {
    term.disconnect()
  })

  return (
    <div
      ref={containerRef}
      class="w-full h-full bg-[#1a1b26]"
      style={{ "min-height": "100px" }}
    />
  )
}
