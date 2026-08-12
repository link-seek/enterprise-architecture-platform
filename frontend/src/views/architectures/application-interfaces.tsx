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

const GET_INTERFACES = gql`
  query GetApplicationInterfaces($spaceId: String!) {
    applicationInterfacesBySpace(spaceId: $spaceId) { id name protocol contract providerModuleId consumerModuleId }
  }
`

const GET_MODULES = gql`
  query GetModulesForInterfaces($spaceId: String!) {
    functionalModulesBySpace(spaceId: $spaceId) { id name }
  }
`

const CREATE_INTERFACE = gql`
  mutation CreateApplicationInterface($spaceId: String!, $name: String!, $protocol: ApplicationInterfaceProtocolEnum!, $contract: String, $providerModuleId: String!, $consumerModuleId: String) {
    applicationInterfaceCreate(spaceId: $spaceId, name: $name, protocol: $protocol, contract: $contract, providerModuleId: $providerModuleId, consumerModuleId: $consumerModuleId) { id name }
  }
`

const UPDATE_INTERFACE = gql`
  mutation UpdateApplicationInterface($id: String!, $name: String, $protocol: ApplicationInterfaceProtocolEnum, $contract: String, $consumerModuleId: String) {
    applicationInterfaceUpdate(id: $id, name: $name, protocol: $protocol, contract: $contract, consumerModuleId: $consumerModuleId) { id name }
  }
`

const DELETE_INTERFACE = gql`
  mutation DeleteApplicationInterface($id: String!) { applicationInterfaceDelete(id: $id) }
`

interface AppInterface { id: string; name: string; protocol: string; contract?: string | null; providerModuleId: string; consumerModuleId?: string | null }
interface InterfaceQuery { applicationInterfacesBySpace?: AppInterface[] }
interface ModuleQuery { functionalModulesBySpace?: { id: string; name: string }[] }

const EMPTY: AppInterface[] = []
const EMPTY_MODULES: { id: string; name: string }[] = []

const InterfaceList = memo(function InterfaceList({ nodes, modules, canEdit, onEdit, onDelete }: {
  nodes: AppInterface[]; modules: { id: string; name: string }[]; canEdit: boolean; onEdit: (i: AppInterface) => void; onDelete: (i: AppInterface) => void
}) {
  if (nodes.length === 0) return <div className="text-center py-8 text-muted-foreground">暂无数据</div>
  const modName = (id?: string | null) => id ? (modules.find(m => m.id === id)?.name ?? id) : '—'
  return (
    <Table>
      <TableHeader>
        <TableRow>
          <TableHead>名称</TableHead>
          <TableHead>协议</TableHead>
          <TableHead>提供方</TableHead>
          <TableHead>消费方</TableHead>
          <TableHead>契约</TableHead>
          {canEdit && <TableHead>操作</TableHead>}
        </TableRow>
      </TableHeader>
      <TableBody>
        {nodes.map((i) => (
          <TableRow key={i.id}>
            <TableCell className="font-medium break-words">{i.name}</TableCell>
            <TableCell><Badge variant="secondary">{i.protocol}</Badge></TableCell>
            <TableCell className="text-muted-foreground">{modName(i.providerModuleId)}</TableCell>
            <TableCell className="text-muted-foreground">{modName(i.consumerModuleId)}</TableCell>
            <TableCell className="text-muted-foreground">{i.contract ?? '—'}</TableCell>
            {canEdit && (
              <TableCell>
                <div className="flex gap-1">
                  <Button variant="ghost" size="sm" aria-label="编辑" onClick={() => onEdit(i)}><Pencil className="h-3.5 w-3.5" /></Button>
                  <Button variant="ghost" size="sm" aria-label="删除" onClick={() => onDelete(i)}><Trash2 className="h-3.5 w-3.5 text-destructive" /></Button>
                </div>
              </TableCell>
            )}
          </TableRow>
        ))}
      </TableBody>
    </Table>
  )
})

export default function ApplicationInterfaces() {
  const { spaceId } = useParams<{ spaceId: string }>()
  const { canEdit } = useSpaceMembership(spaceId)
  const [dialogOpen, setDialogOpen] = useState(false)
  const [editing, setEditing] = useState<AppInterface | null>(null)
  const [deleting, setDeleting] = useState<AppInterface | null>(null)
  const { data, loading, error } = useQuery<InterfaceQuery>(GET_INTERFACES, { variables: { spaceId }, skip: !spaceId })
  const { data: modData } = useQuery<ModuleQuery>(GET_MODULES, { variables: { spaceId }, skip: !spaceId })

  const handleEdit = useCallback((i: AppInterface) => { setEditing(i); setDialogOpen(true) }, [])
  const handleDelete = useCallback((i: AppInterface) => setDeleting(i), [])

  return (
    <div className="p-4 md:p-6 space-y-4">
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-semibold">应用接口</h1>
        {canEdit && (
          <Button onClick={() => { setEditing(null); setDialogOpen(true) }}>
            <Plus className="h-4 w-4 mr-2" />新建接口
          </Button>
        )}
      </div>
      <Card>
        <CardHeader><CardTitle>接口列表</CardTitle></CardHeader>
        <CardContent>
          {loading && <div className="text-center py-8 text-muted-foreground">加载中...</div>}
          {Boolean(error) && <div className="text-center py-8 text-destructive">加载失败</div>}
          {data && <InterfaceList nodes={data.applicationInterfacesBySpace ?? EMPTY} modules={modData?.functionalModulesBySpace ?? EMPTY_MODULES} canEdit={canEdit} onEdit={handleEdit} onDelete={handleDelete} />}
        </CardContent>
      </Card>
      <InterfaceCrudDialog open={dialogOpen} onOpenChange={setDialogOpen} editing={editing} spaceId={spaceId} modules={modData?.functionalModulesBySpace ?? EMPTY_MODULES} />
      <InterfaceDeleteDialog item={deleting} onConfirm={() => setDeleting(null)} spaceId={spaceId} />
    </div>
  )
}

function InterfaceCrudDialog({ open, onOpenChange, editing, spaceId, modules }: {
  open: boolean; onOpenChange: (v: boolean) => void; editing: AppInterface | null; spaceId?: string; modules: { id: string; name: string }[]
}) {
  const [name, setName] = useState('')
  const [protocol, setProtocol] = useState('workflow_dispatch')
  const [contract, setContract] = useState('')
  const [providerModuleId, setProviderModuleId] = useState('')
  const [consumerModuleId, setConsumerModuleId] = useState('')
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [createMut] = useMutation(CREATE_INTERFACE)
  const [updateMut] = useMutation(UPDATE_INTERFACE)

  useEffect(() => {
    if (open) {
      setError(null)
      if (editing) { setName(editing.name); setProtocol(editing.protocol); setContract(editing.contract ?? ''); setProviderModuleId(editing.providerModuleId); setConsumerModuleId(editing.consumerModuleId ?? '') }
      else { setName(''); setProtocol('workflow_dispatch'); setContract(''); setProviderModuleId(modules[0]?.id ?? ''); setConsumerModuleId('') }
    }
  }, [open, editing, modules])

  async function handleSubmit() {
    setLoading(true); setError(null)
    try {
      if (editing) {
        await updateMut({ variables: { id: editing.id, name, protocol, contract: contract || null, consumerModuleId: consumerModuleId || null }, refetchQueries: [{ query: GET_INTERFACES, variables: { spaceId } }] })
      } else {
        await createMut({ variables: { spaceId, name, protocol, contract: contract || null, providerModuleId, consumerModuleId: consumerModuleId || null }, refetchQueries: [{ query: GET_INTERFACES, variables: { spaceId } }] })
      }
      onOpenChange(false)
    } catch (err) { setError(err instanceof Error ? err.message : '操作失败') }
    finally { setLoading(false) }
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader><DialogTitle>{editing ? '编辑接口' : '新建接口'}</DialogTitle></DialogHeader>
        <div className="space-y-4 py-4">
          {error && <div className="rounded-md bg-destructive/10 p-3 text-sm text-destructive">{error}</div>}
          <div className="space-y-2"><Label htmlFor="ai-name">名称</Label><Input id="ai-name" value={name} onChange={e => setName(e.target.value)} /></div>
          <div className="space-y-2">
            <Label>协议</Label>
            <select className="w-full rounded-md border bg-background px-3 py-2 text-sm" value={protocol} onChange={e => setProtocol(e.target.value)}>
              <option value="workflow_dispatch">workflow_dispatch</option><option value="api">api</option><option value="webhook">webhook</option>
            </select>
          </div>
          {!editing && (
            <div className="space-y-2">
              <Label>提供方模块</Label>
              <select className="w-full rounded-md border bg-background px-3 py-2 text-sm" value={providerModuleId} onChange={e => setProviderModuleId(e.target.value)}>
                {modules.map(m => <option key={m.id} value={m.id}>{m.name}</option>)}
              </select>
            </div>
          )}
          <div className="space-y-2"><Label htmlFor="ai-contract">契约</Label><Input id="ai-contract" value={contract} onChange={e => setContract(e.target.value)} /></div>
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>取消</Button>
          <Button onClick={handleSubmit} disabled={loading || !name || (!editing && !providerModuleId)}>{loading ? <Loader2 className="h-4 w-4 animate-spin" /> : editing ? '保存' : '创建'}</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}

function InterfaceDeleteDialog({ item, onConfirm, spaceId }: { item: AppInterface | null; onConfirm: () => void; spaceId?: string }) {
  const [deleteMut] = useMutation(DELETE_INTERFACE)
  const [loading, setLoading] = useState(false)
  async function handleDelete() {
    if (!item) return; setLoading(true)
    try { await deleteMut({ variables: { id: item.id }, refetchQueries: [{ query: GET_INTERFACES, variables: { spaceId } }] }); onConfirm() }
    catch (err) { console.error(err) } finally { setLoading(false) }
  }
  return (
    <Dialog open={!!item} onOpenChange={onConfirm}>
      <DialogContent>
        <DialogHeader><DialogTitle>确认删除</DialogTitle></DialogHeader>
        <p className="py-4 text-sm text-muted-foreground">确定要删除接口「{item?.name}」吗？</p>
        <DialogFooter>
          <Button variant="outline" onClick={onConfirm}>取消</Button>
          <Button variant="destructive" onClick={handleDelete} disabled={loading}>{loading ? <Loader2 className="h-4 w-4 animate-spin" /> : '删除'}</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}