import { Show, createSignal, createEffect } from 'solid-js'
import { useAuth } from './hooks/useAuth'
import { useFileTree } from './hooks/useFileTree'
import LoginPage from './components/LoginPage'
import Terminal from './components/Terminal'
import FileTree from './components/FileTree'

function escapeShellArg(arg: string): string {
  return "'" + arg.replace(/'/g, "'\\''") + "'"
}

const FILES_ICON_SVG =
  '<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><path d="M13 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V9z"/><polyline points="13 2 13 9 20 9"/></svg>'

const CLOSE_ICON_SVG =
  '<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 16 16" fill="currentColor"><path d="M8 8.707l3.646 3.647.708-.707L8.707 8l3.647-3.646-.707-.708L8 7.293 4.354 3.646l-.707.708L7.293 8l-3.646 3.646.707.708L8 8.707z"/></svg>'

const CHEVRON_DOWN_SVG =
  '<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 16 16" fill="currentColor"><path d="M6 4l4 4-4 4"/></svg>'

export default function App() {
  const auth = useAuth()
  const fileTree = useFileTree()
  const [sidebarOpen, setSidebarOpen] = createSignal(true)
  const [filesSectionOpen, setFilesSectionOpen] = createSignal(true)
  let terminalHandle: { sendCommand: (cmd: string) => void } | null = null

  createEffect(() => {
    if (auth.isLoggedIn()) {
      fileTree.loadRoot()
    }
  })

  function handleNavigate(path: string) {
    if (terminalHandle) {
      terminalHandle.sendCommand(`cd ${escapeShellArg(path)}`)
    }
  }

  return (
    <Show when={auth.isLoggedIn()} fallback={<LoginPage />}>
      <div class="h-screen w-screen bg-[#1e1e1e] flex">
        {/* Activity Bar (always visible) */}
        <div
          class="flex flex-col items-center flex-shrink-0"
          style={{
            width: '48px',
            'background-color': '#333333',
            'border-right': '1px solid #3c3c3c',
          }}
        >
          <button
            onClick={() => setSidebarOpen(!sidebarOpen())}
            class="flex items-center justify-center"
            style={{
              width: '48px',
              height: '48px',
              color: sidebarOpen() ? '#ffffff' : '#858585',
              'border-left': sidebarOpen()
                ? '2px solid #ffffff'
                : '2px solid transparent',
              background: 'none',
              border: 'none',
              cursor: 'pointer',
            }}
            title="Explorer"
          >
            <span
              style={{
                display: 'inline-flex',
                'align-items': 'center',
                'justify-content': 'center',
              }}
              innerHTML={FILES_ICON_SVG}
            />
          </button>
        </div>

        {/* Sidebar Panel */}
        <Show when={sidebarOpen()}>
          <div
            class="flex flex-col flex-shrink-0"
            style={{
              width: '260px',
              'background-color': '#252526',
              'border-right': '1px solid #3c3c3c',
            }}
          >
            {/* Sidebar Title Header */}
            <div
              class="flex items-center justify-between"
              style={{
                padding: '0 12px',
                height: '35px',
                'flex-shrink': '0',
              }}
            >
              <span
                style={{
                  'font-size': '11px',
                  'font-weight': '400',
                  'letter-spacing': '1px',
                  'text-transform': 'uppercase',
                  color: '#bbbbbb',
                  'font-family': 'system-ui, -apple-system, sans-serif',
                }}
              >
                Explorer
              </span>
              <button
                onClick={() => setSidebarOpen(false)}
                class="flex items-center justify-center"
                style={{
                  width: '22px',
                  height: '22px',
                  background: 'none',
                  border: 'none',
                  cursor: 'pointer',
                  color: '#858585',
                  'border-radius': '3px',
                }}
                title="Close sidebar"
              >
                <span innerHTML={CLOSE_ICON_SVG} />
              </button>
            </div>

            {/* FILES Section Header */}
            <div
              class="section-header"
              onClick={() => setFilesSectionOpen(!filesSectionOpen())}
            >
              <span
                class="chevron"
                classList={{ expanded: filesSectionOpen() }}
                style={{
                  width: '16px',
                  height: '16px',
                  'margin-right': '4px',
                  'flex-shrink': '0',
                }}
                innerHTML={CHEVRON_DOWN_SVG}
              />
              <span>Files</span>
            </div>

            {/* File Tree */}
            <Show when={filesSectionOpen()}>
              <div class="flex-1 overflow-hidden">
                <FileTree
                  nodes={fileTree.nodes()}
                  onToggle={(path) => fileTree.toggleExpand(path)}
                  onNavigate={handleNavigate}
                />
              </div>
            </Show>
          </div>
        </Show>

        {/* Main Content (Terminal) */}
        <div class="flex-1 flex flex-col min-w-0">
          <Terminal
            token={auth.token}
            currentPath={() => '/'}
            onRef={(handle) => {
              terminalHandle = handle
            }}
          />
        </div>
      </div>
    </Show>
  )
}
