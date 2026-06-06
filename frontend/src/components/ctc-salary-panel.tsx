import axios from '@/lib/axios';
import { ExternalLink, Save, Trash2 } from 'lucide-react';
import { Link } from 'react-router-dom';
import { useCallback, useEffect, useState } from 'react';

import { Button } from '@/components/ui/button';
import { Checkbox } from '@/components/ui/checkbox';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Separator } from '@/components/ui/separator';
import { handleApiError, handleApiResponse } from '@/lib/toast';

interface ComponentLine {
    component_id: number;
    name: string;
    pct: number;
    amount: number;
    is_employer?: boolean;
}

interface SalaryComponentEarning {
    id: number;
    name: string;
    calculation_type: string;
    pct: number;
    flat: number;
}

interface StatutoryPreview {
    yearly_ctc: number;
    monthly_ctc: number;
    employer_pf: number;
    employer_esi: number;
    employer_lw: number;
    total_employer: number;
    gross: number;
    basic: number;
    hra: number;
    conveyance: number;
    special: number;
    basic_pct: number;
    hra_pct: number;
    conv_pct: number;
    special_pct: number;
    employee_pf: number;
    employee_esi: number;
    employee_lw: number;
    prof_tax: number;
    employee_tds: number;
    total_employee_deductions: number;
    net_take_home: number;
    pf_applicable: boolean;
    esi_applicable: boolean;
    esi_applies: boolean;
    esi_note?: string | null;
    earning_lines?: ComponentLine[];
    deduction_lines?: ComponentLine[];
    split_source?: string;
}

const fmt = (v: number) =>
    '₹ ' + v.toLocaleString('en-IN', { minimumFractionDigits: 2, maximumFractionDigits: 2 });

const PreviewRow = ({
    label,
    value,
    muted,
    bold,
    negative,
}: {
    label: string;
    value: string;
    muted?: boolean;
    bold?: boolean;
    negative?: boolean;
}) => (
    <div className={`flex justify-between ${muted ? 'text-muted-foreground text-xs' : ''} ${bold ? 'font-semibold' : ''}`}>
        <span>{label}</span>
        <span className={negative ? 'text-destructive' : ''}>{value}</span>
    </div>
);

function formatEarningLabel(line: ComponentLine) {
    if (line.pct > 0) return `${line.name} (${line.pct}% of Gross)`;
    return line.name;
}

function formatComponentSummary(earnings: SalaryComponentEarning[]) {
    const pctLines = earnings.filter((e) => e.calculation_type.includes('percentage') && e.pct > 0);
    if (pctLines.length === 0) return 'Configure earnings in Salary Components';
    return pctLines.map((e) => `${e.name} ${e.pct}%`).join(' · ');
}

export function CtcSalaryPanel({
    userId,
    onCtcChange,
}: {
    userId: number;
    onCtcChange?: (hasCtc: boolean) => void;
}) {
    const [loading, setLoading] = useState(true);
    const [saving, setSaving] = useState(false);
    const [clearing, setClearing] = useState(false);
    const [previewLoading, setPreviewLoading] = useState(false);
    const [earningsConfig, setEarningsConfig] = useState<SalaryComponentEarning[]>([]);
    const [yearlyCtc, setYearlyCtc] = useState('');
    const [effectiveFrom, setEffectiveFrom] = useState(new Date().toISOString().split('T')[0]);
    const [preview, setPreview] = useState<StatutoryPreview | null>(null);
    const [hasPfComponent, setHasPfComponent] = useState(false);
    const [hasEsiComponent, setHasEsiComponent] = useState(false);
    const [pfApplicable, setPfApplicable] = useState(true);
    const [esiApplicable, setEsiApplicable] = useState(true);
    const [hasSavedCtc, setHasSavedCtc] = useState(false);

    const fetchPreview = useCallback(async () => {
        const y = parseFloat(yearlyCtc);
        if (!y || y <= 0) return;
        setPreviewLoading(true);
        try {
            const res = await axios.post('/admin/salaries/ctc-preview', {
                yearly_ctc: y,
                pf_applicable: pfApplicable,
                esi_applicable: esiApplicable,
            });
            if (res.data.success) setPreview(res.data.data);
        } catch {
            /* keep last preview */
        } finally {
            setPreviewLoading(false);
        }
    }, [yearlyCtc, pfApplicable, esiApplicable]);

    const load = async () => {
        setLoading(true);
        try {
            const res = await axios.get(`/admin/users/${userId}/ctc-profile`);
            if (res.data.success) {
                const d = res.data.data;
                setEarningsConfig(d.salary_components?.earnings || []);
                setHasPfComponent(d.salary_components?.has_pf ?? false);
                setHasEsiComponent(d.salary_components?.has_esi ?? false);
                if (d.profile) {
                    setYearlyCtc(String(d.profile.yearly_ctc));
                    setEffectiveFrom(d.profile.effective_from?.split('T')[0] || effectiveFrom);
                    setPfApplicable(d.profile.pf_applicable ?? true);
                    setEsiApplicable(d.profile.esi_applicable ?? true);
                    setHasSavedCtc(true);
                    onCtcChange?.(true);
                } else {
                    setHasSavedCtc(false);
                    onCtcChange?.(false);
                }
                if (d.split_preview) setPreview(d.split_preview);
            }
        } catch (e) {
            handleApiError(e);
        } finally {
            setLoading(false);
        }
    };

    useEffect(() => {
        void load();
    }, [userId]);

    useEffect(() => {
        const t = setTimeout(() => void fetchPreview(), 300);
        return () => clearTimeout(t);
    }, [fetchPreview]);

    const handleSave = async () => {
        const y = parseFloat(yearlyCtc);
        if (!y || y <= 0) return;
        setSaving(true);
        try {
            const res = await axios.post(`/admin/users/${userId}/ctc-profile`, {
                yearly_ctc: y,
                effective_from: effectiveFrom,
                pf_applicable: pfApplicable,
                esi_applicable: esiApplicable,
            });
            handleApiResponse(res);
            if (res.data.data?.split_preview) setPreview(res.data.data.split_preview);
            if (res.data.data?.salary_components?.earnings) {
                setEarningsConfig(res.data.data.salary_components.earnings);
            }
            setHasSavedCtc(true);
            onCtcChange?.(true);
        } catch (e) {
            handleApiError(e);
        } finally {
            setSaving(false);
        }
    };

    const handleClearCtc = async () => {
        if (!confirm('Remove CTC profile? Payroll will use manual salary structure instead.')) return;
        setClearing(true);
        try {
            const res = await axios.delete(`/admin/users/${userId}/ctc-profile`);
            handleApiResponse(res);
            setYearlyCtc('');
            setPreview(null);
            setHasSavedCtc(false);
            onCtcChange?.(false);
        } catch (e) {
            handleApiError(e);
        } finally {
            setClearing(false);
        }
    };

    if (loading) {
        return <p className="py-4 text-center text-sm text-muted-foreground">Loading…</p>;
    }

    const earningLines = preview?.earning_lines?.filter((l) => l.amount > 0) ?? [];
    const employeeDeductions = preview?.deduction_lines?.filter((l) => l.amount > 0 && !l.is_employer) ?? [];
    const employerDeductions = preview?.deduction_lines?.filter((l) => l.amount > 0 && l.is_employer) ?? [];
    const showEmployeeDeductions = preview && employeeDeductions.length > 0;

    return (
        <div className="space-y-4">
            <div className="grid gap-3">
                <div>
                    <Label>Yearly CTC (₹)</Label>
                    <Input
                        type="number"
                        min="0"
                        step="1000"
                        value={yearlyCtc}
                        onChange={(e) => setYearlyCtc(e.target.value)}
                        placeholder="e.g. 300000"
                    />
                </div>
                <div className="rounded-md border bg-muted/40 px-3 py-2 text-sm">
                    <p className="text-xs font-semibold uppercase text-muted-foreground">Earnings split</p>
                    <p className="mt-1 text-muted-foreground">{formatComponentSummary(earningsConfig)}</p>
                    <Link
                        to="/admin/salaries/components"
                        className="mt-1 inline-flex items-center gap-1 text-xs text-primary hover:underline"
                    >
                        Edit in Salary Components
                        <ExternalLink className="h-3 w-3" />
                    </Link>
                </div>
                {(hasPfComponent || hasEsiComponent) && (
                    <div className="space-y-2 rounded-md border px-3 py-2">
                        <p className="text-xs font-semibold uppercase text-muted-foreground">Statutory for this employee</p>
                        {hasPfComponent && (
                            <label className="flex items-center gap-2 text-sm cursor-pointer">
                                <Checkbox
                                    checked={pfApplicable}
                                    onCheckedChange={(v) => setPfApplicable(v === true)}
                                />
                                Provident Fund (PF)
                            </label>
                        )}
                        {hasEsiComponent && (
                            <label className="flex items-center gap-2 text-sm cursor-pointer">
                                <Checkbox
                                    checked={esiApplicable}
                                    onCheckedChange={(v) => setEsiApplicable(v === true)}
                                />
                                Employee State Insurance (ESI)
                            </label>
                        )}
                    </div>
                )}
                <div>
                    <Label>Effective From</Label>
                    <Input type="date" value={effectiveFrom} onChange={(e) => setEffectiveFrom(e.target.value)} />
                </div>
            </div>

            {preview && (
                <>
                    <Separator />
                    <div className="space-y-2 text-sm">
                        <p className="text-xs font-semibold uppercase text-muted-foreground">
                            Monthly CTC Breakdown
                            {previewLoading && <span className="ml-2 font-normal normal-case">(updating…)</span>}
                        </p>

                        <PreviewRow label="Monthly CTC" value={fmt(preview.monthly_ctc)} bold />

                        <p className="pt-1 text-xs font-medium text-muted-foreground">
                            Earnings (from Salary Components)
                        </p>
                        {earningLines.length > 0 ? (
                            earningLines.map((line) => (
                                <PreviewRow
                                    key={line.component_id}
                                    label={formatEarningLabel(line)}
                                    value={fmt(line.amount)}
                                    muted
                                />
                            ))
                        ) : (
                            <>
                                <PreviewRow label={`Basic (${preview.basic_pct}%)`} value={fmt(preview.basic)} muted />
                                <PreviewRow label={`HRA (${preview.hra_pct}%)`} value={fmt(preview.hra)} muted />
                                <PreviewRow label={`Conveyance (${preview.conv_pct}%)`} value={fmt(preview.conveyance)} muted />
                                {preview.special > 0 && (
                                    <PreviewRow label={`Special (${preview.special_pct}%)`} value={fmt(preview.special)} muted />
                                )}
                            </>
                        )}

                        {preview.esi_note && (
                            <p className="text-xs text-amber-700 bg-amber-50 border border-amber-200 rounded px-2 py-1.5">
                                {preview.esi_note}
                            </p>
                        )}

                        {showEmployeeDeductions && (
                            <>
                                <Separator className="my-2" />
                                <p className="text-xs font-medium text-muted-foreground">Employee Deductions</p>
                                {employeeDeductions.map((line) => (
                                    <PreviewRow
                                        key={`${line.component_id}-${line.name}`}
                                        label={line.name}
                                        value={`− ${fmt(line.amount)}`}
                                        muted
                                        negative
                                    />
                                ))}
                                <PreviewRow
                                    label="Net Take-home (approx.)"
                                    value={fmt(preview.net_take_home)}
                                    bold
                                />
                            </>
                        )}

                        {employerDeductions.length > 0 && (
                            <>
                                <Separator className="my-2" />
                                <p className="text-xs font-medium text-muted-foreground">
                                    Employer Deductions (from Salary Components)
                                </p>
                                {employerDeductions.map((line) => (
                                    <PreviewRow
                                        key={`${line.component_id}-${line.name}`}
                                        label={line.name}
                                        value={fmt(line.amount)}
                                        muted
                                    />
                                ))}
                            </>
                        )}
                    </div>
                </>
            )}

            <div className="flex justify-between gap-2">
                {hasSavedCtc && (
                    <Button
                        size="sm"
                        variant="outline"
                        className="text-destructive"
                        onClick={handleClearCtc}
                        disabled={clearing}
                        type="button"
                    >
                        <Trash2 className="mr-1 h-4 w-4" />
                        {clearing ? 'Removing…' : 'Remove CTC'}
                    </Button>
                )}
                <Button size="sm" onClick={handleSave} disabled={saving || !yearlyCtc} className="ml-auto">
                    <Save className="mr-1 h-4 w-4" />
                    {saving ? 'Saving…' : 'Save CTC Structure'}
                </Button>
            </div>
        </div>
    );
}
