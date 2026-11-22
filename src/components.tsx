import { Segments } from './Controls'
import { useEffect, useId, useRef, useState, type ReactNode } from 'react'
import { invoke } from '@tauri-apps/api/core'
import {
  AlertCircle,
  AlertTriangle,
  Ban,
  CheckCircle2,
  ChevronRight,
  CircleHelp,
  Copy,
  ExternalLink,
  FileSearch,
  MinusCircle,
  ScanLine,
  LoaderCircle,
  ShieldCheck,
  X,
  Download,
} from 'lucide-react'
import { errorKey, type Key, type Translator } from './i18n'
import { statuses, type Check, type Format, type Lang, type Report, type Status } from './types'

export const statusIcons = {
  ok: CheckCircle2,
  warning: AlertTriangle,
  error: Ban,
  skipped: MinusCircle,
  unknown: CircleHelp,
}
export function Pill({ status, t }: { status: Status; t: Translator }) {
  const Icon = statusIcons[status]
  return (
    <span className={'pill ' + status}>
      <Icon size={13} />
      {t(status)}
    </span>
  )
}
export function Start({
  t,
  busy,
  again,
  disabled,
  onClick,
}: {
  t: Translator
  busy: boolean
  again: boolean
  disabled: boolean
  onClick: () => void
}) {
  return (
    <button
      data-testid="scan"
      className="button primary"
      disabled={busy || disabled}
      onClick={onClick}
    >
      {busy ? <LoaderCircle size={16} className="working-icon" /> : <ScanLine size={16} />}
      {t(busy ? 'running' : again ? 'again' : 'start')}
    </button>
  )
}
export function Empty({ t, history = false }: { t: Translator; history?: boolean }) {
  return (
    <div className="empty">
      <FileSearch size={30} />
      <strong>{t(history ? 'historyEmpty' : 'noData')}</strong>
      <span>{t('noDataText')}</span>
    </div>
  )
}
export function CheckList({
  checks,
  lang,
  t,
  onDetail,
}: {
  checks: Check[]
  lang: Lang
  t: Translator
  onDetail: (c: Check) => void
}) {
  if (!checks.length) return <p className="quiet pad">{t('noResults')}</p>
  return (
    <div className="check-list">
      {checks.map((c) => {
        const Icon = statusIcons[c.status]
        return (
          <button className="check-row" key={c.id} onClick={() => onDetail(c)} data-check={c.id}>
            <span className={'check-icon ' + c.status}>
              <Icon size={17} />
            </span>
            <span className="row-text">
              <strong>{c.title[lang]}</strong>
              <small>{c.summary[lang]}</small>
            </span>
            <Pill status={c.status} t={t} />
            <ChevronRight size={15} className="chevron" />
          </button>
        )
      })}
    </div>
  )
}
export function Filter({ v, set, t }: { v: string; set: (s: string) => void; t: Translator }) {
  return (
    <Segments
      label={t('filter')}
      compact
      value={v}
      onChange={set}
      choices={[
        { value: 'all', label: t('allShort') },
        ...statuses.map((status) => ({ value: status, label: t(status), state: status })),
      ]}
    />
  )
}
export function Modal({
  title,
  children,
  close,
  wide = false,
}: {
  title: string
  children: ReactNode
  close: () => void
  wide?: boolean
}) {
  const ref = useRef<HTMLDialogElement>(null),
    id = useId()
  useEffect(() => {
    const d = ref.current
    d?.showModal()
    return () => d?.close()
  }, [])
  return (
    <dialog
      ref={ref}
      className={wide ? 'modal wide' : 'modal'}
      onCancel={(e) => {
        e.preventDefault()
        close()
      }}
      aria-labelledby={id}
      onClick={(e) => {
        if (e.target === e.currentTarget) close()
      }}
    >
      <div className="modal-inner">
        <header>
          <h2 id={id}>{title}</h2>
          <button
            className="icon-btn"
            aria-label={document.documentElement.lang === 'en' ? 'Close' : '关闭'}
            onClick={close}
          >
            <X size={19} />
          </button>
        </header>
        {children}
      </div>
    </dialog>
  )
}
export async function copyText(text: string) {
  try {
    await navigator.clipboard.writeText(text)
    return
  } catch {
    const el = document.createElement('textarea')
    el.value = text
    el.style.position = 'fixed'
    el.style.opacity = '0'
    const parent = document.querySelector('dialog[open]') ?? document.body
    parent.appendChild(el)
    el.select()
    try {
      if (!document.execCommand('copy')) throw Error('copy')
    } finally {
      el.remove()
    }
  }
}
export function Detail({
  item,
  report,
  lang,
  t,
  mask,
  close,
  notify,
}: {
  item: Check
  report: Report
  lang: Lang
  t: Translator
  mask: boolean
  close: () => void
  notify: (s: string) => void
}) {
  const f = item.finding
  function screenCommand(cmd: string) {
    return mask && report.interpreter
      ? cmd.replaceAll(report.interpreter.replaceAll("'", "''"), '<python.exe>')
      : cmd
  }
  return (
    <Modal title={t('details')} close={close}>
      <div className="modal-body">
        <div className="detail-title">
          <Pill status={item.status} t={t} />
          <span className="mono subtle">{item.id}</span>
        </div>
        <h2>{item.title[lang]}</h2>
        <p>{item.summary[lang]}</p>
        <h3>{t('evidence')}</h3>
        {item.evidence.length ? (
          <dl className="evidence">
            {item.evidence.map((ev, i) => (
              <div key={i}>
                <dt>{ev.key}</dt>
                <dd>{mask && ev.private ? t('privateValue') : ev.value}</dd>
              </div>
            ))}
          </dl>
        ) : (
          <p className="quiet">{t('evidenceNone')}</p>
        )}
        {f && (
          <>
            <div className="section-label">
              <h3>{t('cause')}</h3>
              <span className="tag">{t(f.confidence)}</span>
              <span className="subtle">
                {t('severity')} · {t(f.severity)}
              </span>
            </div>
            <p>{f.cause[lang]}</p>
            <h3>{t('impact')}</h3>
            <p>{f.impact[lang]}</p>
            <h3>{t('actions')}</h3>
            <ol>
              {f.steps.map((step, i) => (
                <li key={i}>{step[lang]}</li>
              ))}
            </ol>
            {f.commands.map((cmd) => (
              <div className="cmd" key={cmd}>
                <code>{screenCommand(cmd)}</code>
                <button
                  aria-label={t('copyCmd')}
                  title={t('copyCmd')}
                  onClick={() =>
                    void copyText(screenCommand(cmd))
                      .then(() => notify(t('copied')))
                      .catch(() => notify(t('copyFail')))
                  }
                >
                  <Copy size={15} />
                </button>
              </div>
            ))}
            {f.docs.length > 0 && (
              <>
                <h3>{t('sources')}</h3>
                {f.docs.map((url) => (
                  <button
                    className="doc"
                    key={url}
                    onClick={() =>
                      void invoke('open_doc', { url }).catch((e) => notify(t(errorKey(e))))
                    }
                  >
                    {new URL(url).hostname}
                    <ExternalLink size={13} />
                  </button>
                ))}
              </>
            )}
          </>
        )}
      </div>
    </Modal>
  )
}
export function Share({
  report,
  lang,
  t,
  close,
  notify,
}: {
  report: Report
  lang: Lang
  t: Translator
  close: () => void
  notify: (s: string) => void
}) {
  const [format, setFormat] = useState<Format>('markdown'),
    [redact, setRedact] = useState(true),
    [preview, setPreview] = useState(''),
    [error, setError] = useState<Key | null>(null),
    [saving, setSaving] = useState(false)
  const english = lang === 'en'
  useEffect(() => {
    let live = true
    setPreview('')
    setError(null)
    invoke<string>('preview_report', { id: report.id, format, redact, english })
      .then((v) => {
        if (live) setPreview(v)
      })
      .catch((e) => {
        if (live) setError(errorKey(e))
      })
    return () => {
      live = false
    }
  }, [report.id, format, redact, english])
  async function save() {
    setSaving(true)
    try {
      if (await invoke<boolean>('save_report', { id: report.id, format, redact, english })) {
        notify(t('saved'))
        close()
      }
    } catch (e) {
      setError(errorKey(e))
    } finally {
      setSaving(false)
    }
  }
  return (
    <Modal title={t('share')} close={close} wide>
      <div className="modal-body share-body">
        <Segments
          label={t('export')}
          value={format}
          onChange={setFormat}
          choices={(['markdown', 'json', 'html'] as const).map((f) => ({
            value: f,
            label: f === 'markdown' ? 'Markdown' : f.toUpperCase(),
          }))}
        />
        <label className="option privacy-option">
          <ShieldCheck size={18} />
          <span>{t('redact')}</span>
          <input
            className="switch"
            type="checkbox"
            checked={redact}
            disabled={report.redacted}
            onChange={(e) => setRedact(e.target.checked)}
          />
        </label>
        {report.redacted && <p className="subtle">{t('archiveOnly')}</p>}
        <div className="section-label">
          <h3>{t('preview')}</h3>
          <span className="tag">{t(redact ? 'redacted' : 'raw')}</span>
        </div>
        <pre data-testid="export-preview" className="preview" tabIndex={0}>
          {preview || t('preparing')}
        </pre>
        {error && (
          <p role="alert" className="error-text">
            <AlertCircle size={15} />
            {t(error)}
          </p>
        )}
        <div className="end">
          <button
            className="button primary"
            disabled={saving || !preview}
            onClick={() => void save()}
          >
            <Download size={16} />
            {t('save')}
          </button>
        </div>
      </div>
    </Modal>
  )
}
