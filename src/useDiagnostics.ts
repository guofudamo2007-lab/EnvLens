import { useEffect, useRef, useState } from 'react'
import { invoke, isTauri } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { errorKey, type Key } from './i18n'
import type { Inventory, Report, Progress } from './types'

interface History {
  reports: Report[]
  unreadable: number
}
export function useDiagnostics() {
  const [scope, setScope] = useState<Inventory | null>(null),
    [target, setTarget] = useState('')
  const [report, setReport] = useState<Report | null>(null),
    [history, setHistory] = useState<History>({ reports: [], unreadable: 0 })
  const [loading, setLoading] = useState(true),
    [progress, setProgress] = useState<Progress | null>(null)
  const [error, setError] = useState<Key | null>(null),
    [version, setVersion] = useState(''),
    [selecting, setSelecting] = useState(false)
  const pending = useRef(false)
  const historyRevision = useRef(0)
  function applyScope(s: Inventory) {
    setScope(s)
    setTarget(s.candidates[0]?.path ?? '')
  }
  useEffect(() => {
    let active = true
    if (!isTauri()) {
      setError('desktopOnly')
      setLoading(false)
      return
    }
    const off = listen<Progress>('scan-progress', (e) => {
      if (active && pending.current) setProgress(e.payload)
    })
    void off.catch(() => {
      if (active) setError('internal')
    })
    const revision = historyRevision.current
    const fail = (e: unknown) => {
      if (active) setError(errorKey(e))
    }
    void invoke<History>('list_history').then((stored) => {
      if (!active || revision !== historyRevision.current) return
      setHistory(stored)
      setReport((current) => current ?? stored.reports[0] ?? null)
    }, fail)
    void invoke<string>('app_version').then((v) => {
      if (active) setVersion(v)
    }, fail)
    void invoke<Inventory>('discover_targets', { atStartup: true })
      .then((items) => {
        if (active) applyScope(items)
      }, fail)
      .finally(() => {
        if (active) setLoading(false)
      })
    return () => {
      active = false
      void off.then((unlisten) => unlisten()).catch(() => {})
    }
  }, [])
  async function choose(kind: 'project' | 'python' | 'system') {
    if (selecting || pending.current) return
    setSelecting(true)
    setError(null)
    try {
      const next =
        kind === 'system'
          ? await invoke<Inventory>('discover_targets')
          : await invoke<Inventory | null>('choose_target', { kind })
      if (next) applyScope(next)
    } catch (e) {
      setError(errorKey(e))
    } finally {
      setSelecting(false)
    }
  }
  async function scan(verifyGpu: boolean) {
    if (pending.current || selecting || !scope) return
    pending.current = true
    setProgress({ step: 0, total: 5, stage: 'system' })
    setError(null)
    try {
      const r = await invoke<{ report: Report; history_error: string | null }>('run_scan', {
        target: target || null,
        verifyGpu,
      })
      historyRevision.current++
      setReport(r.report)
      if (r.history_error) setError(errorKey(r.history_error))
      const revision = historyRevision.current
      void invoke<History>('list_history')
        .then((stored) => {
          if (historyRevision.current === revision) setHistory(stored)
        })
        .catch((e) => {
          if (historyRevision.current === revision) setError(errorKey(e))
        })
    } catch (e) {
      setError(errorKey(e))
    } finally {
      pending.current = false
      setProgress(null)
    }
  }
  return {
    scope,
    target,
    setTarget,
    report,
    setReport,
    history,
    loading,
    progress,
    error,
    setError,
    version,
    selecting,
    choose,
    scan,
  }
}
export type Diagnostics = ReturnType<typeof useDiagnostics>
