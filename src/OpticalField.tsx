import { useEffect, useRef, type PointerEvent } from 'react'
import optic from './assets/optic.webp'

export function OpticalField() {
  const stage = useRef<HTMLDivElement>(null)
  const frame = useRef(0)
  const point = useRef({ x: 0, y: 0 })
  useEffect(() => () => cancelAnimationFrame(frame.current), [])
  function move(e: PointerEvent<HTMLDivElement>) {
    if (e.pointerType !== 'mouse' || window.matchMedia('(prefers-reduced-motion: reduce)').matches)
      return
    const box = e.currentTarget.getBoundingClientRect()
    point.current = {
      x: ((e.clientX - box.left) / box.width - 0.5) * 2,
      y: ((e.clientY - box.top) / box.height - 0.5) * 2,
    }
    if (frame.current) return
    frame.current = requestAnimationFrame(() => {
      frame.current = 0
      const { x, y } = point.current
      stage.current?.style.setProperty(
        '--lens-pose',
        'translate(' +
          (x * 4).toFixed(1) +
          'px,' +
          (y * 3).toFixed(1) +
          'px) rotate(' +
          (x * 2.2).toFixed(2) +
          'deg)',
      )
    })
  }
  function rest() {
    cancelAnimationFrame(frame.current)
    frame.current = 0
    stage.current?.style.removeProperty('--lens-pose')
  }
  return (
    <div
      ref={stage}
      className="optical-field"
      aria-hidden="true"
      onPointerMove={move}
      onPointerLeave={rest}
    >
      <div className="draft-plane" />
      <i className="fiducial f1" />
      <i className="fiducial f2" />
      <img
        className="optic-assembly"
        src={optic}
        alt=""
        width="559"
        height="525"
        draggable={false}
      />
    </div>
  )
}
