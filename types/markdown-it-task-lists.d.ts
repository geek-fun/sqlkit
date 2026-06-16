declare module 'markdown-it-task-lists' {
  import type MarkdownIt from 'markdown-it'

  type TaskListsOptions = {
    enabled?: boolean
    label?: boolean
    labelAfter?: boolean
  }
  const taskLists: (md: MarkdownIt, options?: TaskListsOptions) => void
  export default taskLists
}
