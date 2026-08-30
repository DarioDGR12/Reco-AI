const invoke = window.__TAURI__
  ? window.__TAURI__.core.invoke
  : async () => {
      throw new Error("Tauri no está disponible");
    };

const log = document.getElementById("log");
const form = document.getElementById("form");
const input = document.getElementById("input");

function append(role, content) {
  const wrap = document.createElement("div");
  wrap.className = `msg ${role}`;
  const who = document.createElement("div");
  who.className = "who";
  who.textContent = role === "user" ? "tú" : role === "system" ? "sys" : "reco";
  const body = document.createElement("div");
  body.className = "body";
  body.textContent = content;
  wrap.append(who, body);
  log.append(wrap);
  log.scrollTop = log.scrollHeight;
}

async function boot() {
  const info = await invoke("session_info");
  document.getElementById("model").textContent = `${info.repo_id}  ·  ${info.filename}`;
  document.getElementById("engine").textContent = info.engine_label;
  const history = await invoke("load_history");
  log.innerHTML = "";
  for (const msg of history) {
    append(msg.role, msg.content);
  }
  input.focus();
}

form.addEventListener("submit", async (event) => {
  event.preventDefault();
  const text = input.value.trim();
  if (!text) return;
  input.value = "";
  append("user", text);
  try {
    const reply = await invoke("send_message", { text });
    append("assistant", reply);
  } catch (err) {
    append("assistant", String(err));
  }
});

boot().catch((err) => append("assistant", String(err)));
