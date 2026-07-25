import { useQuery } from '@apollo/client/react'
import { gql } from '@apollo/client'
import { useParams, Link } from 'react-router-dom'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { Badge } from '@/components/ui/badge'
import { ArrowLeft } from 'lucide-react'

const GET_VALUE_STREAM_DETAIL = gql`
  query GetValueStreamDetail($id: String!) {
    valueStreams(filters: { id: { eq: $id } }) {
      nodes {
        id
        name
        description
        businessVersion
        status
        triggeringEvent
        endDeliverable
        valueProposition
        createdAt
        updatedAt
      }
    }
  }
`

const GET_VALUE_STREAM_STAGES = gql`
  query GetValueStreamStages($valueStreamId: String!) {
    valueStreamStages(filters: { valueStreamId: { eq: $valueStreamId } }) {
      nodes {
        id
        name
        description
        sequenceOrder
        stageType
        status
        input
        output
        ownerId
        objectives
        metrics
      }
    }
  }
`

const STAGE_TYPE_LABELS: Record<string, string> = {
  Design: '设计',
  Production: '生产',
  Sales: '销售',
  Delivery: '交付',
  Custom: '自定义',
}

export default function ValueStreamDetail() {
  const { id, spaceId } = useParams<{ id: string; spaceId: string }>()
  const { data, loading, error } = useQuery(GET_VALUE_STREAM_DETAIL, {
    variables: { id },
  })
  const { data: stagesData } = useQuery(GET_VALUE_STREAM_STAGES, {
    variables: { valueStreamId: id },
    skip: !id,
  })

  const vs = data?.valueStreams?.nodes?.[0]
  const stages = (stagesData?.valueStreamStages?.nodes ?? []) as Array<{
    id: string
    name: string
    description?: string | null
    sequenceOrder: number
    stageType: string
    status: string
    input?: string | null
    output?: string | null
    ownerId?: string | null
    objectives?: string[] | null
    metrics?: Record<string, string> | null
  }>
  stages.sort((a, b) => a.sequenceOrder - b.sequenceOrder)

  const backPath = spaceId
    ? `/spaces/${spaceId}/architectures/value-streams`
    : '/architectures/value-streams'

  return (
    <div className="p-6 space-y-4">
      <Link to={backPath}>
        <Button variant="ghost" size="sm" className="gap-2">
          <ArrowLeft className="h-4 w-4" />
          返回列表
        </Button>
      </Link>

      {loading && <div className="text-center py-8 text-muted-foreground">加载中...</div>}
      {error && <div className="text-center py-8 text-destructive">加载失败</div>}
      {vs && (
        <Card>
          <CardHeader>
            <div className="flex items-center justify-between">
              <CardTitle className="text-2xl">{vs.name}</CardTitle>
              <Badge>{vs.status}</Badge>
            </div>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="grid grid-cols-2 gap-4">
              <div>
                <p className="text-sm text-muted-foreground">描述</p>
                <p>{vs.description}</p>
              </div>
              <div>
                <p className="text-sm text-muted-foreground">版本</p>
                <p>{vs.businessVersion}</p>
              </div>
              <div>
                <p className="text-sm text-muted-foreground">价值主张</p>
                <p>{vs.valueProposition ?? '—'}</p>
              </div>
              <div>
                <p className="text-sm text-muted-foreground">触发事件</p>
                <p>{vs.triggeringEvent ?? '—'}</p>
              </div>
              <div>
                <p className="text-sm text-muted-foreground">最终交付物</p>
                <p>{vs.endDeliverable ?? '—'}</p>
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
      )}

      {vs && (
        <Card>
          <CardHeader>
            <CardTitle>价值流阶段</CardTitle>
          </CardHeader>
          <CardContent>
            {stages.length === 0 ? (
              <p className="text-sm text-muted-foreground">暂无阶段。可在价值流中拆分设计、生产、销售、交付等阶段。</p>
            ) : (
              <ol className="relative border-l border-muted pl-6 space-y-6">
                {stages.map((stage) => (
                  <li key={stage.id} className="relative">
                    <span className="absolute -left-[31px] flex h-6 w-6 items-center justify-center rounded-full border bg-background text-xs font-medium">
                      {stage.sequenceOrder}
                    </span>
                    <div className="flex items-center gap-2">
                      <h3 className="font-medium">{stage.name}</h3>
                      <Badge variant="secondary">{STAGE_TYPE_LABELS[stage.stageType] ?? stage.stageType}</Badge>
                      <Badge variant="outline">{stage.status}</Badge>
                    </div>
                    {stage.description && (
                      <p className="mt-1 text-sm text-muted-foreground">{stage.description}</p>
                    )}
                    <div className="mt-2 grid grid-cols-2 gap-2 text-sm">
                      <div>
                        <span className="text-muted-foreground">输入：</span>
                        {stage.input ?? '—'}
                      </div>
                      <div>
                        <span className="text-muted-foreground">输出：</span>
                        {stage.output ?? '—'}
                      </div>
                    </div>
                    {stage.objectives && stage.objectives.length > 0 && (
                      <div className="mt-2 text-sm">
                        <span className="text-muted-foreground">目标：</span>
                        {stage.objectives.join('、')}
                      </div>
                    )}
                    {stage.metrics && Object.keys(stage.metrics).length > 0 && (
                      <div className="mt-2 text-sm">
                        <span className="text-muted-foreground">指标：</span>
                        {Object.entries(stage.metrics).map(([k, v]) => `${k}=${v}`).join('、')}
                      </div>
                    )}
                  </li>
                ))}
              </ol>
            )}
          </CardContent>
        </Card>
      )}
    </div>
  )
}
