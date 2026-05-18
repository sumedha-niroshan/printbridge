// src/PrintClient.ts
var DEFAULT_URL = "wss://127.0.0.1:8282";
var DEFAULT_RECONNECT_DELAY = 1e3;
var MAX_RECONNECT_DELAY = 3e4;
var PrintClient = class {
  constructor(opts = {}) {
    this.ws = null;
    this.pending = /* @__PURE__ */ new Map();
    this.listeners = /* @__PURE__ */ new Map();
    this.reconnectAttempts = 0;
    this.reconnectTimer = null;
    this._connected = false;
    this._destroyed = false;
    this.msgCounter = 0;
    this.url = opts.url ?? DEFAULT_URL;
    this.reconnect = opts.reconnect ?? true;
    this.maxReconnectAttempts = opts.maxReconnectAttempts ?? 10;
    this.reconnectDelay = opts.reconnectDelay ?? DEFAULT_RECONNECT_DELAY;
  }
  // ─── Connection ────────────────────────────────────────────────────────────
  connect() {
    return new Promise((resolve, reject) => {
      if (this._connected) {
        resolve();
        return;
      }
      this.ws = new WebSocket(this.url);
      const onOpen = () => {
        this._connected = true;
        this.reconnectAttempts = 0;
        this.reconnectDelay = DEFAULT_RECONNECT_DELAY;
        this.emit("connect");
        resolve();
        this.ws.removeEventListener("error", onError);
      };
      const onError = (e) => {
        reject(new Error("PrintBridge connection failed. Is the service running?"));
        this.ws.removeEventListener("open", onOpen);
      };
      this.ws.addEventListener("open", onOpen, { once: true });
      this.ws.addEventListener("error", onError, { once: true });
      this.ws.addEventListener("message", (e) => this.handleMessage(e.data));
      this.ws.addEventListener("close", () => {
        this._connected = false;
        this.emit("disconnect");
        for (const [, p] of this.pending) {
          p.reject(new Error("WebSocket closed"));
        }
        this.pending.clear();
        if (this.reconnect && !this._destroyed) {
          this.scheduleReconnect();
        }
      });
    });
  }
  disconnect() {
    this._destroyed = true;
    this.reconnect = false;
    if (this.reconnectTimer) clearTimeout(this.reconnectTimer);
    this.ws?.close();
  }
  get connected() {
    return this._connected;
  }
  // ─── API ──────────────────────────────────────────────────────────────────
  async listPrinters() {
    const res = await this.send("listPrinters", {});
    return res.printers;
  }
  async print(opts) {
    const data = opts.data instanceof Uint8Array ? this.uint8ToBase64(opts.data) : opts.data;
    return this.send("print", {
      printer: opts.printer,
      type: opts.type,
      data,
      copies: opts.copies ?? 1
    });
  }
  async status() {
    return this.send("status", {});
  }
  async ping() {
    const res = await this.send("ping", {});
    return res.pong;
  }
  // ─── Events ───────────────────────────────────────────────────────────────
  on(event, cb) {
    if (!this.listeners.has(event)) this.listeners.set(event, /* @__PURE__ */ new Set());
    this.listeners.get(event).add(cb);
    return this;
  }
  off(event, cb) {
    this.listeners.get(event)?.delete(cb);
    return this;
  }
  emit(event, ...args) {
    this.listeners.get(event)?.forEach((cb) => cb(...args));
  }
  // ─── Internals ────────────────────────────────────────────────────────────
  send(action, payload) {
    return new Promise((resolve, reject) => {
      if (!this._connected || !this.ws) {
        reject(new Error("Not connected to PrintBridge"));
        return;
      }
      const id = `pb_${++this.msgCounter}_${Date.now()}`;
      this.pending.set(id, { resolve, reject });
      const msg = JSON.stringify({ action, id, ...payload, payload });
      this.ws.send(msg);
      setTimeout(() => {
        if (this.pending.has(id)) {
          this.pending.delete(id);
          reject(new Error(`Request ${action} timed out`));
        }
      }, 15e3);
    });
  }
  handleMessage(raw) {
    let msg;
    try {
      msg = JSON.parse(raw);
    } catch {
      return;
    }
    const pending = this.pending.get(msg.id);
    if (!pending) return;
    this.pending.delete(msg.id);
    if (msg.success) {
      pending.resolve(msg.data ?? {});
    } else {
      pending.reject(new Error(msg.error ?? "Unknown error"));
    }
  }
  scheduleReconnect() {
    if (this.reconnectAttempts >= this.maxReconnectAttempts) {
      this.emit("error", new Error("Max reconnect attempts reached"));
      return;
    }
    this.reconnectAttempts++;
    const delay = Math.min(this.reconnectDelay * this.reconnectAttempts, MAX_RECONNECT_DELAY);
    this.reconnectTimer = setTimeout(() => {
      this.connect().catch(() => {
      });
    }, delay);
  }
  uint8ToBase64(bytes) {
    let binary = "";
    bytes.forEach((b) => binary += String.fromCharCode(b));
    return btoa(binary);
  }
};
export {
  PrintClient
};
