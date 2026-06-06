import * as faceapi from '@vladmandic/face-api';
import { Camera, CheckCircle2, Loader2, MapPin, RefreshCw, ShieldAlert, XCircle } from 'lucide-react';
import { useEffect, useRef, useState } from 'react';

import { Button } from '@/components/ui/button';
import {
    Dialog,
    DialogContent,
    DialogDescription,
    DialogFooter,
    DialogHeader,
    DialogTitle,
} from '@/components/ui/dialog';
import { Spinner } from '@/components/ui/spinner';

// Prefer local models in public/face-models; fallback to CDN when missing.
const FACE_MODEL_PATH_CANDIDATES = [
    '/face-models',
    'https://cdn.jsdelivr.net/npm/@vladmandic/face-api/model',
    'https://unpkg.com/@vladmandic/face-api/model',
];
// Euclidean distance threshold for the 128-dim face recognition embedding.
// Same person (photo vs live): typically < 0.45 | Different person: typically > 0.55
const FACE_DISTANCE_THRESHOLD = 0.50;
// Number of consecutive camera frames that must all pass
const VERIFICATION_FRAMES = 3;

type GeoLocationPayload = {
    lat: number;
    lng: number;
    accuracy?: number | null;
};

type IpLocationPayload = {
    ip?: string;
    city?: string;
    region?: string;
    country?: string;
    lat?: number | null;
    lng?: number | null;
};

type LocationPayload = {
    geo: GeoLocationPayload;
    ip: IpLocationPayload;
};

export type ClockInVerificationPayload = {
    face_verified: boolean;
    face_match_score: number | null;
    location: LocationPayload;
};

type ClockInFaceDialogProps = {
    open: boolean;
    onOpenChange: (open: boolean) => void;
    onVerify: (payload: ClockInVerificationPayload) => void;
    userPhotoUrl: string | null;
    busy?: boolean;
};

export default function ClockInFaceDialog({
    open,
    onOpenChange,
    onVerify,
    userPhotoUrl,
    busy = false,
}: ClockInFaceDialogProps) {
    const videoRef = useRef<HTMLVideoElement | null>(null);
    const canvasRef = useRef<HTMLCanvasElement | null>(null);
    const modelsLoadedRef = useRef(false);
    const referenceDescriptorRef = useRef<Float32Array | null>(null);

    const [modelsLoading, setModelsLoading] = useState(false);
    const [modelsReady, setModelsReady] = useState(false);
    const [cameraReady, setCameraReady] = useState(false);
    const [cameraStatus, setCameraStatus] = useState<'idle' | 'pending' | 'granted' | 'denied'>('idle');
    const [locationStatus, setLocationStatus] = useState<'idle' | 'pending' | 'granted' | 'denied'>('idle');
    const [locationLoading, setLocationLoading] = useState(false);
    const [locationData, setLocationData] = useState<LocationPayload | null>(null);
    const [error, setError] = useState<string | null>(null);
    const [permissionsGranted, setPermissionsGranted] = useState(false);
    const [debugInfo, setDebugInfo] = useState<{ frames: number[]; avg: number; passed: boolean } | null>(null);
    const activeModelPathRef = useRef<string | null>(null);

    const skipFaceVerification = !userPhotoUrl;
    const isReady = locationData && (skipFaceVerification || (modelsReady && cameraReady));

    useEffect(() => {
        if (!open) {
            stopCamera();
            setError(null);
            setLocationData(null);
            setCameraReady(false);
            setCameraStatus('idle');
            setLocationStatus('idle');
            setPermissionsGranted(false);
            return;
        }

        setError(null);
        if (modelsLoadedRef.current) {
            setModelsReady(true);
        }
        // Check existing permission states without prompting
        void checkExistingPermissions();
    }, [open]);

    // Once required permissions are granted, proceed with loading
    useEffect(() => {
        if (skipFaceVerification && locationStatus === 'granted') {
            setPermissionsGranted(true);
        } else if (cameraStatus === 'granted' && locationStatus === 'granted') {
            setPermissionsGranted(true);
        }
    }, [cameraStatus, locationStatus, skipFaceVerification]);

    const checkExistingPermissions = async () => {
        try {
            const cameraPerm = await navigator.permissions.query({ name: 'camera' as PermissionName });
            if (cameraPerm.state === 'granted') {
                setCameraStatus('granted');
                void startCamera();
                void loadModels();
            }
        } catch {
            // permissions API may not support camera query in all browsers
        }
        try {
            const geoPerm = await navigator.permissions.query({ name: 'geolocation' });
            if (geoPerm.state === 'granted') {
                setLocationStatus('granted');
                void loadLocation();
            }
        } catch {
            // ignore
        }
    };

    const requestCameraPermission = async () => {
        setCameraStatus('pending');
        setError(null);
        await startCamera();
    };

    const requestLocationPermission = async () => {
        setLocationStatus('pending');
        setError(null);
        await loadLocation();
    };

    const requestAllPermissions = async () => {
        setError(null);
        if (!skipFaceVerification && cameraStatus !== 'granted') {
            setCameraStatus('pending');
        }
        if (locationStatus !== 'granted') {
            setLocationStatus('pending');
        }
        await Promise.all([
            !skipFaceVerification && cameraStatus !== 'granted' ? startCamera() : Promise.resolve(),
            locationStatus !== 'granted' ? loadLocation() : Promise.resolve(),
        ]);
    };

    const handleClockInWithoutFace = () => {
        if (!locationData) return;
        onVerify({
            face_verified: false,
            face_match_score: null,
            location: locationData,
        });
    };

    const startCamera = async () => {
        try {
            const stream = await navigator.mediaDevices.getUserMedia({
                video: { facingMode: 'user' },
                audio: false,
            });
            if (videoRef.current) {
                videoRef.current.srcObject = stream;
                await videoRef.current.play();
                setCameraReady(true);
                setCameraStatus('granted');
                void loadModels();
            }
        } catch {
            setCameraStatus('denied');
            setError('Camera permission was denied. Please allow camera access in your browser settings and try again.');
        }
    };

    const stopCamera = () => {
        const stream = videoRef.current?.srcObject as MediaStream | null;
        stream?.getTracks().forEach((track) => track.stop());
        if (videoRef.current) {
            videoRef.current.srcObject = null;
        }
    };

    const loadImageElement = async (src: string): Promise<HTMLImageElement> => {
        return await new Promise((resolve, reject) => {
            const image = new Image();
            image.crossOrigin = 'anonymous';
            image.onload = () => resolve(image);
            image.onerror = () => reject(new Error('Failed to load image'));
            image.src = src;
        });
    };

    const resolveWorkingModelPath = async (): Promise<string | null> => {
        const probeFile = 'ssd_mobilenetv1_model-weights_manifest.json';
        for (const candidate of FACE_MODEL_PATH_CANDIDATES) {
            try {
                const res = await fetch(`${candidate}/${probeFile}`, {
                    method: 'GET',
                    cache: 'no-store',
                });
                if (!res.ok) continue;
                const text = await res.text();
                // Vite can return index.html for missing local assets (200 + <!DOCTYPE...>).
                // Validate this is actual manifest JSON before selecting the candidate.
                if (text.trimStart().startsWith('<')) continue;
                const parsed = JSON.parse(text) as Array<{ paths?: string[]; weights?: unknown }>;
                if (Array.isArray(parsed) && parsed.length > 0 && parsed[0]?.weights) {
                    const firstShard = parsed[0]?.paths?.[0];
                    if (!firstShard) continue;
                    // Ensure shard binary is also reachable, not just the manifest.
                    const shardRes = await fetch(`${candidate}/${firstShard}`, {
                        method: 'GET',
                        cache: 'no-store',
                    });
                    if (!shardRes.ok) continue;
                    return candidate;
                }
            } catch {
                // keep trying next candidate
            }
        }
        return null;
    };

    const loadModels = async () => {
        if (modelsLoadedRef.current || modelsLoading) {
            return;
        }

        if (!userPhotoUrl) {
            return;
        }

        setModelsLoading(true);
        try {
            const modelPath = activeModelPathRef.current ?? (await resolveWorkingModelPath());
            if (!modelPath) {
                setError(
                    'Face models are unavailable. Add model files to frontend/public/face-models or allow internet access for CDN fallback.',
                );
                return;
            }
            activeModelPathRef.current = modelPath;

            // Load required nets (detection + landmarks + recognition embedding)
            await Promise.all([
                faceapi.nets.ssdMobilenetv1.loadFromUri(modelPath),
                faceapi.nets.faceLandmark68Net.loadFromUri(modelPath),
                faceapi.nets.faceRecognitionNet.loadFromUri(modelPath),
            ]);

            // Load reference image directly; user photos are served from same app domain.
            const referenceImage = await loadImageElement(userPhotoUrl);

            const detection = await faceapi
                .detectSingleFace(referenceImage, new faceapi.SsdMobilenetv1Options({ minConfidence: 0.5 }))
                .withFaceLandmarks()
                .withFaceDescriptor();

            if (!detection) {
                setError('No face detected in your profile photo. Please upload a clear front-facing photo.');
                return;
            }

            referenceDescriptorRef.current = detection.descriptor;
            modelsLoadedRef.current = true;
            setModelsReady(true);
        } catch (e) {
            console.error('Face model init failed:', e);
            setError(
                `Unable to initialize face detection. Model source: ${activeModelPathRef.current ?? 'not resolved'}.`,
            );
        } finally {
            setModelsLoading(false);
        }
    };

    const loadLocation = async () => {
        if (locationLoading) {
            return;
        }

        setLocationLoading(true);
        try {
            // Step 1: Request geolocation permission (this triggers the browser prompt)
            const geo = await new Promise<GeoLocationPayload>((resolve, reject) => {
                navigator.geolocation.getCurrentPosition(
                    (position) => {
                        resolve({
                            lat: position.coords.latitude,
                            lng: position.coords.longitude,
                            accuracy: position.coords.accuracy,
                        });
                    },
                    (geoError) => {
                        reject(geoError);
                    },
                    { enableHighAccuracy: true, timeout: 15000 },
                );
            });

            // Geolocation succeeded — mark permission as granted immediately
            setLocationStatus('granted');

            // Step 2: IP lookup is optional — don't fail if it errors
            let ipInfo: IpLocationPayload = {
                ip: 'unknown',
                city: undefined,
                region: undefined,
                country: undefined,
                lat: null,
                lng: null,
            };

            try {
                const ipResponse = await fetch('https://ipapi.co/json/', {
                    signal: AbortSignal.timeout(10000),
                    headers: { 'Accept': 'application/json' }
                });
                if (ipResponse.ok) {
                    const ipData = (await ipResponse.json()) as {
                        ip?: string;
                        city?: string;
                        region?: string;
                        country_name?: string;
                        latitude?: number;
                        longitude?: number;
                        error?: boolean;
                    };
                    if (!ipData.error && ipData.ip) {
                        ipInfo = {
                            ip: ipData.ip,
                            city: ipData.city,
                            region: ipData.region,
                            country: ipData.country_name,
                            lat: ipData.latitude ?? null,
                            lng: ipData.longitude ?? null,
                        };
                    }
                }
            } catch {
                // IP lookup failed — use default 'unknown'
            }

            setLocationData({ geo, ip: ipInfo });
        } catch (err) {
            // Only the geolocation call can reach here
            const geoErr = err as GeolocationPositionError | undefined;
            if (geoErr && 'code' in geoErr && geoErr.code === 1) {
                // PERMISSION_DENIED
                setLocationStatus('denied');
                setError('Location permission was denied. Please allow location access in your browser settings and try again.');
            } else if (geoErr && 'code' in geoErr && geoErr.code === 3) {
                // TIMEOUT
                setLocationStatus('denied');
                setError('Location request timed out. Please check your device location settings and try again.');
            } else {
                setLocationStatus('denied');
                setError('Unable to determine your location. Please ensure location services are enabled and try again.');
            }
        } finally {
            setLocationLoading(false);
        }
    };

    const handleVerify = async () => {
        if (!referenceDescriptorRef.current) {
            setError('Face reference data is not ready.');
            return;
        }

        if (!locationData) {
            setError('Location permission is required to clock in.');
            return;
        }

        if (!videoRef.current || !canvasRef.current) {
            setError('Camera is not ready.');
            return;
        }

        setError(null);
        setDebugInfo(null);

        const video = videoRef.current;
        const canvas = canvasRef.current;
        const context = canvas.getContext('2d');
        if (!context) {
            setError('Unable to capture camera frame.');
            return;
        }

        // Capture VERIFICATION_FRAMES frames and average the distances.
        // This reduces single-frame noise and makes spoofing harder.
        const distances: number[] = [];
        for (let frame = 0; frame < VERIFICATION_FRAMES; frame++) {
            if (frame > 0) {
                await new Promise<void>((r) => setTimeout(r, 150));
            }

            canvas.width = video.videoWidth;
            canvas.height = video.videoHeight;
            context.drawImage(video, 0, 0, canvas.width, canvas.height);

            const detection = await faceapi
                .detectSingleFace(canvas, new faceapi.SsdMobilenetv1Options({ minConfidence: 0.5 }))
                .withFaceLandmarks()
                .withFaceDescriptor();

            if (!detection) {
                setError(`No face detected in frame ${frame + 1}. Keep your face centred and well-lit.`);
                return;
            }

            distances.push(faceapi.euclideanDistance(referenceDescriptorRef.current, detection.descriptor));
        }

        const avgDistance = distances.reduce((a, b) => a + b, 0) / distances.length;
        // Score reported as 1 - distance (1 = perfect match, 0 = completely different)
        const matchScore = Number(Math.max(0, 1 - avgDistance).toFixed(4));
        const isMatch = avgDistance <= FACE_DISTANCE_THRESHOLD;

        setDebugInfo({ frames: distances, avg: avgDistance, passed: isMatch });

        if (!isMatch) {
            setError('Face does not match the profile photo.');
            return;
        }

        onVerify({
            face_verified: true,
            face_match_score: matchScore,
            location: locationData,
        });
    };

    const PermissionStatusIcon = ({ status }: { status: 'idle' | 'pending' | 'granted' | 'denied' }) => {
        switch (status) {
            case 'pending':
                return <Loader2 className="h-4 w-4 animate-spin text-muted-foreground" />;
            case 'granted':
                return <CheckCircle2 className="h-4 w-4 text-green-600 dark:text-green-400" />;
            case 'denied':
                return <XCircle className="h-4 w-4 text-destructive" />;
            default:
                return <div className="h-4 w-4 rounded-full border-2 border-muted-foreground/40" />;
        }
    };

    return (
        <Dialog open={open} onOpenChange={onOpenChange}>
            <DialogContent className="sm:max-w-3xl">
                <DialogHeader>
                    <DialogTitle>{skipFaceVerification ? 'Confirm clock in' : 'Verify your face'}</DialogTitle>
                    <DialogDescription>
                        {skipFaceVerification
                            ? 'Location permission is required to clock in. Face verification is optional when no profile photo is set.'
                            : permissionsGranted
                                ? 'Position your face in the camera and click verify.'
                                : 'Camera and location permissions are required to clock in. Please grant access to continue.'}
                    </DialogDescription>
                </DialogHeader>

                {/* Permission prompt step */}
                {!permissionsGranted && (
                    <div className="space-y-4">
                        <div className="rounded-lg border bg-muted/30 p-4 space-y-3">
                            <div className="flex items-center gap-2 text-sm font-medium">
                                <ShieldAlert className="h-4 w-4 text-primary" />
                                Permissions Required
                            </div>

                            {/* Camera permission */}
                            <div className="flex items-center justify-between rounded-md border bg-background px-4 py-3">
                                <div className="flex items-center gap-3">
                                    <PermissionStatusIcon status={cameraStatus} />
                                    <div className="flex items-center gap-2">
                                        <Camera className="h-4 w-4 text-muted-foreground" />
                                        <div>
                                            <p className="text-sm font-medium">Camera Access</p>
                                            <p className="text-xs text-muted-foreground">
                                                {cameraStatus === 'granted'
                                                    ? 'Permission granted'
                                                    : cameraStatus === 'denied'
                                                        ? 'Permission denied — check browser settings'
                                                        : cameraStatus === 'pending'
                                                            ? 'Requesting permission...'
                                                            : 'Required for face verification'}
                                            </p>
                                        </div>
                                    </div>
                                </div>
                                {(cameraStatus === 'idle' || cameraStatus === 'denied') && (
                                    <Button
                                        type="button"
                                        variant="outline"
                                        size="sm"
                                        onClick={requestCameraPermission}
                                    >
                                        {cameraStatus === 'denied' ? (
                                            <><RefreshCw className="mr-1.5 h-3 w-3" /> Retry</>
                                        ) : (
                                            'Allow'
                                        )}
                                    </Button>
                                )}
                            </div>

                            {/* Location permission */}
                            <div className="flex items-center justify-between rounded-md border bg-background px-4 py-3">
                                <div className="flex items-center gap-3">
                                    <PermissionStatusIcon status={locationStatus} />
                                    <div className="flex items-center gap-2">
                                        <MapPin className="h-4 w-4 text-muted-foreground" />
                                        <div>
                                            <p className="text-sm font-medium">Location Access</p>
                                            <p className="text-xs text-muted-foreground">
                                                {locationStatus === 'granted'
                                                    ? 'Permission granted'
                                                    : locationStatus === 'denied'
                                                        ? 'Permission denied — check browser settings'
                                                        : locationStatus === 'pending'
                                                            ? 'Requesting permission...'
                                                            : 'Required for attendance verification'}
                                            </p>
                                        </div>
                                    </div>
                                </div>
                                {(locationStatus === 'idle' || locationStatus === 'denied') && (
                                    <Button
                                        type="button"
                                        variant="outline"
                                        size="sm"
                                        onClick={requestLocationPermission}
                                    >
                                        {locationStatus === 'denied' ? (
                                            <><RefreshCw className="mr-1.5 h-3 w-3" /> Retry</>
                                        ) : (
                                            'Allow'
                                        )}
                                    </Button>
                                )}
                            </div>
                        </div>

                        {/* Grant All button */}
                        {(cameraStatus !== 'granted' || locationStatus !== 'granted') && (
                            <Button
                                type="button"
                                className="w-full"
                                onClick={requestAllPermissions}
                                disabled={cameraStatus === 'pending' || locationStatus === 'pending'}
                            >
                                {(cameraStatus === 'pending' || locationStatus === 'pending') ? (
                                    <><Loader2 className="mr-2 h-4 w-4 animate-spin" /> Requesting Permissions...</>
                                ) : (
                                    'Grant All Permissions & Continue'
                                )}
                            </Button>
                        )}
                    </div>
                )}

                {/* Verification step — shown after permissions granted */}
                {permissionsGranted && !skipFaceVerification && (
                    <div className="grid gap-4 sm:grid-cols-2">
                        <div className="space-y-2">
                            <p className="text-sm font-medium">Camera</p>
                            <div className="aspect-video w-full overflow-hidden rounded-md border bg-muted">
                                <video
                                    ref={videoRef}
                                    className="h-full w-full object-cover"
                                    muted
                                    playsInline
                                />
                            </div>
                            {/* Debug calibration panel — shows real distance scores */}
                            {debugInfo && (
                                <div className={`rounded-md border p-2 text-xs font-mono space-y-1 ${
                                    debugInfo.passed
                                        ? 'border-green-500 bg-green-50 dark:bg-green-950/30 text-green-800 dark:text-green-300'
                                        : 'border-red-400 bg-red-50 dark:bg-red-950/30 text-red-800 dark:text-red-300'
                                }`}>
                                    <div className="font-semibold">
                                        {debugInfo.passed ? '✓ MATCH' : '✗ NO MATCH'} — threshold: {FACE_DISTANCE_THRESHOLD}
                                    </div>
                                    {debugInfo.frames.map((d, i) => (
                                        <div key={i}>
                                            Frame {i + 1}: {d.toFixed(4)}&nbsp;
                                            <span className={d <= FACE_DISTANCE_THRESHOLD ? 'text-green-600' : 'text-red-500'}>
                                                ({d <= FACE_DISTANCE_THRESHOLD ? 'pass' : 'fail'})
                                            </span>
                                        </div>
                                    ))}
                                    <div className="border-t pt-1 font-semibold">
                                        Avg: {debugInfo.avg.toFixed(4)}
                                    </div>
                                </div>
                            )}
                        </div>

                        <div className="space-y-2">
                            <p className="text-sm font-medium">Profile photo</p>
                            <div className="flex aspect-video w-full items-center justify-center overflow-hidden rounded-md border bg-muted">
                                {userPhotoUrl ? (
                                    <img
                                        src={userPhotoUrl}
                                        alt="Profile"
                                        className="h-full w-full object-cover"
                                    />
                                ) : (
                                    <span className="text-sm text-muted-foreground">
                                        No photo uploaded
                                    </span>
                                )}
                            </div>
                            <div className="space-y-1 text-xs text-muted-foreground">
                                {modelsLoading && <div className="flex items-center gap-1.5"><Loader2 className="h-3 w-3 animate-spin" /> Loading face models...</div>}
                                {locationLoading && <div className="flex items-center gap-1.5"><Loader2 className="h-3 w-3 animate-spin" /> Fetching location...</div>}
                                {cameraReady && <div className="flex items-center gap-1.5"><CheckCircle2 className="h-3 w-3 text-green-600 dark:text-green-400" /> Camera ready</div>}
                                {locationData && <div className="flex items-center gap-1.5"><CheckCircle2 className="h-3 w-3 text-green-600 dark:text-green-400" /> Location acquired</div>}
                                {modelsReady && <div className="flex items-center gap-1.5"><CheckCircle2 className="h-3 w-3 text-green-600 dark:text-green-400" /> Face models ready</div>}
                            </div>
                        </div>
                    </div>
                )}

                {/* Location details display */}
                {permissionsGranted && locationData && (
                    <div className="rounded-lg border bg-muted/30 p-3 space-y-2">
                        <div className="flex items-center gap-2 text-sm font-medium">
                            <MapPin className="h-4 w-4 text-primary" />
                            Location Details
                        </div>
                        <div className="grid gap-2 text-xs">
                            <div className="grid grid-cols-2 gap-2">
                                <div>
                                    <span className="text-muted-foreground">GPS Coordinates:</span>
                                    <p className="font-mono mt-0.5">
                                        {locationData.geo.lat.toFixed(6)}, {locationData.geo.lng.toFixed(6)}
                                    </p>
                                </div>
                                {locationData.geo.accuracy && (
                                    <div>
                                        <span className="text-muted-foreground">Accuracy:</span>
                                        <p className="mt-0.5">±{Math.round(locationData.geo.accuracy)}m</p>
                                    </div>
                                )}
                            </div>
                            {locationData.ip.ip && locationData.ip.ip !== 'unknown' && (
                                <div className="grid grid-cols-2 gap-2">
                                    <div>
                                        <span className="text-muted-foreground">IP Address:</span>
                                        <p className="font-mono mt-0.5">{locationData.ip.ip}</p>
                                    </div>
                                    {(locationData.ip.city || locationData.ip.region || locationData.ip.country) && (
                                        <div>
                                            <span className="text-muted-foreground">Location:</span>
                                            <p className="mt-0.5">
                                                {[locationData.ip.city, locationData.ip.region, locationData.ip.country]
                                                    .filter(Boolean)
                                                    .join(', ')}
                                            </p>
                                        </div>
                                    )}
                                </div>
                            )}
                        </div>
                    </div>
                )}

                {error && (
                    <div className="rounded-md border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive">
                        {error}
                    </div>
                )}

                <DialogFooter>
                    <Button
                        type="button"
                        variant="outline"
                        onClick={() => onOpenChange(false)}
                        disabled={busy}
                    >
                        Cancel
                    </Button>
                    {permissionsGranted && (
                        <Button
                            type="button"
                            onClick={skipFaceVerification ? handleClockInWithoutFace : handleVerify}
                            disabled={!isReady || busy || (!skipFaceVerification && (modelsLoading || locationLoading))}
                        >
                            {busy && <Spinner size="sm" className="mr-2" />}
                            {busy ? 'Clocking In...' : skipFaceVerification ? 'Clock In' : 'Verify & Clock In'}
                        </Button>
                    )}
                </DialogFooter>

                <canvas ref={canvasRef} className="hidden" />
            </DialogContent>
        </Dialog>
    );
}
