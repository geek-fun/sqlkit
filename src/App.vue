<script setup lang="ts">
import { ref } from 'vue'
import AppLayout from './components/layout/AppLayout.vue'
import SQLEditor from './components/SQLEditor.vue'
import { Button } from './components/ui/button'
import { Input } from './components/ui/input'
import { Label } from './components/ui/label'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from './components/ui/card'
import { Dialog, DialogContent, DialogDescription, DialogTitle, DialogTrigger } from './components/ui/dialog'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from './components/ui/table'

const name = ref('')
const dialogOpen = ref(false)
const sqlQuery = ref(`-- Sample SQL Query
SELECT 
  u.id,
  u.name,
  u.email,
  COUNT(o.id) as order_count
FROM users u
LEFT JOIN orders o ON u.id = o.user_id
WHERE u.created_at > '2024-01-01'
GROUP BY u.id, u.name, u.email
ORDER BY order_count DESC
LIMIT 10;`)

const sampleData = [
  { id: 1, database: 'PostgreSQL', status: 'Connected', host: 'localhost:5432' },
  { id: 2, database: 'MySQL', status: 'Disconnected', host: 'localhost:3306' },
  { id: 3, database: 'SQLite', status: 'Connected', host: 'local' },
]

const handleExecuteQuery = (query: string) => {
  console.log('Executing query:', query)
  // TODO: Implement query execution logic
}
</script>

<template>
  <AppLayout>
    <div class="space-y-6">
      <div>
        <h1 class="text-3xl font-bold tracking-tight">Welcome to SQLKit</h1>
        <p class="text-muted-foreground mt-2">
          AI-powered cross-platform SQL database GUI client
        </p>
      </div>

      <!-- Components Demo Section -->
      <div class="grid gap-6 md:grid-cols-2">
        <!-- Form Card -->
        <Card>
          <CardHeader>
            <CardTitle>Quick Connection</CardTitle>
            <CardDescription>Connect to your database quickly</CardDescription>
          </CardHeader>
          <CardContent class="space-y-4">
            <div class="space-y-2">
              <Label for="name">Database Name</Label>
              <Input id="name" v-model="name" placeholder="Enter database name..." />
            </div>
            <div class="flex gap-2">
              <Button>Connect</Button>
              <Button variant="outline">Cancel</Button>
            </div>
          </CardContent>
        </Card>

        <!-- Actions Card -->
        <Card>
          <CardHeader>
            <CardTitle>Component Showcase</CardTitle>
            <CardDescription>Try out different UI components</CardDescription>
          </CardHeader>
          <CardContent class="space-y-4">
            <div class="flex flex-wrap gap-2">
              <Button variant="default">Default</Button>
              <Button variant="secondary">Secondary</Button>
              <Button variant="outline">Outline</Button>
              <Button variant="ghost">Ghost</Button>
              <Button variant="destructive">Destructive</Button>
            </div>
            <Dialog v-model:open="dialogOpen">
              <DialogTrigger as-child>
                <Button variant="outline">Open Dialog</Button>
              </DialogTrigger>
              <DialogContent>
                <DialogTitle>Dialog Example</DialogTitle>
                <DialogDescription>
                  This is a sample dialog component. You can use it for modals, confirmations, and more.
                </DialogDescription>
                <div class="flex justify-end gap-2 mt-4">
                  <Button variant="outline" @click="dialogOpen = false">Cancel</Button>
                  <Button @click="dialogOpen = false">Confirm</Button>
                </div>
              </DialogContent>
            </Dialog>
          </CardContent>
        </Card>
      </div>

      <!-- Table Demo -->
      <Card>
        <CardHeader>
          <CardTitle>Database Connections</CardTitle>
          <CardDescription>Manage your database connections</CardDescription>
        </CardHeader>
        <CardContent>
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>ID</TableHead>
                <TableHead>Database</TableHead>
                <TableHead>Status</TableHead>
                <TableHead>Host</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              <TableRow v-for="item in sampleData" :key="item.id">
                <TableCell>{{ item.id }}</TableCell>
                <TableCell class="font-medium">{{ item.database }}</TableCell>
                <TableCell>
                  <span
                    :class="[
                      'inline-flex items-center rounded-full px-2 py-1 text-xs font-medium',
                      item.status === 'Connected'
                        ? 'bg-green-100 text-green-700 dark:bg-green-900 dark:text-green-300'
                        : 'bg-red-100 text-red-700 dark:bg-red-900 dark:text-red-300',
                    ]"
                  >
                    {{ item.status }}
                  </span>
                </TableCell>
                <TableCell>{{ item.host }}</TableCell>
              </TableRow>
            </TableBody>
          </Table>
        </CardContent>
      </Card>

      <!-- SQL Editor Demo -->
      <Card>
        <CardHeader>
          <CardTitle>SQL Editor</CardTitle>
          <CardDescription>
            Monaco Editor with syntax highlighting, auto-completion, and keyboard shortcuts
          </CardDescription>
        </CardHeader>
        <CardContent class="space-y-4">
          <SQLEditor
            v-model="sqlQuery"
            :height="'300px'"
            dialect="sql"
            @execute="handleExecuteQuery"
          />
          <div class="flex gap-2">
            <Button @click="handleExecuteQuery(sqlQuery)">
              Execute Query (Ctrl+Enter)
            </Button>
            <Button variant="outline" @click="sqlQuery = ''">
              Clear
            </Button>
          </div>
        </CardContent>
      </Card>
    </div>
  </AppLayout>
</template>
