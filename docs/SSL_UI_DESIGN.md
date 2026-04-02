# SSL/TLS Configuration UI Design

## Overview

Database-specific SSL configuration with dropdown selector + conditional fields.

---

## Shared Components

### SSL Mode Dropdown (All Databases)

```
┌─────────────────────────────────────────────────────┐
│ SSL/TLS Mode:  [▼ Disable SSL              ]       │
│                  ├─ Disable SSL                      │
│                  ├─ Prefer SSL (try encryption)     │
│                  ├─ Require SSL (always encrypt)    │
│                  ├─ Verify CA (verify certificate)  │
│                  └─ Verify Full (verify all)        │
└─────────────────────────────────────────────────────┘
```

### Backend Value Mapping

| UI Label | Backend Value | Description |
|----------|---------------|-------------|
| Disable SSL | `disable` | No SSL/TLS encryption |
| Prefer SSL | `prefer` | Try SSL, fallback to plain if unavailable |
| Require SSL | `require` | SSL required, skip cert validation (self-signed OK) |
| Verify CA | `verify-ca` | SSL required, verify server certificate |
| Verify Full | `verify-full` | SSL required, verify certificate + hostname |

---

## PostgreSQL

### UI Layout

```
┌───────────────────────────────────────────────────────────────┐
│ SSL/TLS Mode:  [▼ Require SSL (always encrypt)    ]          │
└───────────────────────────────────────────────────────────────┘

                    ▼ When "Verify CA" or "Verify Full" selected:

┌───────────────────────────────────────────────────────────────┐
│ ┌─ Certificate Settings ─────────────────────────────────────┐│
│ │                                                            ││
│ │  CA Certificate        [________________] [Browse...]     ││
│ │  Client Certificate    [________________] [Browse...]     ││
│ │  Client Private Key    [________________] [Browse...]     ││
│ │                                                            ││
│ └────────────────────────────────────────────────────────────┘│
└───────────────────────────────────────────────────────────────┘
```

### Fields

| Field | Type | Shown When | Required |
|-------|------|------------|----------|
| SSL Mode | Select | Always | Yes |
| CA Certificate | File path | `verify-ca` or `verify-full` | No (uses system trust store if empty) |
| Client Certificate | File path | `verify-ca` or `verify-full` | No (for client auth) |
| Client Private Key | File path | `verify-ca` or `verify-full` | No (for client auth) |

### Backend Properties

```typescript
{
  sslMode: 'disable' | 'prefer' | 'require' | 'verify-ca' | 'verify-full',
  sslCaCert?: string,      // Path to CA certificate
  sslClientCert?: string,  // Path to client certificate
  sslClientKey?: string,   // Path to client private key
}
```

---

## MySQL

### UI Layout

```
┌───────────────────────────────────────────────────────────────┐
│ SSL/TLS Mode:  [▼ Require SSL (always encrypt)    ]          │
└───────────────────────────────────────────────────────────────┘

                    ▼ When "Verify CA" or "Verify Full" selected:

┌───────────────────────────────────────────────────────────────┐
│ ┌─ Certificate Settings ─────────────────────────────────────┐│
│ │                                                            ││
│ │  CA Certificate        [________________] [Browse...]     ││
│ │  Client Certificate    [________________] [Browse...]     ││
│ │  Client Private Key    [________________] [Browse...]     ││
│ │                                                            ││
│ └────────────────────────────────────────────────────────────┘│
└───────────────────────────────────────────────────────────────┘
```

### Fields

Same as PostgreSQL (MySQL 8.0+ supports similar SSL modes)

---

## SQL Server

### UI Layout

```
┌───────────────────────────────────────────────────────────────┐
│ SSL/TLS Mode:  [▼ Require SSL (always encrypt)    ]          │
└───────────────────────────────────────────────────────────────┘

                    ▼ When NOT "Disable SSL" selected:

┌───────────────────────────────────────────────────────────────┐
│ ┌─ SSL Options ──────────────────────────────────────────────┐│
│ │                                                            ││
│ │  ☐ Trust server certificate                               ││
│ │     (Accept self-signed certificates)                      ││
│ │                                                            ││
│ └────────────────────────────────────────────────────────────┘│
└───────────────────────────────────────────────────────────────┘
```

### Fields

| Field | Type | Shown When | Default |
|-------|------|------------|---------|
| SSL Mode | Select | Always | `prefer` |
| Trust server certificate | Checkbox | `prefer`, `require` | unchecked |

### Backend Properties

```typescript
{
  sslMode: 'disable' | 'prefer' | 'require' | 'verify-ca' | 'verify-full',
  trustServerCertificate?: boolean,
}
```

---

## SQLite

### UI Layout

```
(No SSL configuration - SQLite is a local file database)
```

**Note:** SQLite databases are local files, no network encryption needed.

---

## MariaDB

Same as MySQL.

---

## Implementation Plan

### Phase 1: Type Definitions

```typescript
// src/types/connection.ts

export type SslMode = 'disable' | 'prefer' | 'require' | 'verify-ca' | 'verify-full'

export interface SslConfig {
  mode: SslMode
  // PostgreSQL / MySQL / MariaDB
  caCertPath?: string
  clientCertPath?: string
  clientKeyPath?: string
  // SQL Server
  trustServerCertificate?: boolean
}

export interface ConnectionFormData {
  // ... existing fields
  ssl: SslConfig
}
```

### Phase 2: UI Components

```
src/components/connections/
├── ssl/
│   ├── SslModeSelect.vue        # Dropdown selector
│   ├── SslCertFields.vue        # Certificate file inputs (PG, MySQL)
│   ├── SslSqlServerOptions.vue  # Trust server cert checkbox
│   └── SslConfigSection.vue     # Composes based on DB type
```

### Phase 3: Store Updates

- Replace `ssl: boolean` with `ssl: SslConfig`
- Update `connectionStore.ts` to handle new structure
- Update API calls to send `ssl_mode` + additional fields

### Phase 4: Backend Updates

- Update Rust `ConnectionConfig` to accept new SSL fields
- Update database adapters to use certificate paths

---

## UI Behavior Matrix

| Database | Mode Selector | Cert Fields | Additional Options |
|----------|---------------|-------------|-------------------|
| PostgreSQL | ✅ All 5 modes | CA, Client Cert, Client Key | None |
| MySQL | ✅ All 5 modes | CA, Client Cert, Client Key | None |
| MariaDB | ✅ All 5 modes | CA, Client Cert, Client Key | None |
| SQL Server | ✅ All 5 modes | None | Trust server cert |
| SQLite | ❌ Hidden | None | None |

---

## Conditional Field Display Logic

```typescript
const showCertFields = computed(() => {
  return ['verify-ca', 'verify-full'].includes(formData.ssl.mode)
    && ['PostgreSQL', 'MySQL', 'MariaDB'].includes(formData.type)
})

const showSqlServerOptions = computed(() => {
  return formData.ssl.mode !== 'disable'
    && formData.type === 'SQLServer'
})

const showSslSection = computed(() => {
  return formData.type !== 'SQLite'
})
```

---

## Default Values

| Database | Default SSL Mode | Reasoning |
|----------|------------------|-----------|
| PostgreSQL | `prefer` | Works with most servers, tries encryption |
| MySQL | `prefer` | Same as PostgreSQL |
| MariaDB | `prefer` | Same as MySQL |
| SQL Server | `prefer` | Same approach |
| SQLite | (hidden) | Local file, no SSL |

---

## i18n Labels

```json
{
  "ssl.mode.disable": "Disable SSL",
  "ssl.mode.prefer": "Prefer SSL (try encryption)",
  "ssl.mode.require": "Require SSL (always encrypt)",
  "ssl.mode.verifyCa": "Verify CA (verify certificate)",
  "ssl.mode.verifyFull": "Verify Full (verify all)",

  "ssl.caCert": "CA Certificate",
  "ssl.clientCert": "Client Certificate",
  "ssl.clientKey": "Client Private Key",
  "ssl.trustServerCert": "Trust server certificate",
  "ssl.trustServerCertHint": "Accept self-signed certificates"
}
```

---

## Questions for Consideration

1. **File browser**: Use native Tauri file dialog for certificate selection?
2. **Certificate preview**: Show certificate details after selection?
3. **Test connection**: Should test SSL connection during setup?
4. **Migration**: How to migrate existing `ssl: boolean` connections?
