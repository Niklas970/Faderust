// Settings panel shared by all Faderust actions. Each <select> or <input>
// with data-setting is bound to that action setting; a select's
// data-source names the list the plugin sends (see src/actions/base.ts).
let socket;
let context;
let actionUuid;
let settings = {};
let options = null;

const fields = () => document.querySelectorAll("[data-setting]");

function send(event, extra) {
  socket?.send(JSON.stringify({ event, context, ...extra }));
}

function save() {
  send("setSettings", { payload: settings });
}

function value(el) {
  if (el.type === "checkbox") return el.checked;
  if (el.dataset.type === "number") {
    const n = parseFloat(el.value);
    return Number.isFinite(n) ? n : undefined;
  }
  return el.value;
}

function fill(select) {
  const list = options?.[select.dataset.source] ?? [];
  const current = settings[select.dataset.setting];
  select.replaceChildren();
  const add = (v, label) => select.append(new Option(label, String(v)));
  for (const extra of select.dataset.extra?.split(";") ?? []) {
    const [v, label] = extra.split("|");
    add(v, label);
  }
  for (const o of list) add(o.value, o.label);
  if (current !== undefined && ![...select.options].some((o) => o.value === String(current))) {
    add(current, "(nicht gefunden)");
  }
  if (current !== undefined) {
    select.value = String(current);
  } else if (select.options.length > 0) {
    // Store what the panel shows, so the key does what it says.
    select.selectedIndex = 0;
    settings[select.dataset.setting] = value(select);
    save();
  }
}

// A group of checkboxes (data-multi names the list) stored as an array of numbers.
function fillMulti(box) {
  const list = options?.[box.dataset.multi] ?? [];
  const chosen = new Set(settings[box.dataset.setting] ?? []);
  box.replaceChildren();
  for (const o of list) {
    const label = document.createElement("label");
    label.className = "check";
    const input = document.createElement("input");
    input.type = "checkbox";
    input.value = String(o.value);
    input.checked = chosen.has(o.value);
    label.append(input, o.label);
    box.append(label);
  }
}

function render() {
  document.getElementById("status").hidden = !options || options.connected;
  for (const el of fields()) {
    if (el.dataset.multi) {
      if (options) fillMulti(el);
    } else if (el.tagName === "SELECT") {
      if (options) fill(el);
      if (options) fill(el);
    } else if (el.type === "checkbox") {
      el.checked = !!settings[el.dataset.setting];
    } else {
      el.value = settings[el.dataset.setting] ?? "";
    }
  }
}

window.connectElgatoStreamDeckSocket = (port, uuid, registerEvent, _info, actionInfo) => {
  context = uuid;
  const info = JSON.parse(actionInfo);
  actionUuid = info.action;
  settings = info.payload?.settings ?? {};
  render();

  socket = new WebSocket(`ws://127.0.0.1:${port}`);
  socket.onopen = () => {
    socket.send(JSON.stringify({ event: registerEvent, uuid }));
    send("sendToPlugin", { action: actionUuid, payload: { event: "getOptions" } });
  };
  socket.onmessage = (msg) => {
    const data = JSON.parse(msg.data);
    if (data.event === "sendToPropertyInspector" && data.payload?.event === "options") {
      options = data.payload;
      render();
    } else if (data.event === "didReceiveSettings") {
      settings = data.payload?.settings ?? {};
      render();
    }
  };
};

document.addEventListener("change", (e) => {
  const box = e.target.closest("[data-multi]");
  if (box) {
    const picked = [...box.querySelectorAll("input:checked")].map((i) => Number(i.value));
    if (picked.length) settings[box.dataset.setting] = picked;
    else delete settings[box.dataset.setting];
    save();
    return;
  }
  const el = e.target.closest("[data-setting]");
  if (!el) return;
  const v = value(el);
  if (v === undefined) delete settings[el.dataset.setting];
  else settings[el.dataset.setting] = v;
  save();
});
