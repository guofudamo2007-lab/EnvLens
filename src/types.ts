export type Lang = 'zh' | 'en'
export type LocalText = Record<Lang, string>
export type Status = 'ok' | 'warning' | 'error' | 'unknown' | 'skipped'
export type Group = 'system' | 'gpu' | 'python' | 'torch' | 'ollama'
export type Page = 'overview' | 'diagnostics' | 'python' | 'gpu' | 'ollama' | 'history' | 'settings'
export type Format = 'markdown' | 'json' | 'html' | 'summary'
export interface Evidence {
  key: string
  value: string
  private: boolean
}
export interface Finding {
  confidence: 'confirmed' | 'likely' | 'undetermined'
  severity: 'high' | 'medium' | 'info'
  impact: LocalText
  cause: LocalText
  steps: LocalText[]
  commands: string[]
  docs: string[]
}
export interface Check {
  id: string
  group: Group
  title: LocalText
  summary: LocalText
  status: Status
  evidence: Evidence[]
  finding: Finding | null
}
export interface Report {
  id: string
  schema_version: number
  started_at: string
  duration_ms: number
  interpreter: string | null
  project: string | null
  redacted: boolean
  checks: Check[]
}
export interface Candidate {
  path: string
  sources: string[]
}
export interface Inventory {
  project: string | null
  candidates: Candidate[]
  launcher: string | null
  notices: string[]
}
export interface Progress {
  step: number
  total: number
  stage: Group | 'complete'
}
export const groups: Group[] = ['system', 'gpu', 'python', 'torch', 'ollama']
export const statuses: Status[] = ['ok', 'warning', 'error', 'unknown', 'skipped']
export function value(r: Report | null, id: string, key: string) {
  return r?.checks.find((c) => c.id === id)?.evidence.find((e) => e.key === key)?.value
}
const dates: Partial<Record<Lang, Intl.DateTimeFormat>> = {}
export function displayTime(iso: string, lang: Lang) {
  const format = (dates[lang] ??= new Intl.DateTimeFormat(lang === 'zh' ? 'zh-CN' : 'en-GB', {
    dateStyle: 'medium',
    timeStyle: 'short',
  }))
  return format.format(new Date(iso))
}
