import { onMount, onCleanup, For } from 'solid-js'

export interface MenuItem {
  label: string
  onClick: () => void
}

interface ContextMenuProps {
  x: number
  y: number
  items: MenuItem[]
  onClose: () => void
}

export default function ContextMenu(props: ContextMenuProps) {
  let menuRef: HTMLDivElement | undefined

  function handleClickOutside(e: MouseEvent) {
    if (menuRef && !menuRef.contains(e.target as Node)) {
      props.onClose()
    }
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      props.onClose()
    }
  }

  onMount(() => {
    document.addEventListener('mousedown', handleClickOutside)
    document.addEventListener('keydown', handleKeyDown)
  })

  onCleanup(() => {
    document.removeEventListener('mousedown', handleClickOutside)
    document.removeEventListener('keydown', handleKeyDown)
  })

  return (
    <div
      ref={menuRef}
      style={{
        position: 'fixed',
        left: `${props.x}px`,
        top: `${props.y}px`,
        'background-color': '#3c3c3c',
        border: '1px solid #454545',
        'border-radius': '4px',
        'box-shadow': '0 2px 8px rgba(0, 0, 0, 0.4)',
        'min-width': '160px',
        padding: '4px 0',
        'z-index': '1000',
        'font-family': 'system-ui, -apple-system, sans-serif',
        'font-size': '13px',
      }}
    >
      <For each={props.items}>
        {(item) => (
          <div
            style={{
              padding: '6px 24px',
              color: '#cccccc',
              cursor: 'pointer',
              'white-space': 'nowrap',
            }}
            onMouseEnter={(e) => {
              (e.currentTarget as HTMLElement).style.backgroundColor = '#094771'
            }}
            onMouseLeave={(e) => {
              (e.currentTarget as HTMLElement).style.backgroundColor = 'transparent'
            }}
            onClick={() => {
              item.onClick()
              props.onClose()
            }}
          >
            {item.label}
          </div>
        )}
      </For>
    </div>
  )
}
