import { useQuery } from '@apollo/client/react'
import { gql } from '@apollo/client'
import { useParams, Link } from 'react-router-dom'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { Badge } from '@/components/ui/badge'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table'
import { ArrowLeft, Plus, Pencil, Trash2 } from 'lucide-react'
import { useState, useMemo } from 'react'
import { useSpaceMembership } from '@/hooks/use-space-membership'
import { StageCrudDialog, StageDeleteDialog, type ValueStreamStage } from './value-stream-stages'

export const GET_VALUE_STREAM_DETAIL = gql`
  query GetValueStreamDetail($spaceId: String!, $id: String!) {
    valueStreamById(spaceId: $spaceId, id: $id) {
      id
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

interface ValueStreamDetailQuery {
  valueStreamById: {
    id: string
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

export default function ValueStreamDetail() {
  const { id, spaceId } = useParams<{ id: string; spaceId: string }>()
  const { isEntityOwner } = useSpaceMembership(spaceId)
  const { data, loading, error } = useQuery<ValueStreamDetailQuery>(GET_VALUE_STREAM_DETAIL, {
    variables: { spaceId, id },
    skip: !spaceId || !id,
  })

  const [stageDialogOpen, setStageDialogOpen] = useState(false)
  const [editingStage, setEditingStage] = useState<ValueStreamStage | null>(null)
  const [deletingStage, setDeletingStage] = useState<ValueStreamStage | null>(null)

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
                        {isEntityOwner(vs.ownerId) && (
                          <TableCell>
                            <div className="flex gap-1">
                              <Button
                                variant="ghost"
                                size="sm"
                                aria-label="编辑"
                                onClick={() => { setEditingStage(stage); setStageDialogOpen(true) }}
                              >
                                <Pencil className="h-3.5 w-3.5" />
                              </Button>
                              <Button
                                variant="ghost"
                                size="sm"
                                aria-label="删除"
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
            </>
          )}
        </>
      )}
    </div>
  )
}
