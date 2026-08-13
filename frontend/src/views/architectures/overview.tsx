import { useQuery } from '@apollo/client/react'
import { gql } from '@apollo/client'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table'
import { Loader2 } from 'lucide-react'
import { Link, useParams } from 'react-router-dom'
import { useMemo } from 'react'

const GET_OVERVIEW_BUSINESS = gql`
  query GetOverviewBusiness($spaceId: String!) {
    valueStreamsBySpace(spaceId: $spaceId) { id name }
    businessCapabilitiesBySpace(spaceId: $spaceId) { id name }
    businessProcessesBySpace(spaceId: $spaceId) { id name }
    valueStreamCountBySpace(spaceId: $spaceId)
    businessCapabilityCountBySpace(spaceId: $spaceId)
    businessProcessCountBySpace(spaceId: $spaceId)
  }
`

const GET_OVERVIEW_APPLICATION = gql`
  query GetOverviewApplication($spaceId: String!) {
    applicationProcessesBySpace(spaceId: $spaceId) { id name }
    functionalModulesBySpace(spaceId: $spaceId) { id name }
    applicationComponentsBySpace(spaceId: $spaceId) { id name }
    applicationInterfacesBySpace(spaceId: $spaceId) { id name }
  }
`

const GET_CROSS_DOMAIN = gql`
  query GetCrossDomain($spaceId: String!) {
    capabilityRealizationsBySpace(spaceId: $spaceId) {
      capabilityId processId processType
    }
    processReferencesBySpace(spaceId: $spaceId) {
      applicationProcessId businessProcessId
    }
  }
`

interface Named { id: string; name: string }
interface CapabilityRealization { capabilityId: string; processId: string; processType: string }
interface ProcessReference { applicationProcessId: string; businessProcessId: string }

interface OverviewBusinessData {
  valueStreamsBySpace?: Named[]
  businessCapabilitiesBySpace?: Named[]
  businessProcessesBySpace?: Named[]
  valueStreamCountBySpace?: number
  businessCapabilityCountBySpace?: number
  businessProcessCountBySpace?: number
}

interface OverviewApplicationData {
  applicationProcessesBySpace?: Named[]
  functionalModulesBySpace?: Named[]
  applicationComponentsBySpace?: Named[]
  applicationInterfacesBySpace?: Named[]
}

interface JumpRow {
  leftId: string
  leftName: string
  rightId: string
  rightName: string
}

function DomainEntryCard({ title, count, to, hint }: { title: string; count: number; to: string; hint: string }) {
  return (
    <Link to={to} className="block h-full">
      <Card className="h-full hover:shadow-md transition-shadow">
        <CardHeader>
          <CardTitle className="flex items-center justify-between gap-2">
            {title}
            <span className="text-2xl font-bold text-primary">{count}</span>
          </CardTitle>
        </CardHeader>
        <CardContent>
          <p className="text-sm text-muted-foreground">{hint}</p>
        </CardContent>
      </Card>
    </Link>
  )
}

function CrossDomainTable({ title, rows, leftKind, rightKind, leftTo, rightTo }: {
  title: string
  rows: JumpRow[]
  leftKind: string
  rightKind: string
  leftTo: string
  rightTo: string
}) {
  return (
    <Card>
      <CardHeader><CardTitle className="text-base">{title}</CardTitle></CardHeader>
      <CardContent>
        {rows.length === 0 ? (
          <div className="text-center py-6 text-sm text-muted-foreground">暂无映射数据</div>
        ) : (
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>业务侧（{leftKind}）</TableHead>
                <TableHead>应用侧（{rightKind}）</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {rows.map((row, i) => (
                <TableRow key={i}>
                  <TableCell>
                    <Link to={leftTo} className="font-medium break-words text-foreground hover:text-primary hover:underline">
                      {row.leftName}
                    </Link>
                  </TableCell>
                  <TableCell>
                    <Link to={rightTo} className="break-words text-foreground hover:text-primary hover:underline">
                      {row.rightName}
                    </Link>
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        )}
      </CardContent>
    </Card>
  )
}

export default function ArchitectureOverview() {
  const { spaceId } = useParams<{ spaceId: string }>()
  const base = `/spaces/${spaceId}/architectures`

  const { data: businessData, loading: businessLoading, error: businessError } = useQuery<OverviewBusinessData>(
    GET_OVERVIEW_BUSINESS,
    { variables: { spaceId }, skip: !spaceId },
  )
  const { data: appData, loading: appLoading, error: appError } = useQuery<OverviewApplicationData>(
    GET_OVERVIEW_APPLICATION,
    { variables: { spaceId }, skip: !spaceId },
  )

  // 跨域支撑：一次聚合查询（capabilityRealizationsBySpace / processReferencesBySpace）
  // 替代逐实体 N+1 查询，避免超出发送端限流。
  const { data: crossData, loading: crossLoading, error: crossError } = useQuery<{
    capabilityRealizationsBySpace: CapabilityRealization[]
    processReferencesBySpace: ProcessReference[]
  }>(GET_CROSS_DOMAIN, { variables: { spaceId }, skip: !spaceId })

  const realizations = crossData?.capabilityRealizationsBySpace ?? []
  const references = crossData?.processReferencesBySpace ?? []

  const capabilityName = useMemo(() => {
    const map = new Map<string, string>()
    for (const cap of businessData?.businessCapabilitiesBySpace ?? []) map.set(cap.id, cap.name)
    return map
  }, [businessData])

  const applicationProcessName = useMemo(() => {
    const map = new Map<string, string>()
    for (const ap of appData?.applicationProcessesBySpace ?? []) map.set(ap.id, ap.name)
    return map
  }, [appData])

  const businessProcessName = useMemo(() => {
    const map = new Map<string, string>()
    for (const bp of businessData?.businessProcessesBySpace ?? []) map.set(bp.id, bp.name)
    return map
  }, [businessData])

  const realizationRows: JumpRow[] = realizations
    .filter((r) => r.processType === 'application_process')
    .map((r) => ({
      leftId: r.capabilityId,
      leftName: capabilityName.get(r.capabilityId) ?? r.capabilityId,
      rightId: r.processId,
      rightName: applicationProcessName.get(r.processId) ?? r.processId,
    }))

  const referenceRows: JumpRow[] = references.map((r) => ({
    leftId: r.businessProcessId,
    leftName: businessProcessName.get(r.businessProcessId) ?? r.businessProcessId,
    rightId: r.applicationProcessId,
    rightName: applicationProcessName.get(r.applicationProcessId) ?? r.applicationProcessId,
  }))

  const loading = businessLoading || appLoading || crossLoading
  const hasError = Boolean(businessError) || Boolean(appError) || Boolean(crossError)

  return (
    <div className="p-4 md:p-6 space-y-6">
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-semibold">架构总览</h1>
      </div>

      {loading && (
        <div className="flex items-center justify-center gap-2 py-8 text-muted-foreground">
          <Loader2 className="h-4 w-4 animate-spin" />加载中...
        </div>
      )}
      {hasError && !loading && <div className="text-center py-8 text-destructive">加载失败</div>}

      {!loading && !hasError && (
        <>
          <section aria-label="业务架构">
            <h2 className="text-lg font-semibold">业务架构</h2>
            <p className="mt-1 text-sm text-muted-foreground">
              价值流 → 阶段 → 能力 → 业务流程，自上而下梳理业务全景
            </p>
            <div className="mt-3 grid gap-6 md:grid-cols-3">
              <DomainEntryCard
                title="价值流"
                count={businessData?.valueStreamCountBySpace ?? 0}
                to={`${base}/value-streams`}
                hint="端到端价值交付链路"
              />
              <DomainEntryCard
                title="业务能力"
                count={businessData?.businessCapabilityCountBySpace ?? 0}
                to={`${base}/capabilities`}
                hint="组织核心能力地图"
              />
              <DomainEntryCard
                title="业务流程"
                count={businessData?.businessProcessCountBySpace ?? 0}
                to={`${base}/processes`}
                hint="流程活动与版本管理"
              />
            </div>
          </section>

          <section aria-label="应用架构">
            <h2 className="text-lg font-semibold">应用架构</h2>
            <p className="mt-1 text-sm text-muted-foreground">
              应用流程 → 功能模块 → 应用组件 → 应用接口，向下支撑业务落地
            </p>
            <div className="mt-3 grid gap-6 md:grid-cols-2 lg:grid-cols-4">
              <DomainEntryCard
                title="应用流程"
                count={appData?.applicationProcessesBySpace?.length ?? 0}
                to={`${base}/application-processes`}
                hint="系统运行流程与自动化任务"
              />
              <DomainEntryCard
                title="功能模块"
                count={appData?.functionalModulesBySpace?.length ?? 0}
                to={`${base}/functional-modules`}
                hint="应用功能边界与分层"
              />
              <DomainEntryCard
                title="应用组件"
                count={appData?.applicationComponentsBySpace?.length ?? 0}
                to={`${base}/applications`}
                hint="系统组成单元与交付物"
              />
              <DomainEntryCard
                title="应用接口"
                count={appData?.applicationInterfacesBySpace?.length ?? 0}
                to={`${base}/application-interfaces`}
                hint="接口契约与数据交换"
              />
            </div>
          </section>

          <section aria-label="跨域支撑">
            <h2 className="text-lg font-semibold">跨域支撑</h2>
            <p className="mt-1 text-sm text-muted-foreground">
              业务侧由应用侧支撑的显式映射，点击任一行可跳转到对应实体页
            </p>
            <div className="mt-3 grid gap-6 lg:grid-cols-2">
              <CrossDomainTable
                title="业务能力 → 应用流程"
                rows={realizationRows}
                leftKind="业务能力"
                rightKind="应用流程"
                leftTo={`${base}/capabilities`}
                rightTo={`${base}/application-processes`}
              />
              <CrossDomainTable
                title="业务流程 ↔ 应用流程"
                rows={referenceRows}
                leftKind="业务流程"
                rightKind="应用流程"
                leftTo={`${base}/processes`}
                rightTo={`${base}/application-processes`}
              />
            </div>
          </section>
        </>
      )}
    </div>
  )
}
