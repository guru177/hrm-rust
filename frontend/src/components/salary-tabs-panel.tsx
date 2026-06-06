import axios from '@/lib/axios';
import { useEffect, useState } from 'react';

import { CtcSalaryPanel } from '@/components/ctc-salary-panel';
import { SalaryStructurePanel } from '@/components/salary-structure-panel';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';

/** CTC + Manual salary tabs. Manual is view-only while CTC is active; remove CTC to edit. */
export function SalaryTabsPanel({ userId }: { userId: number }) {
    const [hasCtc, setHasCtc] = useState(false);

    useEffect(() => {
        axios
            .get<{ data?: { profile?: { yearly_ctc?: number } } }>(`/admin/users/${userId}/ctc-profile`)
            .then((res) => {
                const yc = res.data?.data?.profile?.yearly_ctc ?? 0;
                setHasCtc(yc > 0);
            })
            .catch(() => setHasCtc(false));
    }, [userId]);

    return (
        <Tabs defaultValue="ctc">
            <TabsList className="grid w-full grid-cols-2 mb-4">
                <TabsTrigger value="ctc">CTC Split</TabsTrigger>
                <TabsTrigger value="manual">Manual Components</TabsTrigger>
            </TabsList>
            <TabsContent value="ctc">
                <CtcSalaryPanel userId={userId} onCtcChange={setHasCtc} />
            </TabsContent>
            <TabsContent value="manual">
                <SalaryStructurePanel userId={userId} hasCtc={hasCtc} onCtcChange={setHasCtc} />
            </TabsContent>
        </Tabs>
    );
}
