import { useMutation } from '@apollo/client/react'
import { gql } from '@apollo/client'
import { useState, useEffect } from 'react'
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogFooter } from '@/components/ui/dialog'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Loader2 } from 'lucide-react'
import { GET_VALUE_STREAMS } from './version-control'
import { KeyValueField, serializeKeyValues, parseKeyValues } from './key-value-field'

// ============================================================================
// Value Stream CRUD
// ============================================================================

// Domain-driven custom mutations (replace seaography auto-CRUD)
const CREATE_VALUE_STREAM = gql`
  mutation ValueStreamCreate($spaceId: String!, $name: String!, $description: String!, $businessVersion: String!, $importance: String!, $triggeringEvent: String, $endDeliverable: String, $stakeholders: [String!]!, $performanceMetrics: String) {
    valueStreamCreate(spaceId: $spaceId, name: $name, description: $description, businessVersion: $businessVersion, importance: $importance, triggeringEvent: $triggeringEvent, endDeliverable: $endDeliverable, stakeholders: $stakeholders, performanceMetrics: $performanceMetrics) {
      id name description businessVersion status importance logicalId ownerId triggeringEvent endDeliverable stakeholders performanceMetrics
    }
  }
`

const UPDATE_VALUE_STREAM = gql`
  mutation ValueStreamUpdate($id: String!, $name: String, $description: String, $importance: String, $triggeringEvent: String, $endDeliverable: String, $stakeholders: [String!], $performanceMetrics: String) {
    valueStreamUpdate(id: $id, name: $name, description: $description, importance: $importance, triggeringEvent: $triggeringEvent, endDeliverable: $endDeliverable, stakeholders: $stakeholders, performanceMetrics: $performanceMetrics) {
      id name description businessVersion status importance logicalId ownerId triggeringEvent endDeliverable stakeholders performanceMetrics
    }
  }
`

const ARCHIVE_VALUE_STREAM = gql`
  mutation ValueStreamArchive($id: String!) {
    valueStreamArchive(id: $id)
  }
`

interface ValueStream {
  id: string
  name: string
  description: string
  businessVersion: string
  status: string
  importance: string
  logicalId: string
  ownerId?: string | null
  triggeringEvent?: string | null
  endDeliverable?: string | null
  stakeholders?: string[] | null
  performanceMetrics?: Record<string, string> | null
}

function splitStakeholders(text: string): string[] {
  return text
    .split(/[,，\n]/)
    .map((s) => s.trim())
    .filter(Boolean)
}

export function ValueStreamCrudDialog({ open, onOpenChange, editing, spaceId }: {
  open: boolean
  onOpenChange: (v: boolean) => void
  editing: ValueStream | null
  spaceId?: string
}) {
  const [name, setName] = useState('')
  const [description, setDescription] = useState('')
  const [version, setVersion] = useState('v1.0')
  const [status, setStatus] = useState('active')
  const [importance, setImportance] = useState('High')
  const [triggeringEvent, setTriggeringEvent] = useState('')
  const [endDeliverable, setEndDeliverable] = useState('')
  const [stakeholdersText, setStakeholdersText] = useState('')
  const [performanceMetricsText, setPerformanceMetricsText] = useState('')
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const [createMut] = useMutation(CREATE_VALUE_STREAM)
  const [updateMut] = useMutation(UPDATE_VALUE_STREAM)

  useEffect(() => {
    if (open) {
      setError(null)
      if (editing) {
        setName(editing.name)
        setDescription(editing.description)
        setVersion(editing.businessVersion)
        setStatus(editing.status)
        setImportance(editing.importance.charAt(0).toUpperCase() + editing.importance.slice(1))
        setTriggeringEvent(editing.triggeringEvent ?? '')
        setEndDeliverable(editing.endDeliverable ?? '')
        setStakeholdersText((editing.stakeholders ?? []).join('\n'))
        setPerformanceMetricsText(serializeKeyValues(editing.performanceMetrics ?? null))
      } else {
        setName('')
        setDescription('')
        setVersion('v1.0')
        setStatus('active')
        setImportance('High')
        setTriggeringEvent('')
        setEndDeliverable('')
        setStakeholdersText('')
        setPerformanceMetricsText('')
      }
    }
  }, [open, editing])

  async function handleSubmit() {
    setLoading(true)
    setError(null)
    try {
      const stakeholders = splitStakeholders(stakeholdersText)
      const performanceMetrics = parseKeyValues(performanceMetricsText)
      if (editing) {
        await updateMut({
          variables: {
            id: editing.id,
            name,
            description,
            importance,
            triggeringEvent: triggeringEvent || null,
            endDeliverable: endDeliverable || null,
            stakeholders,
            performanceMetrics,
          },
          refetchQueries: [{ query: GET_VALUE_STREAMS, variables: { spaceId } }],
        })
      } else {
        await createMut({
          variables: {
            spaceId,
            name,
            description,
            businessVersion: version,
            importance,
            triggeringEvent: triggeringEvent || null,
            endDeliverable: endDeliverable || null,
            stakeholders,
            performanceMetrics,
          },
          refetchQueries: [{ query: GET_VALUE_STREAMS, variables: { spaceId } }],
        })
      }
      onOpenChange(false)
    } catch (err) {
      setError(err instanceof Error ? err.message : '操作失败')
    } finally {
      setLoading(false)
    }
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{editing ? '编辑价值流' : '新建价值流'}</DialogTitle>
        </DialogHeader>
        <div className="space-y-4 py-4">
          {error && <div className="rounded-md bg-destructive/10 p-3 text-sm text-destructive">{error}</div>}
          <div className="space-y-2">
            <Label htmlFor="value-stream-name">名称</Label>
            <Input id="value-stream-name" value={name} onChange={e => setName(e.target.value)} placeholder="价值流名称" />
          </div>
          <div className="space-y-2">
            <Label htmlFor="value-stream-description">描述</Label>
            <Input id="value-stream-description" value={description} onChange={e => setDescription(e.target.value)} placeholder="价值流描述" />
          </div>
          <div className="space-y-2">
            <Label htmlFor="value-stream-triggering-event">触发事件</Label>
            <Input id="value-stream-triggering-event" value={triggeringEvent} onChange={e => setTriggeringEvent(e.target.value)} placeholder="触发价值流开始的事件" />
          </div>
          <div className="space-y-2">
            <Label htmlFor="value-stream-end-deliverable">最终交付物</Label>
            <Input id="value-stream-end-deliverable" value={endDeliverable} onChange={e => setEndDeliverable(e.target.value)} placeholder="价值变现的最终结果" />
          </div>
          <div className="space-y-2">
            <Label htmlFor="value-stream-stakeholders">利益相关方</Label>
            <Input id="value-stream-stakeholders" value={stakeholdersText} onChange={e => setStakeholdersText(e.target.value)} placeholder="每行一个，或用逗号分隔" />
          </div>
          <KeyValueField
            id="value-stream-performance-metrics"
            label="绩效指标"
            value={performanceMetricsText}
            onChange={setPerformanceMetricsText}
            placeholder={'每行一个，格式：指标 = 目标值\n例如：交付周期 = ≤14天'}
          />
          <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
            <div className="space-y-2">
              <Label htmlFor="value-stream-version">版本</Label>
              <Input id="value-stream-version" aria-label="版本" value={version} onChange={e => setVersion(e.target.value)} />
            </div>
            <div className="space-y-2">
              <Label htmlFor="value-stream-status">状态</Label>
              <select id="value-stream-status" className="w-full rounded-md border bg-background px-3 py-2 text-sm" value={status} onChange={e => setStatus(e.target.value)}>
                <option value="active">active</option>
                <option value="archived">archived</option>
              </select>
            </div>
          </div>
          <div className="space-y-2">
            <Label htmlFor="value-stream-importance">重要性</Label>
            <select id="value-stream-importance" className="w-full rounded-md border bg-background px-3 py-2 text-sm" value={importance} onChange={e => setImportance(e.target.value)}>
              <option value="Critical">Critical</option>
              <option value="High">High</option>
              <option value="Medium">Medium</option>
              <option value="Low">Low</option>
            </select>
          </div>
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>取消</Button>
          <Button onClick={handleSubmit} disabled={loading || !name}>
            {loading ? <Loader2 className="h-4 w-4 animate-spin" /> : editing ? '保存' : '创建'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}

export function ValueStreamDeleteDialog({ item, onConfirm, spaceId }: {
  item: ValueStream | null
  onConfirm: () => void
  spaceId?: string
}) {
  const [archiveMut] = useMutation(ARCHIVE_VALUE_STREAM)
  const [loading, setLoading] = useState(false)

  async function handleDelete() {
    if (!item) return
    setLoading(true)
    try {
      await archiveMut({
        variables: { id: item.id },
        refetchQueries: [{ query: GET_VALUE_STREAMS, variables: { spaceId } }],
      })
      onConfirm()
    } catch (err) {
      console.error('Archive failed:', err)
    } finally {
      setLoading(false)
    }
  }

  return (
    <Dialog open={!!item} onOpenChange={() => onConfirm()}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>确认归档</DialogTitle>
        </DialogHeader>
        <p className="py-4 text-sm text-muted-foreground">
          确定要归档价值流「{item?.name}」吗？归档后不可修改，但可通过版本控制创建新版本。
        </p>
        <DialogFooter>
          <Button variant="outline" onClick={onConfirm}>取消</Button>
          <Button variant="destructive" onClick={handleDelete} disabled={loading}>
            {loading ? <Loader2 className="h-4 w-4 animate-spin" /> : '归档'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}

export { GET_VALUE_STREAMS }
export type { ValueStream }
