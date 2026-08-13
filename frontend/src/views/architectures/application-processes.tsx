import { useQuery, useMutation } from '@apollo/client/react'
import { gql } from '@apollo/client'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table'
import { Badge } from '@/components/ui/badge'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from '@/components/ui/dropdown-menu'
import { Plus, Pencil, Trash2, Loader2, MoreVertical, Workflow } from 'lucide-react'
import { useState, useEffect, useCallback, useMemo, memo } from 'react'
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogFooter } from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { useParams } from 'react-router-dom'
import { useSpaceMembership } from '@/hooks/use-space-membership'
import { useIsMobile } from '@/hooks/use-media-query'
import { CrossDomainDialog, type CrossDomainItem } from './cross-domain-dialog'

const GET_APPLICATION_PROCESSES = gql`
  query GetApplicationProcesses($spaceId: String!) {
    applicationProcessesBySpace(spaceId: $spaceId) {
      id name description trigger timeout retry status
    }
  }
`

const CREATE_APPLICATION_PROCESS = gql`
  mutation CreateApplicationProcess($spaceId: String!, $name: String!, $description: String, $trigger: ApplicationProcessTriggerEnum!, $timeout: Int, $retry: Int) {
    applicationProcessCreate(spaceId: $spaceId, name: $name, description: $description, trigger: $trigger, timeout: $timeout, retry: $retry) { id name }
  }
`

const UPDATE_APPLICATION_PROCESS = gql`
  mutation UpdateApplicationProcess($id: String!, $name: String, $description: String, $trigger: ApplicationProcessTriggerEnum, $timeout: Int, $retry: Int, $status: LifecycleStatusEnum) {
    applicationProcessUpdate(id: $id, name: $name, description: $description, trigger: $trigger, timeout: $timeout, retry: $retry, status: $status) { id name }
  }
`

const DELETE_APPLICATION_PROCESS = gql`
  mutation DeleteApplicationProcess($id: String!) {
    applicationProcessDelete(id: $id)
  }
`

// 跨域关联（R3）：应用流程支撑了哪些业务流程（process_references）。
const GET_PROCESS_REFERENCES_BY_APPLICATION = gql`
  query GetProcessReferencesByApplication($applicationProcessId: String!) {
    processReferencesByApplicationProcess(applicationProcessId: $applicationProcessId) {
      applicationProcessId businessProcessId
    }
  }
`

const GET_BUSINESS_PROCESSES_BY_SPACE = gql`
  query GetBusinessProcessesForNames($spaceId: String!) {
    businessProcessesBySpace(spaceId: $spaceId) { id name }
  }
`

interface ApplicationProcess {
  id: string; name: string; description: string; trigger: string
  timeout: number | null; retry: number | null; status: string
}

interface ApplicationProcessesQuery {
  applicationProcessesBySpace?: ApplicationProcess[]
}

const EMPTY_PROCESSES: ApplicationProcess[] = []

const TRIGGER_LABELS: Record<string, string> = {
  push: '推送', pull_request: '合并请求', schedule: '定时', manual: '手动', webhook: 'Webhook',
}

const ProcessList = memo(function ProcessList({ nodes, canEdit, isMobile, onEdit, onDelete, onBusinessSupport }: {
  nodes: ApplicationProcess[]
  canEdit: boolean
  isMobile: boolean
  onEdit: (p: ApplicationProcess) => void
  onDelete: (p: ApplicationProcess) => void
  onBusinessSupport: (p: ApplicationProcess) => void
}) {
  if (nodes.length === 0) {
    return <div className="text-center py-8 text-muted-foreground">暂无数据</div>
  }

  if (isMobile) {
    return (
      <div className="space-y-3">
        {nodes.map((p) => (
          <div key={p.id} className="rounded-lg border p-4 space-y-2">
            <div className="flex items-start justify-between gap-2">
              <div className="min-w-0">
                <p className="font-medium truncate" title={p.name}>{p.name}</p>
                <p className="text-xs text-muted-foreground truncate" title={p.description}>{p.description}</p>
              </div>
              <Badge variant="outline" className="shrink-0">{p.status}</Badge>
            </div>
            <div className="flex flex-wrap gap-1 text-xs text-muted-foreground">
              <Badge variant="secondary">{TRIGGER_LABELS[p.trigger] ?? p.trigger}</Badge>
              <span>超时: {p.timeout ?? '-'}</span>
              <span>重试: {p.retry ?? '-'}</span>
            </div>
            <div className="flex justify-end gap-1 pt-1">
              <Button variant="ghost" size="sm" aria-label="支撑业务" title="支撑业务" onClick={() => onBusinessSupport(p)}>
                <Workflow className="h-3.5 w-3.5" />
              </Button>
              {canEdit && (
                <DropdownMenu>
                  <DropdownMenuTrigger asChild>
                    <Button variant="ghost" size="sm" className="h-9 w-9 p-0" aria-label="更多操作">
                      <MoreVertical className="h-4 w-4" />
                    </Button>
                  </DropdownMenuTrigger>
                  <DropdownMenuContent align="end">
                    <DropdownMenuItem onClick={() => onEdit(p)}>
                      <Pencil className="h-4 w-4 mr-2" />编辑
                    </DropdownMenuItem>
                    <DropdownMenuItem className="text-destructive" onClick={() => onDelete(p)}>
                      <Trash2 className="h-4 w-4 mr-2" />删除
                    </DropdownMenuItem>
                  </DropdownMenuContent>
                </DropdownMenu>
              )}
            </div>
          </div>
        ))}
      </div>
    )
  }

  return (
    <Table>
      <TableHeader>
        <TableRow>
          <TableHead>名称</TableHead>
          <TableHead>描述</TableHead>
          <TableHead>触发方式</TableHead>
          <TableHead>超时(秒)</TableHead>
          <TableHead>重试次数</TableHead>
          <TableHead>状态</TableHead>
          <TableHead>操作</TableHead>
        </TableRow>
      </TableHeader>
      <TableBody>
        {nodes.map((p) => (
          <TableRow key={p.id}>
            <TableCell className="font-medium">{p.name}</TableCell>
            <TableCell className="text-muted-foreground">{p.description}</TableCell>
            <TableCell>{TRIGGER_LABELS[p.trigger] ?? p.trigger}</TableCell>
            <TableCell>{p.timeout ?? '-'}</TableCell>
            <TableCell>{p.retry ?? '-'}</TableCell>
            <TableCell><Badge variant="outline">{p.status}</Badge></TableCell>
            <TableCell>
              <div className="flex gap-1">
                <Button variant="ghost" size="sm" aria-label="支撑业务" title="支撑业务" onClick={() => onBusinessSupport(p)}>
                  <Workflow className="h-3.5 w-3.5" />
                </Button>
                {canEdit && (
                  <>
                    <Button variant="ghost" size="sm" onClick={() => onEdit(p)} aria-label="编辑">
                      <Pencil className="h-3.5 w-3.5" />
                    </Button>
                    <Button variant="ghost" size="sm" onClick={() => onDelete(p)} aria-label="删除">
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
  )
})

export default function ApplicationProcesses() {
  const { spaceId } = useParams<{ spaceId: string }>()
  const { canEdit } = useSpaceMembership(spaceId)
  const isMobile = useIsMobile()
  const [dialogOpen, setDialogOpen] = useState(false)
  const [editing, setEditing] = useState<ApplicationProcess | null>(null)
  const [deleting, setDeleting] = useState<ApplicationProcess | null>(null)
  const [businessSupport, setBusinessSupport] = useState<ApplicationProcess | null>(null)
  const { data, loading, error } = useQuery<ApplicationProcessesQuery>(GET_APPLICATION_PROCESSES, { variables: { spaceId }, skip: !spaceId })

  const handleEdit = useCallback((p: ApplicationProcess) => { setEditing(p); setDialogOpen(true) }, [])
  const handleDelete = useCallback((p: ApplicationProcess) => setDeleting(p), [])
  const handleBusinessSupport = useCallback((p: ApplicationProcess) => setBusinessSupport(p), [])

  return (
    <div className="p-4 md:p-6 space-y-4">
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-semibold">应用流程</h1>
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
          {Boolean(error) && <div className="text-center py-8 text-destructive">加载失败</div>}
          {data && (
            <ProcessList
              nodes={data.applicationProcessesBySpace ?? EMPTY_PROCESSES}
              canEdit={canEdit}
              isMobile={isMobile}
              onEdit={handleEdit}
              onDelete={handleDelete}
              onBusinessSupport={handleBusinessSupport}
            />
          )}
        </CardContent>
      </Card>
      <ProcessCrudDialog open={dialogOpen} onOpenChange={setDialogOpen} editing={editing} spaceId={spaceId} />
      <ProcessDeleteDialog item={deleting} onConfirm={() => setDeleting(null)} spaceId={spaceId} />
      <BusinessSupportDialog
        process={businessSupport}
        spaceId={spaceId}
        onOpenChange={(v) => { if (!v) setBusinessSupport(null) }}
      />
    </div>
  )
}

function ProcessCrudDialog({ open, onOpenChange, editing, spaceId }: {
  open: boolean; onOpenChange: (v: boolean) => void; editing: ApplicationProcess | null; spaceId?: string
}) {
  const [name, setName] = useState('')
  const [description, setDescription] = useState('')
  const [trigger, setTrigger] = useState('push')
  const [timeout, setTimeout] = useState('')
  const [retry, setRetry] = useState('')
  const [status, setStatus] = useState('active')
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [createMut] = useMutation(CREATE_APPLICATION_PROCESS)
  const [updateMut] = useMutation(UPDATE_APPLICATION_PROCESS)

  useEffect(() => {
    if (open) {
      setError(null)
      if (editing) {
        setName(editing.name); setDescription(editing.description); setTrigger(editing.trigger)
        setTimeout(editing.timeout?.toString() || ''); setRetry(editing.retry?.toString() || '')
        setStatus(editing.status)
      } else {
        setName(''); setDescription(''); setTrigger('push'); setTimeout(''); setRetry(''); setStatus('active')
      }
    }
  }, [open, editing])

  async function handleSubmit() {
    setLoading(true); setError(null)
    try {
      const to = timeout ? parseInt(timeout) : null
      const rt = retry ? parseInt(retry) : null
      if (editing) {
        await updateMut({
          variables: { id: editing.id, name, description, trigger, timeout: to, retry: rt, status },
          refetchQueries: [{ query: GET_APPLICATION_PROCESSES, variables: { spaceId } }],
        })
      } else {
        await createMut({
          variables: { spaceId, name, description, trigger, timeout: to, retry: rt },
          refetchQueries: [{ query: GET_APPLICATION_PROCESSES, variables: { spaceId } }],
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
          <div className="grid grid-cols-1 sm:grid-cols-3 gap-4">
            <div className="space-y-2">
              <Label>触发方式</Label>
              <select className="w-full rounded-md border bg-background px-3 py-2 text-sm" value={trigger} onChange={e => setTrigger(e.target.value)}>
                <option value="push">推送</option><option value="pull_request">合并请求</option><option value="schedule">定时</option><option value="manual">手动</option><option value="webhook">Webhook</option>
              </select>
            </div>
            <div className="space-y-2"><Label>超时(秒)</Label><Input type="number" value={timeout} onChange={e => setTimeout(e.target.value)} /></div>
            <div className="space-y-2"><Label>重试次数</Label><Input type="number" value={retry} onChange={e => setRetry(e.target.value)} /></div>
          </div>
          {editing && (
            <div className="space-y-2">
              <Label>状态</Label>
              <select className="w-full rounded-md border bg-background px-3 py-2 text-sm" value={status} onChange={e => setStatus(e.target.value)}>
                <option value="active">活跃</option><option value="inactive">停用</option><option value="draft">草稿</option>
              </select>
            </div>
          )}
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>取消</Button>
          <Button onClick={handleSubmit} disabled={loading || !name}>{loading ? <Loader2 className="h-4 w-4 animate-spin" /> : editing ? '保存' : '创建'}</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}

function ProcessDeleteDialog({ item, onConfirm, spaceId }: { item: ApplicationProcess | null; onConfirm: () => void; spaceId?: string }) {
  const [deleteMut] = useMutation(DELETE_APPLICATION_PROCESS)
  const [loading, setLoading] = useState(false)
  async function handleDelete() {
    if (!item) return; setLoading(true)
    try { await deleteMut({ variables: { id: item.id }, refetchQueries: [{ query: GET_APPLICATION_PROCESSES, variables: { spaceId } }] }); onConfirm() }
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

// 跨域关联（R3）：展示该应用流程支撑的业务流程（process_references），
// 行可跳转到业务流程页。
function BusinessSupportDialog({ process, spaceId, onOpenChange }: {
  process: ApplicationProcess | null
  spaceId?: string
  onOpenChange: (v: boolean) => void
}) {
  const { data, loading } = useQuery<{ processReferencesByApplicationProcess?: { applicationProcessId: string; businessProcessId: string }[] }>(
    GET_PROCESS_REFERENCES_BY_APPLICATION,
    { variables: { applicationProcessId: process?.id }, skip: !process?.id },
  )
  const { data: businessProcessesData } = useQuery<{ businessProcessesBySpace?: { id: string; name: string }[] }>(
    GET_BUSINESS_PROCESSES_BY_SPACE,
    { variables: { spaceId }, skip: !spaceId },
  )

  const businessProcessName = useMemo(() => {
    const map = new Map<string, string>()
    for (const bp of businessProcessesData?.businessProcessesBySpace ?? []) map.set(bp.id, bp.name)
    return map
  }, [businessProcessesData])

  const items: CrossDomainItem[] = (data?.processReferencesByApplicationProcess ?? [])
    .map((r) => ({
      id: r.businessProcessId,
      name: businessProcessName.get(r.businessProcessId) ?? r.businessProcessId,
    }))

  return (
    <CrossDomainDialog
      open={!!process}
      onOpenChange={onOpenChange}
      title={`支撑的业务流程 - ${process?.name}`}
      items={items}
      loading={loading}
      to={`/spaces/${spaceId}/architectures/processes`}
    />
  )
}