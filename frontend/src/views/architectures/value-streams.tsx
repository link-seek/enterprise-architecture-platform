import { useQuery, useMutation, useApolloClient } from '@apollo/client/react'
import { gql } from '@apollo/client'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table'
import { Badge, type BadgeProps } from '@/components/ui/badge'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from '@/components/ui/dropdown-menu'
import { Link, useParams } from 'react-router-dom'
import { Plus, Pencil, Trash2, History, GitBranch, MoreVertical, UserRoundCog, type LucideIcon } from 'lucide-react'
import { useState, useCallback, useEffect, memo } from 'react'
import { ValueStreamCrudDialog, ValueStreamDeleteDialog } from './crud'
import { VersionHistoryDialog, ArchiveButton, GET_VALUE_STREAMS, GET_VALUE_STREAM_VERSIONS } from './version-control'
import { VersionPublishDialog, suggestNextVersion, type PublishImpactItem } from './version-publish-dialog'
import { GET_STAGE_CAPABILITIES } from './value-stream-stages'
import { TransferOwnershipDialog } from './transfer-ownership-dialog'
import { useSpaceMembership } from '@/hooks/use-space-membership'
import { useIsMobile } from '@/hooks/use-media-query'

const TRANSFER_VALUE_STREAM_OWNERSHIP = gql`
  mutation ValueStreamTransferOwnership($id: String!, $newOwnerId: String!) {
    valueStreamTransferOwnership(id: $id, newOwnerId: $newOwnerId) {
      id ownerId
    }
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
  endDeliverable?: string | null
}

interface ValueStreamsQuery {
  valueStreamsBySpace?: ValueStream[]
}

const EMPTY_VALUE_STREAMS: ValueStream[] = []

const PUBLISH_VALUE_STREAM_VERSION = gql`
  mutation ValueStreamCreateVersion($currentId: String!, $newVersion: String!, $newName: String, $newDescription: String) {
    valueStreamCreateVersion(currentId: $currentId, newVersion: $newVersion, newName: $newName, newDescription: $newDescription) {
      id name description businessVersion status importance logicalId
    }
  }
`

const GET_STAGES_BY_VS = gql`
  query GetStagesByValueStream($valueStreamId: String!) {
    valueStreamStagesByValueStream(valueStreamId: $valueStreamId) {
      id name sequenceOrder
    }
  }
`

// 价值流发布包装：取阶段及各阶段关联能力，喂给统一发布弹窗。
function ValueStreamPublishDialog({ item, spaceId, onOpenChange }: {
  item: ValueStream | null
  spaceId: string
  onOpenChange: (v: boolean) => void
}) {
  const client = useApolloClient()
  const [items, setItems] = useState<PublishImpactItem[]>([])
  const [impactLoading, setImpactLoading] = useState(false)
  const [impactError, setImpactError] = useState<string | null>(null)
  const [publishMut] = useMutation(PUBLISH_VALUE_STREAM_VERSION)
  const itemId = item?.id

  useEffect(() => {
    if (!itemId) return
    let cancelled = false
    setImpactLoading(true)
    setImpactError(null)
    setItems([])
    ;(async () => {
      const { data } = await client.query<{ valueStreamStagesByValueStream?: { id: string; name: string }[] }>({
        query: GET_STAGES_BY_VS,
        variables: { valueStreamId: itemId },
        fetchPolicy: 'network-only',
      })
      const stages = data?.valueStreamStagesByValueStream ?? []
      const rows = await Promise.all(stages.map(async (s) => {
        const r = await client.query<{ capabilitiesByStage?: { id: string; name: string }[] }>({
          query: GET_STAGE_CAPABILITIES,
          variables: { stageId: s.id },
          fetchPolicy: 'network-only',
        })
        const caps = r.data?.capabilitiesByStage ?? []
        return {
          id: s.id,
          name: `阶段 ${s.name}`,
          detail: caps.length > 0 ? caps.map(c => c.name).join('、') : '无关联能力',
        } as PublishImpactItem
      }))
      if (!cancelled) {
        setItems(rows)
        setImpactLoading(false)
      }
    })().catch((err) => {
      if (!cancelled) {
        setImpactError(err instanceof Error ? err.message : '加载失败')
        setImpactLoading(false)
      }
    })
    return () => { cancelled = true }
  }, [itemId, client])

  return (
    <VersionPublishDialog
      open={!!item}
      onOpenChange={onOpenChange}
      title="发布新版本"
      currentVersion={item?.businessVersion}
      suggestedVersion={suggestNextVersion(item?.businessVersion)}
      impactIntro="以下阶段及其能力关联将整体迁移到新版，旧版进入 deprecated 兼容期："
      impactItems={items}
      impactLoading={impactLoading}
      impactError={impactError}
      impactEmptyText="该价值流暂无阶段，直接发布新版。"
      confirmLabel="确认发布"
      successTitle="发布成功，阶段与关联已迁移到新版:"
      onConfirm={async (newVersion) => {
        if (!item) return []
        const res = await publishMut({
          variables: {
            currentId: item.id,
            newVersion,
            newName: item.name,
            newDescription: item.description,
          },
          refetchQueries: [{ query: GET_VALUE_STREAMS, variables: { spaceId } }],
        })
        void res
        return items
      }}
    />
  )
}

type BadgeVariant = NonNullable<BadgeProps['variant']>

function statusColor(status: string): BadgeVariant {
  switch (status) {
    case 'active': return 'default'
    case 'archived': return 'destructive'
    default: return 'outline'
  }
}

interface ValueStreamAction {
  icon: LucideIcon
  label: string
  onClick: () => void
  destructive?: boolean
}

function ActionButton({ action }: { action: ValueStreamAction }) {
  const Icon = action.icon
  return (
    <Button variant="ghost" size="sm" aria-label={action.label} title={action.label} onClick={action.onClick}>
      <Icon className={`h-3.5 w-3.5${action.destructive ? ' text-destructive' : ''}`} />
    </Button>
  )
}

function buildActions(vs: ValueStream, isOwned: boolean, onEdit: (vs: ValueStream) => void, onVersion: (vs: ValueStream) => void, onHistory: (vs: ValueStream) => void, onDelete: (vs: ValueStream) => void, onTransfer: (vs: ValueStream) => void): ValueStreamAction[] {
  const actions: ValueStreamAction[] = [
    { icon: History, label: '历史', onClick: () => onHistory(vs) },
  ]
  if (isOwned) {
    actions.unshift(
      { icon: Pencil, label: '编辑', onClick: () => onEdit(vs) },
      // 统一发布策略：仅 active 行可发布新版。
      ...(vs.status === 'active'
        ? [{ icon: GitBranch, label: '新版本', onClick: () => onVersion(vs) } as ValueStreamAction]
        : []),
      { icon: UserRoundCog, label: '转移所有权', onClick: () => onTransfer(vs) },
      { icon: Trash2, label: '删除', onClick: () => onDelete(vs), destructive: true },
    )
  }
  return actions
}

const ValueStreamList = memo(function ValueStreamList({ nodes, isOwned, isMobile, detailBase, spaceId, onEdit, onDelete, onVersion, onHistory, onTransfer }: {
  nodes: ValueStream[]
  isOwned: (vs: ValueStream) => boolean
  isMobile: boolean
  detailBase: string
  spaceId: string
  onEdit: (vs: ValueStream) => void
  onDelete: (vs: ValueStream) => void
  onVersion: (vs: ValueStream) => void
  onHistory: (vs: ValueStream) => void
  onTransfer: (vs: ValueStream) => void
}) {
  if (nodes.length === 0) {
    return <div className="text-center py-8 text-muted-foreground">暂无数据</div>
  }
  if (isMobile) {
    return (
      <div className="space-y-3">
        {nodes.map((vs) => {
          const owned = isOwned(vs)
          const actions = buildActions(vs, owned, onEdit, onVersion, onHistory, onDelete, onTransfer)
          return (
          <div key={vs.id} className="rounded-lg border p-4 space-y-2">
            <div className="flex items-start justify-between gap-2">
              <div className="min-w-0">
                <p className="font-medium truncate" title={vs.name}>{vs.name}</p>
                {vs.endDeliverable && (
                  <p className="text-xs text-muted-foreground truncate" title={vs.endDeliverable}>交付物: {vs.endDeliverable}</p>
                )}
                {vs.description && (
                  <p className="text-xs text-muted-foreground truncate" title={vs.description}>{vs.description}</p>
                )}
              </div>
              <div className="flex gap-1 shrink-0">
                <Badge variant="secondary" className="font-mono">{vs.businessVersion}</Badge>
                <Badge variant={statusColor(vs.status)}>{vs.status}</Badge>
              </div>
            </div>
            <div className="flex items-center justify-between pt-1">
              <Link to={`${detailBase}/${vs.id}`}>
                {/* 移动端使用 outline 增强触摸可见性，桌面端使用 ghost 保持简洁 */}
                <Button variant="outline" size="sm">查看</Button>
              </Link>
              {owned && (
                <div className="flex items-center gap-1">
                  {vs.status !== 'archived' && <ArchiveButton id={vs.id} spaceId={spaceId} />}
                  <DropdownMenu>
                    <DropdownMenuTrigger asChild>
                      <Button variant="ghost" size="sm" className="h-11 w-11 p-0" aria-label="更多操作" title="更多操作">
                        <MoreVertical className="h-4 w-4" />
                      </Button>
                    </DropdownMenuTrigger>
                    <DropdownMenuContent align="end">
                      {actions.map((action) => {
                        const Icon = action.icon
                        return (
                          <DropdownMenuItem key={action.label} className={action.destructive ? 'text-destructive' : undefined} onClick={action.onClick}>
                            <Icon className="h-4 w-4 mr-2" />{action.label}
                          </DropdownMenuItem>
                        )
                      })}
                    </DropdownMenuContent>
                  </DropdownMenu>
                </div>
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
          <TableHead>最终交付物</TableHead>
          <TableHead>版本</TableHead>
          <TableHead>状态</TableHead>
          <TableHead>操作</TableHead>
        </TableRow>
      </TableHeader>
      <TableBody>
        {nodes.map((vs) => {
          const owned = isOwned(vs)
          const actions = buildActions(vs, owned, onEdit, onVersion, onHistory, onDelete, onTransfer)
          return (
          <TableRow key={vs.id}>
            <TableCell className="font-medium">
              {vs.name}
              <span className="ml-2 text-xs text-muted-foreground">{vs.description}</span>
            </TableCell>
            <TableCell>{vs.endDeliverable || '-'}</TableCell>
            <TableCell>
              <Badge variant="secondary" className="font-mono">{vs.businessVersion}</Badge>
            </TableCell>
            <TableCell>
              <Badge variant={statusColor(vs.status)}>{vs.status}</Badge>
            </TableCell>
            <TableCell>
              <div className="flex gap-1">
                <Link to={`${detailBase}/${vs.id}`}>
                  <Button variant="ghost" size="sm">查看</Button>
                </Link>
                {owned && (
                  <>
                    {actions.filter((a) => !a.destructive).map((action) => (
                      <ActionButton key={action.label} action={action} />
                    ))}
                    {vs.status !== 'archived' && <ArchiveButton id={vs.id} spaceId={spaceId} />}
                    {actions.filter((a) => a.destructive).map((action) => (
                      <ActionButton key={action.label} action={action} />
                    ))}
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
})

export default function ValueStreams() {
  const { spaceId } = useParams<{ spaceId: string }>()
  const { canEdit, isEntityOwner } = useSpaceMembership(spaceId)
  const isMobile = useIsMobile()
  const [dialogOpen, setDialogOpen] = useState(false)
  const [editing, setEditing] = useState<ValueStream | null>(null)
  const [deleting, setDeleting] = useState<ValueStream | null>(null)
  const [historyLogicalId, setHistoryLogicalId] = useState<string | null>(null)
  const [versionItem, setVersionItem] = useState<ValueStream | null>(null)
  const [transferItem, setTransferItem] = useState<ValueStream | null>(null)

  const { data, loading, error } = useQuery<ValueStreamsQuery>(GET_VALUE_STREAMS, {
    variables: { spaceId },
    skip: !spaceId,
  })

  const handleEdit = useCallback((vs: ValueStream) => { setEditing(vs); setDialogOpen(true) }, [])
  const handleDelete = useCallback((vs: ValueStream) => setDeleting(vs), [])
  const handleVersion = useCallback((vs: ValueStream) => { setVersionItem(vs) }, [])
  const handleHistory = useCallback((vs: ValueStream) => { setHistoryLogicalId(vs.logicalId) }, [])
  const handleTransfer = useCallback((vs: ValueStream) => setTransferItem(vs), [])

  if (!spaceId) {
    return (
      <div className="p-6">
        <div className="text-center py-8 text-destructive">缺少空间标识，无法加载价值流。</div>
      </div>
    )
  }

  const detailBase = `/spaces/${spaceId}/architectures/value-streams`

  return (
    <div className="p-4 md:p-6 space-y-4">
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-semibold">价值流</h1>
        {canEdit && (
          <Button onClick={() => { setEditing(null); setDialogOpen(true) }}>
            <Plus className="h-4 w-4 mr-2" />
            新建价值流
          </Button>
        )}
      </div>

      <Card>
        <CardHeader><CardTitle>价值流列表</CardTitle></CardHeader>
        <CardContent>
          {loading && <div className="text-center py-8 text-muted-foreground">加载中...</div>}
          {Boolean(error) && <div className="text-center py-8 text-destructive">加载失败</div>}
          {data && (
            <>
              <ValueStreamList
                nodes={data.valueStreamsBySpace ?? EMPTY_VALUE_STREAMS}
                isOwned={(vs) => isEntityOwner(vs.ownerId)}
                isMobile={isMobile}
                detailBase={detailBase}
                spaceId={spaceId}
                onEdit={handleEdit}
                onDelete={handleDelete}
                onVersion={handleVersion}
                onHistory={handleHistory}
                onTransfer={handleTransfer}
              />
              <div className="flex items-center justify-between mt-4">
                <p className="text-sm text-muted-foreground">共 {data.valueStreamsBySpace?.length ?? 0} 条</p>
              </div>
            </>
          )}
        </CardContent>
      </Card>

      <ValueStreamCrudDialog open={dialogOpen} onOpenChange={setDialogOpen} editing={editing} spaceId={spaceId} />
      <ValueStreamDeleteDialog item={deleting} onConfirm={() => setDeleting(null)} spaceId={spaceId} />
      <VersionHistoryDialog open={historyLogicalId !== null} onOpenChange={(v) => { if (!v) setHistoryLogicalId(null) }} spaceId={spaceId} logicalId={historyLogicalId} query={GET_VALUE_STREAM_VERSIONS} dataKey="valueStreamsBySpaceAndLogicalId" />
      <ValueStreamPublishDialog item={versionItem} spaceId={spaceId} onOpenChange={(v) => { if (!v) setVersionItem(null) }} />
      <TransferOwnershipDialog
        open={!!transferItem}
        onOpenChange={(v) => { if (!v) setTransferItem(null) }}
        entityId={transferItem?.id ?? null}
        spaceId={spaceId}
        entityLabel="价值流"
        mutation={TRANSFER_VALUE_STREAM_OWNERSHIP}
        refetchQueries={[{ query: GET_VALUE_STREAMS, variables: { spaceId } }]}
      />
    </div>
  )
}
