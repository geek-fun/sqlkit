import { deleteQueryFile, listSavedQueries, loadQueryFile, saveQueryFile, saveQueryFileAs } from '@/datasources/fileApi'

jest.mock('@tauri-apps/api/core', () => ({
  invoke: jest.fn(),
}))

jest.mock('@tauri-apps/plugin-dialog', () => ({
  save: jest.fn(),
}))

const invoke = jest.requireMock('@tauri-apps/api/core').invoke as jest.Mock
const showSaveDialog = jest.requireMock('@tauri-apps/plugin-dialog').save as jest.Mock

describe('fileApi', () => {
  beforeEach(() => {
    jest.clearAllMocks()
  })

  describe('saveQueryFile', () => {
    it('saves query file with content', async () => {
      invoke.mockResolvedValue({
        success: true,
        file_path: '/path/to/query.sql',
        message: 'File saved',
      })

      const result = await saveQueryFile('SELECT * FROM users')

      expect(invoke).toHaveBeenCalledWith('save_query_file', {
        content: 'SELECT * FROM users',
        filePath: undefined,
        fileName: undefined,
      })
      expect(result.success).toBe(true)
      expect(result.file_path).toBe('/path/to/query.sql')
    })

    it('saves query file with specified path', async () => {
      invoke.mockResolvedValue({
        success: true,
        file_path: '/custom/path/query.sql',
        message: 'File saved',
      })

      const result = await saveQueryFile('SELECT 1', '/custom/path/query.sql')

      expect(invoke).toHaveBeenCalledWith('save_query_file', {
        content: 'SELECT 1',
        filePath: '/custom/path/query.sql',
        fileName: undefined,
      })
      expect(result.file_path).toBe('/custom/path/query.sql')
    })

    it('saves query file with custom filename', async () => {
      invoke.mockResolvedValue({
        success: true,
        file_path: '/path/myquery.sql',
        message: 'File saved',
      })

      const _result = await saveQueryFile('SELECT 1', undefined, 'myquery')

      expect(invoke).toHaveBeenCalledWith('save_query_file', {
        content: 'SELECT 1',
        filePath: undefined,
        fileName: 'myquery',
      })
    })

    it('returns failure result on error', async () => {
      invoke.mockResolvedValue({
        success: false,
        message: 'Permission denied',
      })

      const result = await saveQueryFile('SELECT 1')

      expect(result.success).toBe(false)
      expect(result.message).toBe('Permission denied')
    })
  })

  describe('saveQueryFileAs', () => {
    it('shows dialog and saves file', async () => {
      showSaveDialog.mockResolvedValue('/user/path/new-query.sql')
      invoke.mockResolvedValue({
        success: true,
        file_path: '/user/path/new-query.sql',
        message: 'Saved',
      })

      const result = await saveQueryFileAs('SELECT 1', 'new-query')

      expect(showSaveDialog).toHaveBeenCalledWith({
        filters: [{ name: 'SQL Files', extensions: ['sql'] }],
        defaultPath: 'new-query.sql',
      })
      expect(result?.success).toBe(true)
    })

    it('adds .sql extension if missing in suggested name', async () => {
      showSaveDialog.mockResolvedValue('/path/query.sql')
      invoke.mockResolvedValue({ success: true, message: 'Saved' })

      await saveQueryFileAs('SELECT 1', 'query')

      expect(showSaveDialog).toHaveBeenCalledWith({
        filters: [{ name: 'SQL Files', extensions: ['sql'] }],
        defaultPath: 'query.sql',
      })
    })

    it('returns null when dialog cancelled', async () => {
      showSaveDialog.mockResolvedValue(null)

      const result = await saveQueryFileAs('SELECT 1')

      expect(result).toBeNull()
      expect(invoke).not.toHaveBeenCalled()
    })

    it('adds .sql extension if missing in selected path', async () => {
      showSaveDialog.mockResolvedValue('/path/myfile')
      invoke.mockResolvedValue({ success: true, message: 'Saved' })

      const _result = await saveQueryFileAs('SELECT 1')

      expect(invoke).toHaveBeenCalledWith('save_query_file', {
        content: 'SELECT 1',
        filePath: '/path/myfile.sql',
      })
    })

    it('does not add .sql extension if already present', async () => {
      showSaveDialog.mockResolvedValue('/path/myfile.sql')
      invoke.mockResolvedValue({ success: true, message: 'Saved' })

      await saveQueryFileAs('SELECT 1')

      expect(invoke).toHaveBeenCalledWith('save_query_file', {
        content: 'SELECT 1',
        filePath: '/path/myfile.sql',
      })
    })

    it('uses default filename when not specified', async () => {
      showSaveDialog.mockResolvedValue('/path/query.sql')
      invoke.mockResolvedValue({ success: true, message: 'Saved' })

      await saveQueryFileAs('SELECT 1')

      expect(showSaveDialog).toHaveBeenCalledWith({
        filters: [{ name: 'SQL Files', extensions: ['sql'] }],
        defaultPath: 'query.sql',
      })
    })
  })

  describe('loadQueryFile', () => {
    it('loads query file content', async () => {
      invoke.mockResolvedValue({
        success: true,
        content: 'SELECT * FROM users',
        message: 'File loaded',
      })

      const result = await loadQueryFile('/path/to/query.sql')

      expect(invoke).toHaveBeenCalledWith('load_query_file', {
        filePath: '/path/to/query.sql',
      })
      expect(result.success).toBe(true)
      expect(result.content).toBe('SELECT * FROM users')
    })

    it('returns failure when file not found', async () => {
      invoke.mockResolvedValue({
        success: false,
        message: 'File not found',
      })

      const result = await loadQueryFile('/missing/file.sql')

      expect(result.success).toBe(false)
      expect(result.message).toBe('File not found')
    })
  })

  describe('listSavedQueries', () => {
    it('returns list of saved query files', async () => {
      invoke.mockResolvedValue(['/path/query1.sql', '/path/query2.sql'])

      const result = await listSavedQueries()

      expect(invoke).toHaveBeenCalledWith('list_saved_queries')
      expect(result).toEqual(['/path/query1.sql', '/path/query2.sql'])
    })

    it('returns empty array when no saved queries', async () => {
      invoke.mockResolvedValue([])

      const result = await listSavedQueries()

      expect(result).toEqual([])
    })
  })

  describe('deleteQueryFile', () => {
    it('deletes query file', async () => {
      invoke.mockResolvedValue('/path/deleted.sql')

      const result = await deleteQueryFile('/path/to/delete.sql')

      expect(invoke).toHaveBeenCalledWith('delete_query_file', {
        filePath: '/path/to/delete.sql',
      })
      expect(result).toBe('/path/deleted.sql')
    })
  })
})
