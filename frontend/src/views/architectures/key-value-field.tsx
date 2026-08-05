/**
 * Edits a `Record<string,string>` (serialized as a JSON object on the backend)
 * using one `key = value` pair per line. Empty input maps to `null` so the
 * caller can decide whether to clear or leave the field unchanged.
 */
export function serializeKeyValues(map: Record<string, string> | null): string {
  if (!map || Object.keys(map).length === 0) return ''
  return Object.entries(map)
    .map(([k, v]) => `${k} = ${v}`)
    .join('\n')
}

export function parseKeyValues(text: string): Record<string, string> | null {
  const trimmed = text.trim()
  if (!trimmed) return null
  const out: Record<string, string> = {}
  for (const line of trimmed.split('\n')) {
    const idx = line.indexOf('=')
    if (idx < 0) continue
    const key = line.slice(0, idx).trim()
    const value = line.slice(idx + 1).trim()
    if (key) out[key] = value
  }
  return Object.keys(out).length > 0 ? out : null
}

export function KeyValueField({ id, label, value, onChange, placeholder }: {
  id: string
  label: string
  value: string
  onChange: (v: string) => void
  placeholder?: string
}) {
  return (
    <div className="space-y-2">
      <label htmlFor={id} className="text-sm font-medium leading-none">{label}</label>
      <textarea
        id={id}
        value={value}
        onChange={e => onChange(e.target.value)}
        placeholder={placeholder ?? '每行一个，格式：指标 = 目标值'}
        rows={3}
        className="flex w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50"
      />
    </div>
  )
}
