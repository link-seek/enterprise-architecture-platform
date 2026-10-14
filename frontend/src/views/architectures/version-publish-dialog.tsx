import { useState, useEffect, useRef, useId } from 'react'
import type { ReactNode } from 'react'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogFooter } from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Loader2 } from 'lucide-react'

// 与后端 `bump_minor` 对齐：X.Y.Z → X.(Y+1).Z（保留 patch）。
// 全串匹配：拒绝 `v` 前缀、两段式、多段等非严格 semver，后端会同样拒绝。
// 注意：前端比后端更严——后端 semver::Version::parse 允许 prerelease/build
// 后缀（如 2.0.0-rc.1），前端输入框不接受；以后端校验为准，前端只做即时提示。
export function isValidSemver(v: string): boolean {
  return /^\d+\.\d+\.\d+$/.test(v.trim())
}

export function suggestNextVersion(current?: string | null): string {
  const m = (current ?? '').trim().match(/^(\d+)\.(\d+)\.(\d+)$/)
  if (!m) return '1.1.0'
  return `${m[1]}.${parseInt(m[2], 10) + 1}.${m[3]}`
}

function getVersionError(trimmed: string, currentVersion?: string | null): string | null {
  if (!trimmed) return null
  if (!isValidSemver(trimmed)) return '版本号须为严格 semver（如 2.0.0，无 v 前缀）'
  if (trimmed === (currentVersion ?? '').trim()) return '新版本号不能与当前版本相同'
  return null
}

function renderImpactList(items: PublishImpactItem[]) {
  return (
    <ul className="space-y-1">
      {items.map(item => (
        <li key={item.id} className="flex items-center gap-2">
          <span>{item.name}</span>
          {item.detail && <span className="font-mono text-xs text-muted-foreground">{item.detail}</span>}
        </li>
      ))}
    </ul>
  )
}

// 统一发布仪式（discussions#752 统一发布策略 v1）：当前版本 → 新版本号 →
// 影响范围预览 → 确认发布 → 成功回执。价值流与业务流程共用同一骨架，
// 差异只在调用方传入的影响列表与提交函数。
export interface PublishImpactItem {
  id: string
  name: string
  detail?: string
}

export function VersionPublishDialog({ open, onOpenChange, title, currentVersion, suggestedVersion, impactIntro, impactItems, impactLoading, impactError, impactEmptyText, confirmLabel, successTitle, onConfirm }: {
  open: boolean
  onOpenChange: (v: boolean) => void
  title: string
  currentVersion?: string | null
  suggestedVersion: string
  impactIntro: ReactNode
  impactItems: PublishImpactItem[]
  impactLoading?: boolean
  impactError?: string | null
  impactEmptyText: string
  confirmLabel?: string
  successTitle?: string
  onConfirm: (newVersion: string) => Promise<PublishImpactItem[]>
}) {
  const [newVersion, setNewVersion] = useState(suggestedVersion)
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [done, setDone] = useState<PublishImpactItem[] | null>(null)
  const prevOpen = useRef(open)
  const inputId = useId()

  // 只在打开瞬间重置，不覆盖用户已输入的值与成功回执。
  useEffect(() => {
    if (open && !prevOpen.current) {
      setNewVersion(suggestedVersion)
      setError(null)
      setDone(null)
    }
    prevOpen.current = open
  }, [open, suggestedVersion])

  const trimmed = newVersion.trim()
  const versionError = getVersionError(trimmed, currentVersion)
  const canConfirm = !loading && !impactLoading && !impactError && trimmed !== '' && versionError === null

  async function handleConfirm() {
    if (!canConfirm) return
    setLoading(true)
    setError(null)
    try {
      const moved = await onConfirm(trimmed)
      setDone(moved)
    } catch (err) {
      setError(err instanceof Error ? err.message : '发布失败')
    } finally {
      setLoading(false)
    }
  }

  // 发布进行中禁止关闭：否则后台继续执行而回执丢失，用户会重复提交。
  function handleOpenChange(v: boolean) {
    if (!v && loading) return
    onOpenChange(v)
  }

  function renderImpact() {
    if (impactLoading) {
      return <div className="text-center py-4 text-muted-foreground">加载影响范围...</div>
    }
    if (impactError) {
      return <div className="rounded-md bg-destructive/10 p-3 text-sm text-destructive">影响范围加载失败：{impactError}</div>
    }
    if (impactItems.length === 0) {
      return <p className="text-muted-foreground">{impactEmptyText}</p>
    }
    return renderImpactList(impactItems)
  }

  return (
    <Dialog open={open} onOpenChange={handleOpenChange}>
      <DialogContent>
        <DialogHeader><DialogTitle>{title}</DialogTitle></DialogHeader>
        <div className="space-y-4 py-4 text-sm">
          {error && <div className="rounded-md bg-destructive/10 p-3 text-sm text-destructive">{error}</div>}
          {done === null ? (
            <>
              <p className="text-muted-foreground">
                当前版本 <Badge variant="secondary" className="font-mono">{currentVersion ?? '-'}</Badge>{' '}
                将进入 <Badge variant="secondary">deprecated</Badge> 兼容期，
                新版本 <Badge>{trimmed || '?'}</Badge> 将成为 active，关联同步迁移。
              </p>
              <div className="space-y-2">
                <Label htmlFor={inputId}>新版本号</Label>
                <Input id={inputId} value={newVersion} onChange={e => setNewVersion(e.target.value)} placeholder="2.0.0" disabled={loading} />
                {versionError && <p className="text-xs text-destructive">{versionError}</p>}
              </div>
              <div className="space-y-2">
                <p className="font-medium">{impactIntro}</p>
                {renderImpact()}
              </div>
            </>
          ) : (
            <>
              <p className="font-medium">{successTitle ?? '发布成功，关联已迁移到新版:'}</p>
              {done.length === 0 ? (
                <p className="text-muted-foreground">无受影响的关联。</p>
              ) : (
                renderImpactList(done)
              )}
            </>
          )}
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={() => handleOpenChange(false)} disabled={loading}>{done === null ? '取消' : '关闭'}</Button>
          {done === null && (
            <Button onClick={handleConfirm} disabled={!canConfirm}>
              {loading ? <Loader2 className="h-4 w-4 animate-spin" /> : (confirmLabel ?? '确认发布')}
            </Button>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
