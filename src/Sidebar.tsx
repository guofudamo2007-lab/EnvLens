import { useLayoutEffect, useRef } from 'react'
import {
  Aperture,
  Boxes,
  ChevronRight,
  Cpu,
  History,
  LayoutDashboard,
  ScanLine,
  Settings2,
  Terminal,
  ShieldCheck,
} from 'lucide-react'
import type { Translator } from './i18n'
import type { Page } from './types'

const icons = {
  overview: LayoutDashboard,
  diagnostics: ScanLine,
  python: Terminal,
  gpu: Cpu,
  ollama: Boxes,
  history: History,
  settings: Settings2,
}
const sections = [
  { label: 'workspace', pages: ['overview', 'diagnostics'] },
  { label: 'tools', pages: ['python', 'gpu', 'ollama'] },
  { label: 'records', pages: ['history'] },
] as const

export function Sidebar({
  page,
  go,
  t,
  expanded,
  version,
}: {
  page: Page
  go: (p: Page) => void
  t: Translator
  expanded: boolean
  version: string
}) {
  const rail = useRef<HTMLElement>(null)
  const positions = useRef(new Map<string, { top: number; height: number }>())
  const marker = useRef<HTMLSpanElement>(null)
  function positionMarker(position: { top: number; height: number } | undefined) {
    if (!position || !marker.current) return
    marker.current.style.transform = 'translateY(' + position.top + 'px)'
    marker.current.style.height = position.height + 'px'
  }
  useLayoutEffect(() => {
    const el = rail.current
    if (!el) return
    const buttons = Array.from(el.querySelectorAll<HTMLButtonElement>('[data-nav]'))
    const measure = () => {
      for (const button of buttons) {
        positions.current.set(button.dataset.nav!, {
          top: button.offsetTop,
          height: button.offsetHeight,
        })
      }
      const active = buttons.find((button) => button.getAttribute('aria-current') === 'page')
      positionMarker(positions.current.get(active?.dataset.nav ?? ''))
    }
    measure()
    const resize = new ResizeObserver(measure)
    resize.observe(el)
    for (const group of el.querySelectorAll('nav,.sidebar-bottom,.brand')) resize.observe(group)
    return () => resize.disconnect()
  }, [t, expanded])
  useLayoutEffect(() => {
    positionMarker(positions.current.get(page))
  }, [page])
  function entry(p: Page) {
    const Icon = icons[p]
    return (
      <button
        type="button"
        className="nav-item"
        key={p}
        data-nav={p}
        aria-label={t(p)}
        title={t(p)}
        aria-current={page === p ? 'page' : undefined}
        onClick={() => go(p)}
      >
        <Icon size={17} />
        <span>{t(p)}</span>
        <ChevronRight className="nav-caret" size={12} />
      </button>
    )
  }
  return (
    <aside ref={rail} id="envlens-nav" className={'sidebar' + (expanded ? ' expanded' : '')}>
      <div className="brand">
        <span className="brand-mark">
          <Aperture size={24} strokeWidth={1.9} />
        </span>
        <b>EnvLens</b>
      </div>
      <span className="nav-indicator" ref={marker} aria-hidden="true" />
      {sections.map(({ label, pages }) => (
        <nav key={label} aria-label={t(label)}>
          <div className="nav-label">{t(label)}</div>
          {pages.map(entry)}
        </nav>
      ))}
      <div className="sidebar-bottom">
        {entry('settings')}
        <div className="brand-foot">
          <ShieldCheck size={13} />
          <span>{t('localOnly')}</span>
          <code>{version ? 'v' + version : '—'}</code>
        </div>
      </div>
    </aside>
  )
}
