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
        createdAt
        updatedAt
      }
    }
  }
`

export default function ValueStreamDetail() {
  const { id, spaceId } = useParams<{ id: string; spaceId: string }>()
  const { data, loading, error } = useQuery(GET_VALUE_STREAM_DETAIL, {
    variables: { id },
  })

  const vs = data?.valueStreams?.nodes?.[0]
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
    </div>
  )
}
