import { useMemo, type ReactNode } from 'react'
import {
  Aperture,
  ArrowRight,
  ArrowUpRight,
  Check,
  ChevronRight,
  CircuitBoard,
  Cpu,
  Database,
  FileSearch,
  Focus,
  Layers3,
  MemoryStick,
  Terminal,
  TriangleAlert,
} from 'lucide-react'
import { OpticalField } from './OpticalField'
import type { Diagnostics } from './useDiagnostics'
import type { Translator } from './i18n'
import {
  displayTime,
  groups,
  statuses,
  value,
  type Check as CheckItem,
  type Group,
  type Lang,
  type Page,
  type Status,
} from './types'
import { Start, statusIcons } from './components'

export const groupIcons = {
  system: CircuitBoard,
  gpu: Cpu,
  python: Terminal,
  torch: Layers3,
  ollama: Database,
}
const groupPage: Record<Group, Page> = {
  system: 'diagnostics',
  gpu: 'gpu',
  python: 'python',
  torch: 'gpu',
  ollama: 'ollama',
}
interface Props {
  d: Diagnostics
  t: Translator
  lang: Lang
  mask: boolean
  go: (p: Page, filter?: Status | 'all') => void
  detail: (c: CheckItem) => void
  scan: () => void
}
function Hardware({
  kind,
  label,
  icon,
  main,
  sub,
  observation,
  detail,
  rail,
}: {
  kind: string
  label: string
  icon: ReactNode
  main: ReactNode
  sub: ReactNode
  observation?: CheckItem
  detail: Props['detail']
  rail?: number
}) {
  return (
    <button
      className={'panel hardware ' + kind}
      disabled={!observation}
      aria-haspopup="dialog"
      onClick={() => observation && detail(observation)}
    >
      <span className="hardware-head">
        <span className="hardware-icon">{icon}</span>
        {label}
        {observation && <ArrowUpRight size={14} className="hardware-open" />}
      </span>
      <strong>{main}</strong>
      <small>{sub}</small>
      {rail !== undefined && (
        <span className="memory-rail" aria-hidden="true">
          <i style={{ width: rail + '%' }} />
        </span>
      )}
    </button>
  )
}

export function Overview({ d, t, lang, mask, go, detail, scan }: Props) {
  const r = d.report,
    busy = !!d.progress
  const stats = useMemo(() => {
    const checks = r?.checks ?? []
    const groupCounts = Object.fromEntries(groups.map((g) => [g, 0])) as Record<Group, number>
    const statusCounts = Object.fromEntries(statuses.map((s) => [s, 0])) as Record<Status, number>
    for (const c of checks) {
      groupCounts[c.group]++
      statusCounts[c.status]++
    }
    return { groupCounts, statusCounts, found: checks.filter((c) => c.finding) }
  }, [r])
  const cpu = value(r, 'system.cpu', 'cpu'),
    threads = value(r, 'system.cpu', 'threads')
  const total = value(r, 'system.memory', 'total_mib'),
    available = value(r, 'system.memory', 'available_mib')
  const gpu = value(r, 'gpu.driver', 'gpu') ?? value(r, 'gpu.adapters', 'adapter')
  const vram = value(r, 'gpu.driver', 'vram_mib'),
    driver = value(r, 'gpu.driver', 'driver')
  const gib = (raw: string) => (Math.round((Number(raw) / 1024) * 10) / 10).toString()
  const used =
    Number(total) > 0 && available !== undefined
      ? Math.max(0, Math.min(100, 100 * (1 - Number(available) / Number(total))))
      : undefined
  const target = d.scope?.candidates.find((c) => c.path === d.target)
  const targetVersion =
    r?.interpreter === d.target ? value(r, 'python.runtime', 'python') : undefined
  const targetName = targetVersion
    ? 'Python ' + targetVersion
    : target
      ? mask
        ? 'python.exe'
        : target.path.split(/[\\/]/).pop()
      : d.loading
        ? t('busyTarget')
        : t('noTarget')
  const obs = (id: string) => r?.checks.find((c) => c.id === id)
  const notReady = r ? t('unknown') : t('waitHardware')
  return (
    <>
      <section className="scan-console" aria-label={t('health')}>
        <div className="hero">
          <OpticalField />
          <div className="hero-content">
            <div className="hero-label">
              <Focus size={14} />
              {t('health')}
            </div>
            <h2>
              {busy ? (
                t('running')
              ) : stats.found.length ? (
                <>
                  <span className="finding-number">{stats.found.length}</span> {t('needReview')}
                </>
              ) : r ? (
                t('checked')
              ) : (
                t('ready')
              )}
            </h2>
            <p>
              {busy ? (
                t(d.progress!.stage)
              ) : r ? (
                <>
                  {t('last')} <time dateTime={r.started_at}>{displayTime(r.started_at, lang)}</time>
                  <i /> {(r.duration_ms / 1000).toFixed(1)} s
                </>
              ) : (
                t('readyDesc')
              )}
            </p>
            <div className="hero-actions">
              <Start
                t={t}
                busy={busy}
                again={!!r}
                disabled={d.loading || d.selecting || !d.scope}
                onClick={scan}
              />
              <button
                className="scope-link"
                onClick={() => go('python')}
                title={mask ? t('select') : d.target || t('select')}
                aria-label={t('select')}
              >
                <Terminal size={15} />
                <span>
                  <small>{t('scanTargetShort')}</small>
                  <b>{targetName}</b>
                </span>
                <ChevronRight size={13} />
              </button>
            </div>
          </div>
        </div>
        {d.progress ? (
          <div className="hero-progress" role="status">
            <span>
              {t('scanProgress')} · {t(d.progress.stage)}
            </span>
            <b>
              {d.progress.step} / {d.progress.total}
            </b>
            <progress max={d.progress.total} value={d.progress.step} />
          </div>
        ) : (
          r && (
            <div className="scan-summary">
              <div className="status-total">
                <b>{r.checks.length}</b>
                <span>{t('items')}</span>
              </div>
              <div className="distribution" aria-hidden="true">
                {statuses.map(
                  (s) =>
                    stats.statusCounts[s] > 0 && (
                      <i key={s} className={s} style={{ flexGrow: stats.statusCounts[s] }} />
                    ),
                )}
              </div>
              <div className="legend">
                {statuses.map((s) => (
                  <button
                    key={s}
                    onClick={() => go('diagnostics', s)}
                    aria-label={t(s) + ' · ' + stats.statusCounts[s]}
                  >
                    <i className={'state-dot ' + s} />
                    <span>{t(s)}</span>
                    <b>{stats.statusCounts[s]}</b>
                  </button>
                ))}
              </div>
            </div>
          )
        )}
      </section>
      <div className="hardware-grid" aria-label={t('hardware')}>
        <Hardware
          kind="cpu"
          icon={<Cpu size={17} />}
          label={t('cpu')}
          main={cpu ?? '—'}
          sub={threads ? threads + ' ' + t('threads') : notReady}
          observation={obs('system.cpu')}
          detail={detail}
        />
        <Hardware
          kind="memory"
          icon={<MemoryStick size={17} />}
          label={t('memory')}
          main={
            total ? (
              <>
                {gib(total)} <em>GiB</em>
              </>
            ) : (
              '—'
            )
          }
          sub={available ? gib(available) + ' GiB ' + t('available') : notReady}
          rail={used}
          observation={obs('system.memory')}
          detail={detail}
        />
        <Hardware
          kind="graphics"
          icon={<CircuitBoard size={17} />}
          label={t('graphics')}
          main={gpu ?? '—'}
          sub={vram ? gib(vram) + ' GiB ' + t('vram') + (driver ? ' · ' + driver : '') : notReady}
          observation={
            gpu && value(r, 'gpu.driver', 'gpu') ? obs('gpu.driver') : obs('gpu.adapters')
          }
          detail={detail}
        />
      </div>
      <div className="bottom-grid">
        <section className="panel">
          <div className="panel-head">
            <h2>
              <TriangleAlert size={16} />
              {t('issues')}
              {r && <span className="count-mark">{stats.found.length}</span>}
            </h2>
            {r && (
              <button className="text-btn" onClick={() => go('diagnostics')}>
                {t('more')}
                <ArrowRight size={13} />
              </button>
            )}
          </div>
          {stats.found.length ? (
            stats.found.slice(0, 4).map((c) => {
              const Icon = statusIcons[c.status]
              return (
                <button key={c.id} className="finding-row" onClick={() => detail(c)}>
                  <span className={'check-icon ' + c.status}>
                    <Icon size={16} />
                  </span>
                  <span className="row-text">
                    <strong>{c.title[lang]}</strong>
                    <small>{c.summary[lang]}</small>
                  </span>
                  <ChevronRight size={14} className="chevron" />
                </button>
              )
            })
          ) : (
            <div className="no-findings">
              <span>{r ? <Check size={21} /> : <FileSearch size={21} />}</span>
              <div>
                <p>{t(r ? 'noFindings' : 'emptyIssues')}</p>
                {r && stats.statusCounts.skipped > 0 && (
                  <button className="text-btn" onClick={() => go('diagnostics', 'skipped')}>
                    {t('reviewSkipped')} <b>{stats.statusCounts.skipped}</b>
                    <ArrowRight size={13} />
                  </button>
                )}
              </div>
            </div>
          )}
        </section>
        <section className="panel">
          <div className="panel-head">
            <h2>
              <Aperture size={16} />
              {t('coverage')}
            </h2>
          </div>
          {groups.map((g) => {
            const Icon = groupIcons[g]
            return (
              <button key={g} className="coverage-row" onClick={() => go(groupPage[g])}>
                <Icon size={15} />
                <span>{t(g)}</span>
                <small>{r ? stats.groupCounts[g] + ' ' + t('items') : '—'}</small>
                <ChevronRight size={13} />
              </button>
            )
          })}
        </section>
      </div>
    </>
  )
}
