import { defineStore } from "pinia";

type AccountState = {
  token: string;
  username: string;
  email: string;
};

export const useAccountStore = defineStore("account", {
  state: (): AccountState => ({
    token: "",
    username: "",
    email: "",
  }),
  persist: true,
  getters: {
    isLoggedIn: (state): boolean => state.token.length > 0,
  },
  actions: {
    setAuth(token: string, username: string, email: string) {
      this.token = token;
      this.username = username;
      this.email = email;
    },
    clearAuth() {
      this.token = "";
      this.username = "";
      this.email = "";
    },
  },
});
