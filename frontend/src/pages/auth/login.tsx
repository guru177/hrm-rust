import { useState, type FormEvent } from 'react';
import { useNavigate } from 'react-router-dom';
import { useAuth } from '@/contexts/AuthContext';
import InputError from '@/components/input-error';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Spinner } from '@/components/ui/spinner';
import AuthLayout from '@/layouts/auth-layout';
import { defaultAdminRoute } from '@/lib/default-route';

export default function Login() {
    const { login } = useAuth();
    const navigate = useNavigate();
    const [email, setEmail] = useState('');
    const [password, setPassword] = useState('');
    const [processing, setProcessing] = useState(false);
    const [error, setError] = useState('');

    async function handleSubmit(e: FormEvent) {
        e.preventDefault();
        setProcessing(true);
        setError('');

        try {
            const perms = await login(email, password);
            const has = (slug: string) => perms.includes('*') || perms.includes(slug);
            navigate(defaultAdminRoute(has), { replace: true });
        } catch (err: any) {
            setError(err.message || 'Invalid credentials');
        } finally {
            setProcessing(false);
        }
    }

    return (
        <AuthLayout
            title="Welcome back"
            description="Enter your credentials to access your account"
        >
            <title>Log in — HRM Portal</title>

            {error && (
                <div className="rounded-lg bg-red-50 p-3 text-sm text-red-800 dark:bg-red-900/20 dark:text-red-400">
                    {error}
                </div>
            )}

            <form onSubmit={handleSubmit} className="space-y-6">
                <div className="space-y-4">
                    <div className="space-y-2">
                        <Label htmlFor="email">Email address</Label>
                        <Input
                            id="email"
                            type="email"
                            value={email}
                            onChange={(e) => setEmail(e.target.value)}
                            required
                            autoFocus
                            tabIndex={1}
                            autoComplete="email"
                            placeholder="name@company.com"
                            className="h-11"
                        />
                    </div>

                    <div className="space-y-2">
                        <Label htmlFor="password">Password</Label>
                        <Input
                            id="password"
                            type="password"
                            value={password}
                            onChange={(e) => setPassword(e.target.value)}
                            required
                            tabIndex={2}
                            autoComplete="current-password"
                            placeholder="••••••••"
                            className="h-11"
                        />
                    </div>
                </div>

                <Button
                    type="submit"
                    className="h-11 w-full"
                    tabIndex={4}
                    disabled={processing}
                    data-test="login-button"
                >
                    {processing && <Spinner size="sm" />}
                    {processing ? 'Signing in...' : 'Sign in'}
                </Button>
            </form>
        </AuthLayout>
    );
}
