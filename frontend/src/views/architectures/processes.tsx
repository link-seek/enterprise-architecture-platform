import { useQuery, useMutation } from '@apollo/client/react'
import { gql } from '@apollo/client'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table'
import { Badge } from '@/components/ui/badge'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from '@/components/ui/dropdown-menu'
import { Plus, Pencil, Trash2, Loader2, MoreVertical, UserRoundCog, GitBranch, AppWindow } from 'lucide-react'
import { useState, useEffect, useCallback, useMemo } from 'react'
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogFooter } from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { useParams } from 'react-router-dom'
import { useSpaceMembership } from '@/hooks/use-space-membership'
import { useIsMobile } from '@/hooks/use-media-query'
import { TransferOwnershipDialog } from './transfer-ownership-dialog'
import { CrossDomainDialog, type CrossDomainItem } from './cross-domain-dialog'
import { friendlyDeleteError } from './crud'

const GET_PROCESSES = gql`
  query GetProcesses($spaceId: String!) {
    businessProcessesBySpace(spaceId: $spaceId) {
      id name description inputs outputs businessVersion logicalId sla cycleTime costPerTransaction status ownerId
    }
  }
`

const TRANSFER_PROCESS_OWNERSHIP = gql`
  mutation ProcessTransferOwnership($id: String!, $newOwnerId: String!) {
    processTransferOwnership(id: $id, newOwnerId: $newOwnerId) {
      id ownerId
    }
  }
`

const CREATE_PROCESS = gql`
  mutation CreateProcess($spaceId: String!, $name: String!, $description: String, $inputs: [String!], $outputs: [String!], $sla: String, $cycleTime: Int, $costPerTransaction: Float) {
    processCreate(spaceId: $spaceId, name: $name, description: $description, inputs: $inputs, outputs: $outputs, sla: $sla, cycleTime: $cycleTime, costPerTransaction: $costPerTransaction) { id name }
  }
`

const UPDATE_PROCESS = gql`
  mutation UpdateProcess($id: String!, $name: String, $description: String, $inputs: [String!], $outputs: [String!], $sla: String, $cycleTime: Int, $costPerTransaction: Float) {
    processUpdate(id: $id, name: $name, description: $description, inputs: $inputs, outputs: $outputs, sla: $sla, cycleTime: $cycleTime, costPerTransaction: $costPerTransaction) { id name }
  }
`

const DELETE_PROCESS = gql`
  mutation DeleteProcess($id: String!) {
    processDelete(id: $id)
  }
`

const PUBLISH_PROCESS_VERSION = gql`
  mutation ProcessPublishVersion($logicalId: String!) {
    processPublishVersion(logicalId: $logicalId) {
      id
      businessVersion
      status
      affectedLinks {
        capabilityId
        capabilityName
        oldVersion
        newVersion
      }
    }
  }
`

const GET_CAPABILITIES_BY_PROCESS = gql`
  query GetCapabilitiesByProcess($processId: String!) {
    capabilitiesByProcess(processId: $processId) {
      id name status
    }
  }
`

// 跨域关联（R3）：业务流程被哪些应用流程引用（process_references）。
const GET_PROCESS_REFERENCES_BY_BUSINESS = gql`
  query GetProcessReferencesByBusiness($businessProcessId: String!) {
    processReferencesByBusinessProcess(businessProcessId: $businessProcessId) {
      applicationProcessId businessProcessId
    }
  }
`

const GET_APPLICATION_PROCESSES_BY_SPACE = gql`
  query GetApplicationProcessesForNames($spaceId: String!) {
    applicationProcessesBySpace(spaceId: $spaceId) { id name }
  }
`

interface Process {
  id: string; name: string; description: string
  inputs?: string[] | null; outputs?: string[] | null
  businessVersion?: string; logicalId?: string
  sla: string | null; cycleTime: number | null; costPerTransaction: number | null; status: string
  ownerId?: string | null
}

interface ProcessesQuery {
  businessProcessesBySpace?: Process[]
}

interface AffectedLink {
  capabilityId: string
  capabilityName: string
  oldVersion: string
  newVersion: string
}

function splitLines(text: string): string[] {
  return text.split('\n').map(s => s.trim()).filter(Boolean)
}

function joinLines(items: string[] | null | undefined): string {
  return (items ?? []).join('\n')
}

function ProcessList({ nodes, isOwned, isMobile, onEdit, onDelete, onTransfer, onPublish, onApplicationSupport }: {
  nodes: Process[]
  isOwned: (p: Process) => boolean
  isMobile: boolean
  onEdit: (p: Process) => void
  onDelete: (p: Process) => void
  onTransfer: (p: Process) => void
  onPublish: (p: Process) => void
  onApplicationSupport: (p: Process) => void
}) {
  if (nodes.length === 0) {
    return <div className="text-center py-8 text-muted-foreground">暂无数据</div>
  }

  if (isMobile) {
    return (
      <div className="space-y-3">
        {nodes.map((p) => {
          const owned = isOwned(p)
          return (
          <div key={p.id} className="rounded-lg border p-4 space-y-2">
            <div className="flex items-start justify-between gap-2">
              <div className="min-w-0">
                <p className="font-medium truncate" title={p.name}>{p.name}</p>
                <p className="text-xs text-muted-foreground truncate" title={p.description}>{p.description}</p>
              </div>
              <div className="flex items-center gap-1 shrink-0">
                {p.businessVersion && <Badge variant="secondary" className="font-mono">{p.businessVersion}</Badge>}
                <Badge variant={p.status === 'deprecated' ? 'secondary' : 'outline'}>{p.status}</Badge>
              </div>
            </div>
            <div className="flex flex-wrap gap-1 text-xs text-muted-foreground">
              <span>SLA: {p.sla ?? '-'}</span>
              <span>周期: {p.cycleTime ?? '-'}</span>
              <span>成本: {p.costPerTransaction ?? '-'}</span>
            </div>
            {(p.inputs?.length || p.outputs?.length) && (
              <div className="flex flex-wrap gap-1">
                {p.inputs?.map(i => <Badge key={`i-${i}`} variant="outline">输入:{i}</Badge>)}
                {p.outputs?.map(o => <Badge key={`o-${o}`} variant="outline">输出:{o}</Badge>)}
              </div>
            )}
            <div className="flex justify-end gap-1 pt-1">
              <Button variant="ghost" size="sm" aria-label="应用支撑" title="应用支撑" onClick={() => onApplicationSupport(p)}>
                <AppWindow className="h-3.5 w-3.5" />
              </Button>
              {owned && (
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
                    <DropdownMenuItem onClick={() => onTransfer(p)}>
                      <UserRoundCog className="h-4 w-4 mr-2" />转移所有权
                    </DropdownMenuItem>
                    <DropdownMenuItem className="text-destructive" onClick={() => onDelete(p)}>
                      <Trash2 className="h-4 w-4 mr-2" />删除
                    </DropdownMenuItem>
                  </DropdownMenuContent>
                </DropdownMenu>
              )}
            </div>
          </div>
          )
        })}
      </div>
    )
  }

  return (
    <Table>
      <TableHeader>
        <TableRow>
          <TableHead>名称</TableHead>
          <TableHead>输入</TableHead>
          <TableHead>输出</TableHead>
          <TableHead>版本</TableHead>
          <TableHead>SLA</TableHead>
          <TableHead>周期(天)</TableHead>
          <TableHead>单次成本</TableHead>
          <TableHead>状态</TableHead>
          <TableHead>操作</TableHead>
        </TableRow>
      </TableHeader>
      <TableBody>
        {nodes.map((p) => {
          const owned = isOwned(p)
          return (
          <TableRow key={p.id}>
            <TableCell className="font-medium">
              {p.name}
              {p.description && (
                // aria-hidden so the cell's accessible name stays the process
                // name (tests match cells by exact name).
                <p aria-hidden="true" className="text-xs text-muted-foreground">{p.description}</p>
              )}
            </TableCell>
            <TableCell className="max-w-[160px]">
              {p.inputs?.length ? (
                <div className="flex flex-wrap gap-1">
                  {p.inputs.map(i => <Badge key={i} variant="outline">{i}</Badge>)}
                </div>
              ) : '-'}
            </TableCell>
            <TableCell className="max-w-[160px]">
              {p.outputs?.length ? (
                <div className="flex flex-wrap gap-1">
                  {p.outputs.map(o => <Badge key={o} variant="outline">{o}</Badge>)}
                </div>
              ) : '-'}
            </TableCell>
            <TableCell className="font-mono text-xs">{p.businessVersion ?? '-'}</TableCell>
            <TableCell>{p.sla ?? '-'}</TableCell>
            <TableCell>{p.cycleTime ?? '-'}</TableCell>
            <TableCell>{p.costPerTransaction ?? '-'}</TableCell>
            <TableCell>
              <Badge variant={p.status === 'archived' ? 'destructive' : p.status === 'deprecated' ? 'secondary' : 'outline'}>{p.status}</Badge>
            </TableCell>
            <TableCell>
              <div className="flex gap-1">
                <Button variant="ghost" size="sm" aria-label="应用支撑" title="应用支撑" onClick={() => onApplicationSupport(p)}>
                  <AppWindow className="h-3.5 w-3.5" />
                </Button>
                {owned && (
                  <>
                    {p.status === 'active' && p.logicalId && (
                      <Button variant="ghost" size="sm" onClick={() => onPublish(p)} aria-label="发布新版本" title="发布新版本">
                        <GitBranch className="h-3.5 w-3.5" />
                      </Button>
                    )}
                    <Button variant="ghost" size="sm" onClick={() => onEdit(p)} aria-label="编辑">
                      <Pencil className="h-3.5 w-3.5" />
                    </Button>
                    <Button variant="ghost" size="sm" onClick={() => onTransfer(p)} aria-label="转移所有权">
                      <UserRoundCog className="h-3.5 w-3.5" />
                    </Button>
                    <Button variant="ghost" size="sm" onClick={() => onDelete(p)} aria-label="删除">
                      <Trash2 className="h-3.5 w-3.5 text-destructive" />
                    </Button>
                  </>
                )}
              </div>
            </TableCell>
          </TableRow>
          )
        })}
      </TableBody>
    </Table>
  )
}

export default function Processes() {
  const { spaceId } = useParams<{ spaceId: string }>()
  const { canEdit, isEntityOwner } = useSpaceMembership(spaceId)
  const isMobile = useIsMobile()
  const [dialogOpen, setDialogOpen] = useState(false)
  const [editing, setEditing] = useState<Process | null>(null)
  const [deleting, setDeleting] = useState<Process | null>(null)
  const [transferItem, setTransferItem] = useState<Process | null>(null)
  const [publishing, setPublishing] = useState<Process | null>(null)
  const [applicationSupport, setApplicationSupport] = useState<Process | null>(null)
  const { data, loading, error } = useQuery<ProcessesQuery>(GET_PROCESSES, { variables: { spaceId }, skip: !spaceId })

  const handleEdit = useCallback((p: Process) => { setEditing(p); setDialogOpen(true) }, [])
  const handleDelete = useCallback((p: Process) => setDeleting(p), [])
  const handleTransfer = useCallback((p: Process) => setTransferItem(p), [])
  const handlePublish = useCallback((p: Process) => setPublishing(p), [])
  const handleApplicationSupport = useCallback((p: Process) => setApplicationSupport(p), [])

  return (
    <div className="p-4 md:p-6 space-y-4">
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
          {Boolean(error) && <div className="text-center py-8 text-destructive">加载失败</div>}
          {data && (
            <ProcessList
              nodes={data.businessProcessesBySpace ?? []}
              isOwned={(p) => isEntityOwner(p.ownerId)}
              isMobile={isMobile}
              onEdit={handleEdit}
              onDelete={handleDelete}
              onTransfer={handleTransfer}
              onPublish={handlePublish}
              onApplicationSupport={handleApplicationSupport}
            />
          )}
        </CardContent>
      </Card>
      <ProcessCrudDialog open={dialogOpen} onOpenChange={setDialogOpen} editing={editing} spaceId={spaceId} />
      <ProcessDeleteDialog item={deleting} onConfirm={() => setDeleting(null)} spaceId={spaceId} />
      <PublishVersionDialog item={publishing} onOpenChange={(v) => { if (!v) setPublishing(null) }} spaceId={spaceId} />
      <ApplicationSupportDialog
        process={applicationSupport}
        spaceId={spaceId}
        onOpenChange={(v) => { if (!v) setApplicationSupport(null) }}
      />
      <TransferOwnershipDialog
        open={!!transferItem}
        onOpenChange={(v) => { if (!v) setTransferItem(null) }}
        entityId={transferItem?.id ?? null}
        spaceId={spaceId}
        entityLabel="流程"
        mutation={TRANSFER_PROCESS_OWNERSHIP}
        refetchQueries={[{ query: GET_PROCESSES, variables: { spaceId } }]}
      />
    </div>
  )
}

function ProcessCrudDialog({ open, onOpenChange, editing, spaceId }: {
  open: boolean; onOpenChange: (v: boolean) => void; editing: Process | null; spaceId?: string
}) {
  const [name, setName] = useState('')
  const [description, setDescription] = useState('')
  const [inputs, setInputs] = useState('')
  const [outputs, setOutputs] = useState('')
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
        setInputs(joinLines(editing.inputs)); setOutputs(joinLines(editing.outputs))
        setSla(editing.sla || ''); setCycleTime(editing.cycleTime?.toString() || '')
        setCost(editing.costPerTransaction?.toString() || '')
      } else {
        setName(''); setDescription(''); setInputs(''); setOutputs(''); setSla(''); setCycleTime(''); setCost('')
      }
    }
  }, [open, editing])

  async function handleSubmit() {
    setLoading(true); setError(null)
    try {
      const ct = cycleTime ? parseInt(cycleTime) : null
      const cp = cost ? parseFloat(cost) : null
      const inputsArr = splitLines(inputs)
      const outputsArr = splitLines(outputs)
      if (editing) {
        await updateMut({
          variables: { id: editing.id, name, description, inputs: inputsArr, outputs: outputsArr, sla, cycleTime: ct, costPerTransaction: cp },
          refetchQueries: [{ query: GET_PROCESSES, variables: { spaceId } }],
        })
      } else {
        await createMut({
          variables: { spaceId, name, description, inputs: inputsArr, outputs: outputsArr, sla, cycleTime: ct, costPerTransaction: cp },
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
          <div className="space-y-2"><Label htmlFor="process-name">名称</Label><Input id="process-name" value={name} onChange={e => setName(e.target.value)} /></div>
          <div className="space-y-2"><Label htmlFor="process-description">描述</Label><Input id="process-description" value={description} onChange={e => setDescription(e.target.value)} /></div>
          <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
            <div className="space-y-2">
              <Label htmlFor="process-inputs">输入</Label>
              <textarea
                id="process-inputs"
                value={inputs}
                onChange={e => setInputs(e.target.value)}
                placeholder={'每行一个，例如：\n需求\nIssue'}
                rows={3}
                className="flex w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50"
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="process-outputs">输出</Label>
              <textarea
                id="process-outputs"
                value={outputs}
                onChange={e => setOutputs(e.target.value)}
                placeholder={'每行一个，例如：\nADR'}
                rows={3}
                className="flex w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50"
              />
            </div>
          </div>
          <div className="grid grid-cols-1 sm:grid-cols-3 gap-4">
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

function PublishVersionDialog({ item, onOpenChange, spaceId }: {
  item: Process | null
  onOpenChange: (v: boolean) => void
  spaceId?: string
}) {
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [done, setDone] = useState<AffectedLink[] | null>(null)
  const [publishMut] = useMutation<{ processPublishVersion?: { affectedLinks?: AffectedLink[] } }>(PUBLISH_PROCESS_VERSION)
  const { data, loading: capsLoading } = useQuery<{ capabilitiesByProcess?: { id: string; name: string; status: string }[] }>(
    GET_CAPABILITIES_BY_PROCESS,
    { variables: { processId: item?.id }, skip: !item?.id },
  )

  useEffect(() => {
    if (item) { setError(null); setDone(null) }
  }, [item])

  async function handlePublish() {
    if (!item?.logicalId) return
    setLoading(true); setError(null)
    try {
      const res = await publishMut({
        variables: { logicalId: item.logicalId },
        refetchQueries: [{ query: GET_PROCESSES, variables: { spaceId } }],
      })
      setDone(res.data?.processPublishVersion?.affectedLinks ?? [])
    } catch (err) {
      setError(err instanceof Error ? err.message : '发布失败')
    } finally { setLoading(false) }
  }

  const linkedCaps = data?.capabilitiesByProcess ?? []

  return (
    <Dialog open={!!item} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader><DialogTitle>发布新版本</DialogTitle></DialogHeader>
        <div className="space-y-4 py-4 text-sm">
          {error && <div className="rounded-md bg-destructive/10 p-3 text-sm text-destructive">{error}</div>}
          {done === null ? (
            <>
              <p className="text-muted-foreground">
                将为流程「{item?.name}」（当前版本 <span className="font-mono">{item?.businessVersion}</span>）发布新版本。
                旧版本将进入 <Badge variant="secondary">deprecated</Badge> 兼容期，新版本为 <Badge>active</Badge>。
              </p>
              {capsLoading ? (
                <div className="text-center py-4 text-muted-foreground">加载受影响能力...</div>
              ) : linkedCaps.length === 0 ? (
                <p className="text-muted-foreground">当前没有能力关联此流程，发布不受影响。</p>
              ) : (
                <>
                  <p className="font-medium">以下能力将受到影响（需重新锚定到新版本）:</p>
                  <ul className="space-y-1">
                    {linkedCaps.map(c => (
                      <li key={c.id} className="flex items-center gap-2">
                        <span>{c.name}</span>
                        <Badge variant="outline">{c.status}</Badge>
                      </li>
                    ))}
                  </ul>
                </>
              )}
            </>
          ) : (
            <>
              <p className="font-medium">发布成功，以下能力已指向旧版本:</p>
              {done.length === 0 ? (
                <p className="text-muted-foreground">无受影响的能力。</p>
              ) : (
                <ul className="space-y-1">
                  {done.map(l => (
                    <li key={l.capabilityId} className="flex items-center gap-2">
                      <span>{l.capabilityName}</span>
                      <span className="font-mono text-xs text-muted-foreground">{l.oldVersion} → {l.newVersion}</span>
                    </li>
                  ))}
                </ul>
              )}
            </>
          )}
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>{done === null ? '取消' : '关闭'}</Button>
          {done === null && (
            <Button onClick={handlePublish} disabled={loading || capsLoading}>
              {loading ? <Loader2 className="h-4 w-4 animate-spin" /> : '确认发布'}
            </Button>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}

function ProcessDeleteDialog({ item, onConfirm, spaceId }: { item: Process | null; onConfirm: () => void; spaceId?: string }) {
  const [deleteMut] = useMutation(DELETE_PROCESS)
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  useEffect(() => { setError(null) }, [item])
  async function handleDelete() {
    if (!item) return; setLoading(true); setError(null)
    try { await deleteMut({ variables: { id: item.id }, refetchQueries: [{ query: GET_PROCESSES, variables: { spaceId } }] }); onConfirm() }
    catch (err) { setError(friendlyDeleteError(err)) } finally { setLoading(false) }
  }
  return (
    <Dialog open={!!item} onOpenChange={onConfirm}>
      <DialogContent>
        <DialogHeader><DialogTitle>确认删除</DialogTitle></DialogHeader>
        <p className="py-4 text-sm text-muted-foreground">确定要删除流程「{item?.name}」吗？</p>
        {error && <div className="rounded-md bg-destructive/10 p-3 text-sm text-destructive">{error}</div>}
        <DialogFooter>
          <Button variant="outline" onClick={onConfirm}>取消</Button>
          <Button variant="destructive" onClick={handleDelete} disabled={loading}>{loading ? <Loader2 className="h-4 w-4 animate-spin" /> : '删除'}</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}

// 跨域关联（R3）：展示引用该业务流程的应用流程（process_references），
// 行可跳转到应用流程页。
function ApplicationSupportDialog({ process, spaceId, onOpenChange }: {
  process: Process | null
  spaceId?: string
  onOpenChange: (v: boolean) => void
}) {
  const { data, loading } = useQuery<{ processReferencesByBusinessProcess?: { applicationProcessId: string; businessProcessId: string }[] }>(
    GET_PROCESS_REFERENCES_BY_BUSINESS,
    { variables: { businessProcessId: process?.id }, skip: !process?.id },
  )
  const { data: appProcessesData } = useQuery<{ applicationProcessesBySpace?: { id: string; name: string }[] }>(
    GET_APPLICATION_PROCESSES_BY_SPACE,
    { variables: { spaceId }, skip: !spaceId },
  )

  const appProcessName = useMemo(() => {
    const map = new Map<string, string>()
    for (const ap of appProcessesData?.applicationProcessesBySpace ?? []) map.set(ap.id, ap.name)
    return map
  }, [appProcessesData])

  const items: CrossDomainItem[] = (data?.processReferencesByBusinessProcess ?? [])
    .map((r) => ({
      id: r.applicationProcessId,
      name: appProcessName.get(r.applicationProcessId) ?? r.applicationProcessId,
    }))

  return (
    <CrossDomainDialog
      open={!!process}
      onOpenChange={onOpenChange}
      title={`被应用流程支撑 - ${process?.name}`}
      items={items}
      loading={loading}
      to={`/spaces/${spaceId}/architectures/application-processes`}
    />
  )
}
