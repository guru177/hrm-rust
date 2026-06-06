import { useNavigate } from 'react-router-dom';
import { useAuth } from '@/contexts/AuthContext';
import axios from '@/lib/axios';
import { Clock, LogIn, LogOut, Calendar, Timer } from 'lucide-react';
import { useState, useEffect } from 'react';

import AttendanceStats from '@/components/attendance/attendance-stats';
import AttendanceTable from '@/components/attendance/attendance-table';
import ClockInFaceDialog, {
    type ClockInVerificationPayload,
} from '@/components/attendance/clock-in-face-dialog';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import AppLayout from '@/layouts/app-layout';
import { handleApiError, handleApiResponse } from '@/lib/toast';
import { usePermissions } from '@/hooks/use-permissions';
import { type SharedData } from '@/types';

export default function AttendancePage() {
    const { user } = useAuth();
    const { hasPermission } = usePermissions();
    const navigate = useNavigate();
    const [todayData, setTodayData] = useState<any>(null);
    const [stats, setStats] = useState<any>(null);
    const [loading, setLoading] = useState(true);
    const [clockingIn, setClockingIn] = useState(false);
    const [clockingOut, setClockingOut] = useState(false);
    const [clockInOpen, setClockInOpen] = useState(false);
    const [elapsedTime, setElapsedTime] = useState(0);

    // Get active clock-in (one without clock-out)
    const activeClockIn = todayData?.active_clock_in;
    // Get the latest completed session
    const latestSession = todayData?.attendances?.[0];
    const allSessions = todayData?.attendances || [];

    useEffect(() => {
        loadData();
    }, []);

    // Timer effect for active session
    useEffect(() => {
        if (!activeClockIn?.clock_in || activeClockIn?.clock_out) return;

        const interval = setInterval(() => {
            const clockInTime = new Date(activeClockIn.clock_in).getTime();
            const now = new Date().getTime();
            const elapsed = Math.floor((now - clockInTime) / 1000); // in seconds
            setElapsedTime(elapsed);
        }, 1000);

        return () => clearInterval(interval);
    }, [activeClockIn?.clock_in, activeClockIn?.clock_out]);

    const loadData = async () => {
        setLoading(true);
        try {
            const [todayRes, statsRes] = await Promise.all([
                axios.get('/admin/attendance/today'),
                axios.get('/admin/attendance/stats'),
            ]);

            setTodayData(todayRes.data.data);
            setStats(statsRes.data.data);
        } catch (error) {
            handleApiError(error);
        } finally {
            setLoading(false);
        }
    };

    const handleClockIn = async (payload: ClockInVerificationPayload) => {
        setClockingIn(true);
        try {
            const response = await axios.post('/admin/attendance/clock-in', payload);
            handleApiResponse(response);
            // Reload attendance data to get updated list
            await loadData();
            setClockInOpen(false);
        } catch (error) {
            handleApiError(error);
        } finally {
            setClockingIn(false);
        }
    };

    const handleClockOut = async () => {
        setClockingOut(true);
        try {
            const response = await axios.post('/admin/attendance/clock-out');
            handleApiResponse(response);
            // Reload attendance data to get updated list
            await loadData();
        } catch (error) {
            handleApiError(error);
        } finally {
            setClockingOut(false);
        }
    };

    const breadcrumbs = [{ label: 'Attendance', href: '/admin/attendance' }];
    const userPhotoUrl = user?.photo
        ? user.photo.startsWith('http')
            ? user.photo
            : `/storage/${user.photo}`
        : null;

    const todayShift = todayData?.shift;
    const formatShiftTime = (value?: string) => {
        if (!value) return '--:--';
        const part = value.slice(0, 5);
        const [h, m] = part.split(':').map(Number);
        if (Number.isNaN(h) || Number.isNaN(m)) return value;
        const d = new Date();
        d.setHours(h, m, 0, 0);
        return d.toLocaleTimeString('en-US', { hour: '2-digit', minute: '2-digit', hour12: true });
    };

    const formatElapsedTime = (seconds: number) => {
        const hours = Math.floor(seconds / 3600);
        const minutes = Math.floor((seconds % 3600) / 60);
        const secs = seconds % 60;
        return `${hours.toString().padStart(2, '0')}:${minutes.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}`;
    };

    // Calculate total duration across all sessions today (including active session)
    const calculateTotalDuration = () => {
        let totalSeconds = 0;

        // Add duration from all completed sessions
        allSessions.forEach((session: any) => {
            if (session.duration_minutes) {
                totalSeconds += session.duration_minutes * 60;
            }
        });

        // Add current active session elapsed time
        if (activeClockIn && !activeClockIn.clock_out) {
            totalSeconds += elapsedTime;
        }

        return totalSeconds;
    };

    const totalDurationSeconds = calculateTotalDuration();

    if (loading) {
        return (
            <AppLayout breadcrumbs={breadcrumbs}>
                
                <div className="flex items-center justify-center min-h-96">
                    <div className="h-8 w-8 animate-spin rounded-full border-4 border-primary border-t-transparent" />
                </div>
            </AppLayout>
        );
    }

    return (
        <AppLayout breadcrumbs={breadcrumbs}>
            

            <div className="space-y-6">
                {/* Header */}
                <div className="flex items-start justify-between gap-4">
                    <div>
                        <h1 className="text-3xl font-bold tracking-tight flex items-center gap-2">
                            <Calendar className="h-8 w-8 text-primary" />
                            Attendance
                        </h1>
                        <p className="text-muted-foreground mt-1">
                            Track your daily clock-in and clock-out times
                        </p>
                    </div>
                    {hasPermission('manage-leave-requests') && (
                        <Button onClick={() => navigate('/admin/leave-requests/manage')}>
                            Leave Requests
                        </Button>
                    )}
                </div>

                {/* Today's Attendance Card */}
                <Card>
                    <CardHeader>
                        <div className="flex items-center justify-between">
                            <div>
                                <CardTitle>Today's Attendance</CardTitle>
                                {todayData?.total_sessions > 0 && (
                                    <p className="text-sm text-muted-foreground mt-1">
                                        {todayData.total_sessions} session{todayData.total_sessions > 1 ? 's' : ''} today
                                    </p>
                                )}
                            </div>
                            {activeClockIn && (
                                <Badge variant="default">Active Session</Badge>
                            )}
                        </div>
                    </CardHeader>
                    <CardContent>
                        <div className="space-y-6">
                            {todayShift && (
                                <div className="flex flex-wrap items-center gap-3 rounded-lg border bg-muted/30 px-4 py-3 text-sm">
                                    <Timer className="h-4 w-4 text-primary" />
                                    <span className="font-medium">
                                        Today&apos;s Shift: {todayShift.template_name || 'Default Shift'}
                                    </span>
                                    <span className="text-muted-foreground">
                                        {formatShiftTime(todayShift.start_time)} – {formatShiftTime(todayShift.end_time)}
                                    </span>
                                    {(todayShift.grace_in_minutes > 0 || todayShift.grace_out_minutes > 0) && (
                                        <span className="text-xs text-muted-foreground">
                                            Grace: +{todayShift.grace_in_minutes}m in / -{todayShift.grace_out_minutes}m out
                                        </span>
                                    )}
                                </div>
                            )}

                            {/* Total Duration Counter (Green) */}
                            {totalDurationSeconds > 0 && (
                                <div className="p-6 bg-gradient-to-r from-green-50 to-emerald-50 dark:from-green-950/50 dark:to-emerald-950/50 rounded-lg border-2 border-green-200 dark:border-green-800/50">
                                    <div className="flex items-center justify-between mb-2">
                                        <p className="text-sm text-muted-foreground font-medium">
                                            TOTAL TIME TODAY
                                        </p>
                                        {activeClockIn && !activeClockIn.clock_out && (
                                            <span className="flex items-center gap-2 text-xs text-green-600 dark:text-green-400">
                                                <span className="inline-block w-2 h-2 bg-green-600 dark:bg-green-400 rounded-full animate-pulse"></span>
                                                Active
                                            </span>
                                        )}
                                    </div>
                                    <p className="text-5xl font-bold text-green-600 dark:text-green-400 font-mono">
                                        {formatElapsedTime(totalDurationSeconds)}
                                    </p>
                                    {todayData?.total_sessions > 1 && (
                                        <p className="text-xs text-muted-foreground mt-2">
                                            Across {todayData.total_sessions} sessions
                                        </p>
                                    )}
                                </div>
                            )}

                            {/* Clock In/Out Status */}
                            <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
                                {/* Clock In */}
                                <div className="p-4 border rounded-lg">
                                    <div className="flex items-center justify-between mb-3">
                                        <span className="text-sm font-medium text-muted-foreground">
                                            {activeClockIn ? 'Current Clock In' : 'Last Clock In'}
                                        </span>
                                        <LogIn className="h-4 w-4 text-green-600" />
                                    </div>
                                    <p className="text-2xl font-bold">
                                        {activeClockIn?.clock_in
                                            ? new Date(activeClockIn.clock_in).toLocaleTimeString('en-US', {
                                                hour: '2-digit',
                                                minute: '2-digit',
                                                hour12: true,
                                            })
                                            : latestSession?.clock_in
                                                ? new Date(latestSession.clock_in).toLocaleTimeString('en-US', {
                                                    hour: '2-digit',
                                                    minute: '2-digit',
                                                    hour12: true,
                                                })
                                                : '--:--'}
                                    </p>
                                    {activeClockIn?.is_late && (
                                        <p className="text-xs text-red-600 mt-2">
                                            ⚠ Late arrival
                                        </p>
                                    )}
                                    {activeClockIn && (
                                        <p className="text-xs text-green-600 mt-2">
                                            ● Active now
                                        </p>
                                    )}
                                </div>

                                {/* Clock Out */}
                                <div className="p-4 border rounded-lg">
                                    <div className="flex items-center justify-between mb-3">
                                        <span className="text-sm font-medium text-muted-foreground">
                                            {activeClockIn ? 'Current Clock Out' : 'Last Clock Out'}
                                        </span>
                                        <LogOut className="h-4 w-4 text-red-600" />
                                    </div>
                                    <p className="text-2xl font-bold">
                                        {activeClockIn?.clock_out
                                            ? new Date(activeClockIn.clock_out).toLocaleTimeString('en-US', {
                                                hour: '2-digit',
                                                minute: '2-digit',
                                                hour12: true,
                                            })
                                            : latestSession?.clock_out
                                                ? new Date(latestSession.clock_out).toLocaleTimeString('en-US', {
                                                    hour: '2-digit',
                                                    minute: '2-digit',
                                                    hour12: true,
                                                })
                                                : '--:--'}
                                    </p>
                                    {(activeClockIn?.is_early_exit || latestSession?.is_early_exit) && (
                                        <p className="text-xs text-orange-600 mt-2">
                                            ⚠ Early exit
                                        </p>
                                    )}
                                </div>

                                {/* Duration */}
                                <div className="p-4 border rounded-lg">
                                    <div className="flex items-center justify-between mb-3">
                                        <span className="text-sm font-medium text-muted-foreground">
                                            {activeClockIn && !activeClockIn.clock_out ? 'Current Session' : 'Last Session'}
                                        </span>
                                        <Clock className="h-4 w-4 text-blue-600" />
                                    </div>
                                    <p className="text-2xl font-bold text-blue-600 dark:text-blue-400 font-mono">
                                        {activeClockIn && !activeClockIn.clock_out
                                            ? formatElapsedTime(elapsedTime)
                                            : latestSession?.duration_minutes
                                                ? `${Math.floor(latestSession.duration_minutes / 60)}h ${latestSession.duration_minutes % 60}m`
                                                : '--:--'}
                                    </p>
                                </div>
                            </div>

                            {/* Action Buttons */}
                            <div className="flex gap-3">
                                <Button
                                    onClick={() => setClockInOpen(true)}
                                    disabled={clockingIn}
                                    className="flex-1"
                                    size="lg"
                                    variant={activeClockIn ? 'outline' : 'default'}
                                >
                                    <LogIn className="mr-2 h-4 w-4" />
                                    {clockingIn ? 'Clocking In...' : activeClockIn ? 'Start New Session' : 'Clock In'}
                                </Button>
                                <Button
                                    onClick={handleClockOut}
                                    disabled={
                                        !activeClockIn ||
                                        clockingOut
                                    }
                                    className="flex-1"
                                    size="lg"
                                    variant={!activeClockIn ? 'outline' : 'default'}
                                >
                                    <LogOut className="mr-2 h-4 w-4" />
                                    {clockingOut ? 'Clocking Out...' : 'Clock Out'}
                                </Button>
                            </div>
                        </div>
                    </CardContent>
                </Card>

                {/* Tabs */}
                <Tabs defaultValue="statistics" className="w-full">
                    <TabsList className="grid w-full grid-cols-2">
                        <TabsTrigger value="statistics">Statistics</TabsTrigger>
                        <TabsTrigger value="history">History</TabsTrigger>
                    </TabsList>

                    <TabsContent value="statistics" className="space-y-4">
                        <AttendanceStats stats={stats} />
                    </TabsContent>

                    <TabsContent value="history" className="space-y-4">
                        <AttendanceTable />
                    </TabsContent>
                </Tabs>
            </div>

            <ClockInFaceDialog
                open={clockInOpen}
                onOpenChange={setClockInOpen}
                onVerify={handleClockIn}
                userPhotoUrl={userPhotoUrl}
                busy={clockingIn}
            />
        </AppLayout>
    );
}
