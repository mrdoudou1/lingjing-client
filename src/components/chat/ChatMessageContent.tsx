import type { ReactNode } from 'react'

function escapeText(value: string) {
  return value.replace(/[&<>"']/g, character => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' })[character] ?? character)
}

function inlineMarkdown(value: string): ReactNode[] {
  const nodes: ReactNode[] = []
  const pattern = /(\*\*[^*]+\*\*|__[^_]+__|\*[^*]+\*|_([^_]+)_|\[([^\]]+)\]\((https?:\/\/[^)]+)\))/g
  let cursor = 0
  let match: RegExpExecArray | null
  while ((match = pattern.exec(value))) {
    if (match.index > cursor) nodes.push(escapeText(value.slice(cursor, match.index)))
    const token = match[0]
    if (token.startsWith('**') || token.startsWith('__')) nodes.push(<strong key={match.index}>{escapeText(token.slice(2, -2))}</strong>)
    else if (token.startsWith('*') || token.startsWith('_')) nodes.push(<em key={match.index}>{escapeText(token.slice(1, -1))}</em>)
    else nodes.push(<a key={match.index} href={match[4]} target="_blank" rel="noreferrer">{escapeText(match[3])}</a>)
    cursor = match.index + token.length
  }
  if (cursor < value.length) nodes.push(escapeText(value.slice(cursor)))
  return nodes
}

export function ChatMessageContent({ content }: { content: string }) {
  const codeMarker = String.fromCharCode(96)
  const lines = content.split(/\r?\n/)
  const blocks: ReactNode[] = []
  let inCode = false
  let code = ''
  lines.forEach((line, index) => {
    if (line.trim().startsWith(codeMarker + codeMarker + codeMarker)) {
      if (inCode) blocks.push(<pre key={'code-' + index}><code>{escapeText(code.replace(/\n$/, ''))}</code></pre>)
      inCode = !inCode
      code = ''
      return
    }
    if (inCode) { code += line + '\n'; return }
    if (!line.trim()) { blocks.push(<br key={'br-' + index} />); return }
    const heading = line.match(/^(#{1,3})\s+(.+)$/)
    if (heading) {
      const Tag = heading[1].length === 1 ? 'h3' : heading[1].length === 2 ? 'h4' : 'h5'
      blocks.push(<Tag key={index}>{inlineMarkdown(heading[2])}</Tag>)
      return
    }
    blocks.push(<div key={index}>{inlineMarkdown(line)}</div>)
  })
  if (inCode) blocks.push(<pre key="code-tail"><code>{escapeText(code)}</code></pre>)
  return <div className="markdown-content">{blocks}</div>
}
