import { useMutation } from '@apollo/client/react'
import { gql } from '@apollo/client'
import { useState, useEffect } from 'react'
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogFooter } from '@/components/ui/dialog'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { ConfirmDialog } from '@/components/ui/confirm-dialog'
import { Loader2 } from 'lucide-react'
import { GET_VALUE_STREAM_DETAIL } from './value-stream-detail'

export interface ValueStreamStage {
  id: string
  name: string
  sequenceOrder: number
  input: string | null
  output: string | null
}

const CREATE_STAGE = gql`
  mutation ValueStreamStageCreate($valueStreamId: String!, $name: String!, $sequenceOrder: Int!, $input: String, $output: String) {
    valueStreamStageCreate(valueStreamId: $valueStreamId, name: $name, sequenceOrder: $sequenceOrder, input: $input, output: $output) {
      id name sequenceOrder input output
    }
  }
`

const UPDATE_STAGE = gql`
  mutation ValueStreamStageUpdate($id: String!, $name: String, $sequenceOrder: Int, $input: String, $output: String) {
    valueStreamStageUpdate(id: $id, name: $name, sequenceOrder: $sequenceOrder, input: $input, output: $output) {
      id name sequenceOrder input output
    }
  }
`

const DELETE_STAGE = gql`
  mutation ValueStreamStageDelete($id: String!) {
    valueStreamStageDelete(id: $id)
  }
`

export function StageCrudDialog({ open, onOpenChange, editing, valueStreamId, spaceId, nextSequenceOrder }: {
  open: boolean
  onOpenChange: (v: boolean) => void
  editing: ValueStreamStage | null
  valueStreamId: string
  spaceId: string
  nextSequenceOrder: number
}) {
  const [name, setName] = useState('')
  const [sequenceOrder, setSequenceOrder] = useState(1)
  const [input, setInput] = useState('')
  const [output, setOutput] = useState('')
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const [createMut] = useMutation(CREATE_STAGE)
  const [updateMut] = useMutation(UPDATE_STAGE)

  useEffect(() => {
    if (open) {
      setError(null)
      if (editing) {
        setName(editing.name)
        setSequenceOrder(editing.sequenceOrder)
        setInput(editing.input ?? '')
        setOutput(editing.output ?? '')
      } else {
        setName('')
        setSequenceOrder(nextSequenceOrder)
        setInput('')
        setOutput('')
      }
    }
  }, [open, editing, nextSequenceOrder])

  async function handleSubmit() {
    setLoading(true)
    setError(null)
    try {
      const refetchQueries = [{ query: GET_VALUE_STREAM_DETAIL, variables: { spaceId, id: valueStreamId } }]
      if (editing) {
        await updateMut({
          variables: {
            id: editing.id,
            name,
            sequenceOrder,
            input: input || null,
            output: output || null,
          },
          refetchQueries,
        })
      } else {
        await createMut({
          variables: {
            valueStreamId,
            name,
            sequenceOrder,
            input: input || null,
            output: output || null,
          },
          refetchQueries,
        })
      }
      onOpenChange(false)
    } catch (err) {
      setError(err instanceof Error ? err.message : '操作失败')
    } finally {
      setLoading(false)
    }
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{editing ? '编辑阶段' : '添加阶段'}</DialogTitle>
        </DialogHeader>
        <div className="space-y-4 py-4">
          {error && <div className="rounded-md bg-destructive/10 p-3 text-sm text-destructive">{error}</div>}
          <div className="space-y-2">
            <Label htmlFor="stage-name">阶段名称</Label>
            <Input id="stage-name" value={name} onChange={e => setName(e.target.value)} placeholder="阶段名称" />
          </div>
          <div className="space-y-2">
            <Label htmlFor="stage-sequence">序号</Label>
            <Input
              id="stage-sequence"
              type="number"
              value={sequenceOrder}
              onChange={e => setSequenceOrder(Number(e.target.value))}
            />
          </div>
          <div className="space-y-2">
            <Label htmlFor="stage-input">输入</Label>
            <Input id="stage-input" value={input} onChange={e => setInput(e.target.value)} placeholder="阶段输入" />
          </div>
          <div className="space-y-2">
            <Label htmlFor="stage-output">输出</Label>
            <Input id="stage-output" value={output} onChange={e => setOutput(e.target.value)} placeholder="阶段输出" />
          </div>
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>取消</Button>
          <Button onClick={handleSubmit} disabled={loading || !name}>
            {loading ? <Loader2 className="h-4 w-4 animate-spin" /> : editing ? '保存' : '创建'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}

export function StageDeleteDialog({ stage, onConfirm, spaceId, valueStreamId }: {
  stage: ValueStreamStage | null
  onConfirm: () => void
  spaceId: string
  valueStreamId: string
}) {
  const [deleteMut] = useMutation(DELETE_STAGE)
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    if (!stage) {
      setError(null)
      setLoading(false)
    }
  }, [stage])

  async function handleDelete() {
    if (!stage) return
    setLoading(true)
    setError(null)
    try {
      await deleteMut({
        variables: { id: stage.id },
        refetchQueries: [{ query: GET_VALUE_STREAM_DETAIL, variables: { spaceId, id: valueStreamId } }],
      })
      onConfirm()
    } catch (err) {
      setError(err instanceof Error ? err.message : '删除失败')
    } finally {
      setLoading(false)
    }
  }

  return (
    <ConfirmDialog
      open={!!stage}
      onOpenChange={() => onConfirm()}
      title="确认删除阶段"
      description={`确定要删除阶段「${stage?.name ?? ''}」吗？此操作为物理删除，不可恢复。`}
      confirmText="确认删除"
      destructive
      loading={loading}
      error={error}
      onConfirm={handleDelete}
    />
  )
}