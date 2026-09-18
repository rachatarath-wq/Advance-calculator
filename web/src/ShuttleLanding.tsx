import { useEffect, useMemo, useRef, useState, type ChangeEvent } from 'react'
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

/** Largest sample index whose time is <= t (ts is monotonic increasing). */
function indexAt(ts: number[], t: number): number {
  const n = ts.length
  if (n <= 1 || t <= ts[0]) return 0
  if (t >= ts[n - 1]) return n - 1
  let lo = 0
  let hi = n - 1
  while (hi - lo > 1) {
    const mid = (lo + hi) >> 1
    if (ts[mid] <= t) lo = mid
    else hi = mid
  }
  return lo
}

/** Linear-interpolate `vals` at time `t`. */
function sampleAt(ts: number[], vals: number[], t: number): number {
  const n = ts.length
  if (n === 0) return 0
  if (t <= ts[0]) return vals[0]
  if (t >= ts[n - 1]) return vals[n - 1]
  const lo = indexAt(ts, t)
  const frac = (t - ts[lo]) / (ts[lo + 1] - ts[lo])
  return vals[lo] + frac * (vals[lo + 1] - vals[lo])
}

type View = 'profile' | 'speed' | 'angles' | 'realtime'

export default function ShuttleLandingPanel() {
  const [h0, setH0] = useState(3000)
  const [v0, setV0] = useState(135)
  const [gamma0, setGamma0] = useState(-12)
  const [flareAlt, setFlareAlt] = useState(350)
  const [alphaFlare, setAlphaFlare] = useState(10)
  const [view, setView] = useState<View>('profile')
  const [result, setResult] = useState<ShuttleSim | null>(null)
  const [simTime, setSimTime] = useState(0)
  const [playing, setPlaying] = useState(false)
  const [speed, setSpeed] = useState(1)
  const simTimeRef = useRef(0)

  // Reset playback whenever the simulation inputs change (new trajectory).
  useEffect(() => {
    simTimeRef.current = 0
    setSimTime(0)
    setPlaying(false)
  }, [result])

  // Real-time playback loop (requestAnimationFrame), advancing simTime at `speed`×.
  useEffect(() => {
    if (!playing || !result?.ok) return
    let raf = 0
    let last = performance.now()
    const step = (now: number) => {
      const dt = (now - last) / 1000
      last = now
      const next = simTimeRef.current + dt * speed
      if (next >= result.t_touch) {
        simTimeRef.current = result.t_touch
        setSimTime(result.t_touch)
        setPlaying(false)
        return
      }
      simTimeRef.current = next
      setSimTime(next)
      raf = requestAnimationFrame(step)
    }
    raf = requestAnimationFrame(step)
    return () => cancelAnimationFrame(raf)
  }, [playing, speed, result])

  const restart = () => {
    simTimeRef.current = 0
    setSimTime(0)
    setPlaying(true)
  }

  const onScrub = (e: ChangeEvent<HTMLInputElement>) => {
    const t = Number(e.target.value)
    simTimeRef.current = t
    setSimTime(t)
    setPlaying(false)
  }

  const live = useMemo(() => {
    if (!result?.ok) return null
    const { ts, xs, ys, vs, gammas, alphas, sinks } = result
    return {
      t: simTime,
      x: sampleAt(ts, xs, simTime),
      y: sampleAt(ts, ys, simTime),
      v: sampleAt(ts, vs, simTime),
      gamma: sampleAt(ts, gammas, simTime),
      alpha: sampleAt(ts, alphas, simTime),
      sink: sampleAt(ts, sinks, simTime),
    }
  }, [result, simTime])

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
    if (view === 'realtime') {
      const idx = indexAt(result.ts, simTime)
      const start = Math.max(0, idx - 60)
      return [
        {
          x: result.xs,
          y: result.ys,
          type: 'scatter',
          mode: 'lines',
          name: 'trajectory',
          line: { color: C.y, width: 1.5 },
          hoverinfo: 'skip',
        },
        {
          x: result.xs.slice(start, idx + 1),
          y: result.ys.slice(start, idx + 1),
          type: 'scatter',
          mode: 'lines',
          name: 'flown path',
          line: { color: C.alpha, width: 3 },
          hoverinfo: 'skip',
        },
        {
          x: [sampleAt(result.ts, result.xs, simTime)],
          y: [sampleAt(result.ts, result.ys, simTime)],
          type: 'scatter',
          mode: 'markers',
          name: 'shuttle',
          marker: {
            symbol: 'triangle-up',
            size: 18,
            color: '#ffffff',
            line: { color: C.alpha, width: 2 },
          },
          text: [`t = ${simTime.toFixed(1)} s`],
          hovertemplate: '%{text}<br>x = %{x:.0f} m<br>y = %{y:.0f} m<extra></extra>',
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
  }, [result, view, simTime])

  const plotLayout = useMemo(() => {
    if (!result?.ok) return {}
    const isProfile = view === 'profile' || view === 'realtime'
    const axisTitle = isProfile ? 'downrange x (m)' : 'time t (s)'
    const yTitle = isProfile ? 'altitude y (m)' : view === 'speed' ? 'speed (m/s)' : 'angle (deg)'
    const xaxis: any = {
      title: { text: axisTitle, font: { color: '#8ea0b8' } },
      zeroline: true,
      zerolinecolor: '#334155',
      gridcolor: '#1e293b',
    }
    const yaxis: any = {
      title: { text: yTitle, font: { color: '#8ea0b8' } },
      zeroline: true,
      zerolinecolor: '#334155',
      gridcolor: '#1e293b',
    }
    if (view === 'realtime') {
      xaxis.range = [0, result.x_touch]
      yaxis.range = [0, result.h0]
    }
    return {
      margin: { l: 52, r: 16, t: 24, b: 44 },
      paper_bgcolor: 'transparent',
      plot_bgcolor: 'transparent',
      font: { color: '#cbd5e1', family: 'Inter, system-ui, -apple-system, sans-serif', size: 12 },
      xaxis,
      yaxis,
      showlegend: true,
      legend: { orientation: 'h', y: 1.12, x: 0, bgcolor: 'transparent', font: { size: 12 } },
      hovermode: 'closest',
      uirevision: view === 'realtime' ? 'realtime' : undefined,
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

        {view === 'realtime' && ok && (
          <section className="card">
            <h2>Realtime playback</h2>
            <div className="replay-controls">
              <button className="replay-btn" onClick={() => setPlaying((p) => !p)}>
                {playing ? '⏸ Pause' : '▶ Play'}
              </button>
              <button className="replay-btn" onClick={restart}>
                ↺ Restart
              </button>
              <div className="speed-btns">
                {[0.5, 1, 2, 4].map((s) => (
                  <button
                    key={s}
                    className={'speed-btn' + (speed === s ? ' active' : '')}
                    onClick={() => setSpeed(s)}
                  >
                    {s}×
                  </button>
                ))}
              </div>
            </div>
            <input
              type="range"
              className="replay-slider"
              min={0}
              max={result!.t_touch}
              step={0.02}
              value={simTime}
              onChange={onScrub}
            />
            {live && (
              <div className="stat-grid">
                <Stat label="t" value={live.t} unit="s" />
                <Stat label="altitude" value={live.y} unit="m" />
                <Stat label="downrange" value={live.x} unit="m" />
                <Stat label="speed" value={live.v} unit="m/s" />
                <Stat label="sink" value={live.sink} unit="m/s" accent />
                <Stat label="γ" value={live.gamma} unit="deg" />
              </div>
            )}
          </section>
        )}
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
          <button className={view === 'realtime' ? 'active' : ''} onClick={() => setView('realtime')}>
            ▶ Realtime
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
