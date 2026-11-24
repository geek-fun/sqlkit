import type { ServerConfig } from '../../src/datasources'
import { expect, test } from './fixtures/app'

const engines = [
  { db_type: 'cassandra', port: 9042 },
  { db_type: 'PostgreSQL', port: 5432 },
  { db_type: 'MySQL', port: 3306 },
]

engines.forEach((engine) => {
  test(`${engine.db_type} suggests and inserts SELECT in the query workspace`, async ({ page, tauri }) => {
    const connection: ServerConfig = {
      id: 'ui-test',
      name: 'Test connection',
      host: 'database.invalid',
      username: 'tester',
      database: 'example',
      ...engine,
    }
    tauri.handle('list_connections', () => [connection])
    tauri.handle('list_ssh_profiles', () => [])
    tauri.handle('connect_server', ({ config }) => {
      expect(config).toMatchObject({ id: connection.id, db_type: connection.db_type })
      return { is_connected: true, current_database: connection.database }
    })
    tauri.handle('list_databases', () => [
      { name: connection.database, is_system: false },
    ])
    tauri.handle('list_schemas', () => [])
    tauri.handle('list_tables', () => [])
    tauri.handle('list_saved_queries', () => [])
    tauri.handle('read_saved_queries_metadata', () => ({ queries: {} }))

    await page.goto('/connections')
    await page.getByRole('heading', { name: connection.name }).dblclick()
    await expect(page).toHaveURL('/queries?connectionId=ui-test')
    await page.getByRole('button', { name: 'New Query', exact: true }).click()

    const editor = page.locator('.sql-editor-container')
    await editor.click()
    await page.keyboard.press('ControlOrMeta+A')
    await page.keyboard.insertText('SELECT id FROM widgets;\n\n')
    await page.keyboard.type('selec')

    const option = page.getByRole('listbox').getByRole('option', { name: /^SELECT\b/ })
    await expect(option).toBeVisible()
    await page.keyboard.press('Escape')
    await expect(option).toBeHidden()
    await page.keyboard.press('ControlOrMeta+Space')
    await expect(option).toBeVisible()
    await page.keyboard.press('Enter')
    await expect(editor.locator('.view-line').last()).toHaveText('SELECT')
  })
})
