/**
 * Axios instance pre-configured with JWT auth for the Rust backend.
 *
 * BaseURL = '/api' ensures all component calls like axios.get('/admin/users')
 * become '/api/admin/users', which Vite proxies to the Rust backend.
 */
import axiosLib, { AxiosHeaders, type InternalAxiosRequestConfig } from 'axios';

const axios = axiosLib.create({
    baseURL: '/api',
    headers: {
        'Content-Type': 'application/json',
        Accept: 'application/json',
    },
});

let refreshInFlight: Promise<boolean> | null = null;

async function tryRefreshToken(): Promise<boolean> {
    const refresh = localStorage.getItem('hrm_refresh_token');
    if (!refresh) return false;
    if (!refreshInFlight) {
        refreshInFlight = (async () => {
            try {
                const res = await fetch('/api/auth/refresh', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json', Accept: 'application/json' },
                    body: JSON.stringify({ refresh_token: refresh }),
                });
                if (!res.ok) return false;
                const json = await res.json();
                const newToken = json?.data?.token;
                const newRefresh = json?.data?.refresh_token;
                if (!newToken) return false;
                localStorage.setItem('hrm_token', newToken);
                if (newRefresh) localStorage.setItem('hrm_refresh_token', newRefresh);
                return true;
            } catch {
                return false;
            } finally {
                refreshInFlight = null;
            }
        })();
    }
    return refreshInFlight;
}

// ── Request interceptor: inject JWT token ───────────────────────────────────
axios.interceptors.request.use((config) => {
    const token = localStorage.getItem('hrm_token');
    if (token) {
        config.headers.Authorization = `Bearer ${token}`;
    }
    if (config.data instanceof FormData) {
        const headers = AxiosHeaders.from(config.headers);
        headers.delete('Content-Type');
        headers.delete('content-type');
        config.headers = headers;
    }
    return config;
});

// ── Response interceptor: refresh on 401, then redirect ─────────────────────
axios.interceptors.response.use(
    (response) => response,
    async (error) => {
        const original = error.config as InternalAxiosRequestConfig & { _retry?: boolean };
        if (
            error.response?.status === 401 &&
            original &&
            !original._retry &&
            !String(original.url || '').includes('/auth/')
        ) {
            original._retry = true;
            const refreshed = await tryRefreshToken();
            if (refreshed) {
                const token = localStorage.getItem('hrm_token');
                if (token) {
                    original.headers.Authorization = `Bearer ${token}`;
                }
                return axios(original);
            }
        }
        if (error.response?.status === 401) {
            localStorage.removeItem('hrm_token');
            localStorage.removeItem('hrm_refresh_token');
            window.location.href = '/login';
        }
        return Promise.reject(error);
    },
);

export default axios;
