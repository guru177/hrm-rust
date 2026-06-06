import axios from '@/lib/axios';
import { Plus, Save } from 'lucide-react';
import { useEffect, useState } from 'react';

import Heading from '@/components/heading';
import { Button } from '@/components/ui/button';
import { Checkbox } from '@/components/ui/checkbox';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import {
    Select,
    SelectContent,
    SelectItem,
    SelectTrigger,
    SelectValue,
} from '@/components/ui/select';
import SettingsLayout from '@/layouts/settings/layout';
import { handleApiError, handleApiResponse } from '@/lib/toast';

interface LeaveType {
    id: number;
    slug: string;
    name: string;
    payment_type: 'paid' | 'unpaid' | 'half_day';
    payment_type_label: string;
    counts_toward_quota: boolean;
    is_active: boolean;
}

export default function LeaveTypesSettingsPage() {
    const [items, setItems] = useState<LeaveType[]>([]);
    const [loading, setLoading] = useState(true);
    const [saving, setSaving] = useState<number | 'new' | null>(null);
    const [draft, setDraft] = useState({
        name: '',
        slug: '',
        payment_type: 'paid' as LeaveType['payment_type'],
        counts_toward_quota: false,
    });

    const load = async () => {
        setLoading(true);
        try {
            const res = await axios.get('/admin/settings/leave-types');
            if (res.data.success) setItems(res.data.data || []);
        } catch (e) {
            handleApiError(e);
        } finally {
            setLoading(false);
        }
    };

    useEffect(() => {
        void load();
    }, []);

    const updateItem = async (item: LeaveType, patch: Partial<LeaveType>) => {
        setSaving(item.id);
        try {
            const res = await axios.put(`/admin/settings/leave-types/${item.id}`, {
                name: patch.name ?? item.name,
                payment_type: patch.payment_type ?? item.payment_type,
                counts_toward_quota: patch.counts_toward_quota ?? item.counts_toward_quota,
                is_active: patch.is_active ?? item.is_active,
            });
            handleApiResponse(res);
            await load();
        } catch (e) {
            handleApiError(e);
        } finally {
            setSaving(null);
        }
    };

    const addItem = async () => {
        if (!draft.name.trim()) return;
        setSaving('new');
        try {
            const res = await axios.post('/admin/settings/leave-types', draft);
            handleApiResponse(res);
            setDraft({ name: '', slug: '', payment_type: 'paid', counts_toward_quota: false });
            await load();
        } catch (e) {
            handleApiError(e);
        } finally {
            setSaving(null);
        }
    };

    return (
        <SettingsLayout>
            <Heading
                title="Leave Types"
                description="Configure how each leave type affects payroll — paid, unpaid (LOP), or half-day."
            />

            {loading ? (
                <p className="text-sm text-muted-foreground">Loading…</p>
            ) : (
                <div className="space-y-6">
                    <div className="rounded-lg border overflow-hidden">
                        <table className="w-full text-sm">
                            <thead className="bg-muted/50">
                                <tr>
                                    <th className="px-3 py-2 text-left font-medium">Name</th>
                                    <th className="px-3 py-2 text-left font-medium">Slug</th>
                                    <th className="px-3 py-2 text-left font-medium">Payroll effect</th>
                                    <th className="px-3 py-2 text-left font-medium">Annual quota</th>
                                    <th className="px-3 py-2 text-left font-medium">Active</th>
                                    <th className="px-3 py-2 text-right font-medium">Save</th>
                                </tr>
                            </thead>
                            <tbody>
                                {items.map((item) => (
                                    <LeaveTypeRow
                                        key={item.id}
                                        item={item}
                                        saving={saving === item.id}
                                        onSave={(patch) => void updateItem(item, patch)}
                                    />
                                ))}
                            </tbody>
                        </table>
                    </div>

                    <div className="rounded-lg border p-4 space-y-3">
                        <h3 className="font-semibold text-sm">Add leave type</h3>
                        <div className="grid gap-3 md:grid-cols-2">
                            <div>
                                <Label>Name</Label>
                                <Input
                                    value={draft.name}
                                    onChange={(e) => setDraft({ ...draft, name: e.target.value })}
                                    placeholder="e.g. Comp Off"
                                />
                            </div>
                            <div>
                                <Label>Slug (optional)</Label>
                                <Input
                                    value={draft.slug}
                                    onChange={(e) => setDraft({ ...draft, slug: e.target.value })}
                                    placeholder="auto from name"
                                />
                            </div>
                            <div>
                                <Label>Payroll effect</Label>
                                <Select
                                    value={draft.payment_type}
                                    onValueChange={(v) =>
                                        setDraft({ ...draft, payment_type: v as LeaveType['payment_type'] })
                                    }
                                >
                                    <SelectTrigger><SelectValue /></SelectTrigger>
                                    <SelectContent>
                                        <SelectItem value="paid">Paid (no LOP)</SelectItem>
                                        <SelectItem value="unpaid">Unpaid (full LOP)</SelectItem>
                                        <SelectItem value="half_day">Half-day (50% LOP)</SelectItem>
                                    </SelectContent>
                                </Select>
                            </div>
                            <label className="flex items-center gap-2 pt-6 text-sm">
                                <Checkbox
                                    checked={draft.counts_toward_quota}
                                    onCheckedChange={(v) =>
                                        setDraft({ ...draft, counts_toward_quota: !!v })
                                    }
                                />
                                Counts toward annual leave quota
                            </label>
                        </div>
                        <Button size="sm" onClick={() => void addItem()} disabled={saving === 'new'}>
                            <Plus className="mr-1 h-4 w-4" />
                            {saving === 'new' ? 'Adding…' : 'Add type'}
                        </Button>
                    </div>
                </div>
            )}
        </SettingsLayout>
    );
}

function LeaveTypeRow({
    item,
    saving,
    onSave,
}: {
    item: LeaveType;
    saving: boolean;
    onSave: (patch: Partial<LeaveType>) => void;
}) {
    const [name, setName] = useState(item.name);
    const [paymentType, setPaymentType] = useState(item.payment_type);
    const [quota, setQuota] = useState(item.counts_toward_quota);
    const [active, setActive] = useState(item.is_active);

    useEffect(() => {
        setName(item.name);
        setPaymentType(item.payment_type);
        setQuota(item.counts_toward_quota);
        setActive(item.is_active);
    }, [item]);

    return (
        <tr className="border-t">
            <td className="px-3 py-2">
                <Input value={name} onChange={(e) => setName(e.target.value)} className="h-8" />
            </td>
            <td className="px-3 py-2 text-muted-foreground">{item.slug}</td>
            <td className="px-3 py-2">
                <Select value={paymentType} onValueChange={(v) => setPaymentType(v as LeaveType['payment_type'])}>
                    <SelectTrigger className="h-8"><SelectValue /></SelectTrigger>
                    <SelectContent>
                        <SelectItem value="paid">Paid</SelectItem>
                        <SelectItem value="unpaid">Unpaid (LOP)</SelectItem>
                        <SelectItem value="half_day">Half-day</SelectItem>
                    </SelectContent>
                </Select>
            </td>
            <td className="px-3 py-2">
                <Checkbox checked={quota} onCheckedChange={(v) => setQuota(!!v)} />
            </td>
            <td className="px-3 py-2">
                <Checkbox checked={active} onCheckedChange={(v) => setActive(!!v)} />
            </td>
            <td className="px-3 py-2 text-right">
                <Button
                    size="sm"
                    variant="outline"
                    disabled={saving}
                    onClick={() =>
                        onSave({
                            name,
                            payment_type: paymentType,
                            counts_toward_quota: quota,
                            is_active: active,
                        })
                    }
                >
                    <Save className="h-3 w-3" />
                </Button>
            </td>
        </tr>
    );
}
