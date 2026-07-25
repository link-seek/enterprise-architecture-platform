import { useQuery, useMutation } from '@apollo/client/react'
import { gql } from '@apollo/client'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table'
import { Badge } from '@/components/ui/badge'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { Plus, Pencil, Trash2, Loader2 } from 'lucide-react'
import { useState, useEffect } from 'react'
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogFooter } from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { useParams } from 'react-router-dom'
import { useSpaceMembership } from '@/hooks/use-space-membership'

const GET_CAPABILITIES = gql`
  query GetCapabilities($spaceId: String) {
    businessCapabilities(filters: { spaceId: { eq: $spaceId } }) {
      nodes { id name description level maturity businessValue status }
      paginationInfo { total }
    }
  }
`

const CREATE_CAPABILITY = gql`
  mutation CreateCapability($spaceId: String!, $name: String!, $description: String!, $level: CapabilityLevel!, $maturity: MaturityLevel!, $businessValue: BusinessValueRating!) {
    capabilityCreate(spaceId: $spaceId, name: $name, description: $description, level: $level, maturity: $maturity, businessValue: $businessValue) { id name }
  }
`

const UPDATE_CAPABILITY = gql`
  mutation UpdateCapability($id: String!, $name: String, $description: String, $level: CapabilityLevel, $maturity: MaturityLevel, $businessValue: BusinessValueRating) {
    capabilityUpdate(id: $id, name: $name, description: $description, level: $level, maturity: $maturity, businessValue: $businessValue) { id name }
  }
`

const DELETE_CAPABILITY = gql`
  mutation DeleteCapability($id: String!) {
    capabilityDelete(id: $id)
  }
`

interface Capability {
  id: string; name: string; description: string
  level: string; maturity: string; businessValue: string; status: string
}

export default function Capabilities() {
  const { spaceId } = useParams<{ spaceId: string }>()
  const { canEdit } = useSpaceMembership(spaceId)
  const [dialogOpen, setDialogOpen] = useState(false)
  const [editing, setEditing] = useState<Capability | null>(null)
  const [deleting, setDeleting] = useState<Capability | null>(null)
  const { data, loading, error } = useQuery(GET_CAPABILITIES, { variables: { spaceId } })

  return (
    <div className="p-6 space-y-4">
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
          {error && <div className="text-center py-8 text-destructive">加载失败</div>}
          {data && (
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
                {data.businessCapabilities.nodes.map((cap: Capability) => (
                  <TableRow key={cap.id}>
                    <TableCell className="font-medium">{cap.name}</TableCell>
                    <TableCell>{cap.level}</TableCell>
                    <TableCell>{cap.maturity}</TableCell>
                    <TableCell>{cap.businessValue}</TableCell>
                    <TableCell><Badge variant="outline">{cap.status}</Badge></TableCell>
                    <TableCell>
                      <div className="flex gap-1">
                        {canEdit && (
                          <>
                            <Button variant="ghost" size="sm" onClick={() => { setEditing(cap); setDialogOpen(true) }}>
                              <Pencil className="h-3.5 w-3.5" />
                            </Button>
                            <Button variant="ghost" size="sm" onClick={() => setDeleting(cap)}>
                              <Trash2 className="h-3.5 w-3.5 text-destructive" />
                            </Button>
                          </>
                        )}
                      </div>
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          )}
        </CardContent>
      </Card>
      <CapabilityCrudDialog open={dialogOpen} onOpenChange={setDialogOpen} editing={editing} spaceId={spaceId} />
      <CapabilityDeleteDialog item={deleting} onConfirm={() => setDeleting(null)} spaceId={spaceId} />
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
          <div className="space-y-2"><Label>名称</Label><Input value={name} onChange={e => setName(e.target.value)} /></div>
          <div className="space-y-2"><Label>描述</Label><Input value={description} onChange={e => setDescription(e.target.value)} /></div>
          <div className="grid grid-cols-3 gap-4">
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
