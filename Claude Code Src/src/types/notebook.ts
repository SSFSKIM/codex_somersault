export type NotebookCellType = 'code' | 'markdown' | 'raw' | string

export type NotebookOutputImage = {
  image_data: string
  media_type: 'image/png' | 'image/jpeg' | string
}

export type NotebookCellSourceOutput = {
  output_type: string
  text?: string
  image?: NotebookOutputImage
  [key: string]: any
}

export type NotebookCellSource = {
  cellType: NotebookCellType
  source: string
  execution_count?: number
  cell_id: string
  language?: string
  outputs?: NotebookCellSourceOutput[]
  [key: string]: any
}

export type NotebookCellOutput = {
  output_type: 'stream' | 'execute_result' | 'display_data' | 'error' | string
  text?: string | string[]
  data?: Record<string, unknown>
  ename?: string
  evalue?: string
  traceback?: string[]
  [key: string]: any
}

export type NotebookCell = {
  id?: string
  cell_type: NotebookCellType
  source: string | string[]
  execution_count?: number | null
  outputs?: NotebookCellOutput[]
  [key: string]: any
}

export type NotebookContent = {
  cells: NotebookCell[]
  metadata: {
    language_info?: {
      name?: string
      [key: string]: any
    }
    [key: string]: any
  }
  [key: string]: any
}
