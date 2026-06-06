const API_BASE = '/api';

/** Get JWT token from localStorage */
function getToken(): string | null {
    return localStorage.getItem('hrm_token');
}

/** Set JWT token */
export function setToken(token: string) {
    localStorage.setItem('hrm_token', token);
}

/** Clear JWT token */
export function clearToken() {
    localStorage.removeItem('hrm_token');
    localStorage.removeItem('hrm_refresh_token');
}

export function setRefreshToken(token: string) {
    localStorage.setItem('hrm_refresh_token', token);
}

export function getRefreshToken(): string | null {
    return localStorage.getItem('hrm_refresh_token');
}

/** Check if user is authenticated */
export function isAuthenticated(): boolean {
    return !!getToken();
}

let refreshInFlight: Promise<boolean> | null = null;

async function tryRefreshToken(): Promise<boolean> {
    const refresh = getRefreshToken();
    if (!refresh) return false;
    if (!refreshInFlight) {
        refreshInFlight = (async () => {
            try {
                const res = await fetch(`${API_BASE}/auth/refresh`, {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json', Accept: 'application/json' },
                    body: JSON.stringify({ refresh_token: refresh }),
                });
                if (!res.ok) return false;
                const json = await res.json();
                const newToken = json?.data?.token;
                const newRefresh = json?.data?.refresh_token;
                if (!newToken) return false;
                setToken(newToken);
                if (newRefresh) setRefreshToken(newRefresh);
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

/** Core fetch wrapper with JWT */
async function apiFetch<T = any>(
    path: string,
    options: RequestInit = {},
    retried = false,
): Promise<{ success: boolean; data: T; type?: string; message?: string; total?: number }> {
    const token = getToken();
    const headers: Record<string, string> = {
        'Content-Type': 'application/json',
        Accept: 'application/json',
        ...(options.headers as Record<string, string> || {}),
    };

    if (token) {
        headers['Authorization'] = `Bearer ${token}`;
    }

    const response = await fetch(`${API_BASE}${path}`, {
        ...options,
        headers,
    });

    if (response.status === 401 && !retried && !path.startsWith('/auth/')) {
        const refreshed = await tryRefreshToken();
        if (refreshed) {
            return apiFetch<T>(path, options, true);
        }
        clearToken();
        window.location.href = '/login';
        throw new Error('Unauthorized');
    }

    const json = await response.json();

    if (!response.ok) {
        throw new Error(json.message || `API error: ${response.status}`);
    }

    return json;
}

/** GET request */
export async function apiGet<T = any>(path: string, params?: Record<string, string | number | undefined>): Promise<{ success: boolean; data: T; total?: number }> {
    let url = path;
    if (params) {
        const searchParams = new URLSearchParams();
        Object.entries(params).forEach(([key, value]) => {
            if (value !== undefined && value !== '') {
                searchParams.set(key, String(value));
            }
        });
        const qs = searchParams.toString();
        if (qs) url += `?${qs}`;
    }
    return apiFetch<T>(url);
}

/** POST request */
export async function apiPost<T = any>(path: string, body?: any): Promise<{ success: boolean; data: T }> {
    return apiFetch<T>(path, {
        method: 'POST',
        body: body ? JSON.stringify(body) : undefined,
    });
}

/** PUT request */
export async function apiPut<T = any>(path: string, body?: any): Promise<{ success: boolean; data: T }> {
    return apiFetch<T>(path, {
        method: 'PUT',
        body: body ? JSON.stringify(body) : undefined,
    });
}

/** PATCH request */
export async function apiPatch<T = any>(path: string, body?: any): Promise<{ success: boolean; data: T }> {
    return apiFetch<T>(path, {
        method: 'PATCH',
        body: body ? JSON.stringify(body) : undefined,
    });
}

/** DELETE request */
export async function apiDelete<T = any>(path: string): Promise<{ success: boolean; data: T }> {
    return apiFetch<T>(path, { method: 'DELETE' });
}
