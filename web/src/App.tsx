import { useEffect, useMemo, useRef, useState } from 'react'
import Katex from './Katex'
import Plot from './Plot'
import AcPowerPanel from './AcPower'
import { analyze, ensureEngine, type Analysis } from './engine'

const SAMPLES = 400
const PLAY_SPEED = 0.35 // full-domain sweep in ~1/speed seconds

const EXAMPLES = [
  'x^2 + sin(x)',
  'x^3 - 3*x',
  'sin(x)',
  'e^x',
  'ln(x)',
  '1/x',
  'x*sin(x)',
  'sqrt(x)',
]

const PALETTE = {
  f: '#4f8cff',
  df: '#ffb14f',
  F: '#34d399',
  sim: '#ff5c8a',
  area: 'rgba(52, 211, 153, 0.28)',
}

function clamp(v: number, lo: number, hi: number) {
  return Math.min(Math.max(v, lo), hi)
}

/** Linear interpolation on a sorted grid; NaN if the surrounding samples are bad. */
function interp(xs: number[], ys: number[], x: number): number {
  if (xs.length === 0) return NaN
  if (x <= xs[0]) return ys[0]
  if (x >= xs[xs.length - 1]) return ys[xs.length - 1]
  let lo = 0
  let hi = xs.length - 1
  while (hi - lo > 1) {
    const mid = (lo + hi) >> 1
    if (xs[mid] <= x) lo = mid
    else hi = mid
  }
  const t = (x - xs[lo]) / (xs[hi] - xs[lo])
  const y0 = ys[lo]
  const y1 = ys[hi]
  if (!Number.isFinite(y0) || !Number.isFinite(y1)) return NaN
  return y0 + t * (y1 - y0)
}

/** Robust [lo, hi] range from the 1st–99th percentiles of all finite samples. */
function robustRange(series: number[][]): [number, number] {
  const vals: number[] = []
  for (const s of series) for (const v of s) if (Number.isFinite(v)) vals.push(v)
  if (vals.length < 4) return [-10, 10]
  vals.sort((a, b) => a - b)
  const lo = vals[Math.floor(vals.length * 0.01)]
  const hi = vals[Math.ceil(vals.length * 0.99)]
  if (!Number.isFinite(lo) || !Number.isFinite(hi) || lo === hi) return [-10, 10]
  const pad = (hi - lo) * 0.12
  return [lo - pad, hi + pad]
}

type Mode = 'derivative' | 'integral'

export default function App() {
  const [view, setView] = useState<'calculus' | 'acpower'>('calculus')
  const [input, setInput] = useState(EXAMPLES[0])
  const [xMin, setXMin] = useState(-8)
  const [xMax, setXMax] = useState(8)
  const [analysis, setAnalysis] = useState<Analysis | null>(null)
  const [status, setStatus] = useState<'loading' | 'ready' | 'error'>('loading')

  const [showF, setShowF] = useState(true)
  const [showDf, setShowDf] = useState(true)
  const [showFint, setShowFint] = useState(true)

  const [mode, setMode] = useState<Mode>('derivative')
  const [simX, setSimX] = useState(0)
  const [intA, setIntA] = useState(-2)
  const [intB, setIntB] = useState(3)
  const [playing, setPlaying] = useState(false)

  // Re-run the engine whenever the expression or domain changes (debounced).
  useEffect(() => {
    let cancelled = false
    const t = setTimeout(async () => {
      setStatus('loading')
      try {
        await ensureEngine()
        const a = analyze(input, xMin, xMax, SAMPLES)
        if (cancelled) return
        setAnalysis(a)
        setStatus(a.ok ? 'ready' : 'error')
        if (a.ok) {
          setSimX((a.x_min + a.x_max) / 2)
          setIntA(a.x_min + (a.x_max - a.x_min) * 0.25)
          setIntB(a.x_min + (a.x_max - a.x_min) * 0.75)
        }
      } catch (err) {
        if (!cancelled) {
          setStatus('error')
          setAnalysis(null)
        }
      }
    }, 250)
    return () => {
      cancelled = true
      clearTimeout(t)
    }
  }, [input, xMin, xMax])

  // Playback loop (requestAnimationFrame) — animates the active sim parameter.
  const playingRef = useRef(playing)
  playingRef.current = playing
  useEffect(() => {
    if (!playing) return
    let raf = 0
    let last = performance.now()
    const span = xMax - xMin
    const step = (now: number) => {
      const dt = (now - last) / 1000
      last = now
      const dx = dt * PLAY_SPEED * span
      if (mode === 'derivative') {
        setSimX((prev) => {
          let nx = prev + dx
          if (nx > xMax) nx = xMin
          return nx
        })
      } else {
        setIntB((prev) => {
          let nb = prev + dx
          if (nb > xMax) nb = intA
          return nb
        })
      }
      raf = requestAnimationFrame(step)
    }
    raf = requestAnimationFrame(step)
    return () => cancelAnimationFrame(raf)
  }, [playing, mode, xMax, xMin, intA])

  const yRange = useMemo(
    () => (analysis?.ok ? robustRange([analysis.ys, analysis.dys, analysis.fs]) : [-10, 10] as [number, number]),
    [analysis],
  )

  const simDerivative = useMemo(() => {
    if (!analysis?.ok) return null
    return {
      x: simX,
      y: interp(analysis.xs, analysis.ys, simX),
      slope: interp(analysis.xs, analysis.dys, simX),
    }
  }, [analysis, simX])

  const areaValue = useMemo(() => {
    if (!analysis?.ok) return null
    const Fa = interp(analysis.xs, analysis.fs, intA)
    const Fb = interp(analysis.xs, analysis.fs, intB)
    return Fb - Fa
  }, [analysis, intA, intB])

  const plotData = useMemo(() => {
    if (!analysis?.ok) return []
    const { xs, ys, dys, fs, x_min, x_max } = analysis

    const empty = { x: [] as number[], y: [] as number[] }

    const traces: any[] = [
      {
        ...empty,
        type: 'scatter',
        mode: 'lines',
        name: 'f(x)',
        visible: showF,
        line: { color: PALETTE.f, width: 2.5 },
        hovertemplate: '(%{x:.2f}, %{y:.2f})<extra>f(x)</extra>',
        x: xs,
        y: ys,
      },
      {
        ...empty,
        type: 'scatter',
        mode: 'lines',
        name: "f'(x)",
        visible: showDf,
        line: { color: PALETTE.df, width: 2, dash: 'dot' },
        hovertemplate: '(%{x:.2f}, %{y:.2f})<extra>f′(x)</extra>',
        x: xs,
        y: dys,
      },
      {
        ...empty,
        type: 'scatter',
        mode: 'lines',
        name: 'F(x)',
        visible: showFint,
        line: { color: PALETTE.F, width: 2, dash: 'dash' },
        hovertemplate: '(%{x:.2f}, %{y:.2f})<extra>F(x)</extra>',
        x: xs,
        y: fs,
      },
    ]

    // --- Differentiation sim: tangent line + moving point ---
    const tangent: any = { ...empty, type: 'scatter', mode: 'lines', name: 'tangent', line: { color: PALETTE.sim, width: 2.2 } }
    const point: any = {
      ...empty,
      type: 'scatter',
      mode: 'markers',
      name: 'point',
      marker: { color: PALETTE.sim, size: 9, line: { color: '#fff', width: 1 } },
      hovertemplate: 'x = %{x:.3f}<br>f(x) = %{y:.3f}<extra></extra>',
    }
    if (mode === 'derivative' && simDerivative && Number.isFinite(simDerivative.y) && Number.isFinite(simDerivative.slope)) {
      const span = (x_max - x_min) * 0.6
      const x0 = simDerivative.x - span / 2
      const x1 = simDerivative.x + span / 2
      const y0 = clamp(simDerivative.y + simDerivative.slope * (x0 - simDerivative.x), yRange[0], yRange[1])
      const y1 = clamp(simDerivative.y + simDerivative.slope * (x1 - simDerivative.x), yRange[0], yRange[1])
      tangent.x = [x0, x1]
      tangent.y = [y0, y1]
      point.x = [simDerivative.x]
      point.y = [simDerivative.y]
    }
    traces.push(tangent, point)

    // --- Integration sim: shaded area under f between a and b ---
    const area: any = {
      ...empty,
      type: 'scatter',
      mode: 'lines',
      name: '∫ area',
      fill: 'tozeroy',
      fillcolor: PALETTE.area,
      line: { color: PALETTE.F, width: 0.8 },
      hoverinfo: 'skip',
    }
    const limits: any = {
      ...empty,
      type: 'scatter',
      mode: 'markers',
      name: 'limits',
      marker: { color: PALETTE.sim, size: 7, symbol: 'line-ns-open' },
      hovertemplate: 'x = %{x:.3f}<extra></extra>',
    }
    if (mode === 'integral') {
      const lo = Math.min(intA, intB)
      const hi = Math.max(intA, intB)
      const ax: number[] = [lo]
      const ay: number[] = [interp(xs, ys, lo)]
      for (let i = 0; i < xs.length; i++) {
        if (xs[i] > lo && xs[i] < hi) {
          ax.push(xs[i])
          ay.push(ys[i])
        }
      }
      ax.push(hi)
      ay.push(interp(xs, ys, hi))
      area.x = ax
      area.y = ay
      limits.x = [intA, intB]
      limits.y = [interp(xs, ys, intA), interp(xs, ys, intB)]
    }
    traces.push(area, limits)

    return traces
  }, [analysis, mode, simX, simDerivative, intA, intB, yRange, showF, showDf, showFint])

  const plotLayout = useMemo(() => {
    if (!analysis?.ok) return {}
    return {
      margin: { l: 52, r: 16, t: 24, b: 44 },
      paper_bgcolor: 'transparent',
      plot_bgcolor: 'transparent',
      font: { color: '#cbd5e1', family: 'Inter, system-ui, -apple-system, sans-serif', size: 12 },
      xaxis: {
        range: [analysis.x_min, analysis.x_max],
        zeroline: true,
        zerolinecolor: '#334155',
        gridcolor: '#1e293b',
      },
      yaxis: {
        range: yRange,
        zeroline: true,
        zerolinecolor: '#334155',
        gridcolor: '#1e293b',
      },
      showlegend: true,
      legend: { orientation: 'h', y: 1.12, x: 0, bgcolor: 'transparent', font: { size: 12 } },
      hovermode: 'closest',
    }
  }, [analysis, yRange])

  const ok = analysis?.ok ?? false

  return (
    <div className="app">
      <header className="topbar">
        <div className="brand">
          <span className="logo">∫</span>
          <div>
            <h1>Calculus Simulator</h1>
            <p>Differentiate · Integrate · Visualize — powered by a Rust math engine (WASM)</p>
          </div>
        </div>
        <div className="status-pill" data-status={status}>
          {status === 'loading' ? 'computing…' : status === 'error' ? 'error' : 'ready'}
        </div>
      </header>

      <nav className="nav-tabs">
        <button className={view === 'calculus' ? 'active' : ''} onClick={() => setView('calculus')}>
          ∫ Calculus
        </button>
        <button className={view === 'acpower' ? 'active' : ''} onClick={() => setView('acpower')}>
          ⚡ AC Power
        </button>
      </nav>

      {view === 'calculus' && (
      <main className="layout">
        <aside className="sidebar">
          <section className="card">
            <label className="field-label" htmlFor="fn-input">
              Function f(x)
            </label>
            <div className="fn-input-row">
              <span className="fn-prefix">f(x) =</span>
              <input
                id="fn-input"
                value={input}
                onChange={(e) => setInput(e.target.value)}
                spellCheck={false}
                autoComplete="off"
                placeholder="e.g. x^2 + sin(x)"
              />
            </div>
            <div className="examples">
              {EXAMPLES.map((ex) => (
                <button key={ex} className="chip" onClick={() => setInput(ex)}>
                  {ex}
                </button>
              ))}
            </div>
            <div className="domain-row">
              <label>
                <span>x min</span>
                <input type="number" value={xMin} onChange={(e) => setXMin(Number(e.target.value))} />
              </label>
              <label>
                <span>x max</span>
                <input type="number" value={xMax} onChange={(e) => setXMax(Number(e.target.value))} />
              </label>
            </div>
          </section>

          <section className="card">
            <h2>Results</h2>
            {!ok ? (
              <p className="error-text">{analysis?.error ?? 'Unable to parse the expression.'}</p>
            ) : (
              <>
                <ResultRow label="f(x)" tex={analysis!.input_latex} color={PALETTE.f} />
                <ResultRow label="f′(x)" tex={analysis!.derivative_latex} color={PALETTE.df} />
                <ResultRow
                  label="∫ f(x) dx"
                  tex={analysis!.integral_latex}
                  color={PALETTE.F}
                  hint={analysis!.integral_symbolic ? undefined : 'numerical'}
                />
                <div className="definite">
                  <span className="definite-label">Definite integral over [{analysis!.x_min}, {analysis!.x_max}]</span>
                  <Katex
                    block
                    tex={`\\int_{${analysis!.x_min}}^{${analysis!.x_max}} f(x)\\,dx = ${
                      analysis!.definite_value != null ? analysis!.definite_value.toFixed(6) : '\\text{undefined}'
                    }`}
                  />
                </div>
              </>
            )}
          </section>

          <section className="card">
            <h2>Curves</h2>
            <div className="toggles">
              <Toggle label="f(x)" color={PALETTE.f} checked={showF} onChange={setShowF} />
              <Toggle label="f′(x)" color={PALETTE.df} checked={showDf} onChange={setShowDf} />
              <Toggle label="F(x)" color={PALETTE.F} checked={showFint} onChange={setShowFint} />
            </div>
          </section>
        </aside>

        <section className="plot-panel">
          <div className="sim-tabs">
            <button className={mode === 'derivative' ? 'active' : ''} onClick={() => { setMode('derivative'); setPlaying(false) }}>
              Differentiation
            </button>
            <button className={mode === 'integral' ? 'active' : ''} onClick={() => { setMode('integral'); setPlaying(false) }}>
              Integration
            </button>
          </div>

          <Plot data={plotData} layout={plotLayout} />

          {ok && (
            <SimControls
              mode={mode}
              analysis={analysis!}
              simDerivative={simDerivative}
              intA={intA}
              intB={intB}
              areaValue={areaValue}
              playing={playing}
              onSimX={setSimX}
              onIntA={(v) => setIntA(Math.min(v, intB - 1e-6))}
              onIntB={(v) => setIntB(Math.max(v, intA + 1e-6))}
              onPlay={() => setPlaying((p) => !p)}
            />
          )}
        </section>
      </main>
      )}

      {view === 'acpower' && <AcPowerPanel />}
    </div>
  )
}

function ResultRow({ label, tex, color, hint }: { label: string; tex: string; color: string; hint?: string }) {
  return (
    <div className="result-row">
      <span className="result-label" style={{ color }}>
        {label}
      </span>
      <Katex block tex={tex} />
      {hint && <span className="hint">{hint}</span>}
    </div>
  )
}

function Toggle({ label, color, checked, onChange }: { label: string; color: string; checked: boolean; onChange: (v: boolean) => void }) {
  return (
    <button className={'toggle' + (checked ? ' on' : '')} onClick={() => onChange(!checked)}>
      <span className="swatch" style={{ background: color, opacity: checked ? 1 : 0.25 }} />
      {label}
    </button>
  )
}

function SimControls({
  mode,
  analysis,
  simDerivative,
  intA,
  intB,
  areaValue,
  playing,
  onSimX,
  onIntA,
  onIntB,
  onPlay,
}: {
  mode: Mode
  analysis: Analysis
  simDerivative: { x: number; y: number; slope: number } | null
  intA: number
  intB: number
  areaValue: number | null
  playing: boolean
  onSimX: (v: number) => void
  onIntA: (v: number) => void
  onIntB: (v: number) => void
  onPlay: () => void
}) {
  const { x_min, x_max } = analysis

  return (
    <div className="sim-controls">
      <button className="play" onClick={onPlay}>
        {playing ? '⏸ Pause' : '▶ Play'}
      </button>

      {mode === 'derivative' ? (
        <div className="sliders">
          <div className="slider-block">
            <div className="slider-head">
              <span>
                x = <b>{simDerivative ? simDerivative.x.toFixed(3) : '—'}</b>
              </span>
              <span>
                slope f′(x) = <b className="accent">{simDerivative ? simDerivative.slope.toFixed(3) : '—'}</b>
              </span>
            </div>
            <input
              type="range"
              min={x_min}
              max={x_max}
              step={(x_max - x_min) / 1000}
              value={simDerivative?.x ?? 0}
              onChange={(e) => onSimX(Number(e.target.value))}
            />
          </div>
          <p className="sim-hint">A point slides along f(x); the dashed line is the tangent, its slope is the derivative.</p>
        </div>
      ) : (
        <div className="sliders">
          <div className="slider-block">
            <div className="slider-head">
              <span>
                a = <b>{intA.toFixed(3)}</b>
              </span>
              <span>
                b = <b>{intB.toFixed(3)}</b>
              </span>
            </div>
            <input type="range" min={x_min} max={x_max} step={(x_max - x_min) / 1000} value={intA} onChange={(e) => onIntA(Number(e.target.value))} />
            <input type="range" min={x_min} max={x_max} step={(x_max - x_min) / 1000} value={intB} onChange={(e) => onIntB(Number(e.target.value))} />
          </div>
          <div className="slider-block">
            <div className="slider-head">
              <span>Area under curve</span>
              <span className="accent">
                ∫<sub>a</sub>
                <sup>b</sup> f(x) dx = <b>{areaValue != null ? areaValue.toFixed(4) : '—'}</b>
              </span>
            </div>
          </div>
          <p className="sim-hint">Shaded region is the definite integral from a to b (drag both sliders or press Play).</p>
        </div>
      )}
    </div>
  )
}
