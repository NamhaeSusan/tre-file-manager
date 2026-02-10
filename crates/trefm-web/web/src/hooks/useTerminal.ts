import { onCleanup } from 'solid-js'
import { Terminal } from '@xterm/xterm'
import { FitAddon } from '@xterm/addon-fit'
import { WebLinksAddon } from '@xterm/addon-web-links'
import { createWsTicket } from '../lib/api'

interface UseTerminalOptions {
  token: () => string | null
  currentPath: () => string
}

// Binary-safe base64 encode: string bytes -> base64
function encodeToBase64(str: string): string {
  const encoder = new TextEncoder()
  const bytes = encoder.encode(str)
  const binString = Array.from(bytes, (b) => String.fromCodePoint(b)).join('')
  return btoa(binString)
}

// Binary-safe base64 decode: base64 -> Uint8Array -> string
function decodeFromBase64(b64: string): string {
  const binString = atob(b64)
  const bytes = Uint8Array.from(binString, (c) => c.codePointAt(0)!)
  return new TextDecoder().decode(bytes)
}

export function useTerminal(options: UseTerminalOptions) {
  let terminal: Terminal | null = null
  let fitAddon: FitAddon | null = null
  let ws: WebSocket | null = null
  let resizeObserver: ResizeObserver | null = null

  async function connect(container: HTMLDivElement) {
    // Create terminal
    terminal = new Terminal({
      cursorBlink: true,
      fontSize: 14,
      fontFamily: 'Menlo, Monaco, "Courier New", monospace',
      theme: {
        background: '#1a1b26',
        foreground: '#c0caf5',
        cursor: '#c0caf5',
      },
    })

    fitAddon = new FitAddon()
    terminal.loadAddon(fitAddon)
    terminal.loadAddon(new WebLinksAddon())
    terminal.open(container)
    fitAddon.fit()

    // Connect WebSocket using single-use ticket (not JWT token in URL)
    const token = options.token()
    const cwd = options.currentPath()
    const wsProtocol = location.protocol === 'https:' ? 'wss:' : 'ws:'

    let wsUrl: string
    if (token) {
      try {
        const { ticket } = await createWsTicket()
        wsUrl = `${wsProtocol}//${location.host}/ws/terminal?ticket=${encodeURIComponent(ticket)}&cwd=${encodeURIComponent(cwd)}`
      } catch {
        terminal?.write('\r\n[Failed to authenticate WebSocket connection]\r\n')
        return
      }
    } else {
      // Dev mode (no auth configured)
      wsUrl = `${wsProtocol}//${location.host}/ws/terminal?cwd=${encodeURIComponent(cwd)}`
    }

    ws = new WebSocket(wsUrl)

    ws.onopen = () => {
      // Send initial resize
      if (terminal && fitAddon) {
        const dims = fitAddon.proposeDimensions()
        if (dims) {
          ws?.send(JSON.stringify({
            type: 'resize',
            cols: dims.cols,
            rows: dims.rows,
          }))
        }
      }
    }

    ws.onmessage = (event) => {
      try {
        const msg = JSON.parse(event.data)
        switch (msg.type) {
          case 'output': {
            const decoded = decodeFromBase64(msg.data)
            terminal?.write(decoded)
            break
          }
          case 'exit':
            terminal?.write('\r\n[Process exited]\r\n')
            break
          case 'error':
            terminal?.write(`\r\n[Error: ${msg.message}]\r\n`)
            break
        }
      } catch {
        // ignore parse errors
      }
    }

    ws.onclose = () => {
      terminal?.write('\r\n[Connection closed]\r\n')
    }

    // Handle Shift+Enter (browser may not pass it through by default)
    terminal.attachCustomKeyEventHandler((event) => {
      if (event.type === 'keydown' && event.key === 'Enter' && event.shiftKey) {
        if (ws && ws.readyState === WebSocket.OPEN) {
          ws.send(JSON.stringify({
            type: 'input',
            data: encodeToBase64('\r'),
          }))
        }
        return false
      }
      return true
    })

    // Terminal input -> WebSocket
    terminal.onData((data) => {
      if (ws && ws.readyState === WebSocket.OPEN) {
        ws.send(JSON.stringify({
          type: 'input',
          data: encodeToBase64(data),
        }))
      }
    })

    // ResizeObserver for auto-fit
    resizeObserver = new ResizeObserver(() => {
      if (fitAddon && terminal) {
        fitAddon.fit()
        const dims = fitAddon.proposeDimensions()
        if (dims && ws && ws.readyState === WebSocket.OPEN) {
          ws.send(JSON.stringify({
            type: 'resize',
            cols: dims.cols,
            rows: dims.rows,
          }))
        }
      }
    })
    resizeObserver.observe(container)
  }

  function disconnect() {
    resizeObserver?.disconnect()
    resizeObserver = null
    ws?.close()
    ws = null
    terminal?.dispose()
    terminal = null
    fitAddon = null
  }

  function focus() {
    terminal?.focus()
  }

  function sendCommand(command: string) {
    if (ws && ws.readyState === WebSocket.OPEN) {
      ws.send(JSON.stringify({
        type: 'input',
        data: encodeToBase64(command + '\n'),
      }))
    }
  }

  onCleanup(() => {
    disconnect()
  })

  return { connect, disconnect, focus, sendCommand }
}
