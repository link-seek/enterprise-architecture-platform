import { useQuery, useMutation } from '@apollo/client/react'
import { gql } from '@apollo/client'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table'
import { Badge } from '@/components/ui/badge'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from '@/components/ui/dropdown-menu'
import { Plus, Pencil, Trash2, Loader2, MoreVertical } from 'lucide-react'
import { useState, useEffect, useCallback, memo } from 'react'
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogFooter } from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { useParams } from 'react-router-dom'
import { useSpaceMembership } from '@/hooks/use-space-membership'
import { useIsMobile } from '@/hooks/use-media-query'

const GET_APPLICATION_COMPONENTS = gql`
  query GetApplicationComponents($spaceId: String!) {
    applicationComponentsBySpace(spaceId: $spaceId) {
      id name type repo path technology status version
    }
  }
`

const CREATE_APPLICATION_COMPONENT = gql`
  mutation CreateApplicationComponent($spaceId: String!, $name: String!, $type: ApplicationComponentTypeEnum!, $repo: String!, $path: String!, $technology: String, $status: ApplicationComponentStatusEnum!, $version: String!) {
    applicationComponentCreate(spaceId: $spaceId, name: $name, type: $type, repo: $repo, path: $path, technology: $technology, status: $status, version: $version) { id name }
  }
`

const UPDATE_APPLICATION_COMPONENT = gql`
  mutation UpdateApplicationComponent($id: String!, $name: String, $type: ApplicationComponentTypeEnum, $repo: String, $path: String, $technology: String, $status: ApplicationComponentStatusEnum, $version: String) {
    applicationComponentUpdate(id: $id, name: $name, type: $type, repo: $repo, path: $path, technology: $technology, status: $status, version: $version) { id name }
  }
`

const DELETE_APPLICATION_COMPONENT = gql`
  mutation DeleteApplicationComponent($id: String!) {
    applicationComponentDelete(id: $id)
  }
`

interface ApplicationComponent {
  id: string; name: string; type: string; repo: string; path: string
  technology: string | null; status: string; version: string
}

interface ApplicationComponentsQuery {
  applicationComponentsBySpace?: ApplicationComponent[]
}

const EMPTY_COMPONENTS: ApplicationComponent[] = []

const TYPE_LABELS: Record<string, string> = {
  workflow: '工作流', script: '脚本', service: '服务', ui: '界面',
}

const ComponentList = memo(function ComponentList({ nodes, canEdit, isMobile, onEdit, onDelete }: {
  nodes: ApplicationComponent[]
  canEdit: boolean
  isMobile: boolean
  onEdit: (c: ApplicationComponent) => void
  onDelete: (c: ApplicationComponent) => void
}) {
  if (nodes.length === 0) {
    return <div className="text-center py-8 text-muted-foreground">暂无数据</div>
  }

  if (isMobile) {
    return (
      <div className="space-y-3">
        {nodes.map((c) => (
          <div key={c.id} className="rounded-lg border p-4 space-y-2">
            <div className="flex items-start justify-between gap-2">
              <p className="font-medium break-words">{c.name}</p>
              <Badge variant="outline">{c.status}</Badge>
            </div>
            <div className="flex flex-wrap gap-1">
              <Badge variant="secondary">{TYPE_LABELS[c.type] ?? c.type}</Badge>
              <Badge variant="secondary">{c.version}</Badge>
            </div>
            <p className="text-xs text-muted-foreground truncate" title={`${c.repo}/${c.path}`}>{c.repo}/{c.path}</p>
            {canEdit && (
              <div className="flex justify-end pt-1">
                <DropdownMenu>
                  <DropdownMenuTrigger asChild>
                    <Button variant="ghost" size="sm" className="h-9 w-9 p-0" aria-label="更多操作">
                      <MoreVertical className="h-4 w-4" />
                    </Button>
                  </DropdownMenuTrigger>
                  <DropdownMenuContent align="end">
                    <DropdownMenuItem onClick={() => onEdit(c)}>
                      <Pencil className="h-4 w-4 mr-2" />编辑
                    </DropdownMenuItem>
                    <DropdownMenuItem className="text-destructive" onClick={() => onDelete(c)}>
                      <Trash2 className="h-4 w-4 mr-2" />删除
                    </DropdownMenuItem>
                  </DropdownMenuContent>
                </DropdownMenu>
              </div>
            )}
          </div>
        ))}
      </div>
    )
  }

  return (
    <Table>
      <TableHeader>
        <TableRow>
          <TableHead>名称</TableHead>
          <TableHead>类型</TableHead>
          <TableHead>仓库</TableHead>
          <TableHead>路径</TableHead>
          <TableHead>技术</TableHead>
          <TableHead>版本</TableHead>
          <TableHead>状态</TableHead>
          {canEdit && <TableHead>操作</TableHead>}
        </TableRow>
      </TableHeader>
      <TableBody>
        {nodes.map((c) => (
          <TableRow key={c.id}>
            <TableCell className="font-medium break-words">{c.name}</TableCell>
            <TableCell>{TYPE_LABELS[c.type] ?? c.type}</TableCell>
            <TableCell className="text-muted-foreground break-words">{c.repo}</TableCell>
            <TableCell className="text-muted-foreground break-words">{c.path}</TableCell>
            <TableCell className="text-muted-foreground">{c.technology ?? '-'}</TableCell>
            <TableCell>{c.version}</TableCell>
            <TableCell><Badge variant="outline">{c.status}</Badge></TableCell>
            {canEdit && (
              <TableCell>
                <div className="flex gap-1">
                  <Button variant="ghost" size="sm" aria-label="编辑" onClick={() => onEdit(c)}>
                    <Pencil className="h-3.5 w-3.5" />
                  </Button>
                  <Button variant="ghost" size="sm" aria-label="删除" onClick={() => onDelete(c)}>
                    <Trash2 className="h-3.5 w-3.5 text-destructive" />
                  </Button>
                </div>
              </TableCell>
            )}
          </TableRow>
        ))}
      </TableBody>
    </Table>
  )
})

export default function Applications() {
  const { spaceId } = useParams<{ spaceId: string }>()
  const { canEdit } = useSpaceMembership(spaceId)
  const isMobile = useIsMobile()
  const [dialogOpen, setDialogOpen] = useState(false)
  const [editing, setEditing] = useState<ApplicationComponent | null>(null)
  const [deleting, setDeleting] = useState<ApplicationComponent | null>(null)
  const { data, loading, error } = useQuery<ApplicationComponentsQuery>(GET_APPLICATION_COMPONENTS, { variables: { spaceId }, skip: !spaceId })

  const handleEdit = useCallback((c: ApplicationComponent) => { setEditing(c); setDialogOpen(true) }, [])
  const handleDelete = useCallback((c: ApplicationComponent) => setDeleting(c), [])

  return (
    <div className="p-4 md:p-6 space-y-4">
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-semibold">应用组件</h1>
        {canEdit && (
          <Button onClick={() => { setEditing(null); setDialogOpen(true) }}>
            <Plus className="h-4 w-4 mr-2" />新建组件
          </Button>
        )}
      </div>
      <Card>
        <CardHeader><CardTitle>组件列表</CardTitle></CardHeader>
        <CardContent>
          {loading && <div className="text-center py-8 text-muted-foreground">加载中...</div>}
          {Boolean(error) && <div className="text-center py-8 text-destructive">加载失败</div>}
          {data && (
            <ComponentList
              nodes={data.applicationComponentsBySpace ?? EMPTY_COMPONENTS}
              canEdit={canEdit}
              isMobile={isMobile}
              onEdit={handleEdit}
              onDelete={handleDelete}
            />
          )}
        </CardContent>
      </Card>
      <ComponentCrudDialog open={dialogOpen} onOpenChange={setDialogOpen} editing={editing} spaceId={spaceId} />
      <ComponentDeleteDialog item={deleting} onConfirm={() => setDeleting(null)} spaceId={spaceId} />
    </div>
  )
}

function ComponentCrudDialog({ open, onOpenChange, editing, spaceId }: {
  open: boolean; onOpenChange: (v: boolean) => void; editing: ApplicationComponent | null; spaceId?: string
}) {
  const [name, setName] = useState('')
  const [type, setType] = useState('workflow')
  const [repo, setRepo] = useState('')
  const [path, setPath] = useState('')
  const [technology, setTechnology] = useState('')
  const [status, setStatus] = useState('draft')
  const [version, setVersion] = useState('v1.0')
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [createMut] = useMutation(CREATE_APPLICATION_COMPONENT)
  const [updateMut] = useMutation(UPDATE_APPLICATION_COMPONENT)

  useEffect(() => {
    if (open) {
      setError(null)
      if (editing) {
        setName(editing.name); setType(editing.type); setRepo(editing.repo); setPath(editing.path)
        setTechnology(editing.technology || ''); setStatus(editing.status); setVersion(editing.version)
      } else {
        setName(''); setType('workflow'); setRepo(''); setPath(''); setTechnology(''); setStatus('draft'); setVersion('v1.0')
      }
    }
  }, [open, editing])

  async function handleSubmit() {
    setLoading(true); setError(null)
    try {
      if (editing) {
        await updateMut({
          variables: { id: editing.id, name, type, repo, path, technology: technology || null, status, version },
          refetchQueries: [{ query: GET_APPLICATION_COMPONENTS, variables: { spaceId } }],
        })
      } else {
        await createMut({
          variables: { spaceId, name, type, repo, path, technology: technology || null, status, version },
          refetchQueries: [{ query: GET_APPLICATION_COMPONENTS, variables: { spaceId } }],
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
        <DialogHeader><DialogTitle>{editing ? '编辑组件' : '新建组件'}</DialogTitle></DialogHeader>
        <div className="space-y-4 py-4">
          {error && <div className="rounded-md bg-destructive/10 p-3 text-sm text-destructive">{error}</div>}
          <div className="space-y-2"><Label>名称</Label><Input value={name} onChange={e => setName(e.target.value)} /></div>
          <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
            <div className="space-y-2">
              <Label>类型</Label>
              <select className="w-full rounded-md border bg-background px-3 py-2 text-sm" value={type} onChange={e => setType(e.target.value)}>
                <option value="workflow">工作流</option><option value="script">脚本</option><option value="service">服务</option><option value="ui">界面</option>
              </select>
            </div>
            <div className="space-y-2">
              <Label>状态</Label>
              <select className="w-full rounded-md border bg-background px-3 py-2 text-sm" value={status} onChange={e => setStatus(e.target.value)}>
                <option value="draft">草稿</option><option value="active">活跃</option><option value="deprecated">已弃用</option>
              </select>
            </div>
          </div>
          <div className="space-y-2"><Label>仓库</Label><Input value={repo} onChange={e => setRepo(e.target.value)} placeholder="org/repo" /></div>
          <div className="space-y-2"><Label>路径</Label><Input value={path} onChange={e => setPath(e.target.value)} placeholder="src/service" /></div>
          <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
            <div className="space-y-2"><Label>技术</Label><Input value={technology} onChange={e => setTechnology(e.target.value)} placeholder="Rust" /></div>
            <div className="space-y-2"><Label>版本</Label><Input value={version} onChange={e => setVersion(e.target.value)} /></div>
          </div>
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>取消</Button>
          <Button onClick={handleSubmit} disabled={loading || !name || !repo || !path || !version}>{loading ? <Loader2 className="h-4 w-4 animate-spin" /> : editing ? '保存' : '创建'}</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}

function ComponentDeleteDialog({ item, onConfirm, spaceId }: { item: ApplicationComponent | null; onConfirm: () => void; spaceId?: string }) {
  const [deleteMut] = useMutation(DELETE_APPLICATION_COMPONENT)
  const [loading, setLoading] = useState(false)
  async function handleDelete() {
    if (!item) return; setLoading(true)
    try { await deleteMut({ variables: { id: item.id }, refetchQueries: [{ query: GET_APPLICATION_COMPONENTS, variables: { spaceId } }] }); onConfirm() }
    catch (err) { console.error(err) } finally { setLoading(false) }
  }
  return (
    <Dialog open={!!item} onOpenChange={onConfirm}>
      <DialogContent>
        <DialogHeader><DialogTitle>确认删除</DialogTitle></DialogHeader>
        <p className="py-4 text-sm text-muted-foreground">确定要删除组件「{item?.name}」吗？</p>
        <DialogFooter>
          <Button variant="outline" onClick={onConfirm}>取消</Button>
          <Button variant="destructive" onClick={handleDelete} disabled={loading}>{loading ? <Loader2 className="h-4 w-4 animate-spin" /> : '删除'}</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}