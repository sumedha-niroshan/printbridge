export type PrintType = 'raw' | 'escpos' | 'text';
export interface PrinterInfo {
    name: string;
    isDefault: boolean;
    isOnline: boolean;
}
export interface PrintOptions {
    /** Printer name from listPrinters() */
    printer: string;
    /** Data format */
    type: PrintType;
    /** Base64-encoded bytes for raw/escpos, plain string for text */
    data: string | Uint8Array;
    /** Number of copies (default: 1) */
    copies?: number;
}
export interface PrintBridgeOptions {
    /** WebSocket URL (default: wss://127.0.0.1:8282) */
    url?: string;
    /** Reconnect automatically (default: true) */
    reconnect?: boolean;
    /** Max reconnect attempts (default: 10) */
    maxReconnectAttempts?: number;
    /** Initial reconnect delay ms (default: 1000) */
    reconnectDelay?: number;
}
export interface StatusResult {
    version: string;
    status: string;
}
export type EventType = 'connect' | 'disconnect' | 'error' | 'printers';
export interface PrintResult {
    printed: boolean;
    copies: number;
}
