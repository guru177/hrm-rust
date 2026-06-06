import axios from '@/lib/axios';
import { useEffect, useState } from 'react';

import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import {
    Select,
    SelectContent,
    SelectItem,
    SelectTrigger,
    SelectValue,
} from '@/components/ui/select';
import { Textarea } from '@/components/ui/textarea';
import { handleApiError, handleApiResponse } from '@/lib/toast';

interface LeaveRequestFormProps {
    onSuccess: () => void;
    onCancel: () => void;
}

export default function LeaveRequestForm({ onSuccess, onCancel }: LeaveRequestFormProps) {
    const [formData, setFormData] = useState({
        leave_type: '',
        start_date: '',
        end_date: '',
        reason: '',
    });
    const [errors, setErrors] = useState<Record<string, string[]>>({});
    const [loading, setLoading] = useState(false);
    const [leaveTypes, setLeaveTypes] = useState<{ value: string; label: string }[]>([]);

    useEffect(() => {
        axios.get('/admin/leave-types').then((res) => {
            if (res.data.success) {
                setLeaveTypes(
                    (res.data.data || []).map((t: { slug: string; name: string; payment_type_label?: string }) => ({
                        value: t.slug,
                        label: t.payment_type_label ? `${t.name} (${t.payment_type_label})` : t.name,
                    })),
                );
            }
        }).catch(() => {
            setLeaveTypes([
                { value: 'sick', label: 'Sick Leave' },
                { value: 'annual', label: 'Annual Leave' },
                { value: 'unpaid', label: 'Unpaid Leave (LOP)' },
            ]);
        });
    }, []);

    const handleSubmit = async (e: React.FormEvent) => {
        e.preventDefault();
        setLoading(true);
        setErrors({});

        try {
            const response = await axios.post('/admin/leave-requests', formData);
            handleApiResponse(response);
            onSuccess();
        } catch (error: any) {
            if (error.response?.data?.errors) {
                setErrors(error.response.data.errors);
            }
            handleApiError(error);
        } finally {
            setLoading(false);
        }
    };

    return (
        <form onSubmit={handleSubmit} className="space-y-4">
            <div className="space-y-2">
                <Label htmlFor="leave_type">
                    Leave Type <span className="text-destructive">*</span>
                </Label>
                <Select
                    value={formData.leave_type}
                    onValueChange={(value) =>
                        setFormData({ ...formData, leave_type: value })
                    }
                >
                    <SelectTrigger>
                        <SelectValue placeholder="Select leave type" />
                    </SelectTrigger>
                    <SelectContent>
                        {leaveTypes.map((type) => (
                            <SelectItem key={type.value} value={type.value}>
                                {type.label}
                            </SelectItem>
                        ))}
                    </SelectContent>
                </Select>
                {errors.leave_type && (
                    <p className="text-sm text-destructive">{errors.leave_type[0]}</p>
                )}
            </div>

            <div className="grid grid-cols-2 gap-4">
                <div className="space-y-2">
                    <Label htmlFor="start_date">
                        Start Date <span className="text-destructive">*</span>
                    </Label>
                    <Input
                        id="start_date"
                        type="date"
                        value={formData.start_date}
                        onChange={(e) =>
                            setFormData({ ...formData, start_date: e.target.value })
                        }
                    />
                    {errors.start_date && (
                        <p className="text-sm text-destructive">{errors.start_date[0]}</p>
                    )}
                </div>

                <div className="space-y-2">
                    <Label htmlFor="end_date">
                        End Date <span className="text-destructive">*</span>
                    </Label>
                    <Input
                        id="end_date"
                        type="date"
                        value={formData.end_date}
                        onChange={(e) =>
                            setFormData({ ...formData, end_date: e.target.value })
                        }
                    />
                    {errors.end_date && (
                        <p className="text-sm text-destructive">{errors.end_date[0]}</p>
                    )}
                </div>
            </div>

            <div className="space-y-2">
                <Label htmlFor="reason">
                    Reason <span className="text-destructive">*</span>
                </Label>
                <Textarea
                    id="reason"
                    value={formData.reason}
                    onChange={(e) =>
                        setFormData({ ...formData, reason: e.target.value })
                    }
                    placeholder="Please provide a detailed reason for your leave request"
                    rows={4}
                />
                {errors.reason && (
                    <p className="text-sm text-destructive">{errors.reason[0]}</p>
                )}
                <p className="text-xs text-muted-foreground">
                    Minimum 10 characters required
                </p>
            </div>

            <div className="flex justify-end gap-3 pt-4">
                <Button type="button" variant="outline" onClick={onCancel} disabled={loading}>
                    Cancel
                </Button>
                <Button type="submit" disabled={loading}>
                    {loading ? 'Submitting...' : 'Submit Request'}
                </Button>
            </div>
        </form>
    );
}
