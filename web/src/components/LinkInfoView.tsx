import React from "react";
import { Badge, Box, Card, DataList, Flex, Spinner, Text } from "@radix-ui/themes";

type LinkMetricsQuery = {
  link_id: number;
  total_hits: number;
  hits_today: number;
  hits_last_30_days: number;
  last_access: string | null;
};

type LinkInfoResponse = {
  owned: boolean;
  protected: boolean;
  data: LinkMetricsQuery | null;
};

function formatDate(iso: string | null): string {
  if (!iso) return "No data";
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return iso;
  return d.toLocaleString();
}

export function LinkInfoView({ alias }: { alias: string }) {
  const [loading, setLoading] = React.useState(true);
  const [error, setError] = React.useState<string | null>(null);
  const [info, setInfo] = React.useState<LinkInfoResponse | null>(null);

  React.useEffect(() => {
    let cancelled = false;

    async function run() {
      setLoading(true);
      setError(null);

      try {
        const res = await fetch(`/api/info/${encodeURIComponent(alias)}`, {
          method: "GET",
          credentials: "include",
          headers: { Accept: "application/json" },
        });

        if (!res.ok) {
          const text = await res.text().catch(() => "");
          throw new Error(text || `Request failed: ${res.status}`);
        }

        const json = (await res.json()) as LinkInfoResponse;
        if (!cancelled) setInfo(json);
      } catch (e) {
        if (!cancelled) setError(e instanceof Error ? e.message : "Unknown error");
      } finally {
        if (!cancelled) setLoading(false);
      }
    }

    run();
    return () => {
      cancelled = true;
    };
  }, [alias]);

  return (
    <Flex align="center" justify="center" height="90vh" direction="column" gap="4">
      <Box style={{ width: "min(720px, 92vw)" }}>
        <Card>
          <Flex direction="column" gap="3">
            <Flex align="center" justify="start">
              <Text size="4" weight="bold">
                Link info
              </Text>
              <Text color="gray">/{alias}</Text>
            </Flex>

            {loading ? (
              <Flex align="center" gap="2">
                <Spinner />
              </Flex>
            ) : error ? (
              <Text color="red">{error}</Text>
            ) : !info ? (
              <Text color="gray">No data</Text>
            ) : (
              <DataList.Root>
                <DataList.Item>
                  <DataList.Label>Status</DataList.Label>
                  <DataList.Value>
                    {info.owned ? (
                      <Badge color="green" variant="soft" radius="full">
                        Belongs to you
                      </Badge>
                    ) : (
                      <Badge color="red" variant="soft" radius="full">
                        Belongs to someone else
                      </Badge>
                    )}
                    {info.protected && (
                      <Badge color="red">Protected</Badge>
                    )}
                  </DataList.Value>
                </DataList.Item>

                {(info.owned && info.data) && (
                  <>
                    <DataList.Item>
                      <DataList.Label>Hits (total)</DataList.Label>
                      <DataList.Value>{info.data.total_hits}</DataList.Value>
                    </DataList.Item>

                    <DataList.Item>
                      <DataList.Label>Hits (today)</DataList.Label>
                      <DataList.Value>{info.data.hits_today}</DataList.Value>
                    </DataList.Item>

                    <DataList.Item>
                      <DataList.Label>Hits (30d)</DataList.Label>
                      <DataList.Value>{info.data.hits_last_30_days}</DataList.Value>
                    </DataList.Item>

                    <DataList.Item>
                      <DataList.Label>Last accessed</DataList.Label>
                      <DataList.Value>{formatDate(info.data.last_access)}</DataList.Value>
                    </DataList.Item>
                  </>
                )}
              </DataList.Root>
            )}
          </Flex>
        </Card>
      </Box>
    </Flex>
  );
}