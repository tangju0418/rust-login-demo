const healthEl = document.getElementById("health");
const echoTimeEl = document.getElementById("echo-time");
const loginFormEl = document.getElementById("login-form");
const loginEmailEl = document.getElementById("login-email");
const loginPasswordEl = document.getElementById("login-password");
const loginResultEl = document.getElementById("login-result");
const refreshResultEl = document.getElementById("refresh-result");
const currentUserResultEl = document.getElementById("current-user-result");
const refreshSubmitEl = document.getElementById("refresh-submit");
const currentUserSubmitEl = document.getElementById("current-user-submit");
const sessionAccessTokenEl = document.getElementById("session-access-token");
const sessionAccessTokenExpiresInEl = document.getElementById("session-access-token-expires-in");
const sessionRefreshTokenEl = document.getElementById("session-refresh-token");
const sessionRefreshTokenExpiresInEl = document.getElementById("session-refresh-token-expires-in");
const sessionLastErrorEl = document.getElementById("session-last-error");

const SESSION_STORAGE_KEY = "auth-demo-session";

const sessionState = loadSessionState();

async function fetchHealth() {
  const res = await fetch("/health/get");
  if (!res.ok) {
    healthEl.textContent = "health: failed";
    return;
  }
  const data = await res.json();
  healthEl.textContent = `health: ${data.status ?? "unknown"}`;
}

async function fetchEchoTime() {
  const res = await fetch("/echo/get");
  if (!res.ok) {
    echoTimeEl.textContent = "echo time: failed";
    return;
  }
  const data = await res.json();
  echoTimeEl.textContent = `echo time: ${data.now ?? "unknown"}`;
}

function loadSessionState() {
  try {
    const raw = window.localStorage.getItem(SESSION_STORAGE_KEY);
    if (!raw) {
      return createDefaultSessionState();
    }
    return {
      ...createDefaultSessionState(),
      ...JSON.parse(raw),
    };
  } catch {
    return createDefaultSessionState();
  }
}

function createDefaultSessionState() {
  return {
    access_token: "",
    access_token_expires_in: null,
    refresh_token: "",
    refresh_token_expires_in: null,
    last_error: null,
  };
}

function persistSessionState() {
  window.localStorage.setItem(SESSION_STORAGE_KEY, JSON.stringify(sessionState));
}

function renderSessionState() {
  sessionAccessTokenEl.textContent = sessionState.access_token || "-";
  sessionAccessTokenExpiresInEl.textContent =
    sessionState.access_token_expires_in == null
      ? "-"
      : String(sessionState.access_token_expires_in);
  sessionRefreshTokenEl.textContent = sessionState.refresh_token || "-";
  sessionRefreshTokenExpiresInEl.textContent =
    sessionState.refresh_token_expires_in == null
      ? "-"
      : String(sessionState.refresh_token_expires_in);
  sessionLastErrorEl.textContent = formatErrorSummary(sessionState.last_error);
}

function updateSessionTokens(data) {
  sessionState.access_token = data.access_token ?? "";
  sessionState.access_token_expires_in = data.access_token_expires_in ?? null;
  sessionState.refresh_token = data.refresh_token ?? "";
  sessionState.refresh_token_expires_in = data.refresh_token_expires_in ?? null;
  sessionState.last_error = null;
  persistSessionState();
  renderSessionState();
}

function updateLastError(error) {
  sessionState.last_error = error;
  persistSessionState();
  renderSessionState();
}

function renderResult(element, value) {
  element.textContent =
    typeof value === "string" ? value : JSON.stringify(value, null, 2);
}

function formatErrorSummary(error) {
  if (!error || !error.name || !error.message) {
    return "-";
  }
  if (error.name === "rate_limited" && error.data?.retry_after_seconds != null) {
    return `${error.name}: ${error.message} | retry_after_seconds=${error.data.retry_after_seconds}`;
  }
  return `${error.name}: ${error.message}`;
}

async function parseJsonResponse(res) {
  const text = await res.text();
  if (!text) {
    return {};
  }
  return JSON.parse(text);
}

async function handleLogin(event) {
  event.preventDefault();

  const payload = {
    email: loginEmailEl.value,
    password: loginPasswordEl.value,
  };

  try {
    const res = await fetch("/login/create", {
      method: "POST",
      headers: {
        "content-type": "application/json",
      },
      body: JSON.stringify(payload),
    });
    const data = await parseJsonResponse(res);

    if (!res.ok) {
      updateLastError(data);
      renderResult(loginResultEl, data);
      return;
    }

    updateSessionTokens(data);
    renderResult(loginResultEl, data);
  } catch (error) {
    renderResult(loginResultEl, `登录请求失败: ${error}`);
  } finally {
    loginPasswordEl.value = "";
  }
}

async function handleRefresh() {
  if (!sessionState.refresh_token) {
    renderResult(refreshResultEl, "当前无可用 refresh_token, 无法执行刷新");
    return;
  }

  try {
    const res = await fetch("/refresh_token/update", {
      method: "PATCH",
      headers: {
        "content-type": "application/json",
      },
      body: JSON.stringify({
        refresh_token: sessionState.refresh_token,
      }),
    });
    const data = await parseJsonResponse(res);

    if (!res.ok) {
      updateLastError(data);
      renderResult(refreshResultEl, data);
      return;
    }

    updateSessionTokens(data);
    renderResult(refreshResultEl, data);
  } catch (error) {
    renderResult(refreshResultEl, `刷新请求失败: ${error}`);
  }
}

async function handleCurrentUser() {
  if (!sessionState.access_token) {
    renderResult(currentUserResultEl, "当前无可用 access_token, 无法获取当前用户");
    return;
  }

  try {
    const res = await fetch("/current_user/get", {
      headers: {
        Authorization: `Bearer ${sessionState.access_token}`,
      },
    });
    const data = await parseJsonResponse(res);

    if (!res.ok) {
      updateLastError(data);
      renderResult(currentUserResultEl, data);
      return;
    }

    renderResult(currentUserResultEl, data);
  } catch (error) {
    renderResult(currentUserResultEl, `获取当前用户失败: ${error}`);
  }
}

loginFormEl.addEventListener("submit", handleLogin);
refreshSubmitEl.addEventListener("click", handleRefresh);
currentUserSubmitEl.addEventListener("click", handleCurrentUser);

renderSessionState();
fetchHealth();
fetchEchoTime();
