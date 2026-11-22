import { useId, useRef, type CSSProperties, type ReactNode } from 'react'
import * as ToggleGroup from '@radix-ui/react-toggle-group'

interface Choice<T extends string> {
  value: T
  label: string
  icon?: ReactNode
  state?: string
}

export function Segments<T extends string>({
  label,
  choices,
  value,
  onChange,
  compact = false,
  visual = false,
}: {
  label: string
  choices: Choice<T>[]
  value: T
  onChange: (value: T) => void
  visual?: boolean
  compact?: boolean
}) {
  const name = useId()
  const arrowMove = useRef(false)
  const style = {
    '--options': choices.length,
    '--choice': choices.findIndex((c) => c.value === value),
  } as CSSProperties
  if (compact) {
    // Native radio inputs keep the six-way status filter cheap to mount on every results page.
    return (
      <div className="segments filter-segments" role="radiogroup" aria-label={label} style={style}>
        <span className="segment-thumb" aria-hidden="true" />
        {choices.map((choice) => (
          <label key={choice.value} className="filter-choice">
            <input
              type="radio"
              name={name}
              value={choice.value}
              checked={choice.value === value}
              aria-label={choice.label}
              onChange={() => onChange(choice.value)}
            />
            {choice.state && <i className={'state-dot ' + choice.state} aria-hidden="true" />}
            <span>{choice.label}</span>
          </label>
        ))}
      </div>
    )
  }
  return (
    <ToggleGroup.Root
      type="single"
      orientation="horizontal"
      value={value}
      aria-label={label}
      className={
        'segments' + (compact ? ' filter-segments' : '') + (visual ? ' visual-segments' : '')
      }
      style={style}
      onPointerDownCapture={() => {
        arrowMove.current = false
      }}
      onKeyDownCapture={(e) => {
        arrowMove.current = ['ArrowRight', 'ArrowLeft', 'Home', 'End'].includes(e.key)
      }}
      onValueChange={(next) => {
        if (next) onChange(next as T)
      }}
    >
      <span className="segment-thumb" aria-hidden="true" />
      {choices.map((choice) => (
        <ToggleGroup.Item
          key={choice.value}
          value={choice.value}
          aria-label={choice.label}
          onFocus={() => {
            // Radix moves focus after keydown. Commit the choice when focus actually arrives.
            if (arrowMove.current) {
              arrowMove.current = false
              onChange(choice.value)
            }
          }}
        >
          {choice.icon}
          {choice.state && <i aria-hidden="true" className={'state-dot ' + choice.state} />}
          <span>{choice.label}</span>
        </ToggleGroup.Item>
      ))}
    </ToggleGroup.Root>
  )
}
