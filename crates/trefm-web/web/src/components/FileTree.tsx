import { For, Show, createSignal } from 'solid-js'
import type { TreeNode } from '../hooks/useFileTree'
import { getFileIcon, getChevronSvg } from '../lib/icons'

const EDITABLE_EXTENSIONS = new Set([
  // Programming languages
  'rs', 'ts', 'tsx', 'js', 'jsx', 'py', 'go', 'java',
  'c', 'cpp', 'cc', 'h', 'hpp', 'cs', 'rb', 'php',
  'swift', 'kt', 'scala', 'lua', 'zig', 'hs', 'ex', 'exs',
  'clj', 'dart', 'r', 'pl', 'elm', 'erl', 'ml', 'v',
  // Web
  'html', 'htm', 'css', 'scss', 'sass', 'less', 'svg',
  // Config/data
  'json', 'yaml', 'yml', 'toml', 'xml', 'ini', 'env',
  'conf', 'cfg', 'properties',
  // Shell/scripts
  'sh', 'bash', 'zsh', 'fish', 'ps1', 'bat', 'cmd',
  // Text-based documents
  'md', 'txt', 'rst', 'adoc', 'org', 'tex', 'log',
  // Dev misc
  'sql', 'graphql', 'gql', 'proto', 'vim', 'dockerfile',
  'makefile', 'cmake', 'gradle', 'lock',
])

const EDITABLE_FILENAMES = new Set([
  'makefile', 'dockerfile', 'vagrantfile', 'gemfile',
  'rakefile', 'procfile', 'brewfile', 'justfile',
  '.gitignore', '.gitattributes', '.editorconfig',
  '.env', '.prettierrc', '.eslintrc', '.babelrc',
])

function isEditable(filename: string): boolean {
  const lower = filename.toLowerCase()
  if (EDITABLE_FILENAMES.has(lower)) return true
  const dotIdx = lower.lastIndexOf('.')
  if (dotIdx === -1) return false
  return EDITABLE_EXTENSIONS.has(lower.slice(dotIdx + 1))
}

interface FileTreeProps {
  nodes: TreeNode[]
  onToggle: (path: string) => void
  onNavigate: (path: string) => void
  onOpenFile: (path: string) => void
}

interface TreeItemProps {
  node: TreeNode
  depth: number
  selectedPath: string | null
  onSelect: (path: string) => void
  onToggle: (path: string) => void
  onNavigate: (path: string) => void
  onOpenFile: (path: string) => void
}

function TreeItem(props: TreeItemProps) {
  function handleClick() {
    props.onSelect(props.node.entry.path)
    if (props.node.entry.is_dir) {
      props.onToggle(props.node.entry.path)
      props.onNavigate(props.node.entry.path)
    } else if (isEditable(props.node.entry.name)) {
      props.onOpenFile(props.node.entry.path)
    }
  }

  const isSelected = () => props.selectedPath === props.node.entry.path
  const isHidden = () => props.node.entry.is_hidden
  const icon = () =>
    getFileIcon(
      props.node.entry.name,
      props.node.entry.is_dir,
      props.node.expanded,
    )
  const chevronSvg = getChevronSvg()

  return (
    <>
      <div
        class={`tree-item flex items-center cursor-pointer relative`}
        classList={{ selected: isSelected() }}
        style={{
          'padding-left': `${props.depth * 16 + 8}px`,
          'padding-top': '4px',
          'padding-bottom': '4px',
          'padding-right': '8px',
          'font-size': '13px',
          'font-family': 'system-ui, -apple-system, sans-serif',
          opacity: isHidden() ? '0.5' : '1',
        }}
        onClick={handleClick}
      >
        {/* Indent guides */}
        <For each={Array.from({ length: props.depth })}>
          {(_, i) => (
            <div
              class="indent-guide"
              style={{ left: `${i() * 16 + 16}px` }}
            />
          )}
        </For>

        {/* Chevron for directories */}
        <Show when={props.node.entry.is_dir}>
          <span
            class="chevron"
            classList={{ expanded: props.node.expanded }}
            style={{
              width: '16px',
              height: '16px',
              'margin-right': '2px',
              'flex-shrink': '0',
              color: '#c5c5c5',
            }}
            innerHTML={chevronSvg}
          />
        </Show>
        <Show when={!props.node.entry.is_dir}>
          <span
            style={{
              width: '16px',
              height: '16px',
              'margin-right': '2px',
              'flex-shrink': '0',
            }}
          />
        </Show>

        {/* File/folder icon */}
        <span
          style={{
            width: '16px',
            height: '16px',
            'margin-right': '6px',
            'flex-shrink': '0',
            color: icon().color,
            display: 'inline-flex',
            'align-items': 'center',
            'justify-content': 'center',
          }}
          innerHTML={icon().svg}
        />

        {/* File name */}
        <span
          style={{
            color: props.node.entry.is_dir ? '#c8c8c8' : '#cccccc',
            'white-space': 'nowrap',
            overflow: 'hidden',
            'text-overflow': 'ellipsis',
          }}
        >
          {props.node.entry.name}
        </span>
      </div>

      {/* Children */}
      <Show when={props.node.expanded && props.node.entry.is_dir}>
        <For each={props.node.children}>
          {(child) => (
            <TreeItem
              node={child}
              depth={props.depth + 1}
              selectedPath={props.selectedPath}
              onSelect={props.onSelect}
              onToggle={props.onToggle}
              onNavigate={props.onNavigate}
              onOpenFile={props.onOpenFile}
            />
          )}
        </For>
      </Show>
    </>
  )
}

export default function FileTree(props: FileTreeProps) {
  const [selectedPath, setSelectedPath] = createSignal<string | null>(null)

  return (
    <div class="sidebar-scroll overflow-y-auto h-full">
      <For each={props.nodes}>
        {(node) => (
          <TreeItem
            node={node}
            depth={0}
            selectedPath={selectedPath()}
            onSelect={setSelectedPath}
            onToggle={props.onToggle}
            onNavigate={props.onNavigate}
            onOpenFile={props.onOpenFile}
          />
        )}
      </For>
    </div>
  )
}
