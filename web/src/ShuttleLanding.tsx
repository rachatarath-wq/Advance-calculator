import { useEffect, useMemo, useState } from 'react'
import Katex from './Katex'
import Plot from './Plot'
import { shuttleLanding, ensureEngine, type ShuttleSim } from './engine'

const C = {
  y: '#4f8cff',
  v: '#ffb14f',
  gamma: '#34d399',
  alpha: '#ff5c8a',
  flare: '#8ea0b8',
}

function fmt(v: number, digits = 3) {
  return Number(v.toFixed(digits))
}

type View = 'profile' | 'speed' | 'angles'

export default function ShuttleLandingPanel() {
  const [h0, setH0] = useState(3000)
  const [v0, setV0] = useState(135)
  const [gamma0, setGamma0] = useState(-12)
  const [flareAlt, setFlareAlt] = useState(350)
  const [alphaFlare, setAlphaFlare] = useState(10)
  const [view, setView] = useState<View>('profile')
  const [result, setResult] = useState<ShuttleSim | null>(null)

  useEffect(() => {
    let cancelled = false
    const t = setTimeout(async () => {
      try {
        await ensureEngine()
        const r = shuttleLanding(h0, v0, gamma0, flareAlt, alphaFlare)
        if (!cancelled) setResult(r)
      } catch {
        if (!cancelled) setResult(null)
      }
    }, 200)
    return () => {
      cancelled = true
      clearTimeout(t)
    }
  }, [h0, v0, gamma0, flareAlt, alphaFlare])

  const plotData = useMemo(() => {
    if (!result?.ok) return []
    if (view === 'profile') {
      const td = {
        x: [result.x_touch],
        y: [3],
        type: 'scatter',
        mode: 'markers',
        name: 'touchdown',
        marker: { color: C.alpha, size: 11, symbol: 'x' },
        hovertemplate: 'x = %{x:.0f} m<extra>touchdown</extra>',
      }
      const flare = {
        x: [result.xs[0], result.xs[result.xs.length - 1]],
        y: [result.flare_alt, result.flare_alt],
        type: 'scatter',
        mode: 'lines',
        name: 'flare altitude',
        line: { color: C.flare, width: 1.5, dash: 'dash' },
        hoverinfo: 'skip',
      }
      return [
        {
          x: result.xs,
          y: result.ys,
          type: 'scatter',
          mode: 'lines',
          name: 'altitude y(x)',
          line: { color: C.y, width: 2.5 },
          hovertemplate: 'x = %{x:.0f} m<br>y = %{y:.0f} m<extra></extra>',
        },
        flare,
        td,
      ]
    }
    if (view === 'speed') {
      return [
        {
          x: result.ts,
          y: result.vs,
          type: 'scatter',
          mode: 'lines',
          name: 'v (m/s)',
          line: { color: C.v, width: 2.5 },
          hovertemplate: 't = %{x:.1f} s<br>v = %{y:.1f} m/s<extra></extra>',
        },
      ]
    }
    return [
      {
        x: result.ts,
        y: result.gammas,
        type: 'scatter',
        mode: 'lines',
        name: 'γ (deg)',
        line: { color: C.gamma, width: 2.2 },
        hovertemplate: 't = %{x:.1f} s<br>γ = %{y:.2f}°<extra></extra>',
      },
      {
        x: result.ts,
        y: result.alphas,
        type: 'scatter',
        mode: 'lines',
        name: 'α (deg)',
        line: { color: C.alpha, width: 2.2, dash: 'dot' },
        hovertemplate: 't = %{x:.1f} s<br>α = %{y:.2f}°<extra></extra>',
      },
    ]
  }, [result, view])

  const plotLayout = useMemo(() => {
    if (!result?.ok) return {}
    const axisTitle = view === 'profile' ? 'downrange x (m)' : 'time t (s)'
    const yTitle = view === 'profile' ? 'altitude y (m)' : view === 'speed' ? 'speed (m/s)' : 'angle (deg)'
    return {
      margin: { l: 52, r: 16, t: 24, b: 44 },
      paper_bgcolor: 'transparent',
      plot_bgcolor: 'transparent',
      font: { color: '#cbd5e1', family: 'Inter, system-ui, -apple-system, sans-serif', size: 12 },
      xaxis: {
        title: { text: axisTitle, font: { color: '#8ea0b8' } },
        zeroline: true,
        zerolinecolor: '#334155',
        gridcolor: '#1e293b',
      },
      yaxis: {
        title: { text: yTitle, font: { color: '#8ea0b8' } },
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
          <h2>Approach & flare</h2>
          <div className="ac-grid">
            <NumField label="Altitude h₀ (m)" value={h0} onChange={setH0} min={10} />
            <NumField label="Speed v₀ (m/s)" value={v0} onChange={setV0} min={1} />
            <NumField label="Glide path γ₀ (deg)" value={gamma0} onChange={setGamma0} min={-89} max={-0.1} />
            <NumField label="Flare altitude (m)" value={flareAlt} onChange={setFlareAlt} min={1} />
            <NumField label="Max flare α (deg)" value={alphaFlare} onChange={setAlphaFlare} min={0.1} max={45} />
          </div>
          <p className="sim-hint">
            Unpowered glide from h₀; below the flare altitude the path rounds out toward a shallow
            touchdown angle, with the angle of attack capped at α.
          </p>
        </section>

        <section className="card">
          <h2>Touchdown telemetry</h2>
          {!ok ? (
            <p className="error-text">{result?.error ?? 'Unable to simulate.'}</p>
          ) : (
            <>
              <div className="stat-grid">
                <Stat label="Time" value={result!.t_touch} unit="s" />
                <Stat label="Downrange" value={result!.x_touch} unit="m" />
                <Stat label="Speed v" value={result!.v_touch} unit="m/s" />
                <Stat label="Sink rate" value={result!.sink_rate} unit="m/s" accent />
                <Stat label="Path γ" value={result!.gamma_touch_deg} unit="deg" />
                <Stat label="AoA α" value={result!.alpha_touch_deg} unit="deg" />
              </div>
              <div className={'verdict' + (result!.safe ? ' verdict-safe' : ' verdict-unsafe')}>
                {result!.safe ? '✓ SAFE LANDING' : '✗ UNSAFE'}
              </div>
            </>
          )}
        </section>

        <section className="card">
          <h2>Safety checks</h2>
          {!ok ? (
            <p className="sim-hint">—</p>
          ) : (
            <div className="check-list">
              {result!.checks.map((c, i) => (
                <div className="check-row" key={i}>
                  <span className={'badge' + (c.pass ? ' badge-pass' : ' badge-fail')}>
                    {c.pass ? '✓' : '✗'}
                  </span>
                  <span className="check-name">{c.name}</span>
                  <span className="check-value">
                    {c.name === 'touchdown speed'
                      ? `${fmt(c.value)} m/s (60–130)`
                      : c.name === 'sink rate'
                        ? `${fmt(c.value)} m/s (≤ 1.5)`
                        : c.name === 'pitch attitude'
                          ? `${fmt(c.value)}° (≤ 25)`
                          : `${fmt(c.value)} m`}
                  </span>
                </div>
              ))}
            </div>
          )}
        </section>

        <section className="card">
          <h2>Short-period mode</h2>
          {ok && (
            <>
              <div className="ac-formula">
                <Katex block tex={result!.phasor_latex} />
              </div>
              <div className="stat-grid">
                <Stat label="σ (decay)" value={result!.sigma} unit="/s" />
                <Stat label="ω_d" value={result!.omega_d} unit="rad/s" />
                <Stat label="Period" value={result!.period_d} unit="s" />
              </div>
            </>
          )}
        </section>
      </aside>

      <section className="plot-panel">
        <div className="sim-tabs">
          <button className={view === 'profile' ? 'active' : ''} onClick={() => setView('profile')}>
            Altitude profile
          </button>
          <button className={view === 'speed' ? 'active' : ''} onClick={() => setView('speed')}>
            Speed
          </button>
          <button className={view === 'angles' ? 'active' : ''} onClick={() => setView('angles')}>
            γ & α
          </button>
        </div>

        <Plot data={plotData} layout={plotLayout} />

        {ok && (
          <>
            <div className="sim-controls">
              <div className="sliders">
                <p className="sim-hint">
                  <Katex tex={result!.eom_latex} />
                </p>
                <p className="sim-hint">
                  Point-mass flight dynamics integrated with Euler / Heun / RK4. The flare uses PI
                  flight-path control; the pitch mode is an underdamped oscillator whose decaying
                  motion is a phasor (Euler's formula).
                </p>
              </div>
            </div>

            <div className="method-table">
              <div className="method-head">
                <span>Integrator (Δt = 0.2 s)</span>
                <span>Sink rate</span>
                <span>v at touchdown</span>
              </div>
              {result!.methods.map((m, i) => (
                <div className="method-row" key={i}>
                  <span className="method-name">{m.name}</span>
                  <span className="method-sink">{fmt(m.sink_rate, 4)} m/s</span>
                  <span className="method-v">{fmt(m.v_touch, 2)} m/s</span>
                </div>
              ))}
            </div>
          </>
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
