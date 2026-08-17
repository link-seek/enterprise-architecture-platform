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
import { friendlyDeleteError } from './crud'

const GET_ORG_UNITS = gql`
  query GetOrgUnits($spaceId: String!) {
    organizationalUnitsBySpace(spaceId: $spaceId) {
      id name type parentId description status
    }
  }
`

const CREATE_ORG_UNIT = gql`
  mutation CreateOrgUnit($spaceId: String!, $name: String!, $type: OrganizationalUnitTypeEnum!, $parentId: String, $description: String, $status: String) {
    organizationalUnitCreate(spaceId: $spaceId, name: $name, type: $type, parentId: $parentId, description: $description, status: $status) { id name }
  }
`

const UPDATE_ORG_UNIT = gql`
  mutation UpdateOrgUnit($id: String!, $name: String, $type: OrganizationalUnitTypeEnum, $parentId: String, $description: String, $status: String) {
    organizationalUnitUpdate(id: $id, name: $name, type: $type, parentId: $parentId, description: $description, status: $status) { id name }
  }
`

const DELETE_ORG_UNIT = gql`
  mutation DeleteOrgUnit($id: String!) {
    organizationalUnitDelete(id: $id)
  }
`

interface OrgUnit {
  id: string; name: string; type: string; parentId?: string | null
  description?: string | null; status: string
}

interface OrgUnitQuery {
  organizationalUnitsBySpace?: OrgUnit[]
}

const EMPTY: OrgUnit[] = []

const OrgUnitList = memo(function OrgUnitList({ nodes, canEdit, onEdit, onDelete }: {
  nodes: OrgUnit[]; canEdit: boolean; onEdit: (o: OrgUnit) => void; onDelete: (o: OrgUnit) => void
}) {
  if (nodes.length === 0) return <div className="text-center py-8 text-muted-foreground">暂无数据</div>
  return (
    <Table>
      <TableHeader>
        <TableRow>
          <TableHead>名称</TableHead>
          <TableHead>类型</TableHead>
          <TableHead>状态</TableHead>
          <TableHead>描述</TableHead>
          {canEdit && <TableHead>操作</TableHead>}
        </TableRow>
      </TableHeader>
      <TableBody>
        {nodes.map((o) => (
          <TableRow key={o.id}>
            <TableCell className="font-medium break-words">{o.name}</TableCell>
            <TableCell><Badge variant="secondary">{o.type}</Badge></TableCell>
            <TableCell><Badge variant="outline">{o.status}</Badge></TableCell>
            <TableCell className="text-muted-foreground">{o.description ?? '—'}</TableCell>
            {canEdit && (
              <TableCell>
                <div className="flex gap-1">
                  <Button variant="ghost" size="sm" aria-label="编辑" onClick={() => onEdit(o)}><Pencil className="h-3.5 w-3.5" /></Button>
                  <Button variant="ghost" size="sm" aria-label="删除" onClick={() => onDelete(o)}><Trash2 className="h-3.5 w-3.5 text-destructive" /></Button>
                </div>
              </TableCell>
            )}
          </TableRow>
        ))}
      </TableBody>
    </Table>
  )
})

export default function OrganizationalUnits() {
  const { spaceId } = useParams<{ spaceId: string }>()
  const { canEdit } = useSpaceMembership(spaceId)
  const [dialogOpen, setDialogOpen] = useState(false)
  const [editing, setEditing] = useState<OrgUnit | null>(null)
  const [deleting, setDeleting] = useState<OrgUnit | null>(null)
  const { data, loading, error } = useQuery<OrgUnitQuery>(GET_ORG_UNITS, { variables: { spaceId }, skip: !spaceId })

  const handleEdit = useCallback((o: OrgUnit) => { setEditing(o); setDialogOpen(true) }, [])
  const handleDelete = useCallback((o: OrgUnit) => setDeleting(o), [])

  return (
    <div className="p-4 md:p-6 space-y-4">
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-semibold">组织单元</h1>
        {canEdit && (
          <Button onClick={() => { setEditing(null); setDialogOpen(true) }}>
            <Plus className="h-4 w-4 mr-2" />新建组织单元
          </Button>
        )}
      </div>
      <Card>
        <CardHeader><CardTitle>组织单元列表</CardTitle></CardHeader>
        <CardContent>
          {loading && <div className="text-center py-8 text-muted-foreground">加载中...</div>}
          {Boolean(error) && <div className="text-center py-8 text-destructive">加载失败</div>}
          {data && <OrgUnitList nodes={data.organizationalUnitsBySpace ?? EMPTY} canEdit={canEdit} onEdit={handleEdit} onDelete={handleDelete} />}
        </CardContent>
      </Card>
      <OrgUnitCrudDialog open={dialogOpen} onOpenChange={setDialogOpen} editing={editing} spaceId={spaceId} />
      <OrgUnitDeleteDialog item={deleting} onConfirm={() => setDeleting(null)} spaceId={spaceId} />
    </div>
  )
}

function OrgUnitCrudDialog({ open, onOpenChange, editing, spaceId }: {
  open: boolean; onOpenChange: (v: boolean) => void; editing: OrgUnit | null; spaceId?: string
}) {
  const [name, setName] = useState('')
  const [type, setType] = useState('team')
  const [description, setDescription] = useState('')
  const [status, setStatus] = useState('active')
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [createMut] = useMutation(CREATE_ORG_UNIT)
  const [updateMut] = useMutation(UPDATE_ORG_UNIT)

  useEffect(() => {
    if (open) {
      setError(null)
      if (editing) {
        setName(editing.name); setType(editing.type); setDescription(editing.description ?? ''); setStatus(editing.status)
      } else {
        setName(''); setType('team'); setDescription(''); setStatus('active')
      }
    }
  }, [open, editing])

  async function handleSubmit() {
    setLoading(true); setError(null)
    try {
      if (editing) {
        await updateMut({ variables: { id: editing.id, name, type, description: description || null, status }, refetchQueries: [{ query: GET_ORG_UNITS, variables: { spaceId } }] })
      } else {
        await createMut({ variables: { spaceId, name, type, description: description || null, status }, refetchQueries: [{ query: GET_ORG_UNITS, variables: { spaceId } }] })
      }
      onOpenChange(false)
    } catch (err) { setError(err instanceof Error ? err.message : '操作失败') }
    finally { setLoading(false) }
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader><DialogTitle>{editing ? '编辑组织单元' : '新建组织单元'}</DialogTitle></DialogHeader>
        <div className="space-y-4 py-4">
          {error && <div className="rounded-md bg-destructive/10 p-3 text-sm text-destructive">{error}</div>}
          <div className="space-y-2"><Label htmlFor="ou-name">名称</Label><Input id="ou-name" value={name} onChange={e => setName(e.target.value)} /></div>
          <div className="space-y-2">
            <Label>类型</Label>
            <select className="w-full rounded-md border bg-background px-3 py-2 text-sm" value={type} onChange={e => setType(e.target.value)}>
              <option value="team">Team</option><option value="role">Role</option><option value="unit">Unit</option><option value="external">External</option>
            </select>
          </div>
          <div className="space-y-2"><Label htmlFor="ou-desc">描述</Label><Input id="ou-desc" value={description} onChange={e => setDescription(e.target.value)} /></div>
          <div className="space-y-2">
            <Label>状态</Label>
            <select className="w-full rounded-md border bg-background px-3 py-2 text-sm" value={status} onChange={e => setStatus(e.target.value)}>
              <option value="active">Active</option><option value="inactive">Inactive</option>
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

function OrgUnitDeleteDialog({ item, onConfirm, spaceId }: { item: OrgUnit | null; onConfirm: () => void; spaceId?: string }) {
  const [deleteMut] = useMutation(DELETE_ORG_UNIT)
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  useEffect(() => { setError(null) }, [item])
  async function handleDelete() {
    if (!item) return; setLoading(true); setError(null)
    try { await deleteMut({ variables: { id: item.id }, refetchQueries: [{ query: GET_ORG_UNITS, variables: { spaceId } }] }); onConfirm() }
    catch (err) { setError(friendlyDeleteError(err)) } finally { setLoading(false) }
  }
  return (
    <Dialog open={!!item} onOpenChange={onConfirm}>
      <DialogContent>
        <DialogHeader><DialogTitle>确认删除</DialogTitle></DialogHeader>
        <p className="py-4 text-sm text-muted-foreground">确定要删除组织单元「{item?.name}」吗？</p>
        {error && <div className="rounded-md bg-destructive/10 p-3 text-sm text-destructive">{error}</div>}
        <DialogFooter>
          <Button variant="outline" onClick={onConfirm}>取消</Button>
          <Button variant="destructive" onClick={handleDelete} disabled={loading}>{loading ? <Loader2 className="h-4 w-4 animate-spin" /> : '删除'}</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}