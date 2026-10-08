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
