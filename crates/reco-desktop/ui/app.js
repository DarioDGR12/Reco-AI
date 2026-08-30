const invoke = window.__TAURI__
  ? window.__TAURI__.core.invoke
  : async () => {
      throw new Error("Tauri no está disponible");
    };

const log = document.getElementById("log");
const empty = document.getElementById("empty");
const form = document.getElementById("form");
const input = document.getElementById("input");
const convos = document.getElementById("convos");

function hideEmpty() {
  if (empty) empty.remove();
}

function append(role, content) {
  hideEmpty();
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

async function renderHistory() {
  const history = await invoke("load_history");
  log.innerHTML = "";
  if (!history.length) {
    log.innerHTML =
      '<div class="empty">Escribe algo para empezar esta conversación.</div>';
    return;
  }
  for (const msg of history) {
    append(msg.role, msg.content);
  }
}

async function renderConvos() {
  const list = await invoke("list_conversations");
  convos.innerHTML = "";
  for (const item of list) {
    const btn = document.createElement("button");
    btn.type = "button";
    btn.className = "convo" + (item.active ? " active" : "");
    btn.textContent = item.title;
    btn.addEventListener("click", async () => {
      await invoke("open_conversation", { id: item.id });
      await renderConvos();
      await renderHistory();
    });
    convos.append(btn);
  }
}

async function boot() {
  const info = await invoke("session_info");
  document.getElementById("model").textContent = info.repo_id;
  document.getElementById("engine").textContent = `${info.filename}  ·  ${info.engine_label}`;
  await renderConvos();
  await renderHistory();
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

input.addEventListener("keydown", (event) => {
  if (event.key === "Enter" && !event.shiftKey) {
    event.preventDefault();
    form.requestSubmit();
  }
});

document.getElementById("new-chat").addEventListener("click", async () => {
  await invoke("new_conversation");
  await renderConvos();
  await renderHistory();
  input.focus();
});

boot().catch((err) => append("assistant", String(err)));
