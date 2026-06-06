import axios from '@/lib/axios';
import { Edit, Save, Trash2, X } from 'lucide-react';
import { useEffect, useState } from 'react';

import { Button } from '@/components/ui/button';
import { Checkbox } from '@/components/ui/checkbox';
import { Input } from '@/components/ui/input';
import { Separator } from '@/components/ui/separator';
import { handleApiError, handleApiResponse } from '@/lib/toast';

interface ComponentItem {
    salary_component_id: number;
    name: string;
    type: 'earning' | 'deduction' | 'reimbursement';
    calculation_type: 'flat_amount' | 'percentage_of_basic' | 'percentage_of_ctc' | 'percentage_of_gross' | null;
    component_default_amount: string | null;
    is_pre_tax: boolean;
    deduction_frequency: 'recurring' | 'one_time' | null;
    assigned_amount: number | null;
    is_assigned: boolean;
}

interface CtcProfileContext {
    yearly_ctc: number;
    monthly_ctc: number;
    template?: {
        name: string;
        basic_pct: number;
        hra_pct: number;
        conv_pct: number;
        special_pct: number;
    };
}

interface SalaryData {
    components: ComponentItem[];
    effective_from: string;
    gross_salary: number;
    total_deductions: number;
    net_salary: number;
    ctc_locked?: boolean;
    read_only_reason?: string | null;
    ctc_profile?: CtcProfileContext | null;
}

type FormValues = Record<number, { enabled: boolean; amount: string }>;

const fmt = (v: number) =>
    '\u20b9\u00a0' + v.toLocaleString('en-IN', { minimumFractionDigits: 2, maximumFractionDigits: 2 });

export function SalaryStructurePanel({
    userId,
    hasCtc = false,
    onCtcChange,
}: {
    userId: number;
    hasCtc?: boolean;
    onCtcChange?: (hasCtc: boolean) => void;
}) {
    const [data, setData] = useState<SalaryData | null>(null);
    const [loading, setLoading] = useState(true);
    const [loadError, setLoadError] = useState<string | null>(null);
    const [rawDebug, setRawDebug] = useState<string | null>(null);
    const [isEditing, setIsEditing] = useState(false);
    const [saving, setSaving] = useState(false);
    const [clearingCtc, setClearingCtc] = useState(false);
    const [effectiveFrom, setEffectiveFrom] = useState('');
    const [formValues, setFormValues] = useState<FormValues>({});

    const applyData = (d: SalaryData) => {
        const v: FormValues = {};
        d.components.forEach((c) => {
            v[c.salary_component_id] = {
                enabled: c.is_assigned,
                amount: c.assigned_amount != null ? String(c.assigned_amount) : '',
            };
        });
        // Batch all state updates together
        setData(d);
        setEffectiveFrom(d.effective_from ?? new Date().toISOString().split('T')[0]);
        setFormValues(v);
    };

    useEffect(() => {
        let cancelled = false;
        setLoadError(null);
        setRawDebug(null);
        setData(null);
        setLoading(true);
        axios
            .get<{ success: boolean; data: SalaryData | null; message?: string }>(
                `/admin/users/${userId}/salary-structure`,
                {
                    headers: {
                        Accept: 'application/json',
                        'X-Requested-With': 'XMLHttpRequest',
                    },
                },
            )
            .then((res) => {
                if (cancelled) return;
                const raw = res.data;
                console.log('[SalaryStructurePanel] raw response:', res.status, JSON.parse(JSON.stringify(raw)));
                setRawDebug(JSON.stringify(raw, null, 2));
                if (!raw.success) {
                    setLoadError(raw.message ?? 'Server returned error.');
                    return;
                }
                const payload = raw.data;
                if (payload && Array.isArray(payload.components)) {
                    applyData(payload);
                } else {
                    console.warn('[SalaryStructurePanel] payload.components missing:', raw);
                    setLoadError('Unexpected API response shape. Check console.');
                }
            })
            .catch((err) => {
                if (cancelled) return;
                console.error('[SalaryStructurePanel] error:', err.response?.status, err.response?.data);
                const status = err.response?.status ?? '?';
                const msg = err.response?.data?.message || err.message || 'Unknown error';
                handleApiError(err);
                setLoadError(`[HTTP ${status}] ${msg}`);
            })
            .finally(() => { if (!cancelled) setLoading(false); });
        return () => { cancelled = true; };
    // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [userId]);


    const reloadStructure = () => {
        setLoading(true);
        setLoadError(null);
        axios
            .get<{ success: boolean; data: SalaryData | null; message?: string }>(
                `/admin/users/${userId}/salary-structure`,
            )
            .then((res) => {
                if (res.data.success && res.data.data?.components) {
                    applyData(res.data.data);
                }
            })
            .catch(handleApiError)
            .finally(() => setLoading(false));
    };

    const handleRemoveCtc = async () => {
        if (!confirm('Remove CTC profile? You can then set up manual salary components for payroll.')) return;
        setClearingCtc(true);
        try {
            const res = await axios.delete(`/admin/users/${userId}/ctc-profile`);
            handleApiResponse(res);
            onCtcChange?.(false);
            reloadStructure();
        } catch (err) {
            handleApiError(err);
        } finally {
            setClearingCtc(false);
        }
    };

    const isLocked = hasCtc || data?.ctc_locked;

    // ID of the "Basic Salary" earning component (used to auto-compute percentage_of_basic)
    const basicId = data?.components.find(
        (c) => c.type === 'earning' && c.name.toLowerCase().includes('basic'),
    )?.salary_component_id;

    const monthlyCtc = data?.ctc_profile?.monthly_ctc ?? 0;

    const pctLabel = (c: ComponentItem) => {
        if (c.calculation_type === 'percentage_of_ctc') {
            return `(${c.component_default_amount}% of CTC)`;
        }
        if (c.calculation_type === 'percentage_of_gross') {
            return `(${c.component_default_amount}% of Gross)`;
        }
        if (c.calculation_type === 'percentage_of_basic') {
            return `(${c.component_default_amount}% of Basic)`;
        }
        return null;
    };

    const resolvePctAmount = (c: ComponentItem, basic: number, mctc: number, gross: number) => {
        const pct = parseFloat(c.component_default_amount ?? '0') || 0;
        if (c.calculation_type === 'percentage_of_ctc') {
            return ((mctc * pct) / 100).toFixed(2);
        }
        if (c.calculation_type === 'percentage_of_gross') {
            return ((gross * pct) / 100).toFixed(2);
        }
        if (c.calculation_type === 'percentage_of_basic') {
            return ((basic * pct) / 100).toFixed(2);
        }
        return '';
    };

    const pctOfBasicComponents = (list: ComponentItem[]) =>
        list.filter((c) => c.calculation_type === 'percentage_of_basic');

    const grossMonthly = data?.gross_salary ?? monthlyCtc;

    const applyPercentageFromBasic = (
        updates: FormValues,
        basic: number,
        list: ComponentItem[],
    ) => {
        pctOfBasicComponents(list).forEach((c) => {
            if (formValues[c.salary_component_id]?.enabled) {
                updates[c.salary_component_id] = {
                    ...formValues[c.salary_component_id],
                    amount: resolvePctAmount(c, basic, monthlyCtc, grossMonthly),
                };
            }
        });
    };

    const handleAmountChange = (id: number, value: string) => {
        const updates: FormValues = { [id]: { ...formValues[id], amount: value } };

        if (id === basicId && data) {
            const basic = parseFloat(value) || 0;
            applyPercentageFromBasic(updates, basic, data.components);
        }

        setFormValues((prev) => ({ ...prev, ...updates }));
    };

    const handleToggle = (id: number, component: ComponentItem) => {
        const newEnabled = !formValues[id]?.enabled;
        let amount = formValues[id]?.amount || '';

        if (newEnabled && !amount && basicId && data) {
            const basic = parseFloat(formValues[basicId]?.amount || '0') || 0;
            if (component.calculation_type === 'percentage_of_basic' || component.calculation_type === 'percentage_of_ctc' || component.calculation_type === 'percentage_of_gross') {
                amount = resolvePctAmount(component, basic, monthlyCtc, grossMonthly);
            }
        }

        setFormValues((prev) => ({ ...prev, [id]: { ...prev[id], enabled: newEnabled, amount } }));
    };

    const liveEarnings =
        data?.components
            .filter((c) => c.type === 'earning' && formValues[c.salary_component_id]?.enabled)
            .reduce((sum, c) => sum + (parseFloat(formValues[c.salary_component_id]?.amount) || 0), 0) ?? 0;

    const liveDeductions =
        data?.components
            .filter((c) => c.type === 'deduction' && formValues[c.salary_component_id]?.enabled)
            .reduce((sum, c) => sum + (parseFloat(formValues[c.salary_component_id]?.amount) || 0), 0) ?? 0;

    const handleSave = async () => {
        setSaving(true);
        const items = Object.entries(formValues)
            .filter(([, v]) => v.enabled)
            .map(([id, v]) => ({
                salary_component_id: parseInt(id),
                amount: parseFloat(v.amount) || 0,
            }));

        try {
            const res = await axios.post(`/admin/users/${userId}/salary-structure`, {
                effective_from: effectiveFrom,
                items,
            });
            handleApiResponse(res);
            if (res.data.data) applyData(res.data.data);
            setIsEditing(false);
        } catch (err) {
            handleApiError(err);
        } finally {
            setSaving(false);
        }
    };

    if (loading) {
        return <p className="py-4 text-center text-sm text-muted-foreground">Loading…</p>;
    }

    if (loadError) {
        return <p className="text-sm text-destructive">{loadError}</p>;
    }

    if (!data || data.components.length === 0) {
        return (
            <div className="space-y-2">
                <p className="text-sm text-muted-foreground">
                    No salary components configured. Please add components from{' '}
                    <a href="/admin/salaries/components" className="text-primary underline">
                        Salary Components
                    </a>{' '}
                    first.
                </p>
                {rawDebug && (
                    <details className="text-xs text-muted-foreground">
                        <summary className="cursor-pointer text-destructive">Debug: API response (click to expand)</summary>
                        <pre className="mt-1 overflow-auto rounded bg-muted p-2 text-xs">{rawDebug}</pre>
                    </details>
                )}
            </div>
        );
    }

    const earnings = data.components.filter((c) => c.type === 'earning');
    const deductions = data.components.filter((c) => c.type === 'deduction');
    const hasAssigned = data.components.some((c) => c.is_assigned);

    // ── View mode ─────────────────────────────────────────────────────
    if (!isEditing) {
        return (
            <div className="space-y-4">
                {isLocked && (
                    <div className="rounded-md border border-amber-200 bg-amber-50 dark:border-amber-800 dark:bg-amber-950/30 px-3 py-3 space-y-2">
                        <p className="text-xs text-amber-800 dark:text-amber-300">
                            Payroll currently uses <strong>CTC Split</strong>. Manual amounts below are synced
                            read-only. Remove CTC to assign components manually for payroll.
                        </p>
                        <Button
                            type="button"
                            size="sm"
                            variant="outline"
                            className="text-destructive border-destructive/30"
                            onClick={handleRemoveCtc}
                            disabled={clearingCtc}
                        >
                            <Trash2 className="mr-1.5 h-4 w-4" />
                            {clearingCtc ? 'Removing CTC…' : 'Remove CTC & enable manual'}
                        </Button>
                    </div>
                )}
                {data.ctc_profile && !isLocked && (
                    <p className="text-xs text-muted-foreground rounded-md bg-muted px-3 py-2">
                        Linked to CTC ₹ {data.ctc_profile.yearly_ctc.toLocaleString('en-IN')}/yr
                        ({fmt(data.ctc_profile.monthly_ctc)}/mo)
                        {data.ctc_profile.template ? ` · ${data.ctc_profile.template.name}` : ''}
                    </p>
                )}
                {!hasAssigned ? (
                    <p className="text-sm text-muted-foreground">No salary structure configured.</p>
                ) : (
                    <>
                        {earnings.some((c) => c.is_assigned) && (
                            <div className="space-y-1.5">
                                <p className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
                                    Earnings
                                </p>
                                {earnings
                                    .filter((c) => c.is_assigned)
                                    .map((c) => (
                                        <div key={c.salary_component_id} className="flex justify-between text-sm">
                                            <span>{c.name}</span>
                                            <span className="font-medium">{fmt(c.assigned_amount ?? 0)}</span>
                                        </div>
                                    ))}
                            </div>
                        )}

                        {deductions.some((c) => c.is_assigned) && (
                            <div className="space-y-1.5">
                                <p className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
                                    Deductions
                                </p>
                                {deductions
                                    .filter((c) => c.is_assigned)
                                    .map((c) => (
                                        <div key={c.salary_component_id} className="flex justify-between text-sm">
                                            <span>{c.name}</span>
                                            <span className="font-medium text-destructive">
                                                {fmt(c.assigned_amount ?? 0)}
                                            </span>
                                        </div>
                                    ))}
                            </div>
                        )}

                        <Separator />

                        <div className="grid grid-cols-3 gap-2">
                            <div className="rounded-md bg-green-50 dark:bg-green-950 p-3 text-center">
                                <p className="text-xs text-muted-foreground">Gross</p>
                                <p className="text-sm font-semibold text-green-700 dark:text-green-400">
                                    {fmt(data.gross_salary)}
                                </p>
                            </div>
                            <div className="rounded-md bg-red-50 dark:bg-red-950 p-3 text-center">
                                <p className="text-xs text-muted-foreground">Deductions</p>
                                <p className="text-sm font-semibold text-red-700 dark:text-red-400">
                                    {fmt(data.total_deductions)}
                                </p>
                            </div>
                            <div className="rounded-md bg-primary/10 p-3 text-center">
                                <p className="text-xs text-muted-foreground">Net</p>
                                <p className="text-sm font-semibold text-primary">{fmt(data.net_salary)}</p>
                            </div>
                        </div>

                        <p className="text-xs text-muted-foreground">
                            Effective from: {new Date(data.effective_from).toLocaleDateString()}
                        </p>
                    </>
                )}

                {!isLocked && (
                <div className="flex justify-end">
                    <Button variant="outline" size="sm" onClick={() => setIsEditing(true)}>
                        <Edit className="mr-1.5 h-4 w-4" />
                        {hasAssigned ? 'Edit' : 'Set Up'}
                    </Button>
                </div>
                )}
            </div>
        );
    }

    // ── Edit mode ─────────────────────────────────────────────────────
    return (
        <div className="space-y-4">
            {/* Earnings */}
            {earnings.length > 0 && (
                <div className="space-y-2">
                    <p className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">Earnings</p>
                    {earnings.map((c) => (
                        <div key={c.salary_component_id} className="flex items-center gap-3">
                            <Checkbox
                                checked={formValues[c.salary_component_id]?.enabled ?? false}
                                onCheckedChange={() => handleToggle(c.salary_component_id, c)}
                            />
                            <div className="min-w-0 flex-1">
                                <p className="text-sm leading-none">
                                    {c.name}
                                    {pctLabel(c) && (
                                        <span className="ml-1.5 text-xs text-muted-foreground">
                                            {pctLabel(c)}
                                        </span>
                                    )}
                                </p>
                            </div>
                            <div className="relative w-36">
                                <span className="absolute left-2.5 top-1/2 -translate-y-1/2 text-xs text-muted-foreground">
                                    ₹
                                </span>
                                <Input
                                    type="number"
                                    min="0"
                                    step="0.01"
                                    placeholder="0.00"
                                    disabled={!formValues[c.salary_component_id]?.enabled}
                                    value={formValues[c.salary_component_id]?.amount ?? ''}
                                    onChange={(e) => handleAmountChange(c.salary_component_id, e.target.value)}
                                    className="h-8 pl-6 text-sm"
                                />
                            </div>
                        </div>
                    ))}
                </div>
            )}

            {/* Deductions */}
            {deductions.length > 0 && (
                <div className="space-y-2">
                    <p className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
                        Deductions
                    </p>
                    {deductions.map((c) => (
                        <div key={c.salary_component_id} className="flex items-center gap-3">
                            <Checkbox
                                checked={formValues[c.salary_component_id]?.enabled ?? false}
                                onCheckedChange={() => handleToggle(c.salary_component_id, c)}
                            />
                            <div className="min-w-0 flex-1">
                                <p className="text-sm leading-none">
                                    {c.name}
                                    {pctLabel(c) && (
                                        <span className="ml-1.5 text-xs text-muted-foreground">
                                            {pctLabel(c)}
                                        </span>
                                    )}
                                </p>
                                <p className="text-xs text-muted-foreground">
                                    {c.is_pre_tax ? 'Pre-Tax' : 'Post-Tax'}
                                    {c.deduction_frequency
                                        ? ` · ${c.deduction_frequency === 'recurring' ? 'Recurring' : 'One Time'}`
                                        : ''}
                                </p>
                            </div>
                            <div className="relative w-36">
                                <span className="absolute left-2.5 top-1/2 -translate-y-1/2 text-xs text-muted-foreground">
                                    ₹
                                </span>
                                <Input
                                    type="number"
                                    min="0"
                                    step="0.01"
                                    placeholder="0.00"
                                    disabled={!formValues[c.salary_component_id]?.enabled}
                                    value={formValues[c.salary_component_id]?.amount ?? ''}
                                    onChange={(e) => handleAmountChange(c.salary_component_id, e.target.value)}
                                    className="h-8 pl-6 text-sm"
                                />
                            </div>
                        </div>
                    ))}
                </div>
            )}

            {/* Effective From */}
            <div className="flex items-center gap-3">
                <span className="flex-1 text-sm text-muted-foreground">Effective From</span>
                <Input
                    type="date"
                    value={effectiveFrom}
                    onChange={(e) => setEffectiveFrom(e.target.value)}
                    className="h-8 w-40 text-sm"
                />
            </div>

            {/* Live summary */}
            <div className="space-y-1.5 rounded-md bg-muted p-3 text-sm">
                <div className="flex justify-between">
                    <span className="text-muted-foreground">Gross Salary</span>
                    <span className="font-medium">{fmt(liveEarnings)}</span>
                </div>
                <div className="flex justify-between">
                    <span className="text-muted-foreground">Total Deductions</span>
                    <span className="font-medium text-destructive">{fmt(liveDeductions)}</span>
                </div>
                <Separator />
                <div className="flex justify-between font-semibold">
                    <span>Net Salary</span>
                    <span className="text-green-600">{fmt(Math.max(0, liveEarnings - liveDeductions))}</span>
                </div>
            </div>

            {/* Actions */}
            <div className="flex justify-end gap-2">
                <Button variant="outline" size="sm" onClick={() => setIsEditing(false)} disabled={saving}>
                    <X className="mr-1 h-4 w-4" /> Cancel
                </Button>
                <Button size="sm" onClick={handleSave} disabled={saving || !effectiveFrom}>
                    <Save className="mr-1 h-4 w-4" />
                    {saving ? 'Saving…' : 'Save'}
                </Button>
            </div>
        </div>
    );
}
