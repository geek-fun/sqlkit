import { invoke } from '@tauri-apps/api/core'
import { defineStore } from 'pinia'

type AccountState = {
  token: string
  username: string
  email: string
}

export const useAccountStore = defineStore('account', {
  state: (): AccountState => ({
    token: '',
    username: '',
    email: '',
  }),
  persist: true,
  getters: {
    isLoggedIn: (state): boolean => state.token.length > 0,
  },
  actions: {
    setAuth(token: string, username: string, email: string) {
      this.token = token
      this.username = username
      this.email = email
    },
    setToken(token: string) {
      this.token = token
    },
    clearAuth() {
      this.token = ''
      this.username = ''
      this.email = ''
      // the refresh lease must not outlive the account on this machine
      invoke('clear_session').catch(() => {
        // best effort — local-only logout still applies
      })
    },
  },
})
