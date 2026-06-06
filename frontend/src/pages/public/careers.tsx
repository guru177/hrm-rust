import axios from '@/lib/axios';
import { Briefcase, MapPin } from 'lucide-react';
import { useEffect, useState } from 'react';
import { Link } from 'react-router-dom';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Textarea } from '@/components/ui/textarea';
import { handleApiError, handleApiResponse } from '@/lib/toast';

interface JobPosting {
    id: number;
    title: string;
    department?: string;
    location?: string;
    employment_type?: string;
    description?: string;
    salary_range?: string;
}

export default function PublicCareersPage() {
    const [jobs, setJobs] = useState<JobPosting[]>([]);
    const [selected, setSelected] = useState<JobPosting | null>(null);
    const [submitting, setSubmitting] = useState(false);
    const [form, setForm] = useState({ name: '', email: '', phone: '', cover_letter: '', resume_url: '' });
    const [tracking, setTracking] = useState('');

    useEffect(() => {
        void loadJobs();
    }, []);

    const loadJobs = async () => {
        try {
            const res = await axios.get('/public/careers');
            setJobs(res.data.data || []);
        } catch (error) {
            handleApiError(error);
        }
    };

    const submitApplication = async () => {
        if (!selected || !form.name || !form.email) return;
        setSubmitting(true);
        try {
            const res = await axios.post('/public/careers/apply', {
                career_id: selected.id,
                ...form,
            });
            handleApiResponse(res);
            setTracking(res.data.data?.tracking_number || '');
            setForm({ name: '', email: '', phone: '', cover_letter: '', resume_url: '' });
        } catch (error) {
            handleApiError(error);
        } finally {
            setSubmitting(false);
        }
    };

    return (
        <div className="min-h-screen bg-background">
            <header className="border-b px-6 py-4 flex items-center justify-between">
                <div className="flex items-center gap-2 font-semibold text-lg">
                    <Briefcase className="h-5 w-5" />
                    Raintech Careers
                </div>
                <Button variant="outline" asChild>
                    <Link to="/login">Employee Login</Link>
                </Button>
            </header>

            <main className="max-w-5xl mx-auto p-6 space-y-6">
                <div>
                    <h1 className="text-3xl font-bold">Open Positions</h1>
                    <p className="text-muted-foreground mt-1">Apply directly — no account required</p>
                </div>

                {tracking && (
                    <Card className="border-green-200 bg-green-50">
                        <CardContent className="pt-6">
                            Application submitted! Tracking number: <strong>{tracking}</strong>
                        </CardContent>
                    </Card>
                )}

                <div className="grid gap-4 md:grid-cols-2">
                    {jobs.map((job) => (
                        <Card
                            key={job.id}
                            className={`cursor-pointer transition-shadow hover:shadow-md ${selected?.id === job.id ? 'ring-2 ring-primary' : ''}`}
                            onClick={() => setSelected(job)}
                        >
                            <CardHeader>
                                <CardTitle>{job.title}</CardTitle>
                                <CardDescription className="flex flex-wrap gap-2 items-center">
                                    {job.location && (
                                        <span className="flex items-center gap-1">
                                            <MapPin className="h-3 w-3" /> {job.location}
                                        </span>
                                    )}
                                    {job.employment_type && <Badge variant="outline">{job.employment_type}</Badge>}
                                </CardDescription>
                            </CardHeader>
                            {job.salary_range && (
                                <CardContent className="text-sm text-muted-foreground">{job.salary_range}</CardContent>
                            )}
                        </Card>
                    ))}
                </div>

                {selected && (
                    <Card>
                        <CardHeader>
                            <CardTitle>Apply for {selected.title}</CardTitle>
                            <CardDescription>{selected.description?.slice(0, 200)}</CardDescription>
                        </CardHeader>
                        <CardContent className="space-y-4">
                            <div className="grid gap-4 sm:grid-cols-2">
                                <div className="space-y-2">
                                    <Label>Full Name *</Label>
                                    <Input value={form.name} onChange={(e) => setForm({ ...form, name: e.target.value })} />
                                </div>
                                <div className="space-y-2">
                                    <Label>Email *</Label>
                                    <Input type="email" value={form.email} onChange={(e) => setForm({ ...form, email: e.target.value })} />
                                </div>
                                <div className="space-y-2">
                                    <Label>Phone</Label>
                                    <Input value={form.phone} onChange={(e) => setForm({ ...form, phone: e.target.value })} />
                                </div>
                            </div>
                            <div className="space-y-2">
                                <Label>Resume URL (Google Drive, LinkedIn, etc.)</Label>
                                <Input
                                    placeholder="https://..."
                                    value={form.resume_url}
                                    onChange={(e) => setForm({ ...form, resume_url: e.target.value })}
                                />
                            </div>
                            <div className="space-y-2">
                                <Label>Cover Letter</Label>
                                <Textarea rows={4} value={form.cover_letter} onChange={(e) => setForm({ ...form, cover_letter: e.target.value })} />
                            </div>
                            <Button onClick={submitApplication} disabled={submitting || !form.name || !form.email}>
                                {submitting ? 'Submitting...' : 'Submit Application'}
                            </Button>
                        </CardContent>
                    </Card>
                )}
            </main>
        </div>
    );
}
