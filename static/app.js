const healthEl = document.getElementById("health")
const echoTimeEl = document.getElementById("echo-time")
const requestContextIpEl = document.getElementById("request-context-ip")
const requestContextUserAgentEl = document.getElementById("request-context-user-agent")

const loginFormEl = document.getElementById("login-form")
const loginEmailEl = document.getElementById("login-email")
const loginPasswordEl = document.getElementById("login-password")
const loginSubmitEl = document.getElementById("login-submit")
const loginResultEl = document.getElementById("login-result")

const refreshSubmitEl = document.getElementById("refresh-submit")
const refreshCurrentTokenEl = document.getElementById("refresh-current-token")
const refreshResultEl = document.getElementById("refresh-result")

const currentUserSubmitEl = document.getElementById("current-user-submit")
const currentAccessTokenEl = document.getElementById("current-access-token")
const currentUserResultEl = document.getElementById("current-user-result")

const riskFormEl = document.getElementById("risk-form")
const riskEmailEl = document.getElementById("risk-email")
const riskPasswordEl = document.getElementById("risk-password")
const riskAttemptCountEl = document.getElementById("risk-attempt-count")
const riskCurrentIpEl = document.getElementById("risk-current-ip")
const riskSingleSubmitEl = document.getElementById("risk-single-submit")
const riskBatchSubmitEl = document.getElementById("risk-batch-submit")
const riskResultListEl = document.getElementById("risk-result-list")

const sessionAccessTokenEl = document.getElementById("session-access-token")
const sessionAccessTokenExpiresInEl = document.getElementById("session-access-token-expires-in")
const sessionRefreshTokenEl = document.getElementById("session-refresh-token")
const sessionRefreshTokenExpiresInEl = document.getElementById("session-refresh-token-expires-in")
const sessionLastErrorEl = document.getElementById("session-last-error")

const SESSION_STORAGE_KEY = "auth-demo-session"
const MAX_RISK_RESULT_COUNT = 60
const MIN_RISK_ATTEMPT_COUNT = 1
const MAX_RISK_ATTEMPT_COUNT = 30

const sessionState = loadSessionState()

let riskAttemptResults = []
let riskActionRunning = false

function createDefaultSessionState() {
  return {
    access_token: "",
    access_token_expires_in: null,
    refresh_token: "",
    refresh_token_expires_in: null,
    last_error: null,
  }
}

function loadSessionState() {
  try {
    const raw = window.localStorage.getItem(SESSION_STORAGE_KEY)
    if (!raw) {
      return createDefaultSessionState()
    }
    return {
      ...createDefaultSessionState(),
      ...JSON.parse(raw),
    }
  } catch {
    return createDefaultSessionState()
  }
}

function persistSessionState() {
  window.localStorage.setItem(SESSION_STORAGE_KEY, JSON.stringify(sessionState))
}

function formatErrorSummary(error) {
  if (!error || !error.name || !error.message) {
    return "-"
  }
  if (error.name === "rate_limited" && error.data?.retry_after_seconds != null) {
    return `${error.name}: ${error.message} | retry_after_seconds=${error.data.retry_after_seconds}`
  }
  return `${error.name}: ${error.message}`
}

function formatTokenPreview(token) {
  if (!token) {
    return "-"
  }
  if (token.length <= 48) {
    return token
  }
  return `${token.slice(0, 24)}...${token.slice(-16)} | len=${token.length}`
}

function renderSessionState() {
  sessionAccessTokenEl.textContent = sessionState.access_token || "-"
  sessionAccessTokenExpiresInEl.textContent =
    sessionState.access_token_expires_in == null
      ? "-"
      : String(sessionState.access_token_expires_in)
  sessionRefreshTokenEl.textContent = sessionState.refresh_token || "-"
  sessionRefreshTokenExpiresInEl.textContent =
    sessionState.refresh_token_expires_in == null
      ? "-"
      : String(sessionState.refresh_token_expires_in)
  sessionLastErrorEl.textContent = formatErrorSummary(sessionState.last_error)
  refreshCurrentTokenEl.textContent = formatTokenPreview(sessionState.refresh_token)
  currentAccessTokenEl.textContent = formatTokenPreview(sessionState.access_token)
}

function updateSessionTokens(data) {
  sessionState.access_token = data.access_token ?? ""
  sessionState.access_token_expires_in = data.access_token_expires_in ?? null
  sessionState.refresh_token = data.refresh_token ?? ""
  sessionState.refresh_token_expires_in = data.refresh_token_expires_in ?? null
  sessionState.last_error = null
  persistSessionState()
  renderSessionState()
}

function updateLastError(error) {
  sessionState.last_error = error
  persistSessionState()
  renderSessionState()
}

function renderResult(element, value, state = "idle") {
  element.dataset.state = state
  element.textContent =
    typeof value === "string" ? value : JSON.stringify(value, null, 2)
}

function buildSyntheticError(name, message, data = {}) {
  return {
    name,
    message,
    data,
  }
}

function setButtonBusy(button, busy, busyText) {
  if (!button) {
    return
  }
  if (busy) {
    button.dataset.originalText = button.textContent
    button.textContent = busyText
  } else if (button.dataset.originalText) {
    button.textContent = button.dataset.originalText
    delete button.dataset.originalText
  }
  button.disabled = busy
}

function setRiskBusy(busy, mode) {
  riskActionRunning = busy
  if (busy) {
    setButtonBusy(riskSingleSubmitEl, true, mode === "single" ? "执行中..." : "单次错误登录")
    setButtonBusy(riskBatchSubmitEl, true, mode === "batch" ? "批量执行中..." : "连续错误登录")
  } else {
    setButtonBusy(riskSingleSubmitEl, false)
    setButtonBusy(riskBatchSubmitEl, false)
  }
}

function appendRiskAttempt(result) {
  riskAttemptResults = [...riskAttemptResults, result].slice(-MAX_RISK_RESULT_COUNT)
  renderRiskAttempts()
}

function renderRiskAttempts() {
  riskResultListEl.innerHTML = ""

  if (riskAttemptResults.length === 0) {
    const emptyEl = document.createElement("div")
    emptyEl.className = "empty-state"
    emptyEl.textContent = "等待执行暴力破解测试..."
    riskResultListEl.appendChild(emptyEl)
    return
  }

  for (const item of riskAttemptResults) {
    const wrapper = document.createElement("article")
    wrapper.className = "attempt-item"
    wrapper.dataset.state = item.state

    const header = document.createElement("div")
    header.className = "attempt-header"

    const title = document.createElement("div")
    title.className = "attempt-title"
    title.textContent = item.title

    const meta = document.createElement("div")
    meta.className = "attempt-meta"
    meta.textContent = `${item.timeLabel} | HTTP ${item.statusCode}`

    const summary = document.createElement("div")
    summary.className = "attempt-summary"
    summary.textContent = item.summary

    const payload = document.createElement("pre")
    payload.className = "attempt-payload"
    payload.textContent =
      typeof item.payload === "string"
        ? item.payload
        : JSON.stringify(item.payload, null, 2)

    header.appendChild(title)
    header.appendChild(meta)
    wrapper.appendChild(header)
    wrapper.appendChild(summary)
    wrapper.appendChild(payload)
    riskResultListEl.appendChild(wrapper)
  }
}

async function parseJsonResponse(res) {
  const text = await res.text()
  if (!text) {
    return {}
  }
  try {
    return JSON.parse(text)
  } catch {
    return { raw_text: text }
  }
}

async function requestLogin(payload) {
  const res = await fetch("/login/create", {
    method: "POST",
    headers: {
      "content-type": "application/json",
    },
    body: JSON.stringify(payload),
  })
  const data = await parseJsonResponse(res)
  return { res, data }
}

function clampRiskAttemptCount(rawValue) {
  const parsed = Number.parseInt(rawValue, 10)
  if (!Number.isFinite(parsed)) {
    return MIN_RISK_ATTEMPT_COUNT
  }
  return Math.min(MAX_RISK_ATTEMPT_COUNT, Math.max(MIN_RISK_ATTEMPT_COUNT, parsed))
}

function toAttemptState(res, payload) {
  if (res.ok) {
    return "success"
  }
  if (payload?.name === "rate_limited") {
    return "warning"
  }
  return "error"
}

function toAttemptSummary(res, payload) {
  if (res.ok) {
    return "请求成功返回 token, 按测试规则本次结果不会写入当前会话。"
  }
  const summary = formatErrorSummary(payload)
  return summary === "-" ? "请求失败, 但返回结构不符合预期。" : summary
}

function buildRiskAttemptResult(attemptNo, totalCount, res, payload) {
  const suffix = totalCount > 1 ? ` / ${totalCount}` : ""
  return {
    title: `第 ${attemptNo}${suffix} 次尝试`,
    timeLabel: new Date().toLocaleTimeString("zh-CN", { hour12: false }),
    statusCode: res.status,
    state: toAttemptState(res, payload),
    summary: toAttemptSummary(res, payload),
    payload,
  }
}

function buildRiskValidationMessage(message) {
  return {
    title: "参数检查",
    timeLabel: new Date().toLocaleTimeString("zh-CN", { hour12: false }),
    statusCode: 0,
    state: "warning",
    summary: message,
    payload: buildSyntheticError("invalid_params", message),
  }
}

function readRiskPayload() {
  return {
    email: riskEmailEl.value,
    password: riskPasswordEl.value,
    count: clampRiskAttemptCount(riskAttemptCountEl.value),
  }
}

async function fetchHealth() {
  try {
    const res = await fetch("/health/get")
    if (!res.ok) {
      healthEl.textContent = "failed"
      return
    }
    const data = await res.json()
    healthEl.textContent = data.status ?? "unknown"
  } catch {
    healthEl.textContent = "failed"
  }
}

async function fetchEchoTime() {
  try {
    const res = await fetch("/echo/get")
    if (!res.ok) {
      echoTimeEl.textContent = "failed"
      return
    }
    const data = await res.json()
    echoTimeEl.textContent = data.now ?? "unknown"
  } catch {
    echoTimeEl.textContent = "failed"
  }
}

async function fetchRequestContext() {
  try {
    const res = await fetch("/request_context/get")
    if (!res.ok) {
      requestContextIpEl.textContent = "failed"
      requestContextUserAgentEl.textContent = "failed"
      riskCurrentIpEl.textContent = "failed"
      return
    }
    const data = await res.json()
    requestContextIpEl.textContent = data.ip ?? "-"
    requestContextUserAgentEl.textContent = data.user_agent ?? "-"
    riskCurrentIpEl.textContent = data.ip ?? "-"
  } catch {
    requestContextIpEl.textContent = "failed"
    requestContextUserAgentEl.textContent = "failed"
    riskCurrentIpEl.textContent = "failed"
  }
}

async function handleLogin(event) {
  event.preventDefault()

  setButtonBusy(loginSubmitEl, true, "登录中...")

  try {
    const payload = {
      email: loginEmailEl.value,
      password: loginPasswordEl.value,
    }
    const { res, data } = await requestLogin(payload)

    if (!res.ok) {
      updateLastError(data)
      renderResult(loginResultEl, data, data?.name === "rate_limited" ? "warning" : "error")
      return
    }

    updateSessionTokens(data)
    renderResult(loginResultEl, data, "success")
  } catch (error) {
    renderResult(loginResultEl, `登录请求失败: ${error}`, "error")
  } finally {
    loginPasswordEl.value = ""
    setButtonBusy(loginSubmitEl, false)
  }
}

async function handleRefresh() {
  if (!sessionState.refresh_token) {
    const error = buildSyntheticError("invalid_params", "当前无可用 refresh_token, 无法执行刷新")
    updateLastError(error)
    renderResult(refreshResultEl, error, "warning")
    return
  }

  setButtonBusy(refreshSubmitEl, true, "刷新中...")

  try {
    const res = await fetch("/refresh_token/update", {
      method: "PATCH",
      headers: {
        "content-type": "application/json",
      },
      body: JSON.stringify({
        refresh_token: sessionState.refresh_token,
      }),
    })
    const data = await parseJsonResponse(res)

    if (!res.ok) {
      updateLastError(data)
      renderResult(refreshResultEl, data, data?.name === "rate_limited" ? "warning" : "error")
      return
    }

    updateSessionTokens(data)
    renderResult(refreshResultEl, data, "success")
  } catch (error) {
    renderResult(refreshResultEl, `刷新请求失败: ${error}`, "error")
  } finally {
    setButtonBusy(refreshSubmitEl, false)
  }
}

async function handleCurrentUser() {
  if (!sessionState.access_token) {
    const error = buildSyntheticError("invalid_params", "当前无可用 access_token, 无法获取当前用户")
    updateLastError(error)
    renderResult(currentUserResultEl, error, "warning")
    return
  }

  setButtonBusy(currentUserSubmitEl, true, "查询中...")

  try {
    const res = await fetch("/current_user/get", {
      headers: {
        Authorization: `Bearer ${sessionState.access_token}`,
      },
    })
    const data = await parseJsonResponse(res)

    if (!res.ok) {
      updateLastError(data)
      renderResult(currentUserResultEl, data, data?.name === "rate_limited" ? "warning" : "error")
      return
    }

    renderResult(currentUserResultEl, data, "success")
  } catch (error) {
    renderResult(currentUserResultEl, `获取当前用户失败: ${error}`, "error")
  } finally {
    setButtonBusy(currentUserSubmitEl, false)
  }
}

async function executeRiskAttempt(attemptNo, totalCount, payload) {
  try {
    const { res, data } = await requestLogin(payload)
    if (!res.ok) {
      updateLastError(data)
    }
    appendRiskAttempt(buildRiskAttemptResult(attemptNo, totalCount, res, data))
  } catch (error) {
    const payload = buildSyntheticError("network_error", `登录请求失败: ${error}`)
    updateLastError(payload)
    appendRiskAttempt({
      title: `第 ${attemptNo}${totalCount > 1 ? ` / ${totalCount}` : ""} 次尝试`,
      timeLabel: new Date().toLocaleTimeString("zh-CN", { hour12: false }),
      statusCode: 0,
      state: "error",
      summary: formatErrorSummary(payload),
      payload,
    })
  }
}

async function handleRiskSingle() {
  if (riskActionRunning) {
    return
  }

  const payload = readRiskPayload()
  if (!payload.email.trim() || !payload.password.trim()) {
    appendRiskAttempt(buildRiskValidationMessage("请填写测试邮箱和错误密码后再执行测试"))
    return
  }

  setRiskBusy(true, "single")
  try {
    await executeRiskAttempt(1, 1, {
      email: payload.email,
      password: payload.password,
    })
  } finally {
    riskPasswordEl.value = ""
    setRiskBusy(false)
  }
}

async function handleRiskBatch() {
  if (riskActionRunning) {
    return
  }

  const payload = readRiskPayload()
  if (!payload.email.trim() || !payload.password.trim()) {
    appendRiskAttempt(buildRiskValidationMessage("请填写测试邮箱和错误密码后再执行测试"))
    return
  }

  setRiskBusy(true, "batch")
  try {
    for (let attemptNo = 1; attemptNo <= payload.count; attemptNo += 1) {
      await executeRiskAttempt(attemptNo, payload.count, {
        email: payload.email,
        password: payload.password,
      })
    }
  } finally {
    riskPasswordEl.value = ""
    setRiskBusy(false)
  }
}

function initDefaultInputs() {
  if (!loginEmailEl.value) {
    loginEmailEl.value = "test1@brain.im"
  }
  if (!riskEmailEl.value) {
    riskEmailEl.value = "test1@brain.im"
  }
}

loginFormEl.addEventListener("submit", handleLogin)
refreshSubmitEl.addEventListener("click", handleRefresh)
currentUserSubmitEl.addEventListener("click", handleCurrentUser)
riskSingleSubmitEl.addEventListener("click", handleRiskSingle)
riskBatchSubmitEl.addEventListener("click", handleRiskBatch)
riskFormEl.addEventListener("submit", (event) => event.preventDefault())

initDefaultInputs()
renderSessionState()
renderRiskAttempts()
renderResult(loginResultEl, "等待登录...", "idle")
renderResult(refreshResultEl, "等待刷新...", "idle")
renderResult(currentUserResultEl, "等待查询...", "idle")
void fetchHealth()
void fetchEchoTime()
void fetchRequestContext()
