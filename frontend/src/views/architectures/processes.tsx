import { useQuery, useMutation } from '@apollo/client/react'
import { gql } from '@apollo/client'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table'
import { Badge } from '@/components/ui/badge'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { Plus, Pencil, Trash2, Loader2 } from 'lucide-react'
import { useState, useEffect } from 'react'
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogFooter } from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { useParams } from 'react-router-dom'
import { useSpaceMembership } from '@/hooks/use-space-membership'

const GET_PROCESSES = gql`
  query GetProcesses($spaceId: String) {
    businessProcesses(filters: { spaceId: { eq: $spaceId } }) {
      nodes { id name description sla cycleTime costPerTransaction status }
      paginationInfo { total }
    }
  }
`

const CREATE_PROCESS = gql`
  mutation CreateProcess($spaceId: String!, $name: String!, $description: String, $sla: String, $cycleTime: Int, $costPerTransaction: Float) {
    processCreate(spaceId: $spaceId, name: $name, description: $description, sla: $sla, cycleTime: $cycleTime, costPerTransaction: $costPerTransaction) { id name }
  }
`

const UPDATE_PROCESS = gql`
  mutation UpdateProcess($id: String!, $name: String, $description: String, $sla: String, $cycleTime: Int, $costPerTransaction: Float) {
    processUpdate(id: $id, name: $name, description: $description, sla: $sla, cycleTime: $cycleTime, costPerTransaction: $costPerTransaction) { id name }
  }
`

const DELETE_PROCESS = gql`
  mutation DeleteProcess($id: String!) {
    processDelete(id: $id)
  }
`

interface Process {
  id: string; name: string; description: string
  sla: string | null; cycleTime: number | null; costPerTransaction: number | null; status: string
}

export default function Processes() {
  const { spaceId } = useParams<{ spaceId: string }>()
  const { canEdit } = useSpaceMembership(spaceId)
  const [dialogOpen, setDialogOpen] = useState(false)
  const [editing, setEditing] = useState<Process | null>(null)
  const [deleting, setDeleting] = useState<Process | null>(null)
  const { data, loading, error } = useQuery(GET_PROCESSES, { variables: { spaceId } })

  return (
    <div className="p-6 space-y-4">
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-semibold">业务流程</h1>
        {canEdit && (
          <Button onClick={() => { setEditing(null); setDialogOpen(true) }}>
            <Plus className="h-4 w-4 mr-2" />新建流程
          </Button>
        )}
      </div>
      <Card>
        <CardHeader><CardTitle>流程列表</CardTitle></CardHeader>
        <CardContent>
          {loading && <div className="text-center py-8 text-muted-foreground">加载中...</div>}
          {error && <div className="text-center py-8 text-destructive">加载失败</div>}
          {data && (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>名称</TableHead>
                  <TableHead>描述</TableHead>
                  <TableHead>SLA</TableHead>
                  <TableHead>周期(天)</TableHead>
                  <TableHead>单次成本</TableHead>
                  <TableHead>状态</TableHead>
                  <TableHead>操作</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {data.businessProcesses.nodes.map((p: Process) => (
                  <TableRow key={p.id}>
                    <TableCell className="font-medium">{p.name}</TableCell>
                    <TableCell className="text-muted-foreground">{p.description}</TableCell>
                    <TableCell>{p.sla || '-'}</TableCell>
                    <TableCell>{p.cycleTime || '-'}</TableCell>
                    <TableCell>{p.costPerTransaction || '-'}</TableCell>
                    <TableCell><Badge variant="outline">{p.status}</Badge></TableCell>
                    <TableCell>
                      <div className="flex gap-1">
                        {canEdit && (
                          <>
                            <Button variant="ghost" size="sm" onClick={() => { setEditing(p); setDialogOpen(true) }}>
                              <Pencil className="h-3.5 w-3.5" />
                            </Button>
                            <Button variant="ghost" size="sm" onClick={() => setDeleting(p)}>
                              <Trash2 className="h-3.5 w-3.5 text-destructive" />
                            </Button>
                          </>
                        )}
                      </div>
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          )}
        </CardContent>
      </Card>
      <ProcessCrudDialog open={dialogOpen} onOpenChange={setDialogOpen} editing={editing} spaceId={spaceId} />
      <ProcessDeleteDialog item={deleting} onConfirm={() => setDeleting(null)} spaceId={spaceId} />
    </div>
  )
}

function ProcessCrudDialog({ open, onOpenChange, editing, spaceId }: {
  open: boolean; onOpenChange: (v: boolean) => void; editing: Process | null; spaceId?: string
}) {
  const [name, setName] = useState('')
  const [description, setDescription] = useState('')
  const [sla, setSla] = useState('')
  const [cycleTime, setCycleTime] = useState('')
  const [cost, setCost] = useState('')
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [createMut] = useMutation(CREATE_PROCESS)
  const [updateMut] = useMutation(UPDATE_PROCESS)

  useEffect(() => {
    if (open) {
      setError(null)
      if (editing) {
        setName(editing.name); setDescription(editing.description)
        setSla(editing.sla || ''); setCycleTime(editing.cycleTime?.toString() || '')
        setCost(editing.costPerTransaction?.toString() || '')
      } else {
        setName(''); setDescription(''); setSla(''); setCycleTime(''); setCost('')
      }
    }
  }, [open, editing])

  async function handleSubmit() {
    setLoading(true); setError(null)
    try {
      const ct = cycleTime ? parseInt(cycleTime) : null
      const cp = cost ? parseFloat(cost) : null
      if (editing) {
        await updateMut({
          variables: { id: editing.id, name, description, sla, cycleTime: ct, costPerTransaction: cp },
          refetchQueries: [{ query: GET_PROCESSES, variables: { spaceId } }],
        })
      } else {
        await createMut({
          variables: { spaceId, name, description, sla, cycleTime: ct, costPerTransaction: cp },
          refetchQueries: [{ query: GET_PROCESSES, variables: { spaceId } }],
        })
      }
      onOpenChange(false)
    } catch (err) {
      setError(err instanceof Error ? err.message : '操作失败')
    } finally { setLoading(false) }
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader><DialogTitle>{editing ? '编辑流程' : '新建流程'}</DialogTitle></DialogHeader>
        <div className="space-y-4 py-4">
          {error && <div className="rounded-md bg-destructive/10 p-3 text-sm text-destructive">{error}</div>}
          <div className="space-y-2"><Label>名称</Label><Input value={name} onChange={e => setName(e.target.value)} /></div>
          <div className="space-y-2"><Label>描述</Label><Input value={description} onChange={e => setDescription(e.target.value)} /></div>
          <div className="grid grid-cols-3 gap-4">
            <div className="space-y-2"><Label>SLA</Label><Input value={sla} onChange={e => setSla(e.target.value)} placeholder="2天" /></div>
            <div className="space-y-2"><Label>周期(天)</Label><Input type="number" value={cycleTime} onChange={e => setCycleTime(e.target.value)} /></div>
            <div className="space-y-2"><Label>单次成本</Label><Input type="number" value={cost} onChange={e => setCost(e.target.value)} /></div>
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

function ProcessDeleteDialog({ item, onConfirm, spaceId }: { item: Process | null; onConfirm: () => void; spaceId?: string }) {
  const [deleteMut] = useMutation(DELETE_PROCESS)
  const [loading, setLoading] = useState(false)
  async function handleDelete() {
    if (!item) return; setLoading(true)
    try { await deleteMut({ variables: { id: item.id }, refetchQueries: [{ query: GET_PROCESSES, variables: { spaceId } }] }); onConfirm() }
    catch (err) { console.error(err) } finally { setLoading(false) }
  }
  return (
    <Dialog open={!!item} onOpenChange={onConfirm}>
      <DialogContent>
        <DialogHeader><DialogTitle>确认删除</DialogTitle></DialogHeader>
        <p className="py-4 text-sm text-muted-foreground">确定要删除流程「{item?.name}」吗？</p>
        <DialogFooter>
          <Button variant="outline" onClick={onConfirm}>取消</Button>
          <Button variant="destructive" onClick={handleDelete} disabled={loading}>{loading ? <Loader2 className="h-4 w-4 animate-spin" /> : '删除'}</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
