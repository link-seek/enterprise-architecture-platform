import { useState } from 'react'
import { useMutation } from '@apollo/client/react'
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogFooter } from '@/components/ui/dialog'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Loader2 } from 'lucide-react'
import { CREATE_SPACE, UPDATE_SPACE, GET_SPACES } from '@/api/spaces'
import type { Space } from '@/api/spaces'

export function SpaceCreateDialog({ open, onOpenChange }: {
  open: boolean
  onOpenChange: (v: boolean) => void
}) {
  const [name, setName] = useState('')
  const [description, setDescription] = useState('')
  const [error, setError] = useState<string | null>(null)

  const [create, { loading }] = useMutation(CREATE_SPACE, {
    refetchQueries: [{ query: GET_SPACES }],
    onCompleted: () => {
      setName('')
      setDescription('')
      setError(null)
      onOpenChange(false)
    },
    onError: (e) => setError(e.message),
  })

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>创建空间</DialogTitle>
        </DialogHeader>
        <div className="space-y-4 py-2">
          <div className="space-y-2">
            <Label htmlFor="space-name">名称</Label>
            <Input
              id="space-name"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="例如：某企业架构"
            />
          </div>
          <div className="space-y-2">
            <Label htmlFor="space-desc">描述</Label>
            <Input
              id="space-desc"
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              placeholder="可选"
            />
          </div>
          {error && <p className="text-sm text-destructive">{error}</p>}
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)} disabled={loading}>
            取消
          </Button>
          <Button
            onClick={() => create({ variables: { name, description: description || null } })}
            disabled={loading || !name.trim()}
          >
            {loading && <Loader2 className="h-4 w-4 mr-2 animate-spin" />}
            创建
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}

export function SpaceEditDialog({ space, open, onOpenChange }: {
  space: Space | null
  open: boolean
  onOpenChange: (v: boolean) => void
}) {
  const [name, setName] = useState('')
  const [description, setDescription] = useState('')
  const [error, setError] = useState<string | null>(null)

  // sync local state when dialog opens
  const [lastId, setLastId] = useState<string | null>(null)
  if (open && space && space.id !== lastId) {
    setLastId(space.id)
    setName(space.name)
    setDescription(space.description ?? '')
    setError(null)
  }
  if (!open && lastId !== null) setLastId(null)

  const [update, { loading }] = useMutation(UPDATE_SPACE, {
    refetchQueries: [{ query: GET_SPACES }],
    onCompleted: () => {
      setError(null)
      onOpenChange(false)
    },
    onError: (e) => setError(e.message),
  })

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>编辑空间</DialogTitle>
        </DialogHeader>
        <div className="space-y-4 py-2">
          <div className="space-y-2">
            <Label htmlFor="edit-space-name">名称</Label>
            <Input
              id="edit-space-name"
              value={name}
              onChange={(e) => setName(e.target.value)}
            />
          </div>
          <div className="space-y-2">
            <Label htmlFor="edit-space-desc">描述</Label>
            <Input
              id="edit-space-desc"
              value={description}
              onChange={(e) => setDescription(e.target.value)}
            />
          </div>
          {error && <p className="text-sm text-destructive">{error}</p>}
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)} disabled={loading}>
            取消
          </Button>
          <Button
            onClick={() =>
              update({
                variables: {
                  id: space?.id,
                  name: name || undefined,
                  description: description || null,
                },
              })
            }
            disabled={loading || !space}
          >
            {loading && <Loader2 className="h-4 w-4 mr-2 animate-spin" />}
            保存
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}