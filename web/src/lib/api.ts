export type AuthMethods = { password: boolean; username: boolean; passkey: boolean };
export type SystemInfo = { version: string; title: string; theme: string; auth: AuthMethods };

export type Slot<T> = { data: T | null; error: string | null; updatedAt: number | null; checkedAt: number | null };

export type Service = {
  name: string;
  url: string;
  group?: string | null;
  description?: string | null;
  icon?: string | null;
};
export type Health = Record<string, { up: boolean; status: number | null; latencyMs: number | null; error: string | null }>;

export type DeviceAddress = { address: string; prefixLen: number | null; interface: string | null; kind: string | null; deprecated: boolean };
export type Device = {
  /** Hostname reported by ipv6who, or the URL's host when it is unreachable. */
  name: string;
  url: string;
  os: "linux" | "macos" | "windows" | string | null;
  /** Readable OS name, e.g. "Fedora Linux 44 (Cloud Edition)". */
  osName: string | null;
  arch: string | null;
  address: string | null;
  interface: string | null;
  addressType: string | null;
  addresses: DeviceAddress[];
  error: string | null;
};

export type Task = {
  id: number;
  title: string;
  due: string | null;
  priority: number;
  project: string | null;
  identifier: string;
  labels: { title: string; hex_color: string }[];
  url: string;
};
export type Todos = { tasks: Task[]; upcomingDays: number };

export type Commit = {
  sha: string;
  title: string;
  repo: string;
  branch: string;
  author: string;
  time: string | null;
  url: string;
  repoUrl: string;
};
export type Forgejo = { commits: Commit[]; heatmap: [number, number][]; profileUrl: string };

export type GithubRepo = {
  name: string;
  description: string | null;
  url: string;
  language: string | null;
  stars: number;
  forks: number;
  pushedAt: string | null;
};
export type Github = {
  user: {
    login: string;
    name: string | null;
    bio: string | null;
    url: string;
    avatar: string | null;
    publicRepos: number;
    followers: number;
    following: number;
  };
  repos: GithubRepo[];
  contributions: { total: number; days: [string, number][] } | null;
};

export type Bookmark = { id: string; title: string; url: string; host: string; tags: string[]; icon: string | null };
export type Bookmarks = { items: Bookmark[]; link: string };

export type HaEntity = {
  id: string;
  name: string;
  state: string | null;
  unit: string | null;
  deviceClass: string | null;
  icon: string | null;
  lastChanged: string | null;
  error: string | null;
};
export type HomeAssistant = { entities: HaEntity[]; link: string };

export type InboxItem = {
  id: string;
  title: string;
  summary: string;
  kind: "text" | "link" | "image" | "file" | "mixed";
  createdAt: string;
  pinned: boolean;
  expiresAt: string | null;
  attachments: number;
  source: string | null;
};
export type InboxStatus = {
  expiresAt: string | null;
  canSend: boolean;
  /** A read-write token from inboxd 0.4.0 on: items can be opened and managed here. */
  canManage?: boolean;
  recent: InboxItem[] | null;
  link: string;
};
export type InboxAttachment = { id: string; name: string; mediaType: string; size: number; preview: boolean };
export type InboxDetail = {
  id: string;
  title: string;
  text: string;
  links: { url: string; title?: string | null }[];
  kind: InboxItem["kind"];
  source: string | null;
  createdAt: string;
  pinned: boolean;
  expiresAt: string | null;
  version: number;
  attachments: InboxAttachment[];
};
export type InboxReceipt = { item_id: string | null; status: string };

export type MailMessage = {
  uid: number;
  subject: string;
  from: string;
  address: string;
  date: string | null;
  unread: boolean;
  messageId: string | null;
};
export type Mail = { messages: MailMessage[]; total: number; unread: number; mailbox: string; link: string | null };

export type DeepseekBalance = { currency: string; totalBalance: string; grantedBalance: string; toppedUpBalance: string };
export type Deepseek = { available: boolean; balances: DeepseekBalance[]; link: string };

export type Widgets = {
  health?: Slot<Health>;
  devices?: Slot<Device[]>;
  vikunja?: Slot<Todos>;
  forgejo?: Slot<Forgejo>;
  github?: Slot<Github>;
  bookmarkd?: Slot<Bookmarks>;
  homeassistant?: Slot<HomeAssistant>;
  inbox?: Slot<InboxStatus>;
  mail?: Slot<Mail>;
  deepseek?: Slot<Deepseek>;
};

export type Dashboard = {
  profile: { name: string; tagline: string; title?: string | null; theme?: string | null };
  services: Service[];
  links: Partial<Record<"vikunja" | "forgejo" | "github" | "bookmarkd" | "homeassistant" | "inbox" | "mail" | "deepseek", string>>;
  widgets: Widgets;
  serverTime: number;
};

const CSRF_KEY = "hp-csrf";
let csrfToken = safeSession(() => sessionStorage.getItem(CSRF_KEY)) ?? "";

function safeSession<T>(read: () => T): T | null {
  try {
    return read();
  } catch {
    return null;
  }
}

function remember(token: string) {
  csrfToken = token;
  safeSession(() => sessionStorage.setItem(CSRF_KEY, token));
}

export class ApiError extends Error {
  constructor(
    public status: number,
    public code: string,
    message: string,
  ) {
    super(message);
  }
}

export async function api<T>(path: string, init: RequestInit = {}): Promise<T> {
  const method = (init.method ?? "GET").toUpperCase();
  const headers = new Headers(init.headers);
  if (init.body && !headers.has("content-type")) headers.set("content-type", "application/json");
  if (!["GET", "HEAD"].includes(method) && csrfToken) headers.set("x-csrf-token", csrfToken);
  const response = await fetch(path, { ...init, headers, credentials: "same-origin" });
  if (!response.ok) {
    const body = (await response.json().catch(() => ({}))) as { error?: string; message?: string };
    // Upstream failures carry a readable Chinese reason after a technical prefix.
    const message = (body.message ?? response.statusText).replace(/^upstream service failed: /, "");
    throw new ApiError(response.status, body.error ?? "request_failed", message);
  }
  if (response.status === 204 || response.status === 202) return undefined as T;
  return response.json() as Promise<T>;
}

export const getSystem = () => api<SystemInfo>("/api/v1/system");
export const getDashboard = () => api<Dashboard>("/api/v1/dashboard");

export async function restoreSession(): Promise<void> {
  const result = await api<{ csrfToken: string }>("/api/v1/auth/session");
  remember(result.csrfToken);
}

export async function login(username: string | null, password: string, rememberMe: boolean): Promise<void> {
  const result = await api<{ csrfToken: string }>("/api/v1/auth/login", {
    method: "POST",
    body: JSON.stringify({ username, password, remember: rememberMe }),
  });
  remember(result.csrfToken);
}

export async function logout(): Promise<void> {
  await api("/api/v1/auth/logout", { method: "POST" }).catch(() => undefined);
  csrfToken = "";
  safeSession(() => sessionStorage.removeItem(CSRF_KEY));
}

export type UpdateSettings = { schedule: "off" | "daily" | "weekly"; weekday: number; time: string; channel: "stable" | "prerelease" };
export type UpdateStatus = {
  current: string;
  platform: string | null;
  installable: boolean;
  settings: UpdateSettings;
  checkedAt: number | null;
  error: string | null;
  available: boolean;
  latest: { version: string; url: string; notes: string; publishedAt: string; prerelease: boolean } | null;
};

export const getUpdate = () => api<UpdateStatus>("/api/v1/update");
export const checkUpdate = () => api<UpdateStatus>("/api/v1/update/check", { method: "POST" });
export const saveUpdateSettings = (settings: UpdateSettings) =>
  api<UpdateStatus>("/api/v1/update/settings", { method: "PATCH", body: JSON.stringify(settings) });
export const installUpdate = (version: string) =>
  api<{ ok: boolean; version: string; restarting: boolean }>("/api/v1/update/install", {
    method: "POST",
    body: JSON.stringify({ version }),
  });

export const sendToInbox = (text: string, title: string | null) =>
  api<InboxReceipt>("/api/v1/inbox", { method: "POST", body: JSON.stringify({ text, title }) });

export const refreshWidget = (key: string) => api<void>(`/api/v1/refresh/${key}`, { method: "POST" });
export const completeTodo = (id: number) => api<void>(`/api/v1/todos/${id}/done`, { method: "POST" });

export type TodoLabel = { id: number; title: string; hex_color: string };
export type TodoOptions = { projects: { id: number; title: string }[]; labels: TodoLabel[]; defaultProject: number | null };
export type NewTodo = { title: string; projectId: number | null; due: string | null; priority: number; labels: number[]; newLabels: string[] };
export const getTodoOptions = () => api<TodoOptions>("/api/v1/todos/options");
export const createTodo = (task: NewTodo) => api<{ id: number }>("/api/v1/todos", { method: "POST", body: JSON.stringify(task) });

const inboxItem = (id: string) => `/api/v1/inbox/items/${encodeURIComponent(id)}`;
export const getInboxItem = (id: string) => api<InboxDetail>(inboxItem(id));
export const updateInboxItem = (id: string, version: number, change: { title?: string; pinned?: boolean }) =>
  api<InboxDetail>(inboxItem(id), { method: "PATCH", body: JSON.stringify({ version, ...change }) });
export const trashInboxItem = (id: string, version: number) =>
  api<{ version: number }>(`${inboxItem(id)}/trash`, { method: "POST", body: JSON.stringify({ version }) });
export const restoreInboxItem = (id: string, version: number) =>
  api<{ version: number }>(`${inboxItem(id)}/restore`, { method: "POST", body: JSON.stringify({ version }) });
export const inboxAttachmentUrl = (id: string, preview = false) =>
  `/api/v1/inbox/attachments/${encodeURIComponent(id)}/${preview ? "preview" : "content"}`;

export class PasskeyCancelled extends Error {}

/** Signs in with a passkey registered in bookmarkd (same RP ID). */
export async function passkeyLogin(rememberMe: boolean): Promise<void> {
  if (!window.PublicKeyCredential || !navigator.credentials) throw new Error("这个浏览器不支持 Passkey。");
  const options = await api<{
    ceremonyId: string;
    publicKey: Record<string, unknown> & { challenge: string; allowCredentials?: { id: string }[] };
  }>("/api/v1/auth/passkey/begin", { method: "POST" });
  const publicKey = {
    ...options.publicKey,
    challenge: fromBase64Url(options.publicKey.challenge),
    allowCredentials: options.publicKey.allowCredentials?.map((item) => ({ ...item, id: fromBase64Url(item.id) })),
  } as PublicKeyCredentialRequestOptions;
  let credential: PublicKeyCredential | null;
  try {
    credential = (await navigator.credentials.get({ publicKey })) as PublicKeyCredential | null;
  } catch (cause) {
    if (cause instanceof DOMException && (cause.name === "NotAllowedError" || cause.name === "AbortError"))
      throw new PasskeyCancelled("已取消 Passkey 登录。");
    throw cause;
  }
  if (!credential) throw new PasskeyCancelled("已取消 Passkey 登录。");
  const response = credential.response as AuthenticatorAssertionResponse;
  const result = await api<{ csrfToken: string }>("/api/v1/auth/passkey/finish", {
    method: "POST",
    body: JSON.stringify({
      ceremonyId: options.ceremonyId,
      remember: rememberMe,
      credential: {
        id: credential.id,
        rawId: toBase64Url(credential.rawId),
        type: credential.type,
        extensions: credential.getClientExtensionResults(),
        response: {
          clientDataJSON: toBase64Url(response.clientDataJSON),
          authenticatorData: toBase64Url(response.authenticatorData),
          signature: toBase64Url(response.signature),
          userHandle: response.userHandle ? toBase64Url(response.userHandle) : null,
        },
      },
    }),
  });
  remember(result.csrfToken);
}

function fromBase64Url(value: string): ArrayBuffer {
  const binary = atob(value.replace(/-/g, "+").replace(/_/g, "/"));
  return Uint8Array.from(binary, (character) => character.charCodeAt(0)).buffer;
}

function toBase64Url(value: ArrayBuffer): string {
  let binary = "";
  for (const byte of new Uint8Array(value)) binary += String.fromCharCode(byte);
  return btoa(binary).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
}
