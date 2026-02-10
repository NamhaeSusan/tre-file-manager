interface FileIconInfo {
  svg: string
  color: string
}

const FOLDER_CLOSED_SVG =
  '<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 16 16" fill="currentColor"><path d="M1.5 1h5l1 2H14.5a1 1 0 0 1 1 1v9a1 1 0 0 1-1 1h-13a1 1 0 0 1-1-1V2a1 1 0 0 1 1-1z"/></svg>'

const FOLDER_OPEN_SVG =
  '<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 16 16" fill="currentColor"><path d="M1.5 1h5l1 2H14.5a1 1 0 0 1 1 1v1H7.5L6 7H.5V2a1 1 0 0 1 1-1zM0 8l1.5-3h14l-1.5 8H1.5L0 8z"/></svg>'

const FILE_SVG =
  '<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 16 16" fill="currentColor"><path d="M3 1h6l4 4v9a1 1 0 0 1-1 1H3a1 1 0 0 1-1-1V2a1 1 0 0 1 1-1zm6 0v4h4"/></svg>'

const EXTENSION_COLORS: Record<string, string> = {
  ts: '#3178c6',
  tsx: '#3178c6',
  js: '#f1e05a',
  jsx: '#f1e05a',
  mjs: '#f1e05a',
  cjs: '#f1e05a',
  rs: '#dea584',
  md: '#519aba',
  mdx: '#519aba',
  json: '#cbcb41',
  toml: '#9c4221',
  yaml: '#cb171e',
  yml: '#cb171e',
  css: '#563d7c',
  scss: '#c6538c',
  less: '#1d365d',
  html: '#e34c26',
  htm: '#e34c26',
  png: '#a074c4',
  jpg: '#a074c4',
  jpeg: '#a074c4',
  gif: '#a074c4',
  svg: '#ffb13b',
  webp: '#a074c4',
  ico: '#a074c4',
  env: '#ecd53f',
  gitignore: '#f05032',
  dockerignore: '#2496ed',
  sh: '#89e051',
  bash: '#89e051',
  zsh: '#89e051',
  py: '#3572a5',
  go: '#00add8',
  lock: '#776e6e',
  _default: '#8b8b8b',
}

const FILENAME_COLORS: Record<string, string> = {
  'Cargo.toml': '#dea584',
  'Cargo.lock': '#dea584',
  'package.json': '#cbcb41',
  'tsconfig.json': '#3178c6',
  '.gitignore': '#f05032',
  Dockerfile: '#2496ed',
  Makefile: '#427819',
  'README.md': '#519aba',
  LICENSE: '#d4a017',
}

function getExtension(name: string): string {
  const dotIndex = name.lastIndexOf('.')
  if (dotIndex <= 0) return ''
  return name.slice(dotIndex + 1).toLowerCase()
}

function getColor(name: string): string {
  if (name in FILENAME_COLORS) return FILENAME_COLORS[name]

  const ext = getExtension(name)
  if (ext && ext in EXTENSION_COLORS) return EXTENSION_COLORS[ext]

  return EXTENSION_COLORS._default
}

export function getFileIcon(
  name: string,
  isDir: boolean,
  isExpanded: boolean,
): FileIconInfo {
  if (isDir) {
    return {
      svg: isExpanded ? FOLDER_OPEN_SVG : FOLDER_CLOSED_SVG,
      color: '#dcb67a',
    }
  }

  return {
    svg: FILE_SVG,
    color: getColor(name),
  }
}

export function getChevronSvg(): string {
  return '<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 16 16" fill="currentColor"><path d="M6 4l4 4-4 4"/></svg>'
}
