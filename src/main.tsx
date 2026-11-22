import { useEffect, useRef, useState } from 'react'
import { createRoot } from 'react-dom/client'
import { invoke } from '@tauri-apps/api/core'
import {
  Activity,
  AlertCircle,
  CheckCircle2,
  ChevronRight,
  ClipboardCopy,
  Download,
  Menu,
  Moon,
  Sun,
  X,
} from 'lucide-react'
import { Sidebar } from './Sidebar'
import { Detail, Share, Start, copyText } from './components'
import { translator, type Key } from './i18n'
import { Overview } from './Overview'
import { HistoryView, Results, Settings } from './Views'
import { useDiagnostics } from './useDiagnostics'
import type { Check, Lang, Page, Status } from './types'
import './style.css'

function usePreference<T extends string>(key: string, fallback: T, allowed: readonly T[]) {
  const [v, set] = useState<T>(() => {
    try {
      const raw = localStorage.getItem(key) as T
      return allowed.includes(raw) ? raw : fallback
    } catch {
      return fallback
    }
  })
  useEffect(() => {
    try {
      localStorage.setItem(key, v)
    } catch {
      /* Session preferences still work. */
    }
  }, [key, v])
  return [v, set] as const
}
const intros: Record<Page, Key> = {
  overview: 'overviewIntro',
  diagnostics: 'diagnosticIntro',
  python: 'pythonIntro',
  gpu: 'gpuIntro',
  ollama: 'ollamaIntro',
  history: 'historyIntro',
  settings: 'settingsIntro',
}
function App() {
  const d = useDiagnostics(),
    [page, setPage] = useState<Page>('overview'),
    [initialFilter, setInitialFilter] = useState<Status | 'all'>('all'),
    [detail, setDetail] = useState<Check | null>(null),
    [share, setShare] = useState(false),
    [toast, setToast] = useState(''),
    [menu, setMenu] = useState(false)
  const [lang, setLang] = usePreference<Lang>('envlens.language', 'zh', ['zh', 'en']),
    [theme, setTheme] = usePreference('envlens.theme', 'dark', ['dark', 'light'] as const)
  const [test, setTest] = usePreference('envlens.gpu-test', 'off', ['on', 'off'] as const),
    [privacy, setPrivacy] = usePreference('envlens.mask', 'off', ['on', 'off'] as const)
  const t = translator(lang),
    mask = privacy === 'on',
    busy = !!d.progress,
    body = useRef<HTMLDivElement>(null)
  useEffect(() => {
    document.documentElement.lang = lang === 'zh' ? 'zh-CN' : 'en'
    document.documentElement.dataset.theme = theme
  }, [lang, theme])
  useEffect(() => {
    if (!toast) return
    const timer = setTimeout(() => setToast(''), 4200)
    return () => clearTimeout(timer)
  }, [toast])
  function go(p: Page, filter: Status | 'all' = 'all') {
    setInitialFilter(filter)
    setPage(p)
    setDetail(null)
    setMenu(false)
    body.current?.scrollTo(0, 0)
  }
  function scan() {
    setDetail(null)
    void d.scan(test === 'on')
  }
  function notify(s: string) {
    setToast(s)
  }
  useEffect(() => {
    if (!menu) return
    const escape = (event: KeyboardEvent) => {
      if (event.key === 'Escape') setMenu(false)
    }
    window.addEventListener('keydown', escape)
    return () => window.removeEventListener('keydown', escape)
  }, [menu])
  async function copy() {
    if (!d.report) return
    try {
      const summary = await invoke<string>('preview_report', {
        id: d.report.id,
        format: 'summary',
        redact: true,
        english: lang === 'en',
      })
      await copyText(summary)
      notify(t('copied'))
    } catch {
      notify(t('copyFail'))
    }
  }
  return (
    <div className="app" data-theme={theme}>
      <Sidebar page={page} go={go} t={t} expanded={menu} version={d.version} />
      {menu && (
        <button className="nav-scrim" aria-label={t('close')} onClick={() => setMenu(false)} />
      )}
      <div className="main-frame">
        <header className="topbar">
          <div className="breadcrumbs">
            <button
              className="mobile-menu icon-btn"
              aria-label={t('workspace')}
              aria-expanded={menu}
              aria-controls="envlens-nav"
              onClick={() => setMenu(!menu)}
            >
              <Menu size={18} />
            </button>
            <span>EnvLens</span>
            <ChevronRight size={13} />
            <b>{t(page)}</b>
          </div>
          <div className="top-actions">
            <span className="local-chip">
              <i className="live-dot" />
              {t('local')}
            </span>
            <span className="vline" />
            <button
              className="icon-btn"
              aria-label={t(theme === 'dark' ? 'light' : 'dark')}
              onClick={() => setTheme(theme === 'dark' ? 'light' : 'dark')}
            >
              {theme === 'dark' ? <Sun size={17} /> : <Moon size={17} />}
            </button>
          </div>
        </header>
        <div className="scroll" ref={body}>
          <main className="content">
            <div className="page-heading">
              <div>
                <h1>{t(page)}</h1>
                {page !== 'overview' && <p>{t(intros[page])}</p>}
              </div>
              {page !== 'settings' && page !== 'history' && (
                <div className="action-row">
                  {d.report && (
                    <button
                      className="button"
                      data-testid="share"
                      onClick={() => setShare(true)}
                      disabled={busy}
                    >
                      <Download size={15} />
                      {t('export')}
                    </button>
                  )}
                  {page !== 'overview' && (
                    <Start
                      t={t}
                      busy={busy}
                      again={!!d.report}
                      disabled={d.loading || d.selecting || !d.scope}
                      onClick={scan}
                    />
                  )}
                </div>
              )}
            </div>
            {d.error && (
              <div className="banner" role="alert">
                <AlertCircle size={16} />
                {t(d.error)}
                <button
                  className="icon-btn"
                  aria-label={t('close')}
                  onClick={() => d.setError(null)}
                >
                  <X size={14} />
                </button>
              </div>
            )}
            {d.progress && page !== 'overview' && (
              <section className="progress-box" role="status">
                <div>
                  <Activity size={15} />
                  {t('scanProgress')} · {t(d.progress.stage)}
                  <b>
                    {d.progress.step} / {d.progress.total}
                  </b>
                </div>
                <progress max={d.progress.total} value={d.progress.step} />
              </section>
            )}
            {page === 'overview' ? (
              <Overview
                d={d}
                t={t}
                lang={lang}
                mask={mask}
                go={go}
                detail={setDetail}
                scan={scan}
              />
            ) : page === 'history' ? (
              <HistoryView
                d={d}
                t={t}
                lang={lang}
                open={(r) => {
                  d.setReport(r)
                  go('diagnostics')
                }}
              />
            ) : page === 'settings' ? (
              <Settings
                t={t}
                lang={lang}
                setLang={setLang}
                light={theme === 'light'}
                setLight={(v) => setTheme(v ? 'light' : 'dark')}
                verify={test === 'on'}
                setVerify={(v) => setTest(v ? 'on' : 'off')}
                mask={mask}
                setMask={(v) => setPrivacy(v ? 'on' : 'off')}
                version={d.version}
                busy={busy}
              />
            ) : (
              <Results
                key={page + initialFilter}
                initialFilter={initialFilter}
                page={page}
                d={d}
                t={t}
                lang={lang}
                mask={mask}
                detail={setDetail}
              />
            )}
            {d.report && page !== 'settings' && page !== 'history' && (
              <footer className="report-foot">
                <span>
                  <span className="led" />
                  {d.report.redacted ? t('archived') : t('report')} ·{' '}
                  <code>{d.report.id.slice(0, 8)}</code>
                </span>
                <button className="text-btn" data-testid="copy" onClick={() => void copy()}>
                  <ClipboardCopy size={14} />
                  {t('copy')}
                </button>
              </footer>
            )}
          </main>
        </div>
      </div>
      {detail && d.report && (
        <Detail
          item={detail}
          report={d.report}
          lang={lang}
          t={t}
          mask={mask}
          close={() => setDetail(null)}
          notify={notify}
        />
      )}
      {share && d.report && (
        <Share report={d.report} lang={lang} t={t} close={() => setShare(false)} notify={notify} />
      )}
      {toast && (
        <div className="toast" role="status">
          <CheckCircle2 size={16} />
          {toast}
        </div>
      )}
    </div>
  )
}
createRoot(document.getElementById('root')!).render(<App />)
