import { invoke } from '@tauri-apps/api/core'
import { defineStore } from 'pinia'

export type SshProfile = {
  id: string
  name: string
  host: string
  port: number
  username: string
  /** "password" | "key" | "agent" | "none" | "" (auto-probe) */
  authMethod: string
  password: string
  keyPath: string
  keyPassphrase: string
  useSshAgent: boolean
  sshAgentSockPath: string
  connectTimeoutSecs: number
  keepaliveIntervalSecs: number
  verifyHostKey: boolean
  exposeLan: boolean
}

export function emptySshProfile(): SshProfile {
  return {
    id: '',
    name: '',
    host: '',
    port: 22,
    username: '',
    authMethod: '',
    password: '',
    keyPath: '',
    keyPassphrase: '',
    useSshAgent: false,
    sshAgentSockPath: '',
    connectTimeoutSecs: 10,
    keepaliveIntervalSecs: 30,
    verifyHostKey: false,
    exposeLan: false,
  }
}

export const useSshProfileStore = defineStore('sshProfiles', {
  state: () => ({
    profiles: [] as SshProfile[],
    loaded: false,
  }),
  getters: {
    byId: state => (id: string) => state.profiles.find(p => p.id === id),
  },
  actions: {
    async fetch() {
      this.profiles = await invoke<SshProfile[]>('list_ssh_profiles')
      this.loaded = true
    },
    async save(profile: SshProfile): Promise<SshProfile> {
      const saved = await invoke<SshProfile>('save_ssh_profile', { profile })
      await this.fetch()
      return saved
    },
    async remove(profileId: string): Promise<void> {
      await invoke('delete_ssh_profile', { profileId })
      this.profiles = this.profiles.filter(p => p.id !== profileId)
    },
  },
})

/**
 * Run the test_ssh_connection IPC with a hard client-side ceiling so a hung
 * handshake/IPC can never wedge the dialog: worst case shows a timeout
 * message instead of an eternally-spinning button.
 */
export async function runSshTest(
  config: unknown,
  remoteHost: string,
  remotePort: number,
  timeoutMs = 30_000,
): Promise<{ success: boolean, message: string, user?: string, host?: string, port?: number }> {
  let timer: ReturnType<typeof setTimeout> | undefined
  try {
    return await Promise.race([
      invoke<{
        success: boolean
        message: string
        user?: string
        host?: string
        port?: number
      }>('test_ssh_connection', { config, remoteHost, remotePort }),
      new Promise<never>((_, reject) => {
        timer = setTimeout(() => reject(new Error('ssh-test-timeout')), timeoutMs)
      }),
    ])
  }
  finally {
    if (timer)
      clearTimeout(timer)
  }
}

/**
 * Page-level SshProfile → Rust-contract transport layer (camelCase SshTunnelConfig).
 * Auth strings must match tunnel.rs's match arms; ''/'none' (auto-probe legacy)
 * normalize to password so previously-created profiles keep connecting.
 */
export function profileToTransportLayer(
  profile: SshProfile,
): import('@/datasources/connectionApi').TransportLayerConfig {
  const authMethod
    = profile.authMethod === 'agent'
      ? 'agent' as const
      : profile.authMethod === 'key'
        ? 'key' as const
        : 'password' as const
  return {
    type: 'ssh',
    enabled: true,
    host: profile.host,
    port: profile.port,
    username: profile.username,
    authMethod,
    password: profile.password || '',
    keyPath: profile.keyPath || '',
    keyPassphrase: profile.keyPassphrase || '',
    useSshAgent: authMethod === 'agent',
    sshAgentSockPath: profile.sshAgentSockPath || '',
    useSystemProxy: false,
    connectTimeoutSecs: profile.connectTimeoutSecs ?? 10,
    keepaliveIntervalSecs: profile.keepaliveIntervalSecs ?? 30,
  }
}
