import { useEffect, useMemo, useState } from 'react'
import Katex from './Katex'
import Plot from './Plot'
import { acPower, ensureEngine, type AcPowerResult } from './engine'

const C = {
  v: '#4f8cff',
  i: '#ffb14f',
  p: '#34d399',
  pavg: '#ff5c8a',
}

function fmt(v: number, digits = 4) {
  return Number(v.toFixed(digits))
}

/** Robust y-range spanning all AC waveforms. */
function acYRange(vs: number[], iVals: number[], ps: number[]): [number, number] {
  let m = 0
  for (const v of vs) if (Number.isFinite(v)) m = Math.max(m, Math.abs(v))
  for (const v of iVals) if (Number.isFinite(v)) m = Math.max(m, Math.abs(v))
  for (const v of ps) if (Number.isFinite(v)) m = Math.max(m, Math.abs(v))
  m = m * 1.15 || 10
  return [-m, m]
}

export default function AcPowerPanel() {
  const [vPeak, setVPeak] = useState(311) // ≈ 220 Vrms · √2
  const [iPeak, setIPeak] = useState(2)
  const [frequency, setFrequency] = useState(50)
  const [phaseDeg, setPhaseDeg] = useState(30)
  const [result, setResult] = useState<AcPowerResult | null>(null)

  useEffect(() => {
    let cancelled = false
    const t = setTimeout(async () => {
      try {
        await ensureEngine()
        const r = acPower(vPeak, iPeak, frequency, phaseDeg, 600)
        if (!cancelled) setResult(r)
      } catch {
        if (!cancelled) setResult(null)
      }
    }, 200)
    return () => {
      cancelled = true
      clearTimeout(t)
    }
  }, [vPeak, iPeak, frequency, phaseDeg])

  const plotData = useMemo(() => {
    if (!result?.ok) return []
    const { ts, vs, i_vals, ps, avg_power, t0, t1 } = result
    return [
      { x: ts, y: vs, type: 'scatter', mode: 'lines', name: 'v(t)  [V]', line: { color: C.v, width: 2 } },
      { x: ts, y: i_vals, type: 'scatter', mode: 'lines', name: 'i(t)  [A]', line: { color: C.i, width: 2, dash: 'dot' } },
      {
        x: ts,
        y: ps,
        type: 'scatter',
        mode: 'lines',
        name: 'p(t)  [W]',
        line: { color: C.p, width: 2.2 },
        fill: 'tozeroy',
        fillcolor: 'rgba(52, 211, 153, 0.18)',
      },
      {
        x: [t0, t1],
        y: [avg_power, avg_power],
        type: 'scatter',
        mode: 'lines',
        name: 'Pₐᵥₑ (avg)',
        line: { color: C.pavg, width: 2, dash: 'dash' },
        hovertemplate: 'P = %{y:.3f} W<extra></extra>',
      },
    ]
  }, [result])

  const plotLayout = useMemo(() => {
    if (!result?.ok) return {}
    return {
      margin: { l: 52, r: 16, t: 24, b: 44 },
      paper_bgcolor: 'transparent',
      plot_bgcolor: 'transparent',
      font: { color: '#cbd5e1', family: 'Inter, system-ui, -apple-system, sans-serif', size: 12 },
      xaxis: {
        title: { text: 't (s)', font: { color: '#8ea0b8' } },
        zeroline: true,
        zerolinecolor: '#334155',
        gridcolor: '#1e293b',
      },
      yaxis: {
        range: acYRange(result.vs, result.i_vals, result.ps),
        zeroline: true,
        zerolinecolor: '#334155',
        gridcolor: '#1e293b',
      },
      showlegend: true,
      legend: { orientation: 'h', y: 1.12, x: 0, bgcolor: 'transparent', font: { size: 12 } },
      hovermode: 'closest',
    }
  }, [result])

  const ok = result?.ok ?? false

  return (
    <div className="layout">
      <aside className="sidebar">
        <section className="card">
          <h2>AC circuit</h2>
          <div className="ac-grid">
            <NumField label="V peak (V)" value={vPeak} onChange={setVPeak} />
            <NumField label="I peak (A)" value={iPeak} onChange={setIPeak} />
            <NumField label="Frequency (Hz)" value={frequency} onChange={setFrequency} min={0.001} />
            <NumField label="Phase φ (deg)" value={phaseDeg} onChange={setPhaseDeg} />
          </div>
          <p className="sim-hint">
            v(t) = V<sub>m</sub>·sin(ωt), &nbsp; i(t) = I<sub>m</sub>·sin(ωt + φ)
          </p>
        </section>

        <section className="card">
          <h2>Power results</h2>
          {!ok ? (
            <p className="error-text">{result?.error ?? 'Unable to compute.'}</p>
          ) : (
            <>
              <div className="ac-formula">
                <Katex block tex={result!.v_latex} />
                <Katex block tex={result!.i_latex} />
                <Katex block tex={result!.integral_latex} />
              </div>
              <div className="stat-grid">
                <Stat label="P (real power)" value={result!.avg_power} unit="W" accent />
                <Stat label="V rms" value={result!.rms_v} unit="V" />
                <Stat label="I rms" value={result!.rms_i} unit="A" />
                <Stat label="S (apparent)" value={result!.apparent_power} unit="VA" />
                <Stat label="Q (reactive)" value={result!.reactive_power} unit="VAR" />
                <Stat label="Power factor" value={result!.power_factor} unit="cos φ" />
              </div>
            </>
          )}
        </section>
      </aside>

      <section className="plot-panel">
        <div className="sim-tabs" style={{ pointerEvents: 'none' }}>
          <button className="active">Average power via ∫ v·i dt</button>
        </div>
        <Plot data={plotData} layout={plotLayout} />
        <p className="sim-hint">
          The shaded area is the energy over two periods; the dashed line is the average (real) power
          P = (1/T)∫₀ᵀ v(t)·i(t) dt. With a phase shift φ, P = V<sub>rms</sub>·I<sub>rms</sub>·cos φ.
        </p>
      </section>
    </div>
  )
}

function NumField({
  label,
  value,
  onChange,
  min,
}: {
  label: string
  value: number
  onChange: (v: number) => void
  min?: number
}) {
  return (
    <label className="num-field">
      <span>{label}</span>
      <input type="number" value={value} min={min} onChange={(e) => onChange(Number(e.target.value))} />
    </label>
  )
}

function Stat({ label, value, unit, accent }: { label: string; value: number; unit: string; accent?: boolean }) {
  return (
    <div className={'stat' + (accent ? ' stat-accent' : '')}>
      <span className="stat-label">{label}</span>
      <span className="stat-value">
        {fmt(value)}
        <small> {unit}</small>
      </span>
    </div>
  )
}
