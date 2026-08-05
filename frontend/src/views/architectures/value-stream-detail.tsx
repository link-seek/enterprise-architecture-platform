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
      createdAt
      updatedAt
    }
    valueStreamStagesByValueStream(valueStreamId: $id) {
      id
      name
      sequenceOrder
      input
      output
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
    createdAt: string
    updatedAt: string
  } | null
  valueStreamStagesByValueStream: ValueStreamStage[]
}

export default function ValueStreamDetail() {
  const { id, spaceId } = useParams<{ id: string; spaceId: string }>()
  const { canEdit } = useSpaceMembership(spaceId)
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
                  <p>{vs.description}</p>
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
                  <p className="text-sm text-muted-foreground">创建时间</p>
                  <p>{new Date(vs.createdAt).toLocaleString('zh-CN')}</p>
                </div>
                <div>
                  <p className="text-sm text-muted-foreground">更新时间</p>
                  <p>{new Date(vs.updatedAt).toLocaleString('zh-CN')}</p>
                </div>
              </div>
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <div className="flex items-center justify-between">
                <CardTitle>价值流阶段</CardTitle>
                {canEdit && id && spaceId && (
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
                      {canEdit && <TableHead>操作</TableHead>}
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {stages.map((stage) => (
                      <TableRow key={stage.id}>
                        <TableCell className="font-mono">{stage.sequenceOrder}</TableCell>
                        <TableCell className="font-medium">{stage.name}</TableCell>
                        <TableCell>{stage.input ?? '-'}</TableCell>
                        <TableCell>{stage.output ?? '-'}</TableCell>
                        {canEdit && (
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
