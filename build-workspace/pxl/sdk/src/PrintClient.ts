import type {
  PrintBridgeOptions,
  PrinterInfo,
  PrintOptions,
  PrintResult,
  StatusResult,
  EventType,
} from './types';

type EventCallback = (...args: any[]) => void;

const DEFAULT_URL = 'wss://127.0.0.1:8282';
const DEFAULT_RECONNECT_DELAY = 1000;
const MAX_RECONNECT_DELAY = 30_000;

export class PrintClient {
  private url: string;
  private ws: WebSocket | null = null;
  private pending = new Map<string, { resolve: Function; reject: Function }>();
  private listeners = new Map<EventType, Set<EventCallback>>();
  private reconnect: boolean;
  private maxReconnectAttempts: number;
  private reconnectDelay: number;
  private reconnectAttempts = 0;
  private reconnectTimer: ReturnType<typeof setTimeout> | null = null;
  private _connected = false;
  private _destroyed = false;
  private msgCounter = 0;

  constructor(opts: PrintBridgeOptions = {}) {
    this.url = opts.url ?? DEFAULT_URL;
    this.reconnect = opts.reconnect ?? true;
    this.maxReconnectAttempts = opts.maxReconnectAttempts ?? 10;
    this.reconnectDelay = opts.reconnectDelay ?? DEFAULT_RECONNECT_DELAY;
  }

  // ─── Connection ────────────────────────────────────────────────────────────

  connect(): Promise<void> {
    return new Promise((resolve, reject) => {
      if (this._connected) { resolve(); return; }

      this.ws = new WebSocket(this.url);

      const onOpen = () => {
        this._connected = true;
        this.reconnectAttempts = 0;
        this.reconnectDelay = DEFAULT_RECONNECT_DELAY;
        this.emit('connect');
        resolve();
        this.ws!.removeEventListener('error', onError);
      };

      const onError = (e: Event) => {
        reject(new Error('PrintBridge connection failed. Is the service running?'));
        this.ws!.removeEventListener('open', onOpen);
      };

      this.ws.addEventListener('open', onOpen, { once: true });
      this.ws.addEventListener('error', onError, { once: true });

      this.ws.addEventListener('message', (e) => this.handleMessage(e.data));

      this.ws.addEventListener('close', () => {
        this._connected = false;
        this.emit('disconnect');
        // Reject all pending requests
        for (const [, p] of this.pending) {
          p.reject(new Error('WebSocket closed'));
        }
        this.pending.clear();

        if (this.reconnect && !this._destroyed) {
          this.scheduleReconnect();
        }
      });
    });
  }

  disconnect(): void {
    this._destroyed = true;
    this.reconnect = false;
    if (this.reconnectTimer) clearTimeout(this.reconnectTimer);
    this.ws?.close();
  }

  get connected(): boolean { return this._connected; }

  // ─── API ──────────────────────────────────────────────────────────────────

  async listPrinters(): Promise<PrinterInfo[]> {
    const res = await this.send<{ printers: PrinterInfo[] }>('listPrinters', {});
    return res.printers;
  }

  async print(opts: PrintOptions): Promise<PrintResult> {
    const data = opts.data instanceof Uint8Array
      ? this.uint8ToBase64(opts.data)
      : opts.data;

    return this.send<PrintResult>('print', {
      printer: opts.printer,
      type: opts.type,
      data,
      copies: opts.copies ?? 1,
    });
  }

  async status(): Promise<StatusResult> {
    return this.send<StatusResult>('status', {});
  }

  async ping(): Promise<boolean> {
    const res = await this.send<{ pong: boolean }>('ping', {});
    return res.pong;
  }

  // ─── Events ───────────────────────────────────────────────────────────────

  on(event: EventType, cb: EventCallback): this {
    if (!this.listeners.has(event)) this.listeners.set(event, new Set());
    this.listeners.get(event)!.add(cb);
    return this;
  }

  off(event: EventType, cb: EventCallback): this {
    this.listeners.get(event)?.delete(cb);
    return this;
  }

  private emit(event: EventType, ...args: any[]) {
    this.listeners.get(event)?.forEach(cb => cb(...args));
  }

  // ─── Internals ────────────────────────────────────────────────────────────

  private send<T>(action: string, payload: Record<string, unknown>): Promise<T> {
    return new Promise((resolve, reject) => {
      if (!this._connected || !this.ws) {
        reject(new Error('Not connected to PrintBridge'));
        return;
      }

      const id = `pb_${++this.msgCounter}_${Date.now()}`;
      this.pending.set(id, { resolve, reject });

      const msg = JSON.stringify({ action, id, ...payload, payload });
      this.ws.send(msg);

      // Timeout after 15s
      setTimeout(() => {
        if (this.pending.has(id)) {
          this.pending.delete(id);
          reject(new Error(`Request ${action} timed out`));
        }
      }, 15_000);
    });
  }

  private handleMessage(raw: string) {
    let msg: any;
    try { msg = JSON.parse(raw); } catch { return; }

    const pending = this.pending.get(msg.id);
    if (!pending) return;
    this.pending.delete(msg.id);

    if (msg.success) {
      pending.resolve(msg.data ?? {});
    } else {
      pending.reject(new Error(msg.error ?? 'Unknown error'));
    }
  }

  private scheduleReconnect() {
    if (this.reconnectAttempts >= this.maxReconnectAttempts) {
      this.emit('error', new Error('Max reconnect attempts reached'));
      return;
    }
    this.reconnectAttempts++;
    const delay = Math.min(this.reconnectDelay * this.reconnectAttempts, MAX_RECONNECT_DELAY);

    this.reconnectTimer = setTimeout(() => {
      this.connect().catch(() => {});
    }, delay);
  }

  private uint8ToBase64(bytes: Uint8Array): string {
    let binary = '';
    bytes.forEach(b => binary += String.fromCharCode(b));
    return btoa(binary);
  }
}
