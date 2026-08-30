const invoke = window.__TAURI__
  ? window.__TAURI__.core.invoke
  : async () => {
      throw new Error("Tauri no está disponible");
    };

const listen = window.__TAURI__?.event?.listen
  ? (name, fn) => window.__TAURI__.event.listen(name, fn)
  : async () => () => {};

const picker = document.getElementById("picker");
const chat = document.getElementById("chat");
const log = document.getElementById("log");
const form = document.getElementById("form");
const input = document.getElementById("input");
const sendBtn = document.getElementById("send");
const convos = document.getElementById("convos");
const cards = document.getElementById("cards");
const local = document.getElementById("local");
const localWrap = document.getElementById("local-wrap");
const catalogMeta = document.getElementById("catalog-meta");
const demoBadge = document.getElementById("demo-badge");

let busy = false;
let downloading = null;

function showPicker() {
  picker.classList.remove("hidden");
  chat.classList.add("hidden");
}

function showChat() {
  picker.classList.add("hidden");
  chat.classList.remove("hidden");
  input.focus();
}

function append(role, content) {
  const wrap = document.createElement("div");
  wrap.className = `msg ${role}`;
  const who = document.createElement("div");
  who.className = "who";
  who.textContent =
    role === "user" ? "tú" : role === "thinking" ? "reco" : role === "system" ? "sys" : "reco";
  const body = document.createElement("div");
  body.className = "body";
  body.textContent = content;
  wrap.append(who, body);
  log.append(wrap);
  log.scrollTop = log.scrollHeight;
  return wrap;
}

function formatScore(total) {
  return `${Math.round((total || 0) * 100)}`;
}

function renderCard(item, { allowDemo = true } = {}) {
  const card = document.createElement("article");
  card.className = "card";
  const params = item.params ? ` · ${item.params}` : "";
  const est = item.size_estimated ? " est." : "";
  card.innerHTML = `
    <div class="repo">${item.repo_id}</div>
    <div class="meta">${item.quant} · ${item.size}${est}${params}</div>
    <div class="why">${item.why || ""}</div>
    <div class="score"><i style="width:${Math.max(8, (item.total || 0) * 100)}%"></i></div>
    <div class="pills">
      ${item.downloaded ? '<span class="pill ok">en disco</span>' : ""}
      ${item.total ? `<span class="pill">score ${formatScore(item.total)}</span>` : ""}
    </div>
    <div class="card-actions">
      <button type="button" class="primary" data-act="open">
        ${item.downloaded ? "Abrir chat" : "Descargar y abrir"}
      </button>
      ${allowDemo && !item.downloaded ? '<button type="button" class="ghost" data-act="demo">Probar demo</button>' : ""}
    </div>
    <div class="progress hidden" data-progress></div>
  `;
  card.querySelector('[data-act="open"]').addEventListener("click", (event) => {
    event.stopPropagation();
    openModel(item, false, card);
  });
  const demoBtn = card.querySelector('[data-act="demo"]');
  if (demoBtn) {
    demoBtn.addEventListener("click", (event) => {
      event.stopPropagation();
      openModel(item, true, card);
    });
  }
  return card;
}

async function openModel(item, demo, card) {
  if (busy) return;
  const progress = card.querySelector("[data-progress]");
  try {
    busy = true;
    if (!demo && !item.downloaded) {
      downloading = item.filename;
      progress.classList.remove("hidden");
      progress.textContent = "Descargando…";
      await invoke("download_model", {
        repoId: item.repo_id,
        filename: item.filename,
      });
      item.downloaded = true;
    }
    const info = await invoke("select_model", {
      repoId: item.repo_id,
      filename: item.filename,
      demo,
    });
    applySession(info);
    await renderConvos();
    await renderHistory();
    showChat();
  } catch (err) {
    progress.classList.remove("hidden");
    progress.textContent = String(err);
  } finally {
    busy = false;
    downloading = null;
  }
}

function applySession(info) {
  document.getElementById("model").textContent = info.repo_id || "Reco AI";
  document.getElementById("engine").textContent = info.has_model
    ? `${info.filename}  ·  ${info.engine_label}`
    : "";
  demoBadge.classList.toggle("hidden", !info.demo);
  document.getElementById("hw-cpu").textContent = info.hardware.cpu;
  document.getElementById("hw-ram").textContent = `${info.hardware.ram} RAM`;
  document.getElementById("hw-gpu").textContent =
    `${info.hardware.gpu} · ${info.hardware.backend}`;
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

async function renderCatalog(refresh = false) {
  catalogMeta.textContent = refresh
    ? "Actualizando Hugging Face…"
    : "Cargando catálogo GGUF…";
  cards.innerHTML = "";
  try {
    const page = await invoke("list_catalog", { refresh });
    const notes = (page.notes || []).join(" · ");
    catalogMeta.textContent = `${page.models.length} modelos · ${page.source}${notes ? " · " + notes : ""}`;
    if (!page.models.length) {
      cards.innerHTML =
        '<p class="dim">No hay GGUF que entren cómodos. Prueba actualizar.</p>';
    }
    for (const item of page.models) {
      cards.append(renderCard(item));
    }
  } catch (err) {
    catalogMeta.textContent = String(err);
  }

  try {
    const disk = await invoke("list_local_models");
    local.innerHTML = "";
    if (!disk.length) {
      localWrap.classList.add("hidden");
      return;
    }
    localWrap.classList.remove("hidden");
    for (const item of disk) {
      local.append(renderCard(item, { allowDemo: false }));
    }
  } catch {
    localWrap.classList.add("hidden");
  }
}

async function boot() {
  await listen("download-progress", (event) => {
    const payload = event.payload || {};
    if (!downloading) return;
    const label = document.querySelector("[data-progress]:not(.hidden)");
    if (!label) return;
    if (payload.done) {
      label.textContent = "Listo";
      return;
    }
    const written = payload.written || 0;
    const total = payload.total;
    if (total) {
      label.textContent = `Descargando ${(written / total * 100).toFixed(0)}%`;
    } else {
      label.textContent = `Descargando ${(written / 1024 / 1024).toFixed(1)} MiB`;
    }
  });

  const info = await invoke("session_info");
  applySession(info);
  if (info.has_model) {
    await renderConvos();
    await renderHistory();
    showChat();
  } else {
    showPicker();
    await renderCatalog(false);
  }
}

form.addEventListener("submit", async (event) => {
  event.preventDefault();
  const text = input.value.trim();
  if (!text || busy) return;
  input.value = "";
  const empty = log.querySelector(".empty");
  if (empty) empty.remove();
  append("user", text);
  const thinking = append("thinking", "pensando…");
  sendBtn.disabled = true;
  busy = true;
  try {
    const reply = await invoke("send_message", { text });
    thinking.remove();
    append("assistant", reply);
  } catch (err) {
    thinking.remove();
    append("assistant", String(err));
  } finally {
    busy = false;
    sendBtn.disabled = false;
    input.focus();
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

document.getElementById("back-models").addEventListener("click", async () => {
  showPicker();
  await renderCatalog(false);
});

document.getElementById("refresh").addEventListener("click", () => {
  renderCatalog(true);
});

boot().catch((err) => {
  catalogMeta.textContent = String(err);
  append("assistant", String(err));
});
