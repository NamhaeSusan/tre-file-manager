import { createSignal } from 'solid-js'
import type { FileEntry } from '../lib/types'
import * as api from '../lib/api'

export interface TreeNode {
  entry: FileEntry
  children: TreeNode[]
  loaded: boolean
  expanded: boolean
}

export function useFileTree() {
  const [nodes, setNodes] = createSignal<TreeNode[]>([])
  const [loading, setLoading] = createSignal(false)
  const [error, setError] = createSignal('')

  async function loadRoot() {
    setLoading(true)
    setError('')
    try {
      const res = await api.listDirectory()
      const treeNodes: TreeNode[] = res.entries.map((entry) => ({
        entry,
        children: [],
        loaded: false,
        expanded: false,
      }))
      setNodes(treeNodes)
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load files')
    } finally {
      setLoading(false)
    }
  }

  async function toggleExpand(path: string) {
    const updated = await updateNodeAtPath(nodes(), path)
    setNodes(updated)
  }

  async function updateNodeAtPath(
    nodeList: TreeNode[],
    targetPath: string,
  ): Promise<TreeNode[]> {
    const result: TreeNode[] = []
    for (const node of nodeList) {
      if (node.entry.path === targetPath) {
        if (node.expanded) {
          // Collapse
          result.push({ ...node, expanded: false })
        } else {
          // Expand + lazy load
          if (!node.loaded) {
            try {
              const res = await api.listDirectory(node.entry.path)
              const children: TreeNode[] = res.entries.map((entry) => ({
                entry,
                children: [],
                loaded: false,
                expanded: false,
              }))
              result.push({ ...node, children, loaded: true, expanded: true })
            } catch {
              result.push({ ...node, loaded: true, expanded: true })
            }
          } else {
            result.push({ ...node, expanded: true })
          }
        }
      } else if (node.children.length > 0) {
        const updatedChildren = await updateNodeAtPath(node.children, targetPath)
        result.push({ ...node, children: updatedChildren })
      } else {
        result.push(node)
      }
    }
    return result
  }

  return { nodes, loading, error, loadRoot, toggleExpand }
}
