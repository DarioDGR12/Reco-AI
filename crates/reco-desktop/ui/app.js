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
const refreshBtn = document.getElementById("refresh");
const newChatBtn = document.getElementById("new-chat");
const composerHint = document.getElementById("composer-hint");

const PLACEHOLDER = "Mensaje…  Enter envía · Shift+Enter salto";
const THINKING_PLACEHOLDER = "Reco está pensando…";

let busy = false;
let thinking = false;
let hasModel = false;
let downloading = null;
let catalogLoading = false;

function escapeHtml(value) {
  return String(value)
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}

function showPicker() {
  picker.classList.remove("hidden");
  chat.classList.add("hidden");
}

function showChat() {
  picker.classList.add("hidden");
  chat.classList.remove("hidden");
  if (!thinking) input.focus();
}

function setBusy(on, { wait = false } = {}) {
  busy = on;
  thinking = wait;
  document.body.classList.toggle("is-busy", on);
  form.classList.toggle("is-thinking", wait);
  input.disabled = wait;
  sendBtn.disabled = on;
  refreshBtn.disabled = catalogLoading;
  newChatBtn.disabled = wait;
  input.placeholder = wait ? THINKING_PLACEHOLDER : PLACEHOLDER;
  composerHint.textContent = wait
    ? "pensando…  espera a que Reco termine"
    : "Enter envía · Shift+Enter salto de línea · Esc vuelve al catálogo";
  for (const btn of convos.querySelectorAll("button")) {
    btn.disabled = wait;
  }
}

function emptyChatHtml() {
  return `
    <div class="empty" id="empty">
      <span class="mark" aria-hidden="true">R</span>
      <p class="empty-title">Esta conversación está lista</p>
      <p>Escribe abajo. Reco responde con el modelo que abriste.</p>
    </div>
  `;
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
  if (role === "thinking") {
    body.innerHTML = `<span class="dots" aria-hidden="true"><i></i><i></i><i></i></span><span>pensando…</span>`;
  } else {
    body.textContent = content;
  }
  wrap.append(who, body);
  log.append(wrap);
  log.scrollTop = log.scrollHeight;
  return wrap;
}

function formatScore(total) {
  return `${Math.round(total || 0)}`;
}

function renderSkeletons(count = 6) {
  cards.innerHTML = "";
  for (let i = 0; i < count; i++) {
    const card = document.createElement("div");
    card.className = "card skeleton";
    card.setAttribute("aria-hidden", "true");
    card.innerHTML = `
      <span class="sk sk-title"></span>
      <span class="sk sk-line"></span>
      <span class="sk sk-line short"></span>
      <span class="sk sk-bar"></span>
      <span class="sk sk-pills"></span>
      <div class="sk-btns">
        <span class="sk sk-btn"></span>
        <span class="sk sk-btn ghost"></span>
      </div>
    `;
    cards.append(card);
  }
}

function renderCatalogState(kind, title, detail, actionLabel, onAction) {
  cards.innerHTML = `
    <div class="state ${kind === "error" ? "state-error" : ""}">
      <strong>${escapeHtml(title)}</strong>
      <p class="dim">${escapeHtml(detail)}</p>
      <button type="button" class="primary" data-retry>${escapeHtml(actionLabel)}</button>
    </div>
  `;
  cards.querySelector("[data-retry]").addEventListener("click", onAction);
}

function renderCard(item, { allowDemo = true } = {}) {
  const card = document.createElement("article");
  card.className = "card";
  const params = item.params ? ` · ${escapeHtml(item.params)}` : "";
  const est = item.size_estimated ? " est." : "";
  const score = Math.max(0, Math.min(100, item.total || 0));
  const showScore = Boolean(item.total);
  card.innerHTML = `
    <div class="repo">${escapeHtml(item.repo_id)}</div>
    <div class="meta">${escapeHtml(item.quant)} · ${escapeHtml(item.size)}${est}${params}</div>
    <div class="file">${escapeHtml(item.filename || "")}</div>
    <div class="why">${escapeHtml(item.why || "")}</div>
    ${
      showScore
        ? `<div class="score-row"><div class="score"><i style="width:${Math.max(8, score)}%"></i></div><span class="score-n">${formatScore(item.total)}</span></div>`
        : ""
    }
    <div class="pills">
      ${item.downloaded ? '<span class="pill ok">en disco</span>' : ""}
      ${showScore ? `<span class="pill">score ${formatScore(item.total)}</span>` : ""}
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
    setBusy(true);
    card.classList.add("is-busy");
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
    card.classList.remove("is-busy");
    downloading = null;
    setBusy(false);
  }
}

function applySession(info) {
  hasModel = Boolean(info.has_model);
  document.getElementById("model").textContent = info.repo_id || "Reco";
  document.getElementById("engine").textContent = info.has_model
    ? `${info.filename}  ·  ${info.engine_label}`
    : "Elige un modelo en el catálogo";
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
    log.innerHTML = emptyChatHtml();
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
    btn.disabled = thinking;
    btn.addEventListener("click", async () => {
      if (thinking) return;
      await invoke("open_conversation", { id: item.id });
      await renderConvos();
      await renderHistory();
    });
    convos.append(btn);
  }
}

async function goToCatalog() {
  if (!hasModel && chat.classList.contains("hidden")) return;
  showPicker();
  await renderCatalog(false);
}

async function renderCatalog(refresh = false) {
  if (catalogLoading) return;
  catalogLoading = true;
  refreshBtn.disabled = true;
  catalogMeta.textContent = refresh
    ? "Consultando Hugging Face…"
    : "Buscando modelos que caben en esta máquina…";
  renderSkeletons();
  try {
    const page = await invoke("list_catalog", { refresh });
    const notes = (page.notes || []).join(" · ");
    catalogMeta.textContent = `${page.models.length} modelos · ${page.source}${notes ? " · " + notes : ""}`;
    cards.innerHTML = "";
    if (!page.models.length) {
      renderCatalogState(
        "empty",
        "Ningún GGUF encaja cómodo",
        "Prueba actualizar. Reco vuelve a mirar Hugging Face con el hardware de esta máquina.",
        "Actualizar",
        () => renderCatalog(true)
      );
    } else {
      for (const item of page.models) {
        cards.append(renderCard(item));
      }
    }
  } catch (err) {
    catalogMeta.textContent = "No se pudo cargar el catálogo";
    renderCatalogState(
      "error",
      "No se pudo cargar el catálogo",
      String(err),
      "Reintentar",
      () => renderCatalog(true)
    );
  } finally {
    catalogLoading = false;
    refreshBtn.disabled = false;
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

function autosize() {
  input.style.height = "auto";
  input.style.height = `${Math.min(input.scrollHeight, 160)}px`;
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
  autosize();
  const empty = log.querySelector(".empty");
  if (empty) empty.remove();
  append("user", text);
  const thinkingMsg = append("thinking", "pensando…");
  setBusy(true, { wait: true });
  try {
    const reply = await invoke("send_message", { text });
    thinkingMsg.remove();
    append("assistant", reply);
  } catch (err) {
    thinkingMsg.remove();
    append("assistant", String(err));
  } finally {
    setBusy(false);
    input.focus();
  }
});

input.addEventListener("keydown", (event) => {
  if (event.key === "Enter" && !event.shiftKey) {
    event.preventDefault();
    form.requestSubmit();
  }
});

input.addEventListener("input", autosize);

document.getElementById("new-chat").addEventListener("click", async () => {
  if (thinking) return;
  await invoke("new_conversation");
  await renderConvos();
  await renderHistory();
  input.focus();
});

document.getElementById("back-models").addEventListener("click", () => {
  goToCatalog();
});

document.addEventListener("keydown", (event) => {
  if (event.key !== "Escape") return;
  if (chat.classList.contains("hidden")) return;
  if (!hasModel) return;
  event.preventDefault();
  goToCatalog();
});

document.getElementById("refresh").addEventListener("click", () => {
  renderCatalog(true);
});

boot().catch((err) => {
  showPicker();
  catalogMeta.textContent = "No se pudo abrir Reco";
  renderCatalogState(
    "error",
    "No se pudo abrir Reco",
    String(err),
    "Reintentar",
    () => {
      boot().catch((again) => {
        catalogMeta.textContent = String(again);
      });
    }
  );
});
