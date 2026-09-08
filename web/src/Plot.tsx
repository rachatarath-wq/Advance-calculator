import { useEffect, useRef } from 'react'
import Plotly from 'plotly.js-dist-min'

interface Props {
  data: any[]
  layout: any
}

const CONFIG = {
  responsive: true,
  scrollZoom: true,
  displaylogo: false,
  modeBarButtonsToRemove: [
    'lasso2d',
    'select2d',
    'autoScale2d',
    'toggleSpikelines',
    'hoverClosestCartesian',
    'hoverCompareCartesian',
  ],
}

/** Thin wrapper around Plotly's `react` API for efficient in-place updates. */
export default function Plot({ data, layout }: Props) {
  const ref = useRef<HTMLDivElement>(null)

  useEffect(() => {
    if (ref.current) {
      Plotly.react(ref.current, data, layout, CONFIG)
    }
  }, [data, layout])

  useEffect(() => {
    const onResize = () => {
      if (ref.current) Plotly.Plots.resize(ref.current)
    }
    window.addEventListener('resize', onResize)
    return () => window.removeEventListener('resize', onResize)
  }, [])

  return <div ref={ref} className="plot" />
}
