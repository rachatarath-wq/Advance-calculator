import { useEffect, useMemo, useRef, useState, type ChangeEvent } from 'react'
import Katex from './Katex'
import Plot from './Plot'
import { moonMission, ensureEngine, type MoonMission } from './engine'

const C = {
  trail: '#64748b',
  path: '#ffb14f',
  earth: '#4f8cff',
  moon: '#94a3b8',
  target: '#34d399',
  speed: '#4f8cff',
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

/** Name of the mission phase active at time `t`. */
function phaseAt(phases: MoonMission['phases'], t: number): string {
  let cur = phases.length ? phases[0].name : ''
  for (const p of phases) if (p.t <= t) cur = p.name
  return cur
}

type View = 'trajectory' | 'speed' | 'realtime'

export default function MoonMissionPanel() {
  const [targetDeg, setTargetDeg] = useState(0)
  const [thrustG, setThrustG] = useState(4)
  const [view, setView] = useState<View>('trajectory')
  const [result, setResult] = useState<MoonMission | null>(null)
  const [simTime, setSimTime] = useState(0)
  const [playing, setPlaying] = useState(false)
  const [speed, setSpeed] = useState(3000)
  const simTimeRef = useRef(0)

  const endTime = result?.ok ? result.ts[result.ts.length - 1] : 0

  // Reset playback whenever the trajectory changes.
  useEffect(() => {
    simTimeRef.current = 0
    setSimTime(0)
    setPlaying(false)
  }, [result])

  // Realtime playback loop — advances simTime at `speed` sim-seconds per real second.
  useEffect(() => {
    if (!playing || !result?.ok) return
    let raf = 0
    let last = performance.now()
    const step = (now: number) => {
      const dt = (now - last) / 1000
      last = now
      const next = simTimeRef.current + dt * speed
      if (next >= endTime) {
        simTimeRef.current = endTime
        setSimTime(endTime)
        setPlaying(false)
        return
      }
      simTimeRef.current = next
      setSimTime(next)
      raf = requestAnimationFrame(step)
    }
    raf = requestAnimationFrame(step)
    return () => cancelAnimationFrame(raf)
  }, [playing, speed, result, endTime])

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
    const { ts, xs, ys, speeds, thrusts, earth_r } = result
    const x = sampleAt(ts, xs, simTime)
    const y = sampleAt(ts, ys, simTime)
    const dist = Math.hypot(x, y)
    return {
      t: simTime,
      x,
      y,
      speed: sampleAt(ts, speeds, simTime),
      thrust: sampleAt(ts, thrusts, simTime),
      dist,
      alt: dist - earth_r,
      phase: phaseAt(result.phases, simTime),
    }
  }, [result, simTime])

  useEffect(() => {
    let cancelled = false
    const t = setTimeout(async () => {
      try {
        await ensureEngine()
        const r = moonMission(targetDeg, thrustG)
        if (!cancelled) setResult(r)
      } catch {
        if (!cancelled) setResult(null)
      }
    }, 200)
    return () => {
      cancelled = true
      clearTimeout(t)
    }
  }, [targetDeg, thrustG])

  // Earth + Moon as Plotly circle shapes (data coordinates).
  const bodiesShapes = useMemo(() => {
    if (!result?.ok) return []
    const { earth_r, moon_r, moon_x } = result
    return [
      {
        type: 'circle',
        xref: 'x',
        yref: 'y',
        x0: -earth_r,
        y0: -earth_r,
        x1: earth_r,
        y1: earth_r,
        fillcolor: 'rgba(79, 140, 255, 0.18)',
        line: { color: C.earth, width: 1.5 },
      },
      {
        type: 'circle',
        xref: 'x',
        yref: 'y',
        x0: moon_x - moon_r,
        y0: -moon_r,
        x1: moon_x + moon_r,
        y1: moon_r,
        fillcolor: 'rgba(148, 163, 184, 0.18)',
        line: { color: C.moon, width: 1.5 },
      },
    ]
  }, [result])

  const plotData = useMemo(() => {
    if (!result?.ok) return []
    const last = result.waypoints_x.length - 1
    const landX = result.waypoints_x[last]
    const landY = result.waypoints_y[last]

    const waypointTrace = {
      x: result.waypoints_x.slice(1, last),
      y: result.waypoints_y.slice(1, last),
      type: 'scatter',
      mode: 'markers',
      name: 'flight plan',
      marker: { color: '#64748b', size: 4, symbol: 'circle-open' },
      hoverinfo: 'skip',
    }
    const landTrace = {
      x: [landX],
      y: [landY],
      type: 'scatter',
      mode: 'markers',
      name: 'landing site',
      marker: { color: C.target, size: 8, symbol: 'star' },
      hoverinfo: 'skip',
    }

    if (view === 'speed') {
      return [
        {
          x: result.ts.map((t) => t / 3600),
          y: result.speeds,
          type: 'scatter',
          mode: 'lines',
          name: 'speed',
          line: { color: C.speed, width: 2.5 },
          hovertemplate: 't = %{x:.2f} h<br>v = %{y:.2f} km/s<extra></extra>',
        },
      ]
    }

    if (view === 'realtime') {
      const idx = indexAt(result.ts, simTime)
      const start = Math.max(0, idx - 150)
      const x = sampleAt(result.ts, result.xs, simTime)
      const y = sampleAt(result.ts, result.ys, simTime)
      return [
        {
          x: result.xs,
          y: result.ys,
          type: 'scatter',
          mode: 'lines',
          name: 'full trajectory',
          line: { color: C.trail, width: 1 },
          hoverinfo: 'skip',
        },
        {
          x: result.xs.slice(start, idx + 1),
          y: result.ys.slice(start, idx + 1),
          type: 'scatter',
          mode: 'lines',
          name: 'flown path',
          line: { color: C.path, width: 2.5 },
          hoverinfo: 'skip',
        },
        {
          x: [x],
          y: [y],
          type: 'scatter',
          mode: 'text',
          text: ['🚀'],
          textfont: { size: 22 },
          name: 'shuttle',
          hovertemplate: 'x = %{x:.0f} km<br>y = %{y:.0f} km<extra>shuttle</extra>',
        },
        waypointTrace,
        landTrace,
      ]
    }

    return [
      {
        x: result.xs,
        y: result.ys,
        type: 'scatter',
        mode: 'lines',
        name: 'trajectory',
        line: { color: C.path, width: 2 },
        hovertemplate: 'x = %{x:.0f} km<br>y = %{y:.0f} km<extra></extra>',
      },
      waypointTrace,
      landTrace,
    ]
  }, [result, view, simTime])

  const plotLayout = useMemo(() => {
    if (!result?.ok) return {}
    const isSpatial = view === 'trajectory' || view === 'realtime'
    const xTitle = isSpatial ? 'x (km)' : 'time t (hours)'
    const yTitle = isSpatial ? 'y (km)' : 'speed (km/s)'

    const xaxis: any = {
      title: { text: xTitle, font: { color: '#8ea0b8' } },
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

    const layout: any = {
      margin: { l: 52, r: 16, t: 24, b: 44 },
      paper_bgcolor: 'transparent',
      plot_bgcolor: 'transparent',
      font: { color: '#cbd5e1', family: 'Inter, system-ui, -apple-system, sans-serif', size: 12 },
      xaxis,
      yaxis,
      showlegend: true,
      legend: { orientation: 'h', y: 1.12, x: 0, bgcolor: 'transparent', font: { size: 12 } },
      hovermode: 'closest',
    }

    if (isSpatial) {
      const { moon_x } = result
      xaxis.range = [-60_000, moon_x + 60_000]
      yaxis.range = [-115_000, 115_000]
      layout.shapes = bodiesShapes
      layout.annotations = [
        { x: 0, y: 0, text: '🌍', showarrow: false, font: { size: 22 } },
        { x: moon_x, y: 0, text: '🌕', showarrow: false, font: { size: 18 } },
      ]
      if (view === 'realtime') layout.uirevision = 'moon-rt'
    }

    return layout
  }, [result, view, bodiesShapes])

  const ok = result?.ok ?? false
  const done = ok ? simTime >= endTime - 1e-9 : false

  return (
    <div className="layout">
      <aside className="sidebar">
        <section className="card">
          <h2>Mission parameters</h2>
          <div className="ac-grid">
            <NumField label="Landing site angle (°)" value={targetDeg} onChange={setTargetDeg} min={0} max={360} />
            <NumField label="Max thrust (g)" value={thrustG} onChange={setThrustG} min={0.2} max={15} step={0.1} />
          </div>
          <p className="sim-hint">
            Launch from Earth's surface, fly a lunar swing-by and return to land at the chosen angle
            around Earth (0° = the launch site). Higher thrust shortens the trip.
          </p>
        </section>

        <section className="card">
          <h2>Mission summary</h2>
          {!ok ? (
            <p className="error-text">{result?.error ?? 'Unable to simulate.'}</p>
          ) : (
            <>
              <div className="stat-grid">
                <Stat label="Duration" value={result!.total_time_h} unit="h" />
                <Stat label="Max speed" value={result!.max_speed_km_s} unit="km/s" accent />
                <Stat label="Max distance" value={result!.max_distance_km} unit="km" />
              </div>
              <div className={'verdict' + (result!.landed ? ' verdict-safe' : ' verdict-unsafe')}>
                {result!.landed ? '✓ MISSION COMPLETE' : '✗ INCOMPLETE'}
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
                {[500, 1500, 3000, 10000].map((s) => (
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
              max={endTime}
              step={Math.max(1, endTime / 1000)}
              value={simTime}
              onChange={onScrub}
            />
            {live && (
              <div className="stat-grid">
                <Stat label="t" value={live.t / 3600} unit="h" />
                <Stat label="speed" value={live.speed} unit="km/s" accent />
                <Stat label="altitude" value={live.alt} unit="km" />
                <Stat label="thrust" value={live.thrust} unit="g" />
              </div>
            )}
            <p className="sim-hint">Phase: {live?.phase ?? '—'}</p>
            {done && (
              <div className={'landing-banner' + (result!.landed ? ' landing-ok' : ' landing-crash')}>
                <span className="landing-verdict">
                  {result!.landed ? '✓ MISSION COMPLETE' : '✗ INCOMPLETE'}
                </span>
              </div>
            )}
          </section>
        )}

        <section className="card">
          <h2>Equations of motion</h2>
          {ok && <Katex block tex={result!.eom_latex} />}
        </section>
      </aside>

      <section className="plot-panel">
        <div className="sim-tabs">
          <button className={view === 'trajectory' ? 'active' : ''} onClick={() => setView('trajectory')}>
            Trajectory
          </button>
          <button className={view === 'speed' ? 'active' : ''} onClick={() => setView('speed')}>
            Speed
          </button>
          <button className={view === 'realtime' ? 'active' : ''} onClick={() => setView('realtime')}>
            ▶ Realtime
          </button>
        </div>

        <Plot data={plotData} layout={plotLayout} />

        {ok && (
          <div className="sim-controls">
            <div className="sliders">
              <p className="sim-hint">
                Point-mass shuttle under the inverse-square gravity of Earth <em>and</em> the Moon,
                steered by a velocity-pursuit autopilot through a flight-plan of waypoints (RK4).
              </p>
              <p className="sim-hint">
                Scroll to zoom the trajectory; the true Earth–Moon distance is to scale, so Earth is
                the small blue disc on the left.
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
