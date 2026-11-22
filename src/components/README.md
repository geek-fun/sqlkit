# UI Components

This directory contains the shadcn-vue UI components used in SQLKit.

## Design Specification

Read [SQLKit Design Specification](../../DESIGN.md) before adding or changing components.
It defines color ownership, compact desktop layouts, component states, accessibility,
and the migration plan for existing styling inconsistencies. Preserve the current blue
theme; keep secondary actions and ordinary badges neutral, and reserve status colors
for actual state. CSS tokens remain the runtime source of truth.

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
- **AppHeader** - Top navigation header
- **AppSidebar** - Left sidebar navigation

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

The theme system is managed by the Pinia `appStore` (`src/store/appStore.ts`):

- `themeType` (`auto` | `dark` | `light`) is the user preference, persisted via pinia-plugin-persistedstate.
- `uiThemeType` (`dark` | `light`) is the resolved effective theme.
- `applyUiTheme` toggles the `theme` attribute and `.dark` class on `<html>`.
- Monaco editor themes are derived from `appStore.uiThemeType` in `useMonacoEditor`.
