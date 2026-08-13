import { useQuery, useMutation } from '@apollo/client/react'
import { gql } from '@apollo/client'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table'
import { Badge } from '@/components/ui/badge'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from '@/components/ui/dropdown-menu'
import { Plus, Pencil, Trash2, Loader2, MoreVertical, UserRoundCog, Link2 } from 'lucide-react'
import { useState, useEffect, useCallback, memo } from 'react'
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogFooter } from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { useParams } from 'react-router-dom'
import { useSpaceMembership } from '@/hooks/use-space-membership'
import { useIsMobile } from '@/hooks/use-media-query'
import { TransferOwnershipDialog } from './transfer-ownership-dialog'

const GET_CAPABILITIES = gql`
  query GetCapabilities($spaceId: String!) {
    businessCapabilitiesBySpace(spaceId: $spaceId) {
      id name description level maturity businessValue status ownerId
    }
  }
`

const TRANSFER_CAPABILITY_OWNERSHIP = gql`
  mutation CapabilityTransferOwnership($id: String!, $newOwnerId: String!) {
    capabilityTransferOwnership(id: $id, newOwnerId: $newOwnerId) {
      id ownerId
    }
  }
`

const CREATE_CAPABILITY = gql`
  mutation CreateCapability($spaceId: String!, $name: String!, $description: String!, $level: CapabilityLevelEnum!, $maturity: MaturityLevelEnum!, $businessValue: BusinessValueRatingEnum!) {
    capabilityCreate(spaceId: $spaceId, name: $name, description: $description, level: $level, maturity: $maturity, businessValue: $businessValue) { id name }
  }
`

const UPDATE_CAPABILITY = gql`
  mutation UpdateCapability($id: String!, $name: String, $description: String, $level: CapabilityLevelEnum, $maturity: MaturityLevelEnum, $businessValue: BusinessValueRatingEnum) {
    capabilityUpdate(id: $id, name: $name, description: $description, level: $level, maturity: $maturity, businessValue: $businessValue) { id name }
  }
`

const DELETE_CAPABILITY = gql`
  mutation DeleteCapability($id: String!) {
    capabilityDelete(id: $id)
  }
`

const GET_PROCESS_RELATIONS = gql`
  query GetProcessRelations($capabilityId: String!) {
    capabilityProcessRelations(capabilityId: $capabilityId) {
      capabilityId
      processId
      logicalId
      processName
      businessVersion
      status
      valid
    }
  }
`

const GET_PROCESSES_BY_SPACE = gql`
  query GetProcessesForReanchor($spaceId: String!) {
    businessProcessesBySpace(spaceId: $spaceId) {
      id logicalId businessVersion status name
    }
  }
`

const CAPABILITY_PROCESS_CREATE = gql`
  mutation CapabilityProcessCreate($capabilityId: String!, $processId: String!) {
    capabilityProcessCreate(capabilityId: $capabilityId, processId: $processId) {
      capabilityId processId
    }
  }
`

const CAPABILITY_PROCESS_DELETE = gql`
  mutation CapabilityProcessDelete($capabilityId: String!, $processId: String!) {
    capabilityProcessDelete(capabilityId: $capabilityId, processId: $processId)
  }
`

interface Capability {
  id: string; name: string; description: string
  level: string; maturity: string; businessValue: string; status: string
  ownerId?: string | null
}

interface CapabilitiesQuery {
  businessCapabilitiesBySpace?: Capability[]
}

interface ProcessRelation {
  capabilityId: string
  processId: string
  logicalId: string
  processName: string
  businessVersion: string
  status: string
  valid: boolean
}

const EMPTY_CAPABILITIES: Capability[] = []

const CapabilityList = memo(function CapabilityList({ nodes, isOwned, isMobile, onEdit, onDelete, onTransfer, onProcesses }: {
  nodes: Capability[]
  isOwned: (cap: Capability) => boolean
  isMobile: boolean
  onEdit: (cap: Capability) => void
  onDelete: (cap: Capability) => void
  onTransfer: (cap: Capability) => void
  onProcesses: (cap: Capability) => void
}) {
  if (nodes.length === 0) {
    return <div className="text-center py-8 text-muted-foreground">暂无数据</div>
  }

  if (isMobile) {
    return (
      <div className="space-y-3">
        {nodes.map((cap) => {
          const owned = isOwned(cap)
          return (
          <div key={cap.id} className="rounded-lg border p-4 space-y-2">
            <div className="flex items-start justify-between gap-2">
              <p className="font-medium break-words">{cap.name}</p>
              <Badge variant="outline">{cap.status}</Badge>
            </div>
            <div className="flex flex-wrap gap-1">
              <Badge variant="secondary">{cap.level}</Badge>
              <Badge variant="secondary">{cap.maturity}</Badge>
              <Badge variant="secondary">{cap.businessValue}</Badge>
            </div>
            {owned && (
              <div className="flex justify-end pt-1">
                <DropdownMenu>
                  <DropdownMenuTrigger asChild>
                    <Button variant="ghost" size="sm" className="h-9 w-9 p-0" aria-label="更多操作">
                      <MoreVertical className="h-4 w-4" />
                    </Button>
                  </DropdownMenuTrigger>
                  <DropdownMenuContent align="end">
                    <DropdownMenuItem onClick={() => onEdit(cap)}>
                      <Pencil className="h-4 w-4 mr-2" />编辑
                    </DropdownMenuItem>
                    <DropdownMenuItem onClick={() => onTransfer(cap)}>
                      <UserRoundCog className="h-4 w-4 mr-2" />转移所有权
                    </DropdownMenuItem>
                    <DropdownMenuItem onClick={() => onProcesses(cap)}>
                      <Link2 className="h-4 w-4 mr-2" />关联流程
                    </DropdownMenuItem>
                    <DropdownMenuItem className="text-destructive" onClick={() => onDelete(cap)}>
                      <Trash2 className="h-4 w-4 mr-2" />删除
                    </DropdownMenuItem>
                  </DropdownMenuContent>
                </DropdownMenu>
              </div>
            )}
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
          <TableHead>层级</TableHead>
          <TableHead>成熟度</TableHead>
          <TableHead>业务价值</TableHead>
          <TableHead>状态</TableHead>
          <TableHead>操作</TableHead>
        </TableRow>
      </TableHeader>
      <TableBody>
        {nodes.map((cap) => {
          const owned = isOwned(cap)
          return (
          <TableRow key={cap.id}>
            <TableCell className="font-medium break-words">{cap.name}</TableCell>
            <TableCell>{cap.level}</TableCell>
            <TableCell>{cap.maturity}</TableCell>
            <TableCell>{cap.businessValue}</TableCell>
            <TableCell><Badge variant="outline">{cap.status}</Badge></TableCell>
            <TableCell>
              {owned && (
                <div className="flex gap-1">
                  <Button variant="ghost" size="sm" aria-label="关联流程" title="关联流程" onClick={() => onProcesses(cap)}>
                    <Link2 className="h-3.5 w-3.5" />
                  </Button>
                  <Button variant="ghost" size="sm" aria-label="编辑" onClick={() => onEdit(cap)}>
                    <Pencil className="h-3.5 w-3.5" />
                  </Button>
                  <Button variant="ghost" size="sm" aria-label="转移所有权" onClick={() => onTransfer(cap)}>
                    <UserRoundCog className="h-3.5 w-3.5" />
                  </Button>
                  <Button variant="ghost" size="sm" aria-label="删除" onClick={() => onDelete(cap)}>
                    <Trash2 className="h-3.5 w-3.5 text-destructive" />
                  </Button>
                </div>
              )}
            </TableCell>
          </TableRow>
          )
        })}
      </TableBody>
    </Table>
  )
})

export default function Capabilities() {
  const { spaceId } = useParams<{ spaceId: string }>()
  const { canEdit, isEntityOwner } = useSpaceMembership(spaceId)
  const isMobile = useIsMobile()
  const [dialogOpen, setDialogOpen] = useState(false)
  const [editing, setEditing] = useState<Capability | null>(null)
  const [deleting, setDeleting] = useState<Capability | null>(null)
  const [transferItem, setTransferItem] = useState<Capability | null>(null)
  const [processesCapability, setProcessesCapability] = useState<Capability | null>(null)
  const { data, loading, error } = useQuery<CapabilitiesQuery>(GET_CAPABILITIES, { variables: { spaceId }, skip: !spaceId })

  const handleEdit = useCallback((cap: Capability) => { setEditing(cap); setDialogOpen(true) }, [])
  const handleDelete = useCallback((cap: Capability) => setDeleting(cap), [])
  const handleTransfer = useCallback((cap: Capability) => setTransferItem(cap), [])
  const handleProcesses = useCallback((cap: Capability) => setProcessesCapability(cap), [])

  return (
    <div className="p-4 md:p-6 space-y-4">
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-semibold">业务能力</h1>
        {canEdit && (
          <Button onClick={() => { setEditing(null); setDialogOpen(true) }}>
            <Plus className="h-4 w-4 mr-2" />新建能力
          </Button>
        )}
      </div>
      <Card>
        <CardHeader><CardTitle>能力列表</CardTitle></CardHeader>
        <CardContent>
          {loading && <div className="text-center py-8 text-muted-foreground">加载中...</div>}
          {Boolean(error) && <div className="text-center py-8 text-destructive">加载失败</div>}
          {data && (
            <CapabilityList
              nodes={data.businessCapabilitiesBySpace ?? EMPTY_CAPABILITIES}
              isOwned={(cap) => isEntityOwner(cap.ownerId)}
              isMobile={isMobile}
              onEdit={handleEdit}
              onDelete={handleDelete}
              onTransfer={handleTransfer}
              onProcesses={handleProcesses}
            />
          )}
        </CardContent>
      </Card>
      <CapabilityCrudDialog open={dialogOpen} onOpenChange={setDialogOpen} editing={editing} spaceId={spaceId} />
      <CapabilityDeleteDialog item={deleting} onConfirm={() => setDeleting(null)} spaceId={spaceId} />
      <ProcessRelationsDialog
        capability={processesCapability}
        spaceId={spaceId}
        onOpenChange={(v) => { if (!v) setProcessesCapability(null) }}
      />
      <TransferOwnershipDialog
        open={!!transferItem}
        onOpenChange={(v) => { if (!v) setTransferItem(null) }}
        entityId={transferItem?.id ?? null}
        spaceId={spaceId}
        entityLabel="能力"
        mutation={TRANSFER_CAPABILITY_OWNERSHIP}
        refetchQueries={[{ query: GET_CAPABILITIES, variables: { spaceId } }]}
      />
    </div>
  )
}

function CapabilityCrudDialog({ open, onOpenChange, editing, spaceId }: {
  open: boolean; onOpenChange: (v: boolean) => void; editing: Capability | null; spaceId?: string
}) {
  const [name, setName] = useState('')
  const [description, setDescription] = useState('')
  const [level, setLevel] = useState('l1')
  const [maturity, setMaturity] = useState('level3')
  const [businessValue, setBusinessValue] = useState('medium')
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [createMut] = useMutation(CREATE_CAPABILITY)
  const [updateMut] = useMutation(UPDATE_CAPABILITY)

  useEffect(() => {
    if (open) {
      setError(null)
      if (editing) {
        setName(editing.name); setDescription(editing.description)
        setLevel(editing.level); setMaturity(editing.maturity)
        setBusinessValue(editing.businessValue)
      } else {
        setName(''); setDescription(''); setLevel('l1'); setMaturity('level3'); setBusinessValue('medium')
      }
    }
  }, [open, editing])

  async function handleSubmit() {
    setLoading(true); setError(null)
    try {
      if (editing) {
        await updateMut({
          variables: { id: editing.id, name, description, level, maturity, businessValue },
          refetchQueries: [{ query: GET_CAPABILITIES, variables: { spaceId } }],
        })
      } else {
        await createMut({
          variables: { spaceId, name, description, level, maturity, businessValue },
          refetchQueries: [{ query: GET_CAPABILITIES, variables: { spaceId } }],
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
        <DialogHeader><DialogTitle>{editing ? '编辑能力' : '新建能力'}</DialogTitle></DialogHeader>
        <div className="space-y-4 py-4">
          {error && <div className="rounded-md bg-destructive/10 p-3 text-sm text-destructive">{error}</div>}
          <div className="space-y-2"><Label htmlFor="capability-name">名称</Label><Input id="capability-name" value={name} onChange={e => setName(e.target.value)} /></div>
          <div className="space-y-2"><Label htmlFor="capability-description">描述</Label><Input id="capability-description" value={description} onChange={e => setDescription(e.target.value)} /></div>
          <div className="grid grid-cols-1 sm:grid-cols-3 gap-4">
            <div className="space-y-2">
              <Label>层级</Label>
              <select className="w-full rounded-md border bg-background px-3 py-2 text-sm" value={level} onChange={e => setLevel(e.target.value)}>
                <option value="l1">L1</option><option value="l2">L2</option><option value="l3">L3</option>
              </select>
            </div>
            <div className="space-y-2">
              <Label>成熟度</Label>
              <select className="w-full rounded-md border bg-background px-3 py-2 text-sm" value={maturity} onChange={e => setMaturity(e.target.value)}>
                <option value="level1">Level 1</option><option value="level2">Level 2</option><option value="level3">Level 3</option><option value="level4">Level 4</option><option value="level5">Level 5</option>
              </select>
            </div>
            <div className="space-y-2">
              <Label>业务价值</Label>
              <select className="w-full rounded-md border bg-background px-3 py-2 text-sm" value={businessValue} onChange={e => setBusinessValue(e.target.value)}>
                <option value="high">High</option><option value="medium">Medium</option><option value="low">Low</option>
              </select>
            </div>
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

function CapabilityDeleteDialog({ item, onConfirm, spaceId }: { item: Capability | null; onConfirm: () => void; spaceId?: string }) {
  const [deleteMut] = useMutation(DELETE_CAPABILITY)
  const [loading, setLoading] = useState(false)
  async function handleDelete() {
    if (!item) return; setLoading(true)
    try { await deleteMut({ variables: { id: item.id }, refetchQueries: [{ query: GET_CAPABILITIES, variables: { spaceId } }] }); onConfirm() }
    catch (err) { console.error(err) } finally { setLoading(false) }
  }
  return (
    <Dialog open={!!item} onOpenChange={onConfirm}>
      <DialogContent>
        <DialogHeader><DialogTitle>确认删除</DialogTitle></DialogHeader>
        <p className="py-4 text-sm text-muted-foreground">确定要删除能力「{item?.name}」吗？</p>
        <DialogFooter>
          <Button variant="outline" onClick={onConfirm}>取消</Button>
          <Button variant="destructive" onClick={handleDelete} disabled={loading}>{loading ? <Loader2 className="h-4 w-4 animate-spin" /> : '删除'}</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}

// 能力↔流程关系（版本锚定可见）：展示流程名 + 版本 + 状态，`valid=false`
// 的关系标红并提供「重新锚定到最新版」（组合 capabilityProcessDelete +
// capabilityProcessCreate 指向该 logicalId 的最新 active 行）。
function ProcessRelationsDialog({ capability, spaceId, onOpenChange }: {
  capability: Capability | null
  spaceId?: string
  onOpenChange: (v: boolean) => void
}) {
  const [reanchoring, setReanchoring] = useState<string | null>(null)
  const [error, setError] = useState<string | null>(null)
  const { data, loading, refetch } = useQuery<{ capabilityProcessRelations?: ProcessRelation[] }>(
    GET_PROCESS_RELATIONS,
    { variables: { capabilityId: capability?.id }, skip: !capability?.id },
  )
  const { data: processesData } = useQuery<{ businessProcessesBySpace?: { id: string; logicalId: string; businessVersion: string; status: string }[] }>(
    GET_PROCESSES_BY_SPACE,
    { variables: { spaceId }, skip: !spaceId },
  )
  const [deleteMut] = useMutation(CAPABILITY_PROCESS_DELETE)
  const [createMut] = useMutation(CAPABILITY_PROCESS_CREATE)

  useEffect(() => {
    if (capability) setError(null)
  }, [capability])

  async function handleReanchor(rel: ProcessRelation) {
    if (!capability) return
    setReanchoring(rel.processId)
    setError(null)
    const latest = (processesData?.businessProcessesBySpace ?? [])
      .find(p => p.logicalId === rel.logicalId && p.status === 'active')
    if (!latest) {
      setError(`未找到流程「${rel.processName}」的最新 active 版本`)
      setReanchoring(null)
      return
    }
    try {
      await deleteMut({ variables: { capabilityId: capability.id, processId: rel.processId } })
      await createMut({ variables: { capabilityId: capability.id, processId: latest.id } })
      refetch()
    } catch (err) {
      setError(err instanceof Error ? err.message : '重新锚定失败')
    } finally {
      setReanchoring(null)
    }
  }

  const relations = data?.capabilityProcessRelations ?? []

  return (
    <Dialog open={!!capability} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader><DialogTitle>关联流程 - {capability?.name}</DialogTitle></DialogHeader>
        <div className="space-y-4 py-4 text-sm">
          {error && <div className="rounded-md bg-destructive/10 p-3 text-sm text-destructive">{error}</div>}
          {loading ? (
            <div className="text-center py-6 text-muted-foreground">加载中...</div>
          ) : relations.length === 0 ? (
            <div className="text-center py-6 text-muted-foreground">暂无关联流程</div>
          ) : (
            <ul className="space-y-2">
              {relations.map(rel => (
                <li key={rel.processId} className={`rounded-md border px-3 py-2 ${rel.valid ? '' : 'border-destructive/50 bg-destructive/5'}`}>
                  <div className="flex flex-wrap items-center gap-2">
                    <span className="font-medium">{rel.processName}</span>
                    <Badge variant="secondary" className="font-mono">{rel.businessVersion}</Badge>
                    <Badge variant={rel.status === 'archived' ? 'destructive' : rel.status === 'deprecated' ? 'secondary' : 'outline'}>{rel.status}</Badge>
                    {!rel.valid && (
                      <>
                        <Badge variant="destructive">已失效</Badge>
                        <Button
                          variant="outline"
                          size="sm"
                          className="ml-auto"
                          disabled={reanchoring === rel.processId}
                          onClick={() => handleReanchor(rel)}
                        >
                          {reanchoring === rel.processId ? <Loader2 className="h-3.5 w-3.5 animate-spin" /> : '重新锚定到最新版'}
                        </Button>
                      </>
                    )}
                  </div>
                  {!rel.valid && (
                    <p className="mt-1 text-xs text-destructive">
                      该流程已发布新版本，此关系仍指向旧版本行，建议重新锚定。
                    </p>
                  )}
                </li>
              ))}
            </ul>
          )}
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>关闭</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
