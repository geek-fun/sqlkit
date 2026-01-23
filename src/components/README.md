# UI Components

This directory contains the shadcn-vue UI components used in SQLKit.

## Components

### Base Components

- **Button** - Customizable button component with multiple variants (default, secondary, outline, ghost, destructive, link)
- **Input** - Text input component with consistent styling
- **Label** - Form label component
- **Card** - Card container with header, title, description, and content sections
- **Dialog** - Modal dialog component for confirmations and forms
- **Table** - Data table components (Table, TableHeader, TableBody, TableRow, TableHead, TableCell)

### Layout Components

Located in `src/components/layout/`:

- **AppLayout** - Main application layout wrapper
- **AppHeader** - Top navigation header with theme toggle
- **AppSidebar** - Left sidebar navigation
- **ThemeToggle** - Dark/light mode toggle button

## Theme

The application uses a custom theme with CSS variables for easy theming. Theme colors are defined in `src/assets/index.css` and support both light and dark modes.

### Using Components

```vue
<script setup lang="ts">
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Input } from '@/components/ui/input'
</script>

<template>
  <Card>
    <CardHeader>
      <CardTitle>Example Card</CardTitle>
    </CardHeader>
    <CardContent>
      <Input placeholder="Enter text..." />
      <Button>Submit</Button>
    </CardContent>
  </Card>
</template>
```

## Theme System

The theme system uses the `useTheme` composable:

```vue
<script setup lang="ts">
import { useTheme } from '@/composables/useTheme'

const { theme: _theme, isDark: _isDark, setTheme: _setTheme, toggleTheme: _toggleTheme } = useTheme()
</script>
```

Theme preferences are persisted in localStorage and respect system preferences.
