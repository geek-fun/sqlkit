import { defineStore } from 'pinia'

type AccountState = {
  sessionRevision: number
  token: string
  /**
   * Device-bound 30-day lease (geekfun#59) — lives with the session so a
   * logout (or a web re-login) invalidates it with the rest.
   */
  refreshToken: string
  username: string
  email: string
  /** Stable console account id — passed through the desktop handoff. */
  userId: string
  /** Console avatar URL — a signed, expiring credential, never a stable link. */
  avatar: string
  /** Signature expiry of `avatar` (unix seconds, 0 = unknown). */
  avatarExpiresAt: number
}

export const useAccountStore = defineStore('account', {
  state: (): AccountState => ({
    sessionRevision: 0,
    token: '',
    refreshToken: '',
    username: '',
    email: '',
    userId: '',
    avatar: '',
    avatarExpiresAt: 0,
  }),
  persist: {
    pick: [
      'token',
      'refreshToken',
      'username',
      'email',
      'userId',
      'avatar',
      'avatarExpiresAt',
    ],
  },
  getters: {
    isLoggedIn: (state): boolean => state.token.length > 0,
  },
  actions: {
    applySessionRefresh(previousRefreshToken: string, accessToken: string, refreshToken: string): void {
      if (!this.isLoggedIn || !previousRefreshToken || this.refreshToken !== previousRefreshToken)
        return
      this.$patch({ token: accessToken, refreshToken })
    },
    setAuth(token: string, username: string, email: string, userId = '', avatar = '', avatarExpiresAt = 0) {
      this.sessionRevision += 1
      this.token = token
      this.username = username
      this.email = email
      this.userId = userId
      this.avatar = avatar
      this.avatarExpiresAt = avatarExpiresAt
      // A web login has no device lease yet — never carry one over.
      this.refreshToken = ''
    },
    setAvatar(avatar: string, avatarExpiresAt = 0) {
      this.avatar = avatar
      this.avatarExpiresAt = avatarExpiresAt
    },
    setToken(token: string) {
      this.token = token
    },
    setRefreshToken(token: string) {
      this.refreshToken = token
    },
    clearAuth() {
      this.sessionRevision += 1
      this.token = ''
      this.refreshToken = ''
      this.username = ''
      this.email = ''
      this.userId = ''
      this.avatar = ''
      this.avatarExpiresAt = 0
    },
  },
})
