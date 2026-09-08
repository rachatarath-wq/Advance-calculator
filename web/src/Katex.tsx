import { useMemo } from 'react'
import katex from 'katex'

interface Props {
  tex: string
  block?: boolean
  className?: string
}

/** Render a LaTeX string with KaTeX (no CDN — bundled locally). */
export default function Katex({ tex, block = false, className }: Props) {
  const html = useMemo(
    () => katex.renderToString(tex || '\\;', { throwOnError: false, displayMode: block }),
    [tex, block],
  )
  return <span className={className} dangerouslySetInnerHTML={{ __html: html }} />
}
