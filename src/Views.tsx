import { Segments } from './Controls'
import { useState } from 'react'
import {
  ArrowRight,
  Check as CheckIcon,
  ChevronRight,
  FileClock,
  FolderOpen,
  GitCompareArrows,
  RefreshCw,
  Terminal,
  SwatchBook,
  SlidersHorizontal,
  Moon,
  Sun,
} from 'lucide-react'
import { CheckList, Empty, Filter, Pill } from './components'
import { displayTime, groups, type Check, type Lang, type Page, type Report } from './types'
import { groupIcons } from './Overview'
import type { Diagnostics } from './useDiagnostics'
import type { Key, Translator } from './i18n'
export function TargetPicker({ d, t, mask }: { d: Diagnostics; t: Translator; mask: boolean }) {
  const disabled = !!d.progress || d.selecting || d.loading,
    scope = d.scope
  return (
    <section className="panel picker">
      <div className="panel-head">
        <h2>
          <Terminal size={17} />
          {t('nextTarget')}
        </h2>
        <span className="subtle">
          {scope?.candidates.length ?? 0} {t('found')}
        </span>
      </div>
      <div className="picker-body">
        {scope?.project && (
          <p className="subtle">
            {t('project')} · {mask ? t('privateValue') : scope.project}
          </p>
        )}
        {scope?.candidates.length ? (
          <div className="candidates" role="radiogroup" aria-label={t('interpreter')}>
            {scope.candidates.map((c, i) => (
              <label
                key={c.path}
                className={'candidate ' + (d.target === c.path ? 'selected' : '')}
              >
                <input
                  name="scan-interpreter"
                  type="radio"
                  aria-label={mask ? 'Python ' + (i + 1) : c.path}
                  checked={c.path === d.target}
                  onChange={() => d.setTarget(c.path)}
                  disabled={disabled}
                />
                <span>
                  <b className="mono">{mask ? 'Python ' + (i + 1) + ' · python.exe' : c.path}</b>
                  <small>
                    {c.sources
                      .map((s) =>
                        ['manual', 'launcher', 'project', 'conda'].includes(s) ? t(s as Key) : s,
                      )
                      .join(' / ')}
                  </small>
                </span>
                {d.target === c.path && <CheckIcon size={16} />}
              </label>
            ))}
          </div>
        ) : (
          <p className="quiet">
            {t(
              d.loading || d.selecting
                ? 'busyTarget'
                : scope?.project
                  ? 'emptyProject'
                  : 'noCandidates',
            )}
          </p>
        )}
        <div className="action-row">
          <button className="button" disabled={disabled} onClick={() => void d.choose('python')}>
            <Terminal size={15} />
            {t('chooseFile')}
          </button>
          <button className="button" disabled={disabled} onClick={() => void d.choose('project')}>
            <FolderOpen size={15} />
            {t('chooseDir')}
          </button>
          <button className="text-btn" disabled={disabled} onClick={() => void d.choose('system')}>
            <RefreshCw size={14} />
            {t('rediscover')}
          </button>
        </div>
      </div>
    </section>
  )
}
export function Results({
  page,
  initialFilter,
  d,
  t,
  lang,
  mask,
  detail,
}: {
  page: Page
  initialFilter: string
  d: Diagnostics
  t: Translator
  lang: Lang
  mask: boolean
  detail: (c: Check) => void
}) {
  const [filter, setFilter] = useState(initialFilter),
    report = d.report
  const selected = groups.filter(
    (g) =>
      page === 'diagnostics' || (page === 'gpu' && (g === 'gpu' || g === 'torch')) || g === page,
  )
  return (
    <>
      {page === 'python' && <TargetPicker d={d} t={t} mask={mask} />}
      {report ? (
        <>
          <div className="result-tools">
            <div>
              <span className="subtle">{t('reportTarget')}</span>
              <code>
                {report.interpreter ? (mask ? t('privateValue') : report.interpreter) : '—'}
              </code>
            </div>
            <Filter v={filter} set={setFilter} t={t} />
          </div>
          {selected.map((g) => {
            const Icon = groupIcons[g],
              checks = report.checks.filter(
                (c) => c.group === g && (filter === 'all' || c.status === filter),
              )
            if (!checks.length) return null
            return (
              <section
                key={g}
                className="panel result-group"
                style={{ containIntrinsicBlockSize: `auto ${44 + checks.length * 57}px` }}
              >
                <div className="panel-head">
                  <h2>
                    <Icon size={17} />
                    {t(g)}
                  </h2>
                  <span className="subtle">
                    {checks.length} {t('items')}
                  </span>
                </div>
                <CheckList checks={checks} t={t} lang={lang} onDetail={detail} />
              </section>
            )
          })}
          {!report.checks.some(
            (c) => selected.includes(c.group) && (filter === 'all' || c.status === filter),
          ) && <div className="panel empty">{t('noResults')}</div>}
        </>
      ) : (
        <div className="panel">
          <Empty t={t} />
        </div>
      )}
    </>
  )
}
export function HistoryView({
  d,
  t,
  lang,
  open,
}: {
  d: Diagnostics
  t: Translator
  lang: Lang
  open: (r: Report) => void
}) {
  const [base, setBase] = useState(''),
    [other, setOther] = useState('')
  const rows = d.history.reports,
    a = rows.find((r) => r.id === base),
    b = rows.find((r) => r.id === other)
  const ids = new Set([...(a?.checks ?? []), ...(b?.checks ?? [])].map((c) => c.id))
  const differences = [...ids]
    .map((id) => ({
      prev: a?.checks.find((c) => c.id === id),
      next: b?.checks.find((c) => c.id === id),
    }))
    .filter((v) => v.prev?.status !== v.next?.status)
  if (!rows.length && !d.history.unreadable)
    return (
      <div className="panel">
        <Empty history t={t} />
      </div>
    )
  return (
    <>
      {!!d.history.unreadable && (
        <p role="alert" className="banner">
          {d.history.unreadable} {t('historyDamaged')}
        </p>
      )}
      {a && (
        <section className="panel compare">
          <div className="panel-head">
            <h2>
              <GitCompareArrows size={17} />
              {t('compare')}
            </h2>
            <button className="text-btn" onClick={() => setBase('')}>
              {t('close')}
            </button>
          </div>
          <div className="pad">
            <p>
              {t('baseline')} · {displayTime(a.started_at, lang)} · {a.id.slice(0, 8)}
            </p>
            <select
              aria-label={t('compared')}
              value={other}
              onChange={(e) => setOther(e.target.value)}
            >
              <option value="">{t('chooseCompare')}</option>
              {rows
                .filter((r) => r.id !== base)
                .map((r) => (
                  <option key={r.id} value={r.id}>
                    {displayTime(r.started_at, lang)} · {r.id.slice(0, 8)}
                  </option>
                ))}
            </select>
            <small className="block quiet">{t('diffHint')}</small>
            {b &&
              (differences.length ? (
                <>
                  <h3>
                    {differences.length} {t('changes')}
                  </h3>
                  {differences.map(({ prev, next }) => (
                    <div className="diff" key={(prev ?? next)!.id}>
                      <strong>{(prev ?? next)!.title[lang]}</strong>
                      {prev ? <Pill status={prev.status} t={t} /> : t('newItem')}
                      <ArrowRight size={14} />
                      {next ? <Pill status={next.status} t={t} /> : t('newItem')}
                    </div>
                  ))}
                </>
              ) : (
                <p>{t('diffNone')}</p>
              ))}
          </div>
        </section>
      )}
      <section className="panel">
        <div className="panel-head">
          <h2>
            <FileClock size={17} />
            {t('archived')}
          </h2>
          <span className="subtle">{rows.length} / 20</span>
        </div>
        {rows.map((r) => (
          <div className="history-row" key={r.id} data-comparing={base === r.id || undefined}>
            <FileClock size={19} />
            <div className="row-text">
              <strong>{displayTime(r.started_at, lang)}</strong>
              <small>
                {r.id.slice(0, 8)} · {r.checks.length} {t('items')} · {t('redacted')}
              </small>
            </div>
            {rows.length > 1 && (
              <button
                className="text-btn"
                aria-pressed={base === r.id}
                onClick={() => {
                  setBase(r.id)
                  setOther(rows.find((x) => x.id !== r.id)?.id ?? '')
                }}
              >
                {t('compare')}
              </button>
            )}
            <button className="button" onClick={() => open(r)}>
              {t('open')}
              <ChevronRight size={14} />
            </button>
          </div>
        ))}
      </section>
    </>
  )
}
interface SettingsProps {
  t: Translator
  lang: Lang
  setLang: (l: Lang) => void
  light: boolean
  setLight: (v: boolean) => void
  verify: boolean
  setVerify: (v: boolean) => void
  mask: boolean
  setMask: (v: boolean) => void
  version: string
  busy: boolean
}
export function Settings({
  t,
  lang,
  setLang,
  light,
  setLight,
  verify,
  setVerify,
  mask,
  setMask,
  version,
  busy,
}: SettingsProps) {
  return (
    <div className="settings">
      <div className="settings-grid">
        <section className="panel appearance">
          <div className="panel-head">
            <h2>
              <SwatchBook size={16} />
              {t('theme')}
            </h2>
          </div>
          <div className="theme-choices">
            <Segments
              label={t('theme')}
              visual
              value={light ? 'light' : 'dark'}
              onChange={(v) => setLight(v === 'light')}
              choices={(['dark', 'light'] as const).map((tone) => ({
                value: tone,
                label: t(tone),
                icon: (
                  <span className={'theme-swatch swatch-' + tone} aria-hidden="true">
                    <i className="swatch-nav">
                      <b />
                      <b />
                      <b />
                    </i>
                    <i className="swatch-body">
                      <b />
                      <b />
                    </i>
                    <i className="swatch-symbol">
                      {tone === 'dark' ? <Moon size={12} /> : <Sun size={12} />}
                    </i>
                  </span>
                ),
              }))}
            />
          </div>
          <label className="setting">
            <b>{t('language')}</b>
            <select
              aria-label={t('language')}
              value={lang}
              onChange={(e) => setLang(e.target.value as Lang)}
            >
              <option value="zh">简体中文</option>
              <option value="en">English</option>
            </select>
          </label>
        </section>
        <section className="panel scan-preferences">
          <div className="panel-head">
            <h2>
              <SlidersHorizontal size={16} />
              {t('scanOptions')}
            </h2>
          </div>
          <label className="setting">
            <span>
              <b>{t('gpuTest')}</b>
              <small>{t('gpuHelp')}</small>
            </span>
            <input
              className="switch"
              type="checkbox"
              checked={verify}
              disabled={busy}
              onChange={(e) => setVerify(e.target.checked)}
            />
          </label>
          <label className="setting">
            <span>
              <b>{t('screenPaths')}</b>
              <small>{t('screenHelp')}</small>
            </span>
            <input
              className="switch"
              type="checkbox"
              checked={mask}
              onChange={(e) => setMask(e.target.checked)}
            />
          </label>
          <div className="setting">
            <span>
              <b>{t('privacy')}</b>
              <small>{t('privacyText')}</small>
            </span>
          </div>
        </section>
      </div>
      <div className="settings-version">
        <span>{t('version')}</span>
        <code>{version || '—'}</code>
      </div>
    </div>
  )
}
