import { useQuery, useMutation } from '@apollo/client/react'
import { gql } from '@apollo/client'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table'
import { Badge } from '@/components/ui/badge'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { Plus, Pencil, Trash2, Loader2 } from 'lucide-react'
import { useState, useEffect, useCallback, memo } from 'react'
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogFooter } from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { useParams } from 'react-router-dom'
import { useSpaceMembership } from '@/hooks/use-space-membership'

const GET_MODULES = gql`
  query GetFunctionalModules($spaceId: String!) {
    functionalModulesBySpace(spaceId: $spaceId) { id name description boundary status parentId }
  }
`

const CREATE_MODULE = gql`
  mutation CreateFunctionalModule($spaceId: String!, $name: String!, $description: String, $boundary: String, $status: FunctionalModuleStatusEnum!, $parentId: String) {
    functionalModuleCreate(spaceId: $spaceId, name: $name, description: $description, boundary: $boundary, status: $status, parentId: $parentId) { id name }
  }
`

const UPDATE_MODULE = gql`
  mutation UpdateFunctionalModule($id: String!, $name: String, $description: String, $boundary: String, $status: FunctionalModuleStatusEnum, $parentId: String) {
    functionalModuleUpdate(id: $id, name: $name, description: $description, boundary: $boundary, status: $status, parentId: $parentId) { id name }
  }
`

const DELETE_MODULE = gql`
  mutation DeleteFunctionalModule($id: String!) { functionalModuleDelete(id: $id) }
`

interface FunctionalModule { id: string; name: string; description?: string | null; boundary?: string | null; status: string; parentId?: string | null }
interface ModuleQuery { functionalModulesBySpace?: FunctionalModule[] }

const EMPTY: FunctionalModule[] = []

const ModuleList = memo(function ModuleList({ nodes, canEdit, onEdit, onDelete }: {
  nodes: FunctionalModule[]; canEdit: boolean; onEdit: (m: FunctionalModule) => void; onDelete: (m: FunctionalModule) => void
}) {
  if (nodes.length === 0) return <div className="text-center py-8 text-muted-foreground">暂无数据</div>
  return (
    <Table>
      <TableHeader>
        <TableRow>
          <TableHead>名称</TableHead>
          <TableHead>状态</TableHead>
          <TableHead>边界</TableHead>
          <TableHead>描述</TableHead>
          {canEdit && <TableHead>操作</TableHead>}
        </TableRow>
      </TableHeader>
      <TableBody>
        {nodes.map((m) => (
          <TableRow key={m.id}>
            <TableCell className="font-medium break-words">{m.name}</TableCell>
            <TableCell><Badge variant="outline">{m.status}</Badge></TableCell>
            <TableCell className="text-muted-foreground">{m.boundary ?? '—'}</TableCell>
            <TableCell className="text-muted-foreground">{m.description ?? '—'}</TableCell>
            {canEdit && (
              <TableCell>
                <div className="flex gap-1">
                  <Button variant="ghost" size="sm" aria-label="编辑" onClick={() => onEdit(m)}><Pencil className="h-3.5 w-3.5" /></Button>
                  <Button variant="ghost" size="sm" aria-label="删除" onClick={() => onDelete(m)}><Trash2 className="h-3.5 w-3.5 text-destructive" /></Button>
                </div>
              </TableCell>
            )}
          </TableRow>
        ))}
      </TableBody>
    </Table>
  )
})

export default function FunctionalModules() {
  const { spaceId } = useParams<{ spaceId: string }>()
  const { canEdit } = useSpaceMembership(spaceId)
  const [dialogOpen, setDialogOpen] = useState(false)
  const [editing, setEditing] = useState<FunctionalModule | null>(null)
  const [deleting, setDeleting] = useState<FunctionalModule | null>(null)
  const { data, loading, error } = useQuery<ModuleQuery>(GET_MODULES, { variables: { spaceId }, skip: !spaceId })

  const handleEdit = useCallback((m: FunctionalModule) => { setEditing(m); setDialogOpen(true) }, [])
  const handleDelete = useCallback((m: FunctionalModule) => setDeleting(m), [])

  return (
    <div className="p-4 md:p-6 space-y-4">
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-semibold">功能模块</h1>
        {canEdit && (
          <Button onClick={() => { setEditing(null); setDialogOpen(true) }}>
            <Plus className="h-4 w-4 mr-2" />新建模块
          </Button>
        )}
      </div>
      <Card>
        <CardHeader><CardTitle>模块列表</CardTitle></CardHeader>
        <CardContent>
          {loading && <div className="text-center py-8 text-muted-foreground">加载中...</div>}
          {Boolean(error) && <div className="text-center py-8 text-destructive">加载失败</div>}
          {data && <ModuleList nodes={data.functionalModulesBySpace ?? EMPTY} canEdit={canEdit} onEdit={handleEdit} onDelete={handleDelete} />}
        </CardContent>
      </Card>
      <ModuleCrudDialog open={dialogOpen} onOpenChange={setDialogOpen} editing={editing} spaceId={spaceId} />
      <ModuleDeleteDialog item={deleting} onConfirm={() => setDeleting(null)} spaceId={spaceId} />
    </div>
  )
}

function ModuleCrudDialog({ open, onOpenChange, editing, spaceId }: {
  open: boolean; onOpenChange: (v: boolean) => void; editing: FunctionalModule | null; spaceId?: string
}) {
  const [name, setName] = useState('')
  const [description, setDescription] = useState('')
  const [boundary, setBoundary] = useState('')
  const [status, setStatus] = useState('draft')
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [createMut] = useMutation(CREATE_MODULE)
  const [updateMut] = useMutation(UPDATE_MODULE)

  useEffect(() => {
    if (open) {
      setError(null)
      if (editing) { setName(editing.name); setDescription(editing.description ?? ''); setBoundary(editing.boundary ?? ''); setStatus(editing.status) }
      else { setName(''); setDescription(''); setBoundary(''); setStatus('draft') }
    }
  }, [open, editing])

  async function handleSubmit() {
    setLoading(true); setError(null)
    try {
      if (editing) {
        await updateMut({ variables: { id: editing.id, name, description: description || null, boundary: boundary || null, status }, refetchQueries: [{ query: GET_MODULES, variables: { spaceId } }] })
      } else {
        await createMut({ variables: { spaceId, name, description: description || null, boundary: boundary || null, status }, refetchQueries: [{ query: GET_MODULES, variables: { spaceId } }] })
      }
      onOpenChange(false)
    } catch (err) { setError(err instanceof Error ? err.message : '操作失败') }
    finally { setLoading(false) }
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader><DialogTitle>{editing ? '编辑模块' : '新建模块'}</DialogTitle></DialogHeader>
        <div className="space-y-4 py-4">
          {error && <div className="rounded-md bg-destructive/10 p-3 text-sm text-destructive">{error}</div>}
          <div className="space-y-2"><Label htmlFor="fm-name">名称</Label><Input id="fm-name" value={name} onChange={e => setName(e.target.value)} /></div>
          <div className="space-y-2"><Label htmlFor="fm-boundary">边界</Label><Input id="fm-boundary" value={boundary} onChange={e => setBoundary(e.target.value)} /></div>
          <div className="space-y-2"><Label htmlFor="fm-desc">描述</Label><Input id="fm-desc" value={description} onChange={e => setDescription(e.target.value)} /></div>
          <div className="space-y-2">
            <Label>状态</Label>
            <select className="w-full rounded-md border bg-background px-3 py-2 text-sm" value={status} onChange={e => setStatus(e.target.value)}>
              <option value="draft">Draft</option><option value="active">Active</option><option value="deprecated">Deprecated</option>
            </select>
          </div>
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>取消</Button>
          <Button onClick={handleSubmit} disabled={loading || !name}>{loading ? <Loader2 className="h-4 w-4 animate-spin" /> : editing ? '保存' : '创建'}</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}

function ModuleDeleteDialog({ item, onConfirm, spaceId }: { item: FunctionalModule | null; onConfirm: () => void; spaceId?: string }) {
  const [deleteMut] = useMutation(DELETE_MODULE)
  const [loading, setLoading] = useState(false)
  async function handleDelete() {
    if (!item) return; setLoading(true)
    try { await deleteMut({ variables: { id: item.id }, refetchQueries: [{ query: GET_MODULES, variables: { spaceId } }] }); onConfirm() }
    catch (err) { console.error(err) } finally { setLoading(false) }
  }
  return (
    <Dialog open={!!item} onOpenChange={onConfirm}>
      <DialogContent>
        <DialogHeader><DialogTitle>确认删除</DialogTitle></DialogHeader>
        <p className="py-4 text-sm text-muted-foreground">确定要删除模块「{item?.name}」吗？</p>
        <DialogFooter>
          <Button variant="outline" onClick={onConfirm}>取消</Button>
          <Button variant="destructive" onClick={handleDelete} disabled={loading}>{loading ? <Loader2 className="h-4 w-4 animate-spin" /> : '删除'}</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}