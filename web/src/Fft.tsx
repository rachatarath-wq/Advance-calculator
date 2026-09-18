import { useEffect, useMemo, useState } from 'react'
import Katex from './Katex'
import Plot from './Plot'
import { fftSpectrum, ensureEngine, type FftResult } from './engine'

const C = {
  signal: '#4f8cff',
  spectrum: '#34d399',
  peak: '#ff5c8a',
}

function fmt(v: number, digits = 4) {
  return Number(v.toFixed(digits))
}

interface Component {
  f: number
  a: number
  p: number
}

const DEFAULT_COMPONENTS: Component[] = [
  { f: 50, a: 1.0, p: 0 },
  { f: 120, a: 0.6, p: 0 },
  { f: 300, a: 0.25, p: 0 },
]

type View = 'signal' | 'spectrum'

export default function FftPanel() {
  const [fs, setFs] = useState(1024)
  const [n, setN] = useState(512)
  const [comps, setComps] = useState<Component[]>(DEFAULT_COMPONENTS)
  const [view, setView] = useState<View>('signal')
  const [result, setResult] = useState<FftResult | null>(null)

  useEffect(() => {
    let cancelled = false
    const t = setTimeout(async () => {
      try {
        await ensureEngine()
        const r = fftSpectrum(
          fs,
          n,
          comps.map((c) => c.f),
          comps.map((c) => c.a),
          comps.map((c) => c.p),
        )
        if (!cancelled) setResult(r)
      } catch {
        if (!cancelled) setResult(null)
      }
    }, 200)
    return () => {
      cancelled = true
      clearTimeout(t)
    }
  }, [fs, n, comps])

  const setComp = (i: number, key: keyof Component, value: number) => {
    setComps((prev) => prev.map((c, j) => (j === i ? { ...c, [key]: value } : c)))
  }

  const plotData = useMemo(() => {
    if (!result?.ok) return []
    if (view === 'signal') {
      return [
        {
          x: result.ts,
          y: result.xs,
          type: 'scatter',
          mode: 'lines',
          name: 'x(t)',
          line: { color: C.signal, width: 2 },
          hovertemplate: 't = %{x:.4f} s<br>x = %{y:.4f}<extra></extra>',
        },
      ]
    }
    // Amplitude spectrum (one-sided), with detected peaks highlighted.
    const peaks = {
      x: result.peak_freqs,
      y: result.peak_amps,
      type: 'scatter',
      mode: 'markers',
      name: 'peaks',
      marker: { color: C.peak, size: 9, symbol: 'x' },
      hovertemplate: 'f = %{x:.3f} Hz<br>A = %{y:.4f}<extra>peak</extra>',
    }
    return [
      {
        x: result.spectrum_freqs,
        y: result.spectrum_amp,
        type: 'scatter',
        mode: 'lines',
        name: '|amplitude|',
        line: { color: C.spectrum, width: 2 },
        hovertemplate: 'f = %{x:.3f} Hz<br>A = %{y:.4f}<extra></extra>',
      },
      peaks,
    ]
  }, [result, view])

  const plotLayout = useMemo(() => {
    if (!result?.ok) return {}
    return {
      margin: { l: 52, r: 16, t: 24, b: 44 },
      paper_bgcolor: 'transparent',
      plot_bgcolor: 'transparent',
      font: { color: '#cbd5e1', family: 'Inter, system-ui, -apple-system, sans-serif', size: 12 },
      xaxis: {
        title: {
          text: view === 'signal' ? 't (s)' : 'frequency (Hz)',
          font: { color: '#8ea0b8' },
        },
        zeroline: true,
        zerolinecolor: '#334155',
        gridcolor: '#1e293b',
      },
      yaxis: {
        title: { text: view === 'signal' ? 'x(t)' : 'amplitude', font: { color: '#8ea0b8' } },
        zeroline: true,
        zerolinecolor: '#334155',
        gridcolor: '#1e293b',
      },
      showlegend: true,
      legend: { orientation: 'h', y: 1.12, x: 0, bgcolor: 'transparent', font: { size: 12 } },
      hovermode: 'closest',
    }
  }, [result, view])

  const ok = result?.ok ?? false

  return (
    <div className="layout">
      <aside className="sidebar">
        <section className="card">
          <h2>Signal (sum of cosines)</h2>
          <div className="ac-grid">
            <NumField label="Sample rate fs (Hz)" value={fs} onChange={setFs} min={1} />
            <NumField label="Samples n" value={n} onChange={setN} min={16} max={8192} step={1} />
          </div>
          <div className="component-list">
            {comps.map((c, i) => (
              <div className="component-row" key={i}>
                <span className="component-title">x<sub>{i + 1}</sub></span>
                <NumField label="f (Hz)" value={c.f} onChange={(v) => setComp(i, 'f', v)} min={0} />
                <NumField label="A" value={c.a} onChange={(v) => setComp(i, 'a', v)} min={0} />
                <NumField label="φ (°)" value={c.p} onChange={(v) => setComp(i, 'p', v)} />
              </div>
            ))}
          </div>
          <p className="sim-hint">
            x(t) = Σᵢ Aᵢ·cos(2π fᵢ t + φᵢ). Choose fᵢ on exact bins (multiples of Δf = fs/n) to
            avoid spectral leakage.
          </p>
        </section>

        <section className="card">
          <h2>FFT parameters</h2>
          {!ok ? (
            <p className="error-text">{result?.error ?? 'Unable to compute.'}</p>
          ) : (
            <>
              <div className="ac-formula">
                <Katex block tex={result!.twiddle_latex} />
                <Katex block tex={result!.dft_latex} />
              </div>
              <div className="stat-grid">
                <Stat label="N (padded)" value={result!.n_padded} unit="" />
                <Stat label="Δf" value={result!.df} unit="Hz" />
                <Stat label="Nyquist" value={result!.nyquist} unit="Hz" />
                <Stat label="max |FFT−DFT|" value={result!.dft_max_err} unit="" accent />
              </div>
            </>
          )}
        </section>

        <section className="card">
          <h2>Detected peaks</h2>
          {!ok ? (
            <p className="sim-hint">Run the FFT to see peaks.</p>
          ) : result!.peak_freqs.length === 0 ? (
            <p className="sim-hint">No peaks above threshold.</p>
          ) : (
            <div className="peak-list">
              {result!.peak_freqs.map((f, i) => (
                <div className="peak-row" key={i}>
                  <span className="peak-freq">{f.toFixed(3)} Hz</span>
                  <span className="peak-amp">A = {fmt(result!.peak_amps[i])}</span>
                </div>
              ))}
            </div>
          )}
        </section>
      </aside>

      <section className="plot-panel">
        <div className="sim-tabs">
          <button className={view === 'signal' ? 'active' : ''} onClick={() => setView('signal')}>
            Signal x(t)
          </button>
          <button className={view === 'spectrum' ? 'active' : ''} onClick={() => setView('spectrum')}>
            Amplitude spectrum
          </button>
        </div>

        <Plot data={plotData} layout={plotLayout} />

        {ok && (
          <div className="sim-controls">
            <div className="sliders">
              <p className="sim-hint">
                <Katex tex={result!.result_latex} />
              </p>
              <p className="sim-hint">
                Radix-2 Cooley–Tukey DIT collapses O(N²) → O(N log N) using the half-turn symmetry
                of the twiddle phasors (Euler's formula). The spectrum is recovered by the
                dependency-free Rust FFT and cross-checked against the naive O(N²) DFT.
              </p>
            </div>
          </div>
        )}
      </section>
    </div>
  )
}

function NumField({
  label,
  value,
  onChange,
  min,
  max,
  step,
}: {
  label: string
  value: number
  onChange: (v: number) => void
  min?: number
  max?: number
  step?: number
}) {
  return (
    <label className="num-field">
      <span>{label}</span>
      <input
        type="number"
        value={value}
        min={min}
        max={max}
        step={step}
        onChange={(e) => onChange(Number(e.target.value))}
      />
    </label>
  )
}

function Stat({ label, value, unit, accent }: { label: string; value: number; unit: string; accent?: boolean }) {
  return (
    <div className={'stat' + (accent ? ' stat-accent' : '')}>
      <span className="stat-label">{label}</span>
      <span className="stat-value">
        {fmt(value)}
        {unit && <small> {unit}</small>}
      </span>
    </div>
  )
}
