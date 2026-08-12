import { useQuery, useApolloClient } from '@apollo/client/react'
import { gql } from '@apollo/client'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Loader2 } from 'lucide-react'
import { useState, useEffect, useMemo } from 'react'
import { useParams } from 'react-router-dom'

const GET_REALIZATIONS_DATA = gql`
  query GetRealizationsData($spaceId: String!) {
    businessProcessesBySpace(spaceId: $spaceId) { id name }
    applicationProcessesBySpace(spaceId: $spaceId) { id name }
    businessCapabilitiesBySpace(spaceId: $spaceId) { id name }
    organizationalUnitsBySpace(spaceId: $spaceId) { id name }
    businessRolesBySpace(spaceId: $spaceId) { id name }
    functionalModulesBySpace(spaceId: $spaceId) { id name }
    applicationInterfacesBySpace(spaceId: $spaceId) { id name }
  }
`

const GET_CAPABILITY_REALIZATIONS = gql`
  query GetCapabilityRealizations($capabilityId: String!) {
    capabilityRealizationsByCapability(capabilityId: $capabilityId) {
      capabilityId processId processType
    }
  }
`

interface Named { id: string; name: string }
interface CapabilityRealization { capabilityId: string; processId: string; processType: string }
interface RealizationsData {
  businessProcessesBySpace?: Named[]
  applicationProcessesBySpace?: Named[]
  businessCapabilitiesBySpace?: Named[]
  organizationalUnitsBySpace?: Named[]
  businessRolesBySpace?: Named[]
  functionalModulesBySpace?: Named[]
  applicationInterfacesBySpace?: Named[]
}

function RealizationTable({ title, rows }: { title: string; rows: { left: string; right: string }[] }) {
  return (
    <Card>
      <CardHeader><CardTitle>{title}</CardTitle></CardHeader>
      <CardContent>
        {rows.length === 0 ? (
          <div className="text-center py-8 text-muted-foreground">暂无数据</div>
        ) : (
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>业务侧</TableHead>
                <TableHead>应用侧</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {rows.map((r, i) => (
                <TableRow key={i}>
                  <TableCell className="font-medium break-words">{r.left}</TableCell>
                  <TableCell className="break-words">{r.right}</TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        )}
      </CardContent>
    </Card>
  )
}

export default function Realizations() {
  const { spaceId } = useParams<{ spaceId: string }>()
  const client = useApolloClient()
  const { data, loading, error } = useQuery<RealizationsData>(GET_REALIZATIONS_DATA, { variables: { spaceId }, skip: !spaceId })
  const [capabilityRealizations, setCapabilityRealizations] = useState<CapabilityRealization[]>([])
  const [realizationsLoading, setRealizationsLoading] = useState(false)

  const capabilities = data?.businessCapabilitiesBySpace ?? []

  useEffect(() => {
    if (!data) return
    let cancelled = false
    async function fetchCapabilityRealizations() {
      if (capabilities.length === 0) { setCapabilityRealizations([]); return }
      setRealizationsLoading(true)
      try {
        const results = await Promise.all(
          capabilities.map((cap) =>
            client.query<{ capabilityRealizationsByCapability: CapabilityRealization[] }>({
              query: GET_CAPABILITY_REALIZATIONS,
              variables: { capabilityId: cap.id },
              fetchPolicy: 'network-only',
            }),
          ),
        )
        if (!cancelled) {
          setCapabilityRealizations(results.flatMap((r) => r.data?.capabilityRealizationsByCapability ?? []))
        }
      } catch {
        if (!cancelled) setCapabilityRealizations([])
      } finally {
        if (!cancelled) setRealizationsLoading(false)
      }
    }
    fetchCapabilityRealizations()
    return () => { cancelled = true }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [data])

  const businessProcessName = useMemo(() => {
    const map = new Map<string, string>()
    for (const bp of data?.businessProcessesBySpace ?? []) map.set(bp.id, bp.name)
    return map
  }, [data])

  const applicationProcessName = useMemo(() => {
    const map = new Map<string, string>()
    for (const ap of data?.applicationProcessesBySpace ?? []) map.set(ap.id, ap.name)
    return map
  }, [data])

  const capabilityName = useMemo(() => {
    const map = new Map<string, string>()
    for (const cap of data?.businessCapabilitiesBySpace ?? []) map.set(cap.id, cap.name)
    return map
  }, [data])

  const processName = (id: string, type: string) => {
    if (type === 'business_process') return businessProcessName.get(id) ?? id
    return applicationProcessName.get(id) ?? id
  }

  const capabilityRows = capabilityRealizations.map((r) => ({
    left: capabilityName.get(r.capabilityId) ?? r.capabilityId,
    right: `${processName(r.processId, r.processType)} (${r.processType})`,
  }))

  const showSpinner = loading || realizationsLoading

  return (
    <div className="p-4 md:p-6 space-y-4">
      <h1 className="text-2xl font-semibold">映射关系</h1>
      {showSpinner && (
        <div className="text-center py-8 text-muted-foreground flex items-center justify-center gap-2">
          <Loader2 className="h-4 w-4 animate-spin" />加载中...
        </div>
      )}
      {Boolean(error) && !loading && <div className="text-center py-8 text-destructive">加载失败</div>}
      {!showSpinner && !error && (
        <>
          <RealizationTable title="业务能力 → 流程（v2.1）" rows={capabilityRows} />
          <div className="text-center py-4 text-sm text-muted-foreground">
            v2.1 变更：ProcessRealization 和 StepRealization 已删除。
            新增关系（Assignment、Participation、ModuleContainment、InterfaceExposure、ProcessReference、Orchestration）
            可通过对应实体页面查看。
          </div>
        </>
      )}
    </div>
  )
}