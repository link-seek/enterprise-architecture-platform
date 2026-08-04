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
    applicationComponentsBySpace(spaceId: $spaceId) { id name }
  }
`

const GET_PROCESS_REALIZATIONS = gql`
  query GetProcessRealizations($businessProcessId: String!) {
    processRealizationsByBusinessProcess(businessProcessId: $businessProcessId) {
      businessProcessId applicationProcessId
    }
  }
`

const GET_CAPABILITY_REALIZATIONS = gql`
  query GetCapabilityRealizations($capabilityId: String!) {
    capabilityRealizationsByCapability(capabilityId: $capabilityId) {
      capabilityId applicationComponentId
    }
  }
`

interface Named { id: string; name: string }
interface ProcessRealization { businessProcessId: string; applicationProcessId: string }
interface CapabilityRealization { capabilityId: string; applicationComponentId: string }
interface RealizationsData {
  businessProcessesBySpace?: Named[]
  applicationProcessesBySpace?: Named[]
  businessCapabilitiesBySpace?: Named[]
  applicationComponentsBySpace?: Named[]
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
  const [processRealizations, setProcessRealizations] = useState<ProcessRealization[]>([])
  const [capabilityRealizations, setCapabilityRealizations] = useState<CapabilityRealization[]>([])
  const [realizationsLoading, setRealizationsLoading] = useState(false)

  const businessProcesses = data?.businessProcessesBySpace ?? []
  const capabilities = data?.businessCapabilitiesBySpace ?? []

  useEffect(() => {
    if (!data) return
    let cancelled = false
    async function fetchProcessRealizations() {
      if (businessProcesses.length === 0) { setProcessRealizations([]); return }
      setRealizationsLoading(true)
      try {
        const results = await Promise.all(
          businessProcesses.map((bp) =>
            client.query<{ processRealizationsByBusinessProcess: ProcessRealization[] }>({
              query: GET_PROCESS_REALIZATIONS,
              variables: { businessProcessId: bp.id },
              fetchPolicy: 'network-only',
            }),
          ),
        )
        if (!cancelled) {
          setProcessRealizations(results.flatMap((r) => r.data?.processRealizationsByBusinessProcess ?? []))
        }
      } catch {
        if (!cancelled) setProcessRealizations([])
      } finally {
        if (!cancelled) setRealizationsLoading(false)
      }
    }
    fetchProcessRealizations()
    return () => { cancelled = true }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [data])

  useEffect(() => {
    if (!data) return
    let cancelled = false
    async function fetchCapabilityRealizations() {
      if (capabilities.length === 0) { setCapabilityRealizations([]); return }
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
      }
    }
    fetchCapabilityRealizations()
    return () => { cancelled = true }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [data])

  const applicationProcessName = useMemo(() => {
    const map = new Map<string, string>()
    for (const ap of data?.applicationProcessesBySpace ?? []) map.set(ap.id, ap.name)
    return map
  }, [data])

  const applicationComponentName = useMemo(() => {
    const map = new Map<string, string>()
    for (const ac of data?.applicationComponentsBySpace ?? []) map.set(ac.id, ac.name)
    return map
  }, [data])

  const businessProcessName = useMemo(() => {
    const map = new Map<string, string>()
    for (const bp of data?.businessProcessesBySpace ?? []) map.set(bp.id, bp.name)
    return map
  }, [data])

  const capabilityName = useMemo(() => {
    const map = new Map<string, string>()
    for (const cap of data?.businessCapabilitiesBySpace ?? []) map.set(cap.id, cap.name)
    return map
  }, [data])

  const processRows = processRealizations.map((r) => ({
    left: businessProcessName.get(r.businessProcessId) ?? r.businessProcessId,
    right: applicationProcessName.get(r.applicationProcessId) ?? r.applicationProcessId,
  }))

  const capabilityRows = capabilityRealizations.map((r) => ({
    left: capabilityName.get(r.capabilityId) ?? r.capabilityId,
    right: applicationComponentName.get(r.applicationComponentId) ?? r.applicationComponentId,
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
          <RealizationTable title="业务流程 → 应用流程" rows={processRows} />
          <RealizationTable title="业务能力 → 应用组件" rows={capabilityRows} />
        </>
      )}
    </div>
  )
}