const healthEl = document.getElementById("health");
const echoTimeEl = document.getElementById("echo-time");

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

fetchHealth();
fetchEchoTime();
