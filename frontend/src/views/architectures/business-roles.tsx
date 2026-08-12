import { useQuery, useMutation } from '@apollo/client/react'
import { gql } from '@apollo/client'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { Plus, Pencil, Trash2, Loader2 } from 'lucide-react'
import { useState, useEffect, useCallback, memo } from 'react'
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogFooter } from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { useParams } from 'react-router-dom'
import { useSpaceMembership } from '@/hooks/use-space-membership'

const GET_BUSINESS_ROLES = gql`
  query GetBusinessRoles($spaceId: String!) {
    businessRolesBySpace(spaceId: $spaceId) { id name responsibilities organizationId }
  }
`

const GET_ORG_UNITS = gql`
  query GetOrgUnitsForRoles($spaceId: String!) {
    organizationalUnitsBySpace(spaceId: $spaceId) { id name }
  }
`

const CREATE_ROLE = gql`
  mutation CreateBusinessRole($spaceId: String!, $name: String!, $responsibilities: String, $organizationId: String!) {
    businessRoleCreate(spaceId: $spaceId, name: $name, responsibilities: $responsibilities, organizationId: $organizationId) { id name }
  }
`

const UPDATE_ROLE = gql`
  mutation UpdateBusinessRole($id: String!, $name: String, $responsibilities: String) {
    businessRoleUpdate(id: $id, name: $name, responsibilities: $responsibilities) { id name }
  }
`

const DELETE_ROLE = gql`
  mutation DeleteBusinessRole($id: String!) { businessRoleDelete(id: $id) }
`

interface BusinessRole { id: string; name: string; responsibilities?: string | null; organizationId: string }
interface BusinessRoleQuery { businessRolesBySpace?: BusinessRole[] }
interface OrgUnitQuery { organizationalUnitsBySpace?: { id: string; name: string }[] }

const EMPTY: BusinessRole[] = []
const EMPTY_ORG: { id: string; name: string }[] = []

const RoleList = memo(function RoleList({ nodes, orgUnits, canEdit, onEdit, onDelete }: {
  nodes: BusinessRole[]; orgUnits: { id: string; name: string }[]; canEdit: boolean; onEdit: (r: BusinessRole) => void; onDelete: (r: BusinessRole) => void
}) {
  if (nodes.length === 0) return <div className="text-center py-8 text-muted-foreground">暂无数据</div>
  const orgName = (id: string) => orgUnits.find(o => o.id === id)?.name ?? id
  return (
    <Table>
      <TableHeader>
        <TableRow>
          <TableHead>名称</TableHead>
          <TableHead>所属组织</TableHead>
          <TableHead>职责</TableHead>
          {canEdit && <TableHead>操作</TableHead>}
        </TableRow>
      </TableHeader>
      <TableBody>
        {nodes.map((r) => (
          <TableRow key={r.id}>
            <TableCell className="font-medium break-words">{r.name}</TableCell>
            <TableCell className="text-muted-foreground">{orgName(r.organizationId)}</TableCell>
            <TableCell className="text-muted-foreground">{r.responsibilities ?? '—'}</TableCell>
            {canEdit && (
              <TableCell>
                <div className="flex gap-1">
                  <Button variant="ghost" size="sm" aria-label="编辑" onClick={() => onEdit(r)}><Pencil className="h-3.5 w-3.5" /></Button>
                  <Button variant="ghost" size="sm" aria-label="删除" onClick={() => onDelete(r)}><Trash2 className="h-3.5 w-3.5 text-destructive" /></Button>
                </div>
              </TableCell>
            )}
          </TableRow>
        ))}
      </TableBody>
    </Table>
  )
})

export default function BusinessRoles() {
  const { spaceId } = useParams<{ spaceId: string }>()
  const { canEdit } = useSpaceMembership(spaceId)
  const [dialogOpen, setDialogOpen] = useState(false)
  const [editing, setEditing] = useState<BusinessRole | null>(null)
  const [deleting, setDeleting] = useState<BusinessRole | null>(null)
  const { data, loading, error } = useQuery<BusinessRoleQuery>(GET_BUSINESS_ROLES, { variables: { spaceId }, skip: !spaceId })
  const { data: orgData } = useQuery<OrgUnitQuery>(GET_ORG_UNITS, { variables: { spaceId }, skip: !spaceId })

  const handleEdit = useCallback((r: BusinessRole) => { setEditing(r); setDialogOpen(true) }, [])
  const handleDelete = useCallback((r: BusinessRole) => setDeleting(r), [])

  return (
    <div className="p-4 md:p-6 space-y-4">
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-semibold">业务角色</h1>
        {canEdit && (
          <Button onClick={() => { setEditing(null); setDialogOpen(true) }}>
            <Plus className="h-4 w-4 mr-2" />新建角色
          </Button>
        )}
      </div>
      <Card>
        <CardHeader><CardTitle>角色列表</CardTitle></CardHeader>
        <CardContent>
          {loading && <div className="text-center py-8 text-muted-foreground">加载中...</div>}
          {Boolean(error) && <div className="text-center py-8 text-destructive">加载失败</div>}
          {data && <RoleList nodes={data.businessRolesBySpace ?? EMPTY} orgUnits={orgData?.organizationalUnitsBySpace ?? EMPTY_ORG} canEdit={canEdit} onEdit={handleEdit} onDelete={handleDelete} />}
        </CardContent>
      </Card>
      <RoleCrudDialog open={dialogOpen} onOpenChange={setDialogOpen} editing={editing} spaceId={spaceId} orgUnits={orgData?.organizationalUnitsBySpace ?? EMPTY_ORG} />
      <RoleDeleteDialog item={deleting} onConfirm={() => setDeleting(null)} spaceId={spaceId} />
    </div>
  )
}

function RoleCrudDialog({ open, onOpenChange, editing, spaceId, orgUnits }: {
  open: boolean; onOpenChange: (v: boolean) => void; editing: BusinessRole | null; spaceId?: string; orgUnits: { id: string; name: string }[]
}) {
  const [name, setName] = useState('')
  const [responsibilities, setResponsibilities] = useState('')
  const [organizationId, setOrganizationId] = useState('')
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [createMut] = useMutation(CREATE_ROLE)
  const [updateMut] = useMutation(UPDATE_ROLE)

  useEffect(() => {
    if (open) {
      setError(null)
      if (editing) { setName(editing.name); setResponsibilities(editing.responsibilities ?? ''); setOrganizationId(editing.organizationId) }
      else { setName(''); setResponsibilities(''); setOrganizationId(orgUnits[0]?.id ?? '') }
    }
  }, [open, editing, orgUnits])

  async function handleSubmit() {
    setLoading(true); setError(null)
    try {
      if (editing) {
        await updateMut({ variables: { id: editing.id, name, responsibilities: responsibilities || null }, refetchQueries: [{ query: GET_BUSINESS_ROLES, variables: { spaceId } }] })
      } else {
        await createMut({ variables: { spaceId, name, responsibilities: responsibilities || null, organizationId }, refetchQueries: [{ query: GET_BUSINESS_ROLES, variables: { spaceId } }] })
      }
      onOpenChange(false)
    } catch (err) { setError(err instanceof Error ? err.message : '操作失败') }
    finally { setLoading(false) }
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader><DialogTitle>{editing ? '编辑角色' : '新建角色'}</DialogTitle></DialogHeader>
        <div className="space-y-4 py-4">
          {error && <div className="rounded-md bg-destructive/10 p-3 text-sm text-destructive">{error}</div>}
          <div className="space-y-2"><Label htmlFor="role-name">名称</Label><Input id="role-name" value={name} onChange={e => setName(e.target.value)} /></div>
          {!editing && (
            <div className="space-y-2">
              <Label>所属组织单元</Label>
              <select className="w-full rounded-md border bg-background px-3 py-2 text-sm" value={organizationId} onChange={e => setOrganizationId(e.target.value)}>
                {orgUnits.map(o => <option key={o.id} value={o.id}>{o.name}</option>)}
              </select>
            </div>
          )}
          <div className="space-y-2"><Label htmlFor="role-resp">职责</Label><Input id="role-resp" value={responsibilities} onChange={e => setResponsibilities(e.target.value)} /></div>
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>取消</Button>
          <Button onClick={handleSubmit} disabled={loading || !name || (!editing && !organizationId)}>{loading ? <Loader2 className="h-4 w-4 animate-spin" /> : editing ? '保存' : '创建'}</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}

function RoleDeleteDialog({ item, onConfirm, spaceId }: { item: BusinessRole | null; onConfirm: () => void; spaceId?: string }) {
  const [deleteMut] = useMutation(DELETE_ROLE)
  const [loading, setLoading] = useState(false)
  async function handleDelete() {
    if (!item) return; setLoading(true)
    try { await deleteMut({ variables: { id: item.id }, refetchQueries: [{ query: GET_BUSINESS_ROLES, variables: { spaceId } }] }); onConfirm() }
    catch (err) { console.error(err) } finally { setLoading(false) }
  }
  return (
    <Dialog open={!!item} onOpenChange={onConfirm}>
      <DialogContent>
        <DialogHeader><DialogTitle>确认删除</DialogTitle></DialogHeader>
        <p className="py-4 text-sm text-muted-foreground">确定要删除角色「{item?.name}」吗？</p>
        <DialogFooter>
          <Button variant="outline" onClick={onConfirm}>取消</Button>
          <Button variant="destructive" onClick={handleDelete} disabled={loading}>{loading ? <Loader2 className="h-4 w-4 animate-spin" /> : '删除'}</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}