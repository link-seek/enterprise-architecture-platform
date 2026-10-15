import { useQuery, useMutation } from '@apollo/client/react'
import { gql } from '@apollo/client'
import { useParams, Link } from 'react-router-dom'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { Badge } from '@/components/ui/badge'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table'
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogFooter } from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { ArrowLeft, Plus, Pencil, Trash2, Link2, Rocket } from 'lucide-react'
import { useState, useMemo } from 'react'
import { useSpaceMembership } from '@/hooks/use-space-membership'
import { StageCrudDialog, StageDeleteDialog, StageCapabilitiesCell, StageCapabilityDialog, type ValueStreamStage } from './value-stream-stages'

export const GET_VALUE_STREAM_DETAIL = gql`
  query GetValueStreamDetail($spaceId: String!, $id: String!) {
    valueStreamById(spaceId: $spaceId, id: $id) {
      id
      logicalId
      name
      description
      businessVersion
      status
      importance
      ownerId
      triggeringEvent
      endDeliverable
      stakeholders
      performanceMetrics
      createdAt
      updatedAt
    }
    valueStreamStagesByValueStream(valueStreamId: $id) {
      id
      name
      sequenceOrder
      input
      output
      description
      objectiveMetrics
      entryCriteria
      exitCriteria
      ownerId
      keyMetrics
    }
  }
`

export const GET_VALUE_STREAM_RUNS = gql`
  query GetValueStreamRuns($valueStreamId: String!) {
    valueStreamRunsByValueStream(valueStreamId: $valueStreamId) {
      id
      repoUrl
      chainPath
      status
      pinnedVersions
      createdAt
      updatedAt
    }
  }
`

interface ValueStreamRun {
  id: string
  repoUrl: string
  chainPath: string
  status: string
  pinnedVersions: Record<string, string> | null
  createdAt: string
  updatedAt: string
}

interface ValueStreamDetailQuery {
  valueStreamById: {
    id: string
    logicalId: string
    name: string
    description: string
    businessVersion: string
    status: string
    importance: string
    ownerId: string | null
    triggeringEvent: string | null
    endDeliverable: string | null
    stakeholders: string[] | null
    performanceMetrics: Record<string, string> | null
    createdAt: string
    updatedAt: string
  } | null
  valueStreamStagesByValueStream: ValueStreamStage[]
}

interface ValueStreamRunsQuery {
  valueStreamRunsByValueStream: ValueStreamRun[]
}

const PROVISION_PILOT = gql`
  mutation ValueStreamProvisionPilot($logicalId: String!, $templateVersion: String!) {
    valueStreamProvisionPilot(logicalId: $logicalId, templateVersion: $templateVersion) {
      runId
      status
      repoUrl
      prUrl
      reused
      templateVersion
    }
  }
`

interface ValueStreamProvisionPilotResult {
  runId: string
  status: string
  repoUrl: string
  prUrl: string
  reused: boolean
  templateVersion: string
}

export default function ValueStreamDetail() {
  const { id, spaceId } = useParams<{ id: string; spaceId: string }>()
  const { isEntityOwner } = useSpaceMembership(spaceId)
  const { data, loading, error } = useQuery<ValueStreamDetailQuery>(GET_VALUE_STREAM_DETAIL, {
    variables: { spaceId, id },
    skip: !spaceId || !id,
  })
  // Runs load independently: if the backend predates the runs migration,
  // this query fails while detail/stages keep rendering (rollout-skew safe).
  const { data: runsData } = useQuery<ValueStreamRunsQuery>(GET_VALUE_STREAM_RUNS, {
    variables: { valueStreamId: id },
    skip: !id,
    errorPolicy: 'ignore',
  })
  const runs = runsData?.valueStreamRunsByValueStream ?? []
  const liveCount = useMemo(() => runs.filter((r) => r.status === 'live').length, [runs])

  const [stageDialogOpen, setStageDialogOpen] = useState(false)
  const [editingStage, setEditingStage] = useState<ValueStreamStage | null>(null)
  const [deletingStage, setDeletingStage] = useState<ValueStreamStage | null>(null)
  const [capabilityStage, setCapabilityStage] = useState<ValueStreamStage | null>(null)
  const [provisionOpen, setProvisionOpen] = useState(false)

  const vs = data?.valueStreamById
  const stages = useMemo(
    () =>
      [...(data?.valueStreamStagesByValueStream ?? [])].sort(
        (a, b) => a.sequenceOrder - b.sequenceOrder,
      ),
    [data?.valueStreamStagesByValueStream],
  )
  const backPath = spaceId
    ? `/spaces/${spaceId}/architectures/value-streams`
    : '/architectures/value-streams'

  return (
    <div className="p-4 md:p-6 space-y-4">
      <Link to={backPath}>
        <Button variant="ghost" size="sm" className="gap-2">
          <ArrowLeft className="h-4 w-4" />
          返回列表
        </Button>
      </Link>

      {loading && <div className="text-center py-8 text-muted-foreground">加载中...</div>}
      {error && <div className="text-center py-8 text-destructive">加载失败: {error.message}</div>}
      {vs && (
        <>
          <Card>
            <CardHeader>
              <div className="flex items-center justify-between">
                <CardTitle className="text-2xl">{vs.name}</CardTitle>
                <Badge>{vs.status}</Badge>
              </div>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
                <div>
                  <p className="text-sm text-muted-foreground">描述</p>
                  <p>{vs.description || '-'}</p>
                </div>
                <div>
                  <p className="text-sm text-muted-foreground">版本</p>
                  <p>{vs.businessVersion}</p>
                </div>
                <div>
                  <p className="text-sm text-muted-foreground">重要性</p>
                  <p>{vs.importance}</p>
                </div>
                <div>
                  <p className="text-sm text-muted-foreground">触发事件</p>
                  <p>{vs.triggeringEvent || '-'}</p>
                </div>
                <div>
                  <p className="text-sm text-muted-foreground">最终交付物</p>
                  <p>{vs.endDeliverable || '-'}</p>
                </div>
                <div>
                  <p className="text-sm text-muted-foreground">利益相关方</p>
                  <p>{vs.stakeholders && vs.stakeholders.length > 0 ? vs.stakeholders.join('、') : '-'}</p>
                </div>
                <div>
                  <p className="text-sm text-muted-foreground">创建时间</p>
                  <p>{new Date(vs.createdAt).toLocaleString('zh-CN')}</p>
                </div>
                <div>
                  <p className="text-sm text-muted-foreground">更新时间</p>
                  <p>{new Date(vs.updatedAt).toLocaleString('zh-CN')}</p>
                </div>
              </div>
              {vs.performanceMetrics && Object.keys(vs.performanceMetrics).length > 0 && (
                <div>
                  <p className="text-sm text-muted-foreground mb-1">绩效指标</p>
                  <ul className="text-sm">
                    {Object.entries(vs.performanceMetrics).map(([k, v]) => (
                      <li key={k}><span className="font-medium">{k}</span> → {v}</li>
                    ))}
                  </ul>
                </div>
              )}
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <div className="flex items-center justify-between">
                <CardTitle>价值流阶段</CardTitle>
                {isEntityOwner(vs.ownerId) && id && spaceId && (
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={() => { setEditingStage(null); setStageDialogOpen(true) }}
                  >
                    <Plus className="h-4 w-4 mr-2" />
                    添加阶段
                  </Button>
                )}
              </div>
            </CardHeader>
            <CardContent>
              {stages.length === 0 ? (
                <div className="text-center py-8 text-muted-foreground">暂无阶段</div>
              ) : (
                <Table>
                  <TableHeader>
                    <TableRow>
                      <TableHead>序号</TableHead>
                      <TableHead>名称</TableHead>
                      <TableHead>输入</TableHead>
                      <TableHead>输出</TableHead>
                      <TableHead>目标指标</TableHead>
                      <TableHead>关联能力</TableHead>
                      {isEntityOwner(vs.ownerId) && <TableHead>操作</TableHead>}
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {stages.map((stage) => (
                      <TableRow key={stage.id}>
                        <TableCell className="font-mono">{stage.sequenceOrder}</TableCell>
                        <TableCell className="font-medium">
                          {stage.name}
                          {stage.description && (
                            <p className="text-xs text-muted-foreground">{stage.description}</p>
                          )}
                        </TableCell>
                        <TableCell>{stage.input ?? '-'}</TableCell>
                        <TableCell>{stage.output ?? '-'}</TableCell>
                        <TableCell>
                          {stage.objectiveMetrics && Object.keys(stage.objectiveMetrics).length > 0 ? (
                            <ul className="text-xs space-y-0.5">
                              {Object.entries(stage.objectiveMetrics).map(([k, v]) => (
                                <li key={k}>{k} → {v}</li>
                              ))}
                            </ul>
                          ) : '-'}
                        </TableCell>
                        <TableCell className="max-w-[180px]">
                          <StageCapabilitiesCell stageId={stage.id} />
                        </TableCell>
                        {isEntityOwner(vs.ownerId) && (
                          <TableCell>
                            <div className="flex gap-1">
                              <Button
                                variant="ghost"
                                size="sm"
                                aria-label="关联能力"
                                title="关联能力"
                                onClick={() => setCapabilityStage(stage)}
                              >
                                <Link2 className="h-3.5 w-3.5" />
                              </Button>
                              <Button
                                variant="ghost"
                                size="sm"
                                aria-label="编辑"
                                title="编辑"
                                onClick={() => { setEditingStage(stage); setStageDialogOpen(true) }}
                              >
                                <Pencil className="h-3.5 w-3.5" />
                              </Button>
                              <Button
                                variant="ghost"
                                size="sm"
                                aria-label="删除"
                                title="删除"
                                onClick={() => setDeletingStage(stage)}
                              >
                                <Trash2 className="h-3.5 w-3.5 text-destructive" />
                              </Button>
                            </div>
                          </TableCell>
                        )}
                      </TableRow>
                    ))}
                  </TableBody>
                </Table>
              )}
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <div className="flex items-center justify-between">
                <CardTitle>运行实例</CardTitle>
                <div className="flex items-center gap-2">
                  {liveCount > 0 && (
                    <Badge>live {liveCount}</Badge>
                  )}
                  {vs && vs.status === 'active' && vs.logicalId && isEntityOwner(vs.ownerId) && id && spaceId && (
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={() => setProvisionOpen(true)}
                      aria-label="开通试点"
                      title="开通试点：创建运行实例并开通消费仓"
                    >
                      <Rocket className="h-4 w-4 mr-2" />
                      开通试点
                    </Button>
                  )}
                </div>
              </div>
            </CardHeader>
            <CardContent>
              {runs.length === 0 ? (
                <div className="text-center py-8 text-muted-foreground">暂无运行实例</div>
              ) : (
                <Table>
                  <TableHeader>
                    <TableRow>
                      <TableHead>仓库</TableHead>
                      <TableHead>链路</TableHead>
                      <TableHead>状态</TableHead>
                      <TableHead>版本 Pin</TableHead>
                      <TableHead>更新时间</TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {runs.map((run) => (
                      <TableRow key={run.id}>
                        <TableCell className="font-mono text-xs max-w-[220px] truncate" title={run.repoUrl}>{run.repoUrl}</TableCell>
                        <TableCell className="font-mono text-xs">{run.chainPath}</TableCell>
                        <TableCell><Badge>{run.status}</Badge></TableCell>
                        <TableCell className="text-xs">
                          {run.pinnedVersions && Object.keys(run.pinnedVersions).length > 0
                            ? Object.entries(run.pinnedVersions).map(([k, v]) => (
                              <span key={k} className="mr-2 font-mono">{k}@{v}</span>
                            ))
                            : '-'}
                        </TableCell>
                        <TableCell className="text-xs text-muted-foreground">{new Date(run.updatedAt).toLocaleString('zh-CN')}</TableCell>
                      </TableRow>
                    ))}
                  </TableBody>
                </Table>
              )}
            </CardContent>
          </Card>

          {id && spaceId && (
            <>
              <StageCrudDialog
                open={stageDialogOpen}
                onOpenChange={setStageDialogOpen}
                editing={editingStage}
                valueStreamId={id}
                spaceId={spaceId}
                nextSequenceOrder={stages.length === 0 ? 1 : Math.max(...stages.map((s) => s.sequenceOrder)) + 1}
              />
              <StageDeleteDialog
                stage={deletingStage}
                onConfirm={() => setDeletingStage(null)}
                spaceId={spaceId}
                valueStreamId={id}
              />
              <StageCapabilityDialog
                stage={capabilityStage}
                spaceId={spaceId}
                open={!!capabilityStage}
                onOpenChange={(v) => { if (!v) setCapabilityStage(null) }}
              />
              {vs && id && (
                <PilotProvisionDialog
                  open={provisionOpen}
                  onOpenChange={setProvisionOpen}
                  valueStreamId={id}
                  logicalId={vs.logicalId}
                  name={vs.name}
                  businessVersion={vs.businessVersion}
                />
              )}
            </>
          )}
        </>
      )}
    </div>
  )
}

function PilotProvisionDialog({ open, onOpenChange, valueStreamId, logicalId, name, businessVersion }: {
  open: boolean
  onOpenChange: (v: boolean) => void
  valueStreamId: string
  logicalId: string
  name: string
  businessVersion: string
}) {
  const [templateVersion, setTemplateVersion] = useState('')
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [done, setDone] = useState<ValueStreamProvisionPilotResult | null>(null)
  const [provisionMut] = useMutation<{ valueStreamProvisionPilot?: ValueStreamProvisionPilotResult }>(PROVISION_PILOT)

  async function handleProvision() {
    if (!logicalId) {
      setError('价值流缺少 logicalId，无法开通')
      return
    }
    if (!templateVersion.trim()) {
      setError('请填写模板版本')
      return
    }
    setLoading(true); setError(null)
    try {
      const res = await provisionMut({
        variables: { logicalId, templateVersion: templateVersion.trim() },
        refetchQueries: [{ query: GET_VALUE_STREAM_RUNS, variables: { valueStreamId } }],
      })
      const out = res.data?.valueStreamProvisionPilot
      if (!out) throw new Error('开通无返回')
      setDone(out)
    } catch (err) {
      setError(err instanceof Error ? err.message : '开通失败')
    } finally { setLoading(false) }
  }

  return (
    <Dialog open={open} onOpenChange={(v) => { if (!v) { setError(null); setDone(null); setTemplateVersion('') } onOpenChange(v) }}>
      <DialogContent>
        <DialogHeader><DialogTitle>开通试点</DialogTitle></DialogHeader>
        <div className="space-y-4 py-4 text-sm">
          {error && <div className="rounded-md bg-destructive/10 p-3 text-sm text-destructive">{error}</div>}
          {done === null ? (
            <>
              <p className="text-muted-foreground">
                将为价值流「{name}」（当前版本 <span className="font-mono">{businessVersion}</span>）创建运行实例并开通消费仓。
                同一价值流重复开通会直接返回已有的 live 运行；同空间已有 live 消费仓时将复用，不重复建仓。
              </p>
              <div className="space-y-2">
                <Label htmlFor="vs-provision-template">模板版本</Label>
                <Input
                  id="vs-provision-template"
                  placeholder="如 v1.0.0-pilot"
                  value={templateVersion}
                  onChange={(e) => setTemplateVersion(e.target.value)}
                  disabled={loading}
                />
              </div>
            </>
          ) : done.status !== 'live' ? (
            <>
              <p className="font-medium">开通处理中（{done.status}），请稍后刷新运行实例列表：</p>
              <ul className="space-y-1 text-sm">
                {done.templateVersion && <li>模板版本 <span className="font-mono text-xs">{done.templateVersion}</span></li>}
                <li className="font-mono text-xs break-all">仓库 {done.repoUrl || '-'}</li>
              </ul>
            </>
          ) : (
            <>
              <p className="font-medium">
                {done.reused ? '已有 live 运行，本次直接复用（未重新建仓）：' : '开通成功，运行实例已上线：'}
              </p>
              <ul className="space-y-1 text-sm">
                <li>状态 <Badge>{done.status}</Badge></li>
                {done.templateVersion && <li>模板版本 <span className="font-mono text-xs">{done.templateVersion}</span></li>}
                <li className="font-mono text-xs break-all">仓库 {done.repoUrl || '-'}</li>
                {done.prUrl && <li className="font-mono text-xs break-all">PR {done.prUrl}</li>}
              </ul>
            </>
          )}
        </div>
        <DialogFooter>
          {done === null && (
            <Button onClick={handleProvision} disabled={loading}>
              {loading ? '开通中...' : '确认开通'}
            </Button>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
