import { useEffect, useMemo, useState } from 'react'
import Katex from './Katex'
import Plot from './Plot'
import {
  chopperPower,
  rlChopper,
  ensureEngine,
  type ChopperPowerResult,
  type RlChopperResult,
} from './engine'

const C = {
  v: '#4f8cff',
  i: '#ffb14f',
  p: '#34d399',
  pavg: '#ff5c8a',
}

function fmt(v: number, digits = 4) {
  return Number(v.toFixed(digits))
}

/** Robust y-range spanning the chopped waveforms. */
function yRange(vs: number[], iVals: number[], ps: number[]): [number, number] {
  let m = 0
  for (const v of vs) if (Number.isFinite(v)) m = Math.max(m, Math.abs(v))
  for (const v of iVals) if (Number.isFinite(v)) m = Math.max(m, Math.abs(v))
  for (const v of ps) if (Number.isFinite(v)) m = Math.max(m, Math.abs(v))
  m = m * 1.15 || 10
  return [-m, m]
}

type Mode = 'resistive' | 'rl'

export default function ChopperPowerPanel() {
  const [mode, setMode] = useState<Mode>('resistive')
  const [vPeak, setVPeak] = useState(311) // ≈ 220 Vrms · √2
  const [loadR, setLoadR] = useState(50) // Ω
  const [frequency, setFrequency] = useState(50)
  // resistive (conduction window α → β)
  const [alpha, setAlpha] = useState(30) // °
  const [beta, setBeta] = useState(150) // °
  // RL (firing angle α)
  const [loadL, setLoadL] = useState(0.0919) // H → φ ≈ 30°
  const [alphaRl, setAlphaRl] = useState(90) // °
  const [result, setResult] = useState<ChopperPowerResult | RlChopperResult | null>(null)

  useEffect(() => {
    let cancelled = false
    const t = setTimeout(async () => {
      try {
        await ensureEngine()
        const r =
          mode === 'resistive'
            ? chopperPower(vPeak, loadR, frequency, alpha, beta, 800)
            : rlChopper(vPeak, loadR, loadL, frequency, alphaRl, 800)
        if (!cancelled) setResult(r)
      } catch {
        if (!cancelled) setResult(null)
      }
    }, 200)
    return () => {
      cancelled = true
      clearTimeout(t)
    }
  }, [mode, vPeak, loadR, frequency, alpha, beta, loadL, alphaRl])

  const plotData = useMemo(() => {
    if (!result?.ok) return []
    const { ts, vs, i_vals, ps, avg_power, t0, t1 } = result
    return [
      { x: ts, y: vs, type: 'scatter', mode: 'lines', name: 'v(t) [V]', line: { color: C.v, width: 2 } },
      { x: ts, y: i_vals, type: 'scatter', mode: 'lines', name: 'i(t) [A]', line: { color: C.i, width: 2, dash: 'dot' } },
      {
        x: ts,
        y: ps,
        type: 'scatter',
        mode: 'lines',
        name: 'p(t) [W]',
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
        range: yRange(result.vs, result.i_vals, result.ps),
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
  const rl = mode === 'rl' ? (result as RlChopperResult | null) : null

  return (
    <div className="layout">
      <aside className="sidebar">
        <section className="card">
          <h2>Chopper (phase control)</h2>
          <div className="sim-tabs" style={{ marginBottom: 12 }}>
            <button className={mode === 'resistive' ? 'active' : ''} onClick={() => setMode('resistive')}>
              Resistive (α → β)
            </button>
            <button className={mode === 'rl' ? 'active' : ''} onClick={() => setMode('rl')}>
              RL / motor (firing α)
            </button>
          </div>
          <div className="ac-grid">
            <NumField label="V peak (V)" value={vPeak} onChange={setVPeak} min={0.001} />
            <NumField label="Load R (Ω)" value={loadR} onChange={setLoadR} min={0.001} />
            <NumField label="Frequency (Hz)" value={frequency} onChange={setFrequency} min={0.001} />
            {mode === 'resistive' ? (
              <>
                <NumField label="α start (deg)" value={alpha} onChange={setAlpha} min={0} max={180} step={1} />
                <NumField label="β end (deg)" value={beta} onChange={setBeta} min={0} max={180} step={1} />
              </>
            ) : (
              <>
                <NumField label="Load L (H)" value={loadL} onChange={setLoadL} min={0.001} />
                <NumField label="Firing α (deg)" value={alphaRl} onChange={setAlphaRl} min={0} max={180} step={1} />
              </>
            )}
          </div>
          <p className="sim-hint">
            {mode === 'resistive'
              ? 'Load conducts only while θ = ωt (mod π) ∈ [α, β] within each half-cycle.'
              : 'TRIAC fires at α; the inductive current persists past the zero crossing to the extinction angle β′.'}
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
                {mode === 'resistive' ? (
                  <Stat label="Conduction duty" value={(result as ChopperPowerResult).conduction_duty} unit="(β−α)/π" />
                ) : (
                  <>
                    <Stat label="Power factor" value={rl!.power_factor} unit="PF" />
                    <Stat label="Extinction β′" value={rl!.extinction_deg} unit="deg" />
                    <Stat label="Conduction" value={rl!.conduction_deg} unit="deg" />
                  </>
                )}
              </div>
              {mode === 'rl' && <p className="sim-hint">{rl!.note}</p>}
            </>
          )}
        </section>
      </aside>

      <section className="plot-panel">
        <div className="sim-tabs" style={{ pointerEvents: 'none' }}>
          <button className="active">
            {mode === 'resistive' ? 'Chopped sine · P = (1/T)∫ v²/R dt' : 'Phase control · P = (1/π)∫ v·i dθ'}
          </button>
        </div>
        <Plot data={plotData} layout={plotLayout} />
        <p className="sim-hint">
          {mode === 'resistive'
            ? 'The sine wave is chopped — outside [α, β] the voltage is zero. The shaded power p(t) = v²/R is integrated over a full period.'
            : 'Fired at α, the current flows until β′ (dashed region). The shaded p(t) = v·i can go negative past π (energy returned to source); the average is the real power.'}
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
        <small> {unit}</small>
      </span>
    </div>
  )
}
