import { useMutation } from '@apollo/client/react'
import { gql } from '@apollo/client'
import { useState, useEffect } from 'react'
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogFooter } from '@/components/ui/dialog'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Loader2 } from 'lucide-react'
import { GET_VALUE_STREAMS } from './version-control'

// ============================================================================
// Value Stream CRUD
// ============================================================================

// Domain-driven custom mutations (replace seaography auto-CRUD)
const CREATE_VALUE_STREAM = gql`
  mutation ValueStreamCreate($spaceId: String!, $name: String!, $description: String!, $businessVersion: String!, $importance: String!) {
    valueStreamCreate(spaceId: $spaceId, name: $name, description: $description, businessVersion: $businessVersion, importance: $importance) {
      id name description businessVersion status importance logicalId
    }
  }
`

const UPDATE_VALUE_STREAM = gql`
  mutation ValueStreamUpdate($id: String!, $name: String, $description: String, $importance: String) {
    valueStreamUpdate(id: $id, name: $name, description: $description, importance: $importance) {
      id name description businessVersion status importance logicalId
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
      } else {
        setName('')
        setDescription('')
        setVersion('v1.0')
        setStatus('active')
        setImportance('High')
      }
    }
  }, [open, editing])

  async function handleSubmit() {
    setLoading(true)
    setError(null)
    try {
      if (editing) {
        await updateMut({
          variables: {
            id: editing.id,
            name,
            description,
            importance,
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
            <Label>名称</Label>
            <Input value={name} onChange={e => setName(e.target.value)} placeholder="价值流名称" />
          </div>
          <div className="space-y-2">
            <Label>描述</Label>
            <Input value={description} onChange={e => setDescription(e.target.value)} placeholder="价值流描述" />
          </div>
          <div className="grid grid-cols-2 gap-4">
            <div className="space-y-2">
              <Label>版本</Label>
              <Input value={version} onChange={e => setVersion(e.target.value)} />
            </div>
            <div className="space-y-2">
              <Label>状态</Label>
              <select className="w-full rounded-md border bg-background px-3 py-2 text-sm" value={status} onChange={e => setStatus(e.target.value)}>
                <option value="active">active</option>
                <option value="archived">archived</option>
              </select>
            </div>
          </div>
          <div className="space-y-2">
            <Label>重要性</Label>
            <select className="w-full rounded-md border bg-background px-3 py-2 text-sm" value={importance} onChange={e => setImportance(e.target.value)}>
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
