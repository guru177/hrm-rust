import { useEffect, useRef, useState } from 'react';

export type BiometricLiveEvent = {
    type: string;
    ts?: string;
    serial_number?: string;
    ip_address?: string;
    last_heartbeat?: string;
    count?: number;
    message?: string;
};

type Options = {
    enabled?: boolean;
    onEvent: (event: BiometricLiveEvent) => void;
};

/**
 * WebSocket live feed from HRM backend when device syncs (heartbeat / punches).
 * Auto-reconnects — keeps the admin UI in sync without manual refresh.
 */
export function useBiometricLive({ enabled = true, onEvent }: Options) {
    const [connected, setConnected] = useState(false);
    const onEventRef = useRef(onEvent);
    onEventRef.current = onEvent;

    useEffect(() => {
        if (!enabled) return;

        const token = localStorage.getItem('hrm_token');
        if (!token) return;

        let ws: WebSocket | null = null;
        let reconnectTimer: ReturnType<typeof setTimeout> | null = null;
        let closed = false;

        const connect = () => {
            const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
            const url = `${protocol}//${window.location.host}/api/admin/biometric/ws?token=${encodeURIComponent(token)}`;
            ws = new WebSocket(url);

            ws.onopen = () => setConnected(true);

            ws.onmessage = (ev) => {
                try {
                    const data = JSON.parse(ev.data as string) as BiometricLiveEvent;
                    onEventRef.current(data);
                } catch {
                    /* ignore */
                }
            };

            ws.onclose = () => {
                setConnected(false);
                if (!closed) {
                    reconnectTimer = setTimeout(connect, 2500);
                }
            };

            ws.onerror = () => ws?.close();
        };

        connect();

        return () => {
            closed = true;
            if (reconnectTimer) clearTimeout(reconnectTimer);
            ws?.close();
            setConnected(false);
        };
    }, [enabled]);

    return { connected };
}

/** Device is "online" if heartbeat within the last N minutes (BIO-PARK polls every few min). */
export const DEVICE_ONLINE_MS = 10 * 60 * 1000;

export function isDeviceOnline(lastHeartbeat: string | null): boolean {
    if (!lastHeartbeat) return false;
    const t = new Date(lastHeartbeat.includes('T') ? lastHeartbeat : `${lastHeartbeat.replace(' ', 'T')}Z`);
    return Date.now() - t.getTime() < DEVICE_ONLINE_MS;
}
