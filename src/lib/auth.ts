import { UserManager, WebStorageStateStore } from "oidc-client-ts";

const KEYCLOAK_URL = "https://auth.wyattau.com/realms/company-realm";
const CLIENT_ID = "ferro";
const REDIRECT_URI = `${window.location.origin}/ui/auth/callback`;

const settings = {
  authority: KEYCLOAK_URL,
  client_id: CLIENT_ID,
  redirect_uri: REDIRECT_URI,
  post_logout_redirect_uri: `${window.location.origin}/ui/`,
  response_type: "code",
  scope: "openid profile email",
  userStore: new WebStorageStateStore({ store: localStorage }),
};

export const userManager = new UserManager(settings);

export async function login() { await userManager.signinRedirect(); }
export async function completeLogin() { return userManager.signinCallback(); }
export async function logout() { await userManager.signoutRedirect(); }
export async function getUser() { return userManager.getUser(); }
