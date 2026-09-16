import { defineStore } from 'pinia'

type AccountState = {
  token: string
  /**
   * Device-bound 30-day lease (geekfun#59) — lives with the session so a
   * logout (or a web re-login) invalidates it with the rest.
   */
  refreshToken: string
  username: string
  email: string
}

export const useAccountStore = defineStore('account', {
  state: (): AccountState => ({
    token: '',
    refreshToken: '',
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
      // A web login has no device lease yet — never carry one over.
      this.refreshToken = ''
    },
    setToken(token: string) {
      this.token = token
    },
    setRefreshToken(token: string) {
      this.refreshToken = token
    },
    clearAuth() {
      this.token = ''
      this.refreshToken = ''
      this.username = ''
      this.email = ''
    },
  },
})
